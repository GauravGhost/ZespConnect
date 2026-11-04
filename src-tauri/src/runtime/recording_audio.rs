//! The sound of a recording: its own audio-only scrcpy server sending AAC, separate from the
//! phone-sound link so a recording never changes where the user hears the phone.

use std::time::{Duration, Instant};

use tauri::AppHandle;
use tokio::io::{AsyncReadExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot};

use crate::error::{AppError, Result};
use crate::runtime::mirror_session::{parse_header, Packet, FRAME_HEADER};
use crate::runtime::server_link::{self, ServerLink};
use crate::services::fmp4::AudioConfig;
use crate::services::scrcpy_server::{self, AudioStart};

/// Playback capture with duplication arrived in Android 13.
pub const RECORDING_AUDIO_MIN_SDK: u32 = 33;
const START_TIMEOUT: Duration = Duration::from_secs(8);
/// AAC frames are a few hundred bytes; anything far larger is a corrupt header.
const MAX_PACKET: usize = 1 << 16;
/// About 20 s of AAC frames; a recorder that far behind has already failed.
const QUEUE: usize = 1024;

/// One AAC frame, with when it reached the computer.
pub struct AudioPacket {
    pub arrived: Instant,
    pub pts: u64,
    pub data: Vec<u8>,
}

pub struct RecordingAudio {
    pub config: AudioConfig,
    pub packets: mpsc::Receiver<AudioPacket>,
    stop: oneshot::Sender<()>,
}

/// Ends a recording's sound stream.
pub struct AudioStop(oneshot::Sender<()>);

impl AudioStop {
    pub fn stop(self) {
        let _ = self.0.send(());
    }
}

impl RecordingAudio {
    pub fn stop(self) {
        let _ = self.stop.send(());
    }

    /// The packets and the stop handle, for a reader that owns them separately.
    pub fn split(self) -> (mpsc::Receiver<AudioPacket>, AudioStop) {
        (self.packets, AudioStop(self.stop))
    }
}

/// Start the phone's AAC stream; returns once the first packet (its config) arrived.
pub async fn open(app: &AppHandle, serial: &str) -> Result<RecordingAudio> {
    let server = server_link::resolve(app)?;
    let scid = scrcpy_server::scid();
    let options = scrcpy_server::recording_audio_args(&server.version, &scid);
    let ServerLink {
        mut child,
        streams,
        teardown,
    } = server_link::open(app, &server, serial, &scid, options, 1).await?;

    let started = match streams.into_iter().next() {
        Some(stream) => begin(stream, &teardown.log_path).await,
        None => Err(AppError::Other("The scrcpy server did not connect".into())),
    };
    let (reader, config) = match started {
        Ok(started) => started,
        Err(error) => {
            let _ = child.kill().await;
            teardown.run().await;
            return Err(error);
        }
    };

    let (packets_tx, packets) = mpsc::channel(QUEUE);
    let (stop, mut stop_rx) = oneshot::channel();
    tauri::async_runtime::spawn(async move {
        tokio::select! {
            _ = forward(reader, packets_tx) => {}
            _ = &mut stop_rx => {}
            _ = child.wait() => {}
        }
        let _ = child.kill().await;
        teardown.run().await;
    });
    Ok(RecordingAudio {
        config,
        packets,
        stop,
    })
}

async fn begin(
    mut stream: TcpStream,
    log_path: &std::path::Path,
) -> Result<(BufReader<TcpStream>, AudioConfig)> {
    let failed =
        |fallback: &str| AppError::CommandFailed(server_link::failure_message(log_path, fallback));
    let mut codec = [0u8; 4];
    let read = tokio::time::timeout(START_TIMEOUT, stream.read_exact(&mut codec)).await;
    if !matches!(read, Ok(Ok(_))) {
        return Err(failed("The phone did not start sending its sound"));
    }
    match scrcpy_server::audio_start(u32::from_be_bytes(codec), scrcpy_server::AAC_CODEC_ID) {
        AudioStart::Streaming => {}
        AudioStart::Unavailable => {
            return Err(AppError::Invalid(
                "The phone could not share its sound".into(),
            ))
        }
        AudioStart::Failed => return Err(failed("The phone could not start sharing its sound")),
    }

    let mut reader = BufReader::new(stream);
    let first = tokio::time::timeout(START_TIMEOUT, read_packet(&mut reader)).await;
    match first {
        Ok(Some((true, _, asc))) => Ok((reader, AudioConfig::from_asc(&asc)?)),
        _ => Err(failed("The phone's sound started without its AAC config")),
    }
}

/// `(config, pts, payload)` of the next media packet, or `None` once the stream ends.
async fn read_packet(reader: &mut BufReader<TcpStream>) -> Option<(bool, u64, Vec<u8>)> {
    let mut header = [0u8; FRAME_HEADER];
    loop {
        reader.read_exact(&mut header).await.ok()?;
        if let Packet::Media {
            config, pts, len, ..
        } = parse_header(&header)
        {
            if len > MAX_PACKET {
                return None;
            }
            let mut payload = vec![0; len];
            reader.read_exact(&mut payload).await.ok()?;
            return Some((config, pts, payload));
        }
    }
}

async fn forward(mut reader: BufReader<TcpStream>, packets: mpsc::Sender<AudioPacket>) {
    while let Some((config, pts, data)) = read_packet(&mut reader).await {
        if config {
            continue;
        }
        let packet = AudioPacket {
            arrived: Instant::now(),
            pts,
            data,
        };
        // The recorder dropped its end, or fell so far behind that it is failing anyway.
        if packets.try_send(packet).is_err() {
            break;
        }
    }
}
