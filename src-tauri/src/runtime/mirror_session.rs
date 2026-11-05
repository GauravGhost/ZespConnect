use std::time::{Duration, Instant};

use tauri::ipc::{Channel, Response};
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::process::Child;
use tokio::sync::mpsc;

use crate::error::{AppError, Result};
use crate::models::{KeyAction, MirrorKind, MirrorSession, MirrorSessionSpec};
use crate::runtime::recording::{VideoPacket, VideoTap};
use crate::runtime::server_link::{self, ServerLink, Teardown};
use crate::runtime::{mirror, phone_audio, session_window};
use crate::services::{apps, control, scrcpy, scrcpy_server};
use crate::state::AppState;

const EVENT_SESSION_STARTED: &str = "mirror:session:started";
const EVENT_SESSION_EXITED: &str = "mirror:session:exited";
const EVENT_SESSION_LOCKED: &str = "mirror:session:locked";
const LOCKED_POLL: Duration = Duration::from_secs(1);
const UNLOCKED_POLL: Duration = Duration::from_secs(2);
const KEYCODE_HOME: u32 = 3;
const START_APP_ATTEMPTS: usize = 6;
const START_APP_RETRY: Duration = Duration::from_millis(900);

/// How long Back gets to close an app before it is launched again (see `reopen_after_back`).
const REOPEN_AFTER_BACK: Duration = Duration::from_millis(600);

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct LockState {
    id: u64,
    locked: bool,
    phone_screen: bool,
}
const READ_BUFFER: usize = 64 * 1024;
// A sane upper bound on a single video packet (guards against a corrupt header).
const MAX_PACKET: usize = 32 * 1024 * 1024;

/// The 12-byte per-packet frame header: 3 flag bits + 61-bit PTS + `u32` size.
pub(crate) const FRAME_HEADER: usize = 12;
const SESSION_FLAG: u64 = 0x8000_0000_0000_0000;
const CONFIG_FLAG: u64 = 0x4000_0000_0000_0000;
const KEY_FLAG: u64 = 0x2000_0000_0000_0000;
const PTS_MASK: u64 = 0x1FFF_FFFF_FFFF_FFFF;

// What one header on the video socket describes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Packet {
    Session,
    Media {
        config: bool,
        key: bool,
        pts: u64,
        len: usize,
    },
}

pub(crate) fn parse_header(bytes: &[u8; FRAME_HEADER]) -> Packet {
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
    let server = server_link::resolve(app)?;
    let adb_bin = server.adb_bin.clone();
    let scid = scrcpy_server::scid();

    let (mut options, virtual_display) = session_options(&adb_bin, &spec).await?;
    let kind = options.kind;

    // Camera controls and resizable displays arrived together in scrcpy 4.0.
    let extended_controls = scrcpy::flex_display_supported(Some(&server.version));
    // A virtual display then follows its window: sized to it now, resized with it after.
    let flex = if extended_controls && options.new_display.is_some() {
        let phone_dpi = if kind == MirrorKind::Desktop {
            None
        } else {
            scrcpy::phone_density(&adb_bin, &spec.serial).await
        };
        session_window::flex_display(app, id, kind, phone_dpi)
    } else {
        None
    };
    let display_scale = flex.as_ref().map(|display| display.pixels_per_point);
    if let Some(display) = flex {
        options.new_display = Some(display.spec);
        options.flex_display = true;
        options.max_size = scrcpy::MAX_DESKTOP_SIDE as u32;
    }

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
    // ("No known display id"), so it is resent until the stream is live (see `resend_start_app`).
    let start_app = if virtual_display {
        spec.package
            .as_deref()
            .map(control::start_app)
            .transpose()?
    } else {
        None
    };

    let options = scrcpy_server::options_args(&server.version, &scid, &options);
    // The server opens video, then control; both must be accepted or it blocks before streaming.
    let ServerLink {
        child,
        streams,
        teardown,
    } = server_link::open(app, &server, &spec.serial, &scid, options, 2).await?;
    let mut streams = streams.into_iter();
    let (Some(video), Some(control_stream)) = (streams.next(), streams.next()) else {
        return Err(AppError::Other("The scrcpy server did not connect".into()));
    };
    let control_channel = spawn_control(control_stream);

    let info = MirrorSession {
        id,
        serial: spec.serial.clone(),
        kind,
        package: spec.package.clone(),
        label: Some(mirror::label_or(&spec.label, &spec.serial)),
        started: crate::util::time::unix_now(),
        phone_screen: screen_was_on,
        flex_display: display_scale.is_some(),
        display_scale: display_scale.unwrap_or(1.0),
        camera_controls: extended_controls && kind == MirrorKind::Camera,
        torch: spec.options.camera_torch,
    };

    // Seed the toggle before `watch_lock`, which confirms it on its first check.
    app.state::<AppState>().set_phone_screen(id, screen_was_on);
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
    let (tap_tx, tap_rx) = tokio::sync::watch::channel(None);
    app.state::<AppState>()
        .set_session(info.clone(), stop_tx, control_channel.clone(), tap_tx);

    // Tell every window the session is live so the app list can flip its button to "close".
    let _ = app.emit(EVENT_SESSION_STARTED, &info);

    // The window's Audio option routes the phone's sound here; it stays a per-device choice after.
    if spec.options.audio {
        let handle = app.clone();
        let serial = spec.serial.clone();
        tauri::async_runtime::spawn(async move {
            let _ = phone_audio::start(&handle, &serial).await;
        });
    }

    if let Some(message) = start_app {
        tauri::async_runtime::spawn(resend_start_app(
            app.clone(),
            id,
            control_channel.clone(),
            message,
        ));
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

    tauri::async_runtime::spawn(supervise(
        app.clone(),
        info.clone(),
        video,
        VideoOut {
            channel,
            tap: tap_rx,
        },
        child,
        stop_rx,
        teardown,
    ));

    Ok(info)
}

/// Back on an app's first screen closes it and leaves its virtual display black, so the app is
/// launched again once Back has run. An app that is still open just stays as it is.
pub fn reopen_after_back(control: ControlChannel, package: &str) -> Result<()> {
    let message = control::start_app(package)?;
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(REOPEN_AFTER_BACK).await;
        let _ = control.send(message);
    });
    Ok(())
}

