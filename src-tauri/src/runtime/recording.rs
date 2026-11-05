use std::collections::HashMap;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::{mpsc, oneshot};

use crate::error::{AppError, Result};
use crate::models::{Device, MirrorKind};
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
    /// Whether the recording's own window is still open to show the result.
    in_window: bool,
}

type Reply = oneshot::Sender<Result<RecordingOutcome>>;

enum Entry {
    Starting,
    Running {
        stop: oneshot::Sender<Reply>,
        started_at: u64,
    },
}

/// What to do with a request to quit the app.
#[derive(Debug, PartialEq, Eq)]
pub enum ExitPlan {
    Allow,
    /// Hold the exit and save these sessions' recordings first.
    Finish(Vec<u64>),
    /// Already saving them; the exit follows on its own.
    Hold,
}

/// A phone whose auto-rotate could not be given back (it was gone), by serial and mDNS name.
struct PendingUnlock {
    serial: String,
    mdns_name: Option<String>,
}

/// Recordings by session id; a session records at most once at a time.
#[derive(Default)]
pub struct Recordings {
    entries: Mutex<HashMap<u64, Entry>>,
    pending_unlocks: Mutex<Vec<PendingUnlock>>,
    exiting: AtomicBool,
    exit_ready: AtomicBool,
}

impl Recordings {
    /// Remember to turn auto-rotate back on once the phone is reachable again.
    fn defer_unlock(&self, serial: &str, mdns_name: Option<&str>) {
        if let Ok(mut pending) = self.pending_unlocks.lock() {
            pending.push(PendingUnlock {
                serial: serial.to_string(),
                mdns_name: mdns_name.map(str::to_string),
            });
        }
    }

    /// Serials (as connected now) of phones waiting for their auto-rotate, forgetting them.
    pub fn take_ready_unlocks(&self, devices: &[Device]) -> Vec<String> {
        let Ok(mut pending) = self.pending_unlocks.lock() else {
            return Vec::new();
        };
        let mut ready = Vec::new();
        pending.retain(|phone| {
            let back = devices.iter().find(|device| {
                device.state.is_connected()
                    && (device.serial == phone.serial
                        || (phone.mdns_name.is_some() && device.mdns_name == phone.mdns_name))
            });
            match back {
                Some(device) => {
                    ready.push(device.serial.clone());
                    false
                }
                None => true,
            }
        });
        ready
    }

    pub fn plan_exit(&self) -> ExitPlan {
        if self.exit_ready.load(Ordering::SeqCst) {
            return ExitPlan::Allow;
        }
        let ids: Vec<u64> = self
            .entries
            .lock()
            .map(|guard| guard.keys().copied().collect())
            .unwrap_or_default();
        if ids.is_empty() {
            ExitPlan::Allow
        } else if self.exiting.swap(true, Ordering::SeqCst) {
            ExitPlan::Hold
        } else {
            ExitPlan::Finish(ids)
        }
    }

    fn ready_to_exit(&self) {
        self.exit_ready.store(true, Ordering::SeqCst);
    }

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
    mdns_name: Option<String>,
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
    let mdns_name = device.as_ref().and_then(|device| device.mdns_name.clone());
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
        mdns_name,
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

/// Give auto-rotate back to phones that left while recording, now that they are back.
pub fn devices_changed(app: &AppHandle, devices: &[Device]) {
    let ready = app
        .state::<AppState>()
        .recordings()
        .take_ready_unlocks(devices);
    if ready.is_empty() {
        return;
    }
    let Ok(adb_bin) = adb::adb_path(&app.state::<AppState>().tools()) else {
        return;
    };
    tauri::async_runtime::spawn(async move {
        for serial in ready {
            scrcpy::unlock_rotation(&adb_bin, &serial).await;
        }
    });
}

/// How long quitting waits for one recording to be saved (a phone copy can be slow).
const EXIT_SAVE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// On quit, save running recordings first; true when the exit has to wait for that.
pub fn hold_exit(app: &AppHandle) -> bool {
    match app.state::<AppState>().recordings().plan_exit() {
        ExitPlan::Allow => false,
        ExitPlan::Hold => true,
        ExitPlan::Finish(ids) => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                for id in ids {
                    let _ = tokio::time::timeout(EXIT_SAVE_TIMEOUT, stop(&app, id)).await;
                }
                app.state::<AppState>().recordings().ready_to_exit();
                app.exit(0);
            });
            true
        }
    }
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

