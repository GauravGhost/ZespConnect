//! In-app mirror sessions: we run the scrcpy server and relay its H.264 output to a Tauri
//! window over an IPC channel, using a reverse adb tunnel (scrcpy's default) so the window
//! belongs to Zesp Connect instead of being a separate scrcpy OS window.

use std::path::Path;
use std::time::Duration;

use tauri::ipc::{Channel, Response};
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::process::Child;
use tokio::sync::mpsc;

use crate::error::{AppError, Result};
use crate::models::{KeyAction, MirrorKind, MirrorSession, MirrorSessionSpec};
use crate::services::{adb, apps, control, scrcpy, scrcpy_server};
use crate::state::AppState;
use crate::util::{self, process, validation};

const EVENT_SESSION_STARTED: &str = "mirror:session:started";
const EVENT_SESSION_EXITED: &str = "mirror:session:exited";
const EVENT_SESSION_LOCKED: &str = "mirror:session:locked";
const LOCKED_POLL: Duration = Duration::from_secs(1);
const UNLOCKED_POLL: Duration = Duration::from_secs(2);
const KEYCODE_HOME: u32 = 3;
const START_APP_ATTEMPTS: usize = 6;
const START_APP_RETRY: Duration = Duration::from_millis(900);

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct LockState {
    id: u64,
    locked: bool,
    phone_screen: bool,
}
// How long to wait for the server to connect back before giving up.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(12);
// How often to look for a connection while also watching for a dead server.
const ACCEPT_POLL: Duration = Duration::from_millis(200);
const READ_BUFFER: usize = 64 * 1024;
// A sane upper bound on a single video packet (guards against a corrupt header).
const MAX_PACKET: usize = 32 * 1024 * 1024;

/// The 12-byte per-packet frame header: 3 flag bits + 61-bit PTS + `u32` size.
const FRAME_HEADER: usize = 12;
const SESSION_FLAG: u64 = 0x8000_0000_0000_0000;
const CONFIG_FLAG: u64 = 0x4000_0000_0000_0000;
const KEY_FLAG: u64 = 0x2000_0000_0000_0000;
const PTS_MASK: u64 = 0x1FFF_FFFF_FFFF_FFFF;

// What one header on the video socket describes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Packet {
    Session,
    Media {
        config: bool,
        key: bool,
        pts: u64,
        len: usize,
    },
}

fn parse_header(bytes: &[u8; FRAME_HEADER]) -> Packet {
    let flags = u64::from_be_bytes(bytes[0..8].try_into().unwrap_or([0u8; 8]));
    if flags & SESSION_FLAG != 0 {
        return Packet::Session;
    }
    Packet::Media {
        config: flags & CONFIG_FLAG != 0,
        key: flags & KEY_FLAG != 0,
        pts: flags & PTS_MASK,
        len: u32::from_be_bytes(bytes[8..12].try_into().unwrap_or([0u8; 4])) as usize,
    }
}

/// The message forwarded to the webview: `[flags][pts][payload]` (bit 0 = key frame, bit 1 = config).
fn encode_message(config: bool, key: bool, pts: u64, payload: &[u8]) -> Vec<u8> {
    let mut flags = 0u8;
    if key {
        flags |= 1;
    }
    if config {
        flags |= 2;
    }

    let mut out = Vec::with_capacity(9 + payload.len());
    out.push(flags);
    out.extend_from_slice(&pts.to_be_bytes());
    out.extend_from_slice(payload);
    out
}

/// Write-only handle to a session's control socket; messages funnel through one writer task, so clones stay ordered.
#[derive(Clone)]
pub struct ControlChannel {
    sender: mpsc::UnboundedSender<Vec<u8>>,
}

impl ControlChannel {
    pub fn send(&self, message: Vec<u8>) -> Result<()> {
        self.sender
            .send(message)
            .map_err(|_| AppError::Other("The mirror session has ended".into()))
    }
}