/// Resend START_APP until the app is drawing (stream live) or the session ends.
async fn resend_start_app(app: AppHandle, id: u64, control: ControlChannel, message: Vec<u8>) {
    for _ in 0..START_APP_ATTEMPTS {
        if app.state::<AppState>().is_streaming(id)
            || app.state::<AppState>().session_control(id).is_none()
        {
            break;
        }
        let _ = control.send(message.clone());
        tokio::time::sleep(START_APP_RETRY).await;
    }
}

/// Relay the video until the session ends, then tear it down, leave the phone as the user had it,
/// and announce the exit.
async fn supervise(
    app: AppHandle,
    info: MirrorSession,
    video: TcpStream,
    out: VideoOut,
    mut child: Child,
    stop: tokio::sync::oneshot::Receiver<()>,
    teardown: Teardown,
) {
    let id = info.id;
    stream_video(app.clone(), id, video, out, &mut child, stop).await;
    // Read before `remove_session` forgets it.
    let phone_screen_on = app.state::<AppState>().phone_screen_on(id);
    let _ = child.kill().await;
    teardown.run().await;
    if info.kind == MirrorKind::Desktop {
        finish_desktop(teardown.adb_bin(), &info.serial, phone_screen_on).await;
    }
    app.state::<AppState>().remove_session(id);
    let _ = app.emit(EVENT_SESSION_EXITED, &info);
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
            if let Some(now) = probe.locked.filter(|now| locked != Some(*now)) {
                on_lock_change(&app, id, &control, locked.is_none(), now);
                locked = Some(now);
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

/// Match the phone screen to a new lock state and tell the window.
fn on_lock_change(
    app: &AppHandle,
    id: u64,
    control: &ControlChannel,
    first_check: bool,
    locked: bool,
) {
    let state = app.state::<AppState>();
    if first_check && locked {
        state.set_phone_screen(id, false);
    }
    if !locked {
        if !state.phone_screen_on(id) {
            let _ = control.send(control::set_display_power(false));
        }
        // Only once capture is up: RESET_VIDEO before that throws and kills the session.
        if state.is_streaming(id) {
            let _ = control.send(control::reset_video());
        }
    }
    let _ = app.emit(
        EVENT_SESSION_LOCKED,
        LockState {
            id,
            locked,
            phone_screen: state.phone_screen_on(id),
        },
    );
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
/// Also returns whether the session uses a virtual display.
async fn session_options(
    adb_bin: &str,
    spec: &MirrorSessionSpec,
) -> Result<(scrcpy_server::ServerOptions, bool)> {
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
        flex_display: false,
        // The desktop is already sized to the screen; 0 tells scrcpy not to cap it.
        max_size: if desktop {
            0
        } else {
            scrcpy::max_size(&spec.options).unwrap_or(scrcpy::DEFAULT_MAX_SIZE)
        },
        max_fps: scrcpy::max_fps(&spec.options),
        // The UI sends a phone-sized default; a desktop raises it to its own floor for a crisp screen.
        bit_rate: scrcpy::bit_rate(&spec.options)
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

    Ok((options, virtual_display))
}

/// Where a session's video goes: its window, and a recording while one runs.
struct VideoOut {
    channel: Channel<Response>,
    tap: tokio::sync::watch::Receiver<Option<VideoTap>>,
}

/// Read the video socket and forward each access unit until the stream ends or the session stops.
async fn stream_video(
    app: AppHandle,
    id: u64,
    video: TcpStream,
    out: VideoOut,
    child: &mut Child,
    mut stop: tokio::sync::oneshot::Receiver<()>,
) {
    let VideoOut { channel, tap } = out;
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
            if let Some(tap) = tap.borrow().as_ref() {
                tap.offer(VideoPacket {
                    arrived: Instant::now(),
                    config,
                    key,
                    pts,
                    data: payload.clone(),
                });
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
