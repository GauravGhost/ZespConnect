//! In-app mirror sessions.
//!
//! We run the scrcpy *server* ourselves (see `services::scrcpy_server`) and relay
//! its H.264 output to a Tauri window over an IPC channel, so the window belongs
//! to Zesp Connect instead of being a separate `scrcpy` OS window.
//!
//! The tunnel is a reverse adb tunnel (scrcpy's default): we listen on a loopback
//! port, `adb reverse` maps the device's abstract socket to it, and the server
//! connects back. That avoids the forward-mode dummy byte and the port race.

use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tauri::ipc::{Channel, Response};
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::process::Child;
use tokio::sync::mpsc;

use crate::error::{AppError, Result};
use crate::models::{MirrorKind, MirrorSession, MirrorSessionSpec};
use crate::services::{adb, apps, control, scrcpy, scrcpy_server};
use crate::state::AppState;
use crate::util::{self, process, validation};

const EVENT_SESSION_EXITED: &str = "mirror:session:exited";
/// How long to wait for the server to connect back before giving up.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(12);
/// How often to look for a connection while also watching for a dead server.
const ACCEPT_POLL: Duration = Duration::from_millis(200);
const READ_BUFFER: usize = 64 * 1024;
/// A sane upper bound on a single video packet (guards against a corrupt header).
const MAX_PACKET: usize = 32 * 1024 * 1024;

/// The 12-byte per-packet frame header: 3 flag bits + 61-bit PTS + `u32` size.
const FRAME_HEADER: usize = 12;
const SESSION_FLAG: u64 = 0x8000_0000_0000_0000;
const CONFIG_FLAG: u64 = 0x4000_0000_0000_0000;
const KEY_FLAG: u64 = 0x2000_0000_0000_0000;
const PTS_MASK: u64 = 0x1FFF_FFFF_FFFF_FFFF;

/// What one header on the video socket describes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Packet {
    /// A capture-session change (device rotation / resize); carries no payload.
    Session,
    /// An H.264 access unit.
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

/// The message forwarded to the webview: `[flags][pts][payload]`, where `flags`
/// bit 0 marks a key frame and bit 1 a codec-config packet.
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

/// Write-only handle to a session's control socket. Messages are queued to a
/// dedicated writer task so callers never touch the socket. Cloning the handle
/// keeps messages ordered: they all funnel through the one writer task.
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

/// Start a session and return once the server is streaming. The stream itself
/// continues in a background task until it ends or the session is stopped.
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

    let (options, kind, virtual_display) = session_options(&adb_bin, &spec).await;

    // On a virtual display the app is launched over the control socket. Where
    // that is unsafe (Android 12–13), foreground the app and mirror the main
    // display instead.
    if kind == MirrorKind::ScreenApp {
        if let Some(package) = spec.package.as_deref() {
            apps::launch(&tools, &spec.serial, package).await?;
        }
    }

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

    // The server opens the sockets in order: video, then (audio), then control.
    // Both must be accepted or the server blocks before it starts streaming.
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
        started: unix_now(),
    };

    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
    app.state::<AppState>()
        .set_session(info.clone(), stop_tx, control_channel.clone());

    let handle = app.clone();
    let task_info = info.clone();
    let task_serial = spec.serial.clone();
    tauri::async_runtime::spawn(async move {
        stream_video(
            video,
            channel,
            &mut child,
            stop_rx,
            control_channel,
            start_app,
        )
        .await;
        let _ = child.kill().await;
        teardown(&adb_bin, &task_serial, &socket, &remote).await;
        handle.state::<AppState>().remove_session(id);
        let _ = std::fs::remove_file(&log_path);
        let _ = handle.emit(EVENT_SESSION_EXITED, &task_info);
    });

    Ok(info)
}

/// Close one session (its supervisor also tears the server down).
pub fn stop(app: &AppHandle, id: u64) {
    let state = app.state::<AppState>();
    if let Some(handle) = state.take_session(id) {
        let _ = handle.stop.send(());
    }
    // Also drop a spec that never became a session (window closed before starting).
    state.remove_session(id);
}

/// Close every session.
pub fn stop_all(app: &AppHandle) {
    for handle in app.state::<AppState>().drain_sessions() {
        let _ = handle.stop.send(());
    }
}

/// Translate the requested mirror into the server's options, downgrading an app
/// window to a main-display mirror on Android versions where a virtual display is
/// unsafe (see `scrcpy::virtual_display_safe`).
async fn session_options(
    adb_bin: &str,
    spec: &MirrorSessionSpec,
) -> (scrcpy_server::ServerOptions, MirrorKind, bool) {
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

    let options = scrcpy_server::ServerOptions {
        kind,
        new_display: virtual_display,
        max_size: spec
            .options
            .resolution
            .map(|value| value.clamp(scrcpy::MIN_RESOLUTION, scrcpy::MAX_RESOLUTION))
            .unwrap_or(scrcpy::DEFAULT_MAX_SIZE),
        max_fps: spec
            .options
            .fps
            .map(|value| value.clamp(1, scrcpy::MAX_FPS_LIMIT)),
        bit_rate: spec
            .options
            .bitrate
            .map(|value| value.clamp(1, scrcpy::MAX_BITRATE)),
        camera_facing: spec.options.camera_facing.clone(),
        camera_zoom: spec.options.camera_zoom,
        camera_torch: spec.options.camera_torch,
        stay_awake: spec.options.stay_awake,
    };

    (options, kind, virtual_display)
}

/// Accept the video and control connections, watching the server so a bad
/// version or option fails fast with the server's own message instead of a
/// silent timeout.
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

/// Read the video socket and forward each access unit to the webview until the
/// stream ends, the server exits, or the session is stopped.
async fn stream_video(
    video: TcpStream,
    channel: Channel<Response>,
    child: &mut Child,
    mut stop: tokio::sync::oneshot::Receiver<()>,
    control_channel: ControlChannel,
    start_app: Option<Vec<u8>>,
) {
    if let Some(message) = start_app {
        let _ = control_channel.send(message);
    }

    let mut reader = BufReader::with_capacity(READ_BUFFER, video);
    let mut payload: Vec<u8> = Vec::new();

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

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
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
