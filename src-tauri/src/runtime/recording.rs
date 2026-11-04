//! Recording a mirror window to MP4: the session's video packets are copied off its read loop
//! (see `VideoTap`), joined with a separate AAC stream, and written by `RecorderCore` on a task
//! of their own, so a slow disk never stalls the live mirror.

use std::collections::HashMap;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::{mpsc, oneshot};

use crate::error::{AppError, Result};
use crate::models::MirrorKind;
use crate::runtime::recording_audio::{self, AudioPacket, RecordingAudio, RECORDING_AUDIO_MIN_SDK};
use crate::runtime::session_window;
use crate::services::recorder::{Ended, RecorderCore};
use crate::services::{adb, capture, control, files, scrcpy};
use crate::state::AppState;

pub const EVENT_CHANGED: &str = "recording:changed";
pub const EVENT_FINISHED: &str = "recording:finished";
const PHONE_RECORDINGS: &str = "/sdcard/Movies/Zesp Connect";
/// About 8 s of video at 60 fps; a recorder that far behind cannot catch up.
const VIDEO_QUEUE: usize = 512;
const NO_SOUND_OLD_ANDROID: &str = "Recorded without sound: it needs Android 13 or newer";
const SIZE_CHANGED: &str = "Recording stopped because the screen size changed";
const DISK_TOO_SLOW: &str = "Recording stopped: the disk could not keep up";

/// One packet of a session's video stream, as read from the server.
pub struct VideoPacket {
    pub arrived: Instant,
    pub config: bool,
    pub key: bool,
    pub pts: u64,
    pub data: Vec<u8>,
}

/// Where a session's video read loop copies packets while it records.
#[derive(Clone)]
pub struct VideoTap {
    packets: mpsc::Sender<VideoPacket>,
    overflowed: Arc<AtomicBool>,
}