fn spawn_control(mut stream: TcpStream) -> ControlChannel {
    let (sender, mut receiver) = mpsc::unbounded_channel::<Vec<u8>>();
    tauri::async_runtime::spawn(async move {
        while let Some(message) = receiver.recv().await {
            if stream.write_all(&message).await.is_err() {
                break;
            }
        }
    });
    ControlChannel { sender }
}

/// Start a session and return once the server is streaming; the stream continues in a background task.
pub async fn start(
    app: &AppHandle,
    id: u64,
    spec: MirrorSessionSpec,
    channel: Channel<Response>,
) -> Result<MirrorSession> {
    let tools = app.state::<AppState>().tools();
    let adb_bin = adb::adb_path(&tools)?;
    let scrcpy_bin = scrcpy::scrcpy_path(&tools)?;

    let version = tools
        .scrcpy
        .version
        .clone()
        .filter(|value| scrcpy_server::supported(Some(value)))
        .ok_or_else(|| {
            AppError::Other("This scrcpy version cannot be embedded; using a window instead".into())
        })?;

    let server = scrcpy_server::server_path(&scrcpy_bin)?;
    let scid = scrcpy_server::scid();
    let socket = scrcpy_server::socket_name(&scid);
    let remote = scrcpy_server::remote_jar(&scid);

    let (options, kind, virtual_display) = session_options(&adb_bin, &spec).await?;

    // Desktop mode wakes the phone so apps draw; its screen is switched off once streaming (see `watch_lock`).
    let screen_was_on = if kind == MirrorKind::Desktop {
        scrcpy::prepare_desktop(&adb_bin, &spec.serial).await?
    } else {
        false
    };

    // On a virtual display the app is launched over the control socket; where that is unsafe we mirror the main one.
    if kind == MirrorKind::ScreenApp {
        if let Some(package) = spec.package.as_deref() {
            apps::launch(&tools, &spec.serial, package).await?;
        }
    }

    // An app only produces video once launched, and the server drops an early START_APP
    // ("No known display id"), so it is resent below until the stream is live.
    let start_app = if virtual_display {
        spec.package
            .as_deref()
            .map(control::start_app)
            .transpose()?
    } else {
        None
    };

    let listener = TcpListener::bind(("127.0.0.1", 0))
        .await
        .map_err(|error| AppError::Io(error.to_string()))?;
    let port = listener
        .local_addr()
        .map_err(|error| AppError::Io(error.to_string()))?
        .port();

    let log_dir = util::ensure_dir(app.state::<AppState>().app_data_dir().join("mirror-logs"))?;
    let log_path = log_dir.join(format!("{scid}.log"));
    let args = scrcpy_server::launch_args(&spec.serial, &remote, &version, &scid, &options);

    adb::push(
        &adb_bin,
        &spec.serial,
        &server.to_string_lossy(),
        &remote,
        adb::LONG_TIMEOUT,
    )
    .await?;
    adb::reverse(
        &adb_bin,
        &spec.serial,
        &format!("localabstract:{socket}"),
        port,
        adb::DEFAULT_TIMEOUT,
    )
    .await?;

    let mut child = process::spawn_captured(&adb_bin, &args, &log_path, &[])?;

    // The server opens video, then (audio), then control; accept both or it blocks before streaming.
    let streams = match accept_streams(&listener, &mut child, &log_path).await {
        Ok(streams) => streams,
        Err(error) => {
            let _ = child.kill().await;
            teardown(&adb_bin, &spec.serial, &socket, &remote).await;
            // Keep the log: startup failures are exactly when it matters.
            return Err(error);
        }
    };
    let (video, control_stream) = streams;
    let control_channel = spawn_control(control_stream);

    let label = if spec.label.trim().is_empty() {
        spec.serial.clone()
    } else {
        spec.label.clone()
    };
    let info = MirrorSession {
        id,
        serial: spec.serial.clone(),
        kind,
        package: spec.package.clone(),
        label: Some(label),
        started: crate::util::time::unix_now(),
        phone_screen: screen_was_on,
    };

    // Seed the toggle before `watch_lock`, which confirms it on its first check.
    app.state::<AppState>().set_phone_screen(id, screen_was_on);
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
    app.state::<AppState>()
        .set_session(info.clone(), stop_tx, control_channel.clone());

    // Tell every window the session is live so the app list can flip its button to "close".
    let _ = app.emit(EVENT_SESSION_STARTED, &info);

    // Resend START_APP until the app is drawing (stream live) or the session ends.
    if let Some(message) = start_app {
        let control = control_channel.clone();
        let handle = app.clone();
        tauri::async_runtime::spawn(async move {
            for _ in 0..START_APP_ATTEMPTS {
                if handle.state::<AppState>().is_streaming(id)
                    || handle.state::<AppState>().session_control(id).is_none()
                {
                    break;
                }
                let _ = control.send(message.clone());
                tokio::time::sleep(START_APP_RETRY).await;
            }
        });
    }

    // Desktop mode only shows once unlocked; watch the lock screen (see `watch_lock`).
    if kind == MirrorKind::Desktop {
        tauri::async_runtime::spawn(watch_lock(
            app.clone(),
            id,
            adb_bin.clone(),
            spec.serial.clone(),
            control_channel.clone(),
        ));
    }

    let handle = app.clone();
    let task_info = info.clone();
    let task_serial = spec.serial.clone();
    tauri::async_runtime::spawn(async move {
        stream_video(handle.clone(), id, video, channel, &mut child, stop_rx).await;
        // Read before `remove_session` forgets it.
        let phone_screen_on = handle.state::<AppState>().phone_screen_on(id);
        let _ = child.kill().await;
        teardown(&adb_bin, &task_serial, &socket, &remote).await;
        if kind == MirrorKind::Desktop {
            finish_desktop(&adb_bin, &task_serial, phone_screen_on).await;
        }
        handle.state::<AppState>().remove_session(id);
        let _ = std::fs::remove_file(&log_path);
        let _ = handle.emit(EVENT_SESSION_EXITED, &task_info);
    });

    Ok(info)
}

