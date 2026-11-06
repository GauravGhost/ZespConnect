use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncReadExt, BufReader};
use tokio::net::TcpStream;
use tokio::process::Child;

use crate::error::{AppError, Result};
use crate::models::SoundRoute;
use crate::runtime::mirror_session::{parse_header, Packet, FRAME_HEADER};
use crate::runtime::server_link::{self, ServerLink};
use crate::services::audio_out::{Buffering, Player};
use crate::services::scrcpy_server::{self, AudioStart};
use crate::state::AppState;

pub const EVENT_CHANGED: &str = "phone-audio:changed";
/// How long the server may take to report whether capture started.
const START_TIMEOUT: Duration = Duration::from_secs(8);
/// Raw packets are at most 1024 frames; anything far larger is a corrupt header.
const MAX_PACKET: usize = 1 << 20;
/// How long a stopped link may take to let go of its device before a restart gives up.
const RESTART_WAIT: Duration = Duration::from_secs(3);
const RESTART_POLL: Duration = Duration::from_millis(50);

/// How the phone's sound is captured and played; the default is the normal phone sound switch.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AudioSettings {
    /// The phone keeps playing its sound too (Android 13+).
    pub keep_on_phone: bool,
    pub low_latency: bool,
    /// An output device name; `None` is the system default.
    pub device: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Changed<'a> {
    serial: &'a str,
    on_computer: bool,
}

fn announce(app: &AppHandle, serial: &str, on_computer: bool) {
    let _ = app.emit(
        EVENT_CHANGED,
        Changed {
            serial,
            on_computer,
        },
    );
    crate::tray::sync(app);
}

/// Play the device's sound on the computer; returns once sound is flowing.
pub async fn start(app: &AppHandle, serial: &str) -> Result<()> {
    start_with(app, serial, &AudioSettings::default()).await
}

/// Play the device's sound on `route`, restarting a running link that plays elsewhere.
pub async fn set_route(app: &AppHandle, serial: &str, route: SoundRoute) -> Result<()> {
    if app.state::<AppState>().phone_audio_route(serial) == route {
        return Ok(());
    }
    if route == SoundRoute::Phone {
        stop(app, serial);
        return Ok(());
    }
    let settings = AudioSettings {
        keep_on_phone: route == SoundRoute::Both,
        ..AudioSettings::default()
    };
    restart_with(app, serial, &settings).await
}

/// Replace any running link with one using `settings`.
pub async fn restart_with(app: &AppHandle, serial: &str, settings: &AudioSettings) -> Result<()> {
    stop(app, serial);
    let deadline = tokio::time::Instant::now() + RESTART_WAIT;
    while app.state::<AppState>().phone_audio().contains(serial) {
        if tokio::time::Instant::now() >= deadline {
            return Err(AppError::Other(
                "The phone's sound did not stop in time".into(),
            ));
        }
        tokio::time::sleep(RESTART_POLL).await;
    }
    start_with(app, serial, settings).await
}

/// Like [`start`], with `settings`; a link that is already running is kept as it is.
pub async fn start_with(app: &AppHandle, serial: &str, settings: &AudioSettings) -> Result<()> {
    if app.state::<AppState>().phone_audio().contains(serial) {
        return Ok(());
    }

    // A saved setting may ask for both on an older phone; it silences the phone there instead.
    let keep_on_phone = settings.keep_on_phone
        && app
            .state::<AppState>()
            .device_api_level(serial)
            .is_none_or(|api| api >= scrcpy_server::AUDIO_DUP_MIN_SDK);
    let settings = &AudioSettings {
        keep_on_phone,
        ..settings.clone()
    };

    let server = server_link::resolve(app)?;
    let scid = scrcpy_server::scid();
    let options = scrcpy_server::audio_args(&server.version, &scid, settings.keep_on_phone);
    let ServerLink {
        mut child,
        streams,
        teardown,
    } = server_link::open(app, &server, serial, &scid, options, 1).await?;

    let opened = match streams.into_iter().next() {
        Some(stream) => begin(stream, &teardown.log_path, settings).await,
        None => Err(AppError::Other("The scrcpy server did not connect".into())),
    };
    let (stream, player) = match opened {
        Ok(opened) => opened,
        Err(error) => {
            let _ = child.kill().await;
            teardown.run().await;
            return Err(error);
        }
    };

    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
    let Some(link_id) = app
        .state::<AppState>()
        .phone_audio()
        .insert(serial, stop_tx)
    else {
        // Another start won the race; keep that one.
        let _ = child.kill().await;
        teardown.run().await;
        return Ok(());
    };
    app.state::<AppState>()
        .set_phone_audio_shared(serial, settings.keep_on_phone);
    announce(app, serial, true);

    let handle = app.clone();
    let serial = serial.to_string();
    tauri::async_runtime::spawn(async move {
        play(stream, player, &mut child, stop_rx).await;
        let _ = child.kill().await;
        teardown.run().await;
        handle
            .state::<AppState>()
            .phone_audio()
            .finished(&serial, link_id);
        announce(&handle, &serial, false);
    });
    Ok(())
}

/// Give the sound back to the phone.
pub fn stop(app: &AppHandle, serial: &str) {
    app.state::<AppState>().phone_audio().stop(serial);
}

/// Read the stream's codec header and open the speakers.
async fn begin(
    mut stream: TcpStream,
    log_path: &std::path::Path,
    settings: &AudioSettings,
) -> Result<(BufReader<TcpStream>, Player)> {
    let mut codec = [0u8; 4];
    let read = tokio::time::timeout(START_TIMEOUT, stream.read_exact(&mut codec)).await;
    if !matches!(read, Ok(Ok(_))) {
        return Err(AppError::CommandFailed(server_link::failure_message(
            log_path,
            "The phone did not start sending its sound",
        )));
    }
    match scrcpy_server::audio_start(u32::from_be_bytes(codec), scrcpy_server::RAW_CODEC_ID) {
        AudioStart::Streaming => {}
        AudioStart::Unavailable => return Err(AppError::Invalid(
            "The phone could not share its sound. It needs Android 11 or newer, and Android 11 \
                 must be unlocked when audio starts."
                .into(),
        )),
        AudioStart::Failed => {
            return Err(AppError::CommandFailed(server_link::failure_message(
                log_path,
                "The phone could not start sharing its sound",
            )))
        }
    }
    let buffering = if settings.low_latency {
        Buffering::LOW_LATENCY
    } else {
        Buffering::NORMAL
    };
    let player = Player::open(settings.device.as_deref(), buffering)?;
    Ok((BufReader::new(stream), player))
}

/// Feed each PCM packet to the speakers until the stream ends or the link is stopped.
async fn play(
    mut reader: BufReader<TcpStream>,
    player: Player,
    child: &mut Child,
    mut stop: tokio::sync::oneshot::Receiver<()>,
) {
    let reading = async {
        let mut header = [0u8; FRAME_HEADER];
        let mut payload = Vec::new();
        loop {
            if reader.read_exact(&mut header).await.is_err() {
                break;
            }
            let (config, len) = match parse_header(&header) {
                Packet::Session => continue,
                Packet::Media { config, len, .. } => (config, len),
            };
            if len > MAX_PACKET {
                break;
            }
            payload.resize(len, 0);
            if reader.read_exact(&mut payload).await.is_err() {
                break;
            }
            if !config {
                player.push(&payload);
            }
        }
    };

    tokio::select! {
        _ = reading => {}
        _ = &mut stop => {}
        _ = child.wait() => {}
    }
}