/// A Stop clicked while the recording was already ending still gets its result.
fn late_reply(reply: Option<Reply>, stop: &mut oneshot::Receiver<Reply>) -> Option<Reply> {
    reply.or_else(|| stop.try_recv().ok())
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

    // Before the slower clean-up, so the file ends when the recording did.
    let stopped = Instant::now();
    let state = app.state::<AppState>();
    state.set_video_tap(context.id, None);
    if let Some(stopper) = audio_stop {
        stopper.stop();
    }
    restore(&app, &context).await;
    if let Some(failure) = failure {
        context.notice = Some(failure);
    }
    let result = save(&app, &context, core, stopped).await;

    state.recordings().remove(context.id);
    let _ = app.emit(
        EVENT_CHANGED,
        Changed {
            id: context.id,
            recording: false,
            started_at: None,
        },
    );
    match late_reply(reply, &mut stop) {
        Some(reply) => {
            let _ = reply.send(result);
        }
        None => {
            let (outcome, error) = match result {
                Ok(outcome) => (Some(outcome), None),
                Err(error) => (None, Some(error.to_string())),
            };
            // The window shows the result while it is open; otherwise the main window does.
            let in_window = app
                .get_webview_window(&session_window::label(context.id))
                .is_some();
            let _ = app.emit(
                EVENT_FINISHED,
                Finished {
                    id: context.id,
                    outcome,
                    error,
                    in_window,
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
    stopped: Instant,
) -> Result<RecordingOutcome> {
    let finished = core
        .finish(stopped)
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
    if context.rotation_locked && !scrcpy::unlock_rotation(&context.adb_bin, &context.serial).await
    {
        // The phone is gone; give auto-rotate back when it returns (see `devices_changed`).
        app.state::<AppState>()
            .recordings()
            .defer_unlock(&context.serial, context.mdns_name.as_deref());
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
    use crate::models::{DeviceState, DeviceTransport};

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

    fn device(serial: &str, mdns: Option<&str>, state: DeviceState) -> Device {
        Device {
            serial: serial.into(),
            display_name: "Pixel".into(),
            model: "Pixel".into(),
            android_version: None,
            sdk: None,
            transport: DeviceTransport::Wifi,
            state,
            connected: state == DeviceState::Device,
            battery: None,
            charging: None,
            address: None,
            mdns_name: mdns.map(Into::into),
        }
    }

    #[test]
    fn deferred_unlock_waits_for_the_phone() {
        let recordings = Recordings::default();
        recordings.defer_unlock("10.0.0.5:4000", Some("adb-ABC"));
        let offline = [device(
            "10.0.0.5:4000",
            Some("adb-ABC"),
            DeviceState::Offline,
        )];
        assert!(recordings.take_ready_unlocks(&offline).is_empty());
        // Back over Wi-Fi on a new port: matched by its mDNS name.
        let back = [device(
            "10.0.0.5:4100",
            Some("adb-ABC"),
            DeviceState::Device,
        )];
        assert_eq!(recordings.take_ready_unlocks(&back), ["10.0.0.5:4100"]);
        assert!(recordings.take_ready_unlocks(&back).is_empty());
    }

    #[test]
    fn exit_waits_for_recordings_once() {
        let recordings = Recordings::default();
        assert_eq!(recordings.plan_exit(), ExitPlan::Allow);
        assert!(recordings.begin(3));
        assert_eq!(recordings.plan_exit(), ExitPlan::Finish(vec![3]));
        // A second exit request while they finish just waits.
        assert_eq!(recordings.plan_exit(), ExitPlan::Hold);
        recordings.ready_to_exit();
        assert_eq!(recordings.plan_exit(), ExitPlan::Allow);
    }

    #[test]
    fn late_stop_still_gets_the_result() {
        let (stop, mut stop_rx) = oneshot::channel::<Reply>();
        let (reply, _outcome) = oneshot::channel();
        // Stop clicked after the loop ended on its own (window closed, size changed).
        stop.send(reply).unwrap();
        assert!(late_reply(None, &mut stop_rx).is_some());

        let (_stop, mut idle_rx) = oneshot::channel::<Reply>();
        assert!(late_reply(None, &mut idle_rx).is_none());
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