/// Follow a desktop session's lock screen until it ends. Android hides the desktop while the
/// phone is locked, so the phone screen is switched to match and a fresh key frame requested on
/// unlock (see the seeded toggle for sessions opened while the phone was in use). Presses HOME
/// when the launcher loses input focus twice in a row, which also dismisses a "not responding"
/// dialog.
async fn watch_lock(
    app: AppHandle,
    id: u64,
    adb_bin: String,
    serial: String,
    control: ControlChannel,
) {
    let mut locked: Option<bool> = None;
    let mut unfocused = false;
    while app.state::<AppState>().session_control(id).is_some() {
        if let Some(probe) = scrcpy::desktop_probe(&adb_bin, &serial).await {
            if let Some(now) = probe.locked {
                if locked != Some(now) {
                    if locked.is_none() && now {
                        app.state::<AppState>().set_phone_screen(id, false);
                    }
                    if !now {
                        if !app.state::<AppState>().phone_screen_on(id) {
                            let _ = control.send(control::set_display_power(false));
                        }
                        // Only once capture is up: RESET_VIDEO before that throws and kills the session.
                        if app.state::<AppState>().is_streaming(id) {
                            let _ = control.send(control::reset_video());
                        }
                    }
                    locked = Some(now);
                    let phone_screen = app.state::<AppState>().phone_screen_on(id);
                    let _ = app.emit(
                        EVENT_SESSION_LOCKED,
                        LockState {
                            id,
                            locked: now,
                            phone_screen,
                        },
                    );
                }
            }
            let stuck = probe.launcher_unfocused && locked == Some(false);
            if stuck && unfocused {
                let _ = control.send(control::inject_keycode(KeyAction::Down, KEYCODE_HOME, 0, 0));
                let _ = control.send(control::inject_keycode(KeyAction::Up, KEYCODE_HOME, 0, 0));
                unfocused = false;
            } else {
                unfocused = stuck;
            }
        }
        let wait = if locked == Some(true) {
            LOCKED_POLL
        } else {
            UNLOCKED_POLL
        };
        tokio::time::sleep(wait).await;
    }
}