impl VideoTap {
    /// Copy a packet without waiting; a full queue marks the recording as failed instead.
    pub fn offer(&self, packet: VideoPacket) {
        if let Err(mpsc::error::TrySendError::Full(_)) = self.packets.try_send(packet) {
            self.overflowed.store(true, Ordering::Relaxed);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RecordingTarget {
    Folder,
    Phone,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingOutcome {
    pub location: String,
    pub on_phone: bool,
    pub notice: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Changed {
    id: u64,
    recording: bool,
    started_at: Option<u64>,
}

/// Sent to the main window when a recording ends without its window asking (closed, lost phone).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Finished {
    id: u64,
    outcome: Option<RecordingOutcome>,
    error: Option<String>,
}

type Reply = oneshot::Sender<Result<RecordingOutcome>>;

enum Entry {
    Starting,
    Running {
        stop: oneshot::Sender<Reply>,
        started_at: u64,
    },
}

/// Recordings by session id; a session records at most once at a time.
#[derive(Default)]
pub struct Recordings {
    entries: Mutex<HashMap<u64, Entry>>,
}

impl Recordings {
    /// Claim a session for a new recording; false when it already records or is starting one.
    fn begin(&self, id: u64) -> bool {
        let Ok(mut guard) = self.entries.lock() else {
            return false;
        };
        if guard.contains_key(&id) {
            return false;
        }
        guard.insert(id, Entry::Starting);
        true
    }

    fn run(&self, id: u64, stop: oneshot::Sender<Reply>, started_at: u64) {
        if let Ok(mut guard) = self.entries.lock() {
            guard.insert(id, Entry::Running { stop, started_at });
        }
    }

    fn remove(&self, id: u64) {
        if let Ok(mut guard) = self.entries.lock() {
            guard.remove(&id);
        }
    }

    /// The running recording's stop handle, leaving the entry so a second start stays blocked.
    fn take_stop(&self, id: u64) -> Option<oneshot::Sender<Reply>> {
        let mut guard = self.entries.lock().ok()?;
        match guard.remove(&id)? {
            Entry::Running { stop, .. } => {
                guard.insert(id, Entry::Starting);
                Some(stop)
            }
            starting => {
                guard.insert(id, starting);
                None
            }
        }
    }

    pub fn contains(&self, id: u64) -> bool {
        self.entries
            .lock()
            .map(|guard| guard.contains_key(&id))
            .unwrap_or(false)
    }

    /// When a running recording started, in Unix milliseconds.
    pub fn started_at(&self, id: u64) -> Option<u64> {
        let guard = self.entries.lock().ok()?;
        match guard.get(&id)? {
            Entry::Running { started_at, .. } => Some(*started_at),
            Entry::Starting => None,
        }
    }
}

fn unix_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

/// Everything a running recording has to give back when it ends.
struct Context {
    id: u64,
    serial: String,
    kind: MirrorKind,
    adb_bin: String,
    path: PathBuf,
    target: RecordingTarget,
    rotation_locked: bool,
    notice: Option<String>,
}

/// Start recording session `id`; returns when it started, in Unix milliseconds.
pub async fn start(
    app: &AppHandle,
    id: u64,
    target: RecordingTarget,
    dir: Option<String>,
    stamp: &str,
) -> Result<u64> {
    let state = app.state::<AppState>();
    if !state.recordings().begin(id) {
        return Err(AppError::Invalid("Already recording".into()));
    }
    let started = prepare(app, id, target, dir, stamp).await;
    if started.is_err() {
        app.state::<AppState>().recordings().remove(id);
    }
    started
}

async fn prepare(
    app: &AppHandle,
    id: u64,
    target: RecordingTarget,
    dir: Option<String>,
    stamp: &str,
) -> Result<u64> {
    let state = app.state::<AppState>();
    let session = state
        .session(id)
        .ok_or_else(|| AppError::Invalid("This window is not mirroring".into()))?;
    let control = state
        .session_control(id)
        .filter(|_| state.is_streaming(id))
        .ok_or_else(|| AppError::Invalid("Wait for the picture to appear first".into()))?;
    let device = state.device(&session.serial);
    let model = device
        .as_ref()
        .map(|device| device.model.clone())
        .unwrap_or_default();
    let sdk = device
        .and_then(|device| device.sdk)
        .and_then(|sdk| sdk.parse::<u32>().ok())
        .unwrap_or(0);
    let adb_bin = adb::adb_path(&state.tools())?;

    let name = capture::file_name(&model, &session.serial, stamp, "mp4");
    let folder = match (target, dir.filter(|dir| !dir.is_empty())) {
        (RecordingTarget::Phone, _) => std::env::temp_dir(),
        (RecordingTarget::Folder, Some(dir)) => PathBuf::from(dir),
        (RecordingTarget::Folder, None) => PathBuf::from(capture::default_dirs()?.videos),
    };
    let path = capture::save_to_folder(&folder, &name, &[])?;
    let file = std::fs::File::create(&path).map_err(|error| {
        AppError::Io(format!("Could not record to {}: {error}", path.display()))
    })?;

    let (audio, notice) = if sdk >= RECORDING_AUDIO_MIN_SDK {
        match recording_audio::open(app, &session.serial).await {
            Ok(audio) => (Some(audio), None),
            Err(error) => (None, Some(format!("Recorded without sound: {error}"))),
        }
    } else {
        (None, Some(NO_SOUND_OLD_ANDROID.to_string()))
    };

    // Only the main display turns with the phone; virtual displays and cameras do not.
    let main_display = matches!(session.kind, MirrorKind::Mirror | MirrorKind::ScreenApp);
    let rotation_locked = main_display && scrcpy::lock_rotation(&adb_bin, &session.serial).await;
    lock_window(app, id, true, session.kind);

    let context = Context {
        id,
        serial: session.serial.clone(),
        kind: session.kind,
        adb_bin,
        path,
        target,
        rotation_locked,
        notice,
    };

    let (packets, video) = mpsc::channel(VIDEO_QUEUE);
    let overflowed = Arc::new(AtomicBool::new(false));
    let tap = VideoTap {
        packets,
        overflowed: overflowed.clone(),
    };
    if !state.set_video_tap(id, Some(tap)) {
        if let Some(audio) = audio {
            audio.stop();
        }
        restore(app, &context).await;
        let _ = std::fs::remove_file(&context.path);
        return Err(AppError::Invalid("The mirror window closed".into()));
    }
    // The file must open on a config and key frame; this asks the encoder for both.
    let _ = control.send(control::reset_video());

    let started_at = unix_millis();
    let (stop, stop_rx) = oneshot::channel();
    state.recordings().run(id, stop, started_at);
    let _ = app.emit(
        EVENT_CHANGED,
        Changed {
            id,
            recording: true,
            started_at: Some(started_at),
        },
    );

    let core = RecorderCore::new(
        BufWriter::new(file),
        audio.as_ref().map(|a| a.config.clone()),
    );
    tauri::async_runtime::spawn(record(
        app.clone(),
        context,
        core,
        video,
        audio,
        overflowed,
        stop_rx,
    ));
    Ok(started_at)
}

/// Stop session `id`'s recording and wait until its file is saved.
pub async fn stop(app: &AppHandle, id: u64) -> Result<RecordingOutcome> {
    let stop = app
        .state::<AppState>()
        .recordings()
        .take_stop(id)
        .ok_or_else(|| AppError::Invalid("Not recording".into()))?;
    let (reply, outcome) = oneshot::channel();
    stop.send(reply)
        .map_err(|_| AppError::Other("The recording already ended".into()))?;
    outcome
        .await
        .map_err(|_| AppError::Other("The recording ended unexpectedly".into()))?
}

async fn next_audio(audio: &mut Option<mpsc::Receiver<AudioPacket>>) -> Option<AudioPacket> {
    match audio.as_mut() {
        Some(packets) => packets.recv().await,
        None => std::future::pending().await,
    }
}

async fn record<W: Write + Send + 'static>(
    app: AppHandle,
    mut context: Context,
    mut core: RecorderCore<BufWriter<W>>,
    mut video: mpsc::Receiver<VideoPacket>,
    audio: Option<RecordingAudio>,
    overflowed: Arc<AtomicBool>,
    mut stop: oneshot::Receiver<Reply>,
) {
    let (mut audio_packets, audio_stop) = match audio {
        Some(audio) => {
            let (packets, stopper) = audio.split();
            (Some(packets), Some(stopper))
        }
        None => (None, None),
    };
    let mut reply = None;
    let mut failure: Option<String> = None;

    loop {
        tokio::select! {
            request = &mut stop => {
                reply = request.ok();
                break;
            }
            packet = video.recv() => {
                // `None`: the session ended (window closed or phone gone).
                let Some(packet) = packet else { break };
                if overflowed.load(Ordering::Relaxed) {
                    failure = Some(DISK_TOO_SLOW.to_string());
                    break;
                }
                match core.push_video(packet.arrived, packet.config, packet.key, packet.pts, &packet.data) {
                    Ok(None) => {}
                    Ok(Some(Ended::SizeChanged)) => {
                        failure = Some(SIZE_CHANGED.to_string());
                        break;
                    }
                    Err(error) => {
                        failure = Some(format!("Recording stopped: {error}"));
                        break;
                    }
                }
            }
            packet = next_audio(&mut audio_packets) => match packet {
                Some(packet) => {
                    if let Err(error) = core.push_audio(packet.arrived, packet.pts, &packet.data) {
                        failure = Some(format!("Recording stopped: {error}"));
                        break;
                    }
                }
                // The sound stream ended early; the video goes on.
                None => audio_packets = None,
            },
        }
    }

    let state = app.state::<AppState>();
    state.set_video_tap(context.id, None);
    if let Some(stopper) = audio_stop {
        stopper.stop();
    }
    restore(&app, &context).await;
    if let Some(failure) = failure {
        context.notice = Some(failure);
    }
    let result = save(&app, &context, core).await;

    state.recordings().remove(context.id);
    let _ = app.emit(
        EVENT_CHANGED,
        Changed {
            id: context.id,
            recording: false,
            started_at: None,
        },
    );
    match reply {
        Some(reply) => {
            let _ = reply.send(result);
        }
        None => {
            let (outcome, error) = match result {
                Ok(outcome) => (Some(outcome), None),
                Err(error) => (None, Some(error.to_string())),
            };
            let _ = app.emit(
                EVENT_FINISHED,
                Finished {
                    id: context.id,
                    outcome,
                    error,
                },
            );
        }
    }
}

/// Close the file and, for the phone, copy it there.
async fn save<W: Write>(
    app: &AppHandle,
    context: &Context,
    core: RecorderCore<BufWriter<W>>,
) -> Result<RecordingOutcome> {
    let finished = core
        .finish()
        .and_then(|written| written.map(|mut out| out.flush()).transpose());
    match finished {
        Ok(Some(())) => {}
        Ok(None) => {
            let _ = std::fs::remove_file(&context.path);
            return Err(AppError::Other("Nothing was recorded".into()));
        }
        Err(error) => {
            return Err(AppError::Io(format!(
                "The recording could not be saved: {error}"
            )))
        }
    }

    let local = context.path.to_string_lossy().into_owned();
    if context.target == RecordingTarget::Folder {
        return Ok(RecordingOutcome {
            location: local,
            on_phone: false,
            notice: context.notice.clone(),
        });
    }
    let tools = app.state::<AppState>().tools();
    match files::send_to_media_dir(&tools, &context.serial, &local, PHONE_RECORDINGS).await {
        Ok(remote) => {
            let _ = std::fs::remove_file(&context.path);
            Ok(RecordingOutcome {
                location: remote,
                on_phone: true,
                notice: context.notice.clone(),
            })
        }
        // The recording is kept on the computer, so nothing is lost.
        Err(error) => Err(AppError::CommandFailed(format!(
            "Could not copy the recording to the phone ({error}). It is saved at {local}"
        ))),
    }
}

/// Give back what a recording held: the phone's rotation and the window's resizing.
async fn restore(app: &AppHandle, context: &Context) {
    if context.rotation_locked {
        scrcpy::unlock_rotation(&context.adb_bin, &context.serial).await;
    }
    lock_window(app, context.id, false, context.kind);
}

/// A recording keeps one picture size, so its window cannot be resized or maximized.
fn lock_window(app: &AppHandle, id: u64, locked: bool, kind: MirrorKind) {
    if let Some(window) = app.get_webview_window(&session_window::label(id)) {
        let _ = window.set_resizable(!locked);
        // Only a desktop window can be maximized at all.
        if kind == MirrorKind::Desktop {
            let _ = window.set_maximizable(!locked);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_start_is_rejected() {
        let recordings = Recordings::default();
        assert!(recordings.begin(7));
        assert!(!recordings.begin(7));
        let (stop, _stop_rx) = oneshot::channel();
        recordings.run(7, stop, 1000);
        assert!(!recordings.begin(7));
        assert_eq!(recordings.started_at(7), Some(1000));

        // Stopping keeps the session claimed until the file is saved.
        assert!(recordings.take_stop(7).is_some());
        assert!(!recordings.begin(7));
        assert!(recordings.take_stop(7).is_none());
        recordings.remove(7);
        assert!(recordings.begin(7));
    }

    #[test]
    fn full_tap_marks_overflow() {
        let (packets, _receiver) = mpsc::channel(1);
        let tap = VideoTap {
            packets,
            overflowed: Arc::new(AtomicBool::new(false)),
        };
        let packet = || VideoPacket {
            arrived: Instant::now(),
            config: false,
            key: false,
            pts: 0,
            data: Vec::new(),
        };
        tap.offer(packet());
        assert!(!tap.overflowed.load(Ordering::Relaxed));
        tap.offer(packet());
        assert!(tap.overflowed.load(Ordering::Relaxed));
    }
}