/// Leave the phone as the user had it: if its screen was on it stays on, otherwise it is put to sleep.
async fn finish_desktop(adb_bin: &str, serial: &str, phone_screen_on: bool) {
    if !phone_screen_on {
        scrcpy::sleep_phone(adb_bin, serial).await;
    }
}

pub fn stop(app: &AppHandle, id: u64) {
    let state = app.state::<AppState>();
    if let Some(handle) = state.take_session(id) {
        // The supervisor clears the rest once it tears down; clearing here would lose the phone-screen choice.
        let _ = handle.stop.send(());
    } else {
        // Drop a spec that never became a session (window closed before starting).
        state.remove_session(id);
    }
}

pub fn stop_all(app: &AppHandle) {
    for handle in app.state::<AppState>().drain_sessions() {
        let _ = handle.stop.send(());
    }
}

/// Translate the requested mirror into server options, downgrading an app window where a virtual display is unsafe.
async fn session_options(
    adb_bin: &str,
    spec: &MirrorSessionSpec,
) -> Result<(scrcpy_server::ServerOptions, MirrorKind, bool)> {
    let virtual_display = match spec.kind {
        MirrorKind::App => {
            scrcpy::virtual_display_safe(scrcpy::android_api(adb_bin, &spec.serial).await)
        }
        _ => false,
    };
    let kind = if spec.kind == MirrorKind::App && !virtual_display {
        MirrorKind::ScreenApp
    } else {
        spec.kind
    };

    let desktop = kind == MirrorKind::Desktop;
    let new_display = if desktop {
        Some(
            spec.desktop_display
                .clone()
                .ok_or_else(|| AppError::Invalid("Desktop mode needs a display size".into()))?,
        )
    } else if virtual_display {
        Some(String::new())
    } else {
        None
    };

    let options = scrcpy_server::ServerOptions {
        kind,
        new_display,
        // The desktop is already sized to the screen; 0 tells scrcpy not to cap it.
        max_size: if desktop {
            0
        } else {
            spec.options
                .resolution
                .map(|value| value.clamp(scrcpy::MIN_RESOLUTION, scrcpy::MAX_RESOLUTION))
                .unwrap_or(scrcpy::DEFAULT_MAX_SIZE)
        },
        max_fps: spec
            .options
            .fps
            .map(|value| value.clamp(1, scrcpy::MAX_FPS_LIMIT)),
        // The UI sends a phone-sized default; a desktop raises it to its own floor for a crisp screen.
        bit_rate: spec
            .options
            .bitrate
            .map(|value| value.clamp(1, scrcpy::MAX_BITRATE))
            .map(|value| {
                if desktop {
                    value.max(scrcpy::DESKTOP_BIT_RATE)
                } else {
                    value
                }
            })
            .or(desktop.then_some(scrcpy::DESKTOP_BIT_RATE)),
        camera_facing: spec.options.camera_facing.clone(),
        camera_zoom: spec.options.camera_zoom,
        camera_torch: spec.options.camera_torch,
        // scrcpy's cleanup (which restores the screen timeout on close) only runs with `stay_awake`.
        stay_awake: spec.options.stay_awake || desktop,
    };

    Ok((options, kind, virtual_display))
}

/// Accept the video and control connections, watching the server so a bad version fails fast.
async fn accept_streams(
    listener: &TcpListener,
    child: &mut Child,
    log_path: &Path,
) -> Result<(TcpStream, TcpStream)> {
    let video = accept(listener, child, log_path).await?;
    let control = accept(listener, child, log_path).await?;
    Ok((video, control))
}

async fn accept(listener: &TcpListener, child: &mut Child, log_path: &Path) -> Result<TcpStream> {
    let deadline = tokio::time::Instant::now() + CONNECT_TIMEOUT;
    loop {
        if let Some(status) = child.try_wait()? {
            return Err(AppError::CommandFailed(failure_message(
                log_path,
                &format!("scrcpy server exited ({status})"),
            )));
        }

        match tokio::time::timeout(ACCEPT_POLL, listener.accept()).await {
            Ok(Ok((stream, _))) => {
                let _ = stream.set_nodelay(true);
                return Ok(stream);
            }
            Ok(Err(error)) => return Err(AppError::Io(error.to_string())),
            Err(_) if tokio::time::Instant::now() >= deadline => {
                return Err(AppError::CommandFailed(failure_message(
                    log_path,
                    "Timed out waiting for the scrcpy server",
                )));
            }
            Err(_) => {}
        }
    }
}

/// Read the video socket and forward each access unit until the stream ends or the session stops.
async fn stream_video(
    app: AppHandle,
    id: u64,
    video: TcpStream,
    channel: Channel<Response>,
    child: &mut Child,
    mut stop: tokio::sync::oneshot::Receiver<()>,
) {
    let mut reader = BufReader::with_capacity(READ_BUFFER, video);
    let mut payload: Vec<u8> = Vec::new();
    // The first forwarded packet means capture is up, which unlocks the control messages sent early.
    let mut announced = false;

    let reading = async {
        let mut header = [0u8; FRAME_HEADER];
        loop {
            if reader.read_exact(&mut header).await.is_err() {
                break;
            }

            let (config, key, pts, len) = match parse_header(&header) {
                Packet::Session => continue,
                Packet::Media {
                    config,
                    key,
                    pts,
                    len,
                } => (config, key, pts, len),
            };
            if len > MAX_PACKET {
                break;
            }

            payload.clear();
            payload.resize(len, 0);
            if reader.read_exact(&mut payload).await.is_err() {
                break;
            }
            if channel
                .send(Response::new(encode_message(config, key, pts, &payload)))
                .is_err()
            {
                break;
            }
            if !announced {
                announced = true;
                app.state::<AppState>().set_streaming(id);
            }
        }
    };

    tokio::select! {
        _ = reading => {}
        _ = &mut stop => {}
        _ = child.wait() => {}
    }
}

/// Remove the reverse tunnel and the pushed server from the device.
async fn teardown(adb_bin: &str, serial: &str, socket: &str, remote: &str) {
    let _ = adb::reverse_remove(
        adb_bin,
        serial,
        &format!("localabstract:{socket}"),
        adb::DEFAULT_TIMEOUT,
    )
    .await;
    let command = format!("rm -f {}", validation::shell_quote(remote));
    let _ = adb::shell(adb_bin, serial, &command, adb::DEFAULT_TIMEOUT).await;
}

fn failure_message(log_path: &Path, fallback: &str) -> String {
    let message = process::read_log_error(log_path);
    if message.is_empty() {
        fallback.to_string()
    } else {
        message
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(flags: u64, size: u32) -> [u8; FRAME_HEADER] {
        let mut bytes = [0u8; FRAME_HEADER];
        bytes[0..8].copy_from_slice(&flags.to_be_bytes());
        bytes[8..12].copy_from_slice(&size.to_be_bytes());
        bytes
    }

    #[test]
    fn parses_a_key_media_packet() {
        let packet = parse_header(&header(KEY_FLAG | 1234, 512));
        assert_eq!(
            packet,
            Packet::Media {
                config: false,
                key: true,
                pts: 1234,
                len: 512,
            }
        );
    }

    #[test]
    fn parses_a_config_packet_without_a_key_flag() {
        let packet = parse_header(&header(CONFIG_FLAG, 42));
        assert_eq!(
            packet,
            Packet::Media {
                config: true,
                key: false,
                pts: 0,
                len: 42,
            }
        );
    }

    #[test]
    fn recognises_session_packets() {
        // The size field is actually the video height here, so it must be ignored.
        assert_eq!(parse_header(&header(SESSION_FLAG, 2400)), Packet::Session);
    }

    #[test]
    fn encodes_the_flags_and_pts_before_the_payload() {
        let message = encode_message(true, true, 7, &[0xAB, 0xCD]);
        assert_eq!(message[0], 3);
        assert_eq!(&message[1..9], &7u64.to_be_bytes());
        assert_eq!(&message[9..], &[0xAB, 0xCD]);

        let delta = encode_message(false, false, 0, &[0x01]);
        assert_eq!(delta[0], 0);
        assert_eq!(delta.len(), 10);
    }
}
