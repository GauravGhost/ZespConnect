use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::RwLock;

use crate::models::{
    Device, MirrorKind, MirrorProcess, MirrorSession, MirrorSessionSpec, ToolKind, ToolStatus,
};
use crate::runtime::mirror_session::ControlChannel;
use crate::util;

/// A tracked scrcpy window plus the channel that asks its watcher to stop it.
pub struct MirrorHandle {
    pub info: MirrorProcess,
    pub stop: tokio::sync::oneshot::Sender<()>,
}

/// A tracked in-app mirror session plus the channel that asks its supervisor to
/// stop it and the one that injects input into the device.
pub struct SessionHandle {
    pub info: MirrorSession,
    pub stop: tokio::sync::oneshot::Sender<()>,
    pub control: ControlChannel,
}

/// One background link per device (phone audio, clipboard sync), each stopped through its sender.
/// Every link gets an id so a link that finishes late cannot unregister its replacement.
#[derive(Default)]
pub struct DeviceLinks {
    next_id: AtomicU64,
    links: RwLock<HashMap<String, (u64, tokio::sync::oneshot::Sender<()>)>>,
}

impl DeviceLinks {
    pub fn serials(&self) -> Vec<String> {
        self.links
            .read()
            .map(|guard| guard.keys().cloned().collect())
            .unwrap_or_default()
    }

    pub fn contains(&self, serial: &str) -> bool {
        self.links
            .read()
            .map(|guard| guard.contains_key(serial))
            .unwrap_or(false)
    }

    /// Register a running link and return its id, or `None` when the device already has one.
    pub fn insert(&self, serial: &str, stop: tokio::sync::oneshot::Sender<()>) -> Option<u64> {
        let mut guard = self.links.write().ok()?;
        if guard.contains_key(serial) {
            return None;
        }
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        guard.insert(serial.to_string(), (id, stop));
        Some(id)
    }

    /// Ask a device's link to stop; its task unregisters it once torn down.
    pub fn stop(&self, serial: &str) {
        if let Ok(mut guard) = self.links.write() {
            if let Some((_, stop)) = guard.remove(serial) {
                let _ = stop.send(());
            }
        }
    }

    /// Forget a link that has ended, unless it was already replaced.
    pub fn finished(&self, serial: &str, id: u64) {
        if let Ok(mut guard) = self.links.write() {
            if guard.get(serial).is_some_and(|(current, _)| *current == id) {
                guard.remove(serial);
            }
        }
    }
}

/// Shared, process-wide state. Guards are only ever held for short,
/// synchronous critical sections (never across `.await`).
pub struct AppState {
    tools: RwLock<ToolStatus>,
    tool_overrides: RwLock<HashMap<String, String>>,
    devices: RwLock<Vec<Device>>,
    mirrors: RwLock<HashMap<u32, MirrorHandle>>,
    sessions: RwLock<HashMap<u64, SessionHandle>>,
    pending_sessions: RwLock<HashMap<u64, MirrorSessionSpec>>,
    aspects: RwLock<HashMap<u64, f64>>,
    /// Sessions whose video stream has forwarded its first packet: the capture is
    /// up, so control messages that would crash the server if sent earlier are safe.
    streaming: RwLock<HashSet<u64>>,
    /// Desktop sessions whose user turned the phone's own screen on.
    phone_screens: RwLock<HashSet<u64>>,
    next_session: AtomicU64,
    app_data_dir: PathBuf,
    tracker: RwLock<Option<tauri::async_runtime::JoinHandle<()>>>,
    logcats: RwLock<HashMap<String, tauri::async_runtime::JoinHandle<()>>>,
    pairing: AtomicBool,
    pairing_cancel: RwLock<Option<tokio::sync::oneshot::Sender<()>>>,
    selected: RwLock<Option<String>>,
    phone_audio: DeviceLinks,
    clipboard_sync: DeviceLinks,
}

impl AppState {
    pub fn new(app_data_dir: PathBuf) -> Self {
        let overrides = load_overrides(&app_data_dir).unwrap_or_default();
        let selected = load_selected(&app_data_dir);
        AppState {
            tools: RwLock::new(ToolStatus::default()),
            tool_overrides: RwLock::new(overrides),
            devices: RwLock::new(Vec::new()),
            mirrors: RwLock::new(HashMap::new()),
            sessions: RwLock::new(HashMap::new()),
            pending_sessions: RwLock::new(HashMap::new()),
            aspects: RwLock::new(HashMap::new()),
            streaming: RwLock::new(HashSet::new()),
            phone_screens: RwLock::new(HashSet::new()),
            next_session: AtomicU64::new(1),
            app_data_dir,
            tracker: RwLock::new(None),
            logcats: RwLock::new(HashMap::new()),
            pairing: AtomicBool::new(false),
            pairing_cancel: RwLock::new(None),
            selected: RwLock::new(selected),
            phone_audio: DeviceLinks::default(),
            clipboard_sync: DeviceLinks::default(),
        }
    }

    /// Devices whose sound plays on the computer.
    pub fn phone_audio(&self) -> &DeviceLinks {
        &self.phone_audio
    }

    /// Devices whose clipboard is kept in sync with the computer's.
    pub fn clipboard_sync(&self) -> &DeviceLinks {
        &self.clipboard_sync
    }

    pub fn app_data_dir(&self) -> PathBuf {
        self.app_data_dir.clone()
    }

    pub fn overrides_file(&self) -> PathBuf {
        self.app_data_dir.join("tool-paths.json")
    }

    pub fn tools(&self) -> ToolStatus {
        self.tools
            .read()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    pub fn set_tools(&self, status: ToolStatus) {
        if let Ok(mut guard) = self.tools.write() {
            *guard = status;
        }
    }

    pub fn adb_path(&self) -> Option<String> {
        self.tools
            .read()
            .ok()
            .and_then(|guard| guard.adb.path.clone())
    }

    pub fn scrcpy_path(&self) -> Option<String> {
        self.tools
            .read()
            .ok()
            .and_then(|guard| guard.scrcpy.path.clone())
    }

    pub fn devices(&self) -> Vec<Device> {
        self.devices
            .read()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    pub fn set_devices(&self, devices: Vec<Device>) {
        if let Ok(mut guard) = self.devices.write() {
            *guard = devices;
        }
    }

    pub fn list_mirrors(&self) -> Vec<MirrorProcess> {
        self.mirrors
            .read()
            .map(|guard| guard.values().map(|handle| handle.info.clone()).collect())
            .unwrap_or_default()
    }

    pub fn set_mirror(&self, info: MirrorProcess, stop: tokio::sync::oneshot::Sender<()>) {
        if let Ok(mut guard) = self.mirrors.write() {
            guard.insert(info.pid, MirrorHandle { info, stop });
        }
    }

    pub fn remove_mirror(&self, pid: u32) {
        if let Ok(mut guard) = self.mirrors.write() {
            guard.remove(&pid);
        }
    }

    pub fn take_mirror(&self, pid: u32) -> Option<MirrorHandle> {
        self.mirrors
            .write()
            .ok()
            .and_then(|mut guard| guard.remove(&pid))
    }

    pub fn drain_mirrors(&self) -> Vec<MirrorHandle> {
        self.mirrors
            .write()
            .map(|mut guard| guard.drain().map(|(_, handle)| handle).collect())
            .unwrap_or_default()
    }

    pub fn next_session_id(&self) -> u64 {
        self.next_session.fetch_add(1, Ordering::SeqCst)
    }

    /// Remember a session's settings until its window asks for the stream.
    pub fn set_pending_session(&self, id: u64, spec: MirrorSessionSpec) {
        if let Ok(mut guard) = self.pending_sessions.write() {
            guard.insert(id, spec);
        }
    }

    pub fn take_pending_session(&self, id: u64) -> Option<MirrorSessionSpec> {
        self.pending_sessions
            .write()
            .ok()
            .and_then(|mut guard| guard.remove(&id))
    }

    /// Remember a session's video aspect ratio (`width / height`) so its window can
    /// keep that shape on every resize.
    pub fn set_aspect(&self, id: u64, aspect: f64) {
        if let Ok(mut guard) = self.aspects.write() {
            guard.insert(id, aspect);
        }
    }

    /// A session's video aspect ratio, once its first frame has revealed it.
    pub fn aspect(&self, id: u64) -> Option<f64> {
        self.aspects
            .read()
            .ok()
            .and_then(|guard| guard.get(&id).copied())
    }

    /// Mark that a session's video stream has started forwarding packets.
    pub fn set_streaming(&self, id: u64) {
        if let Ok(mut guard) = self.streaming.write() {
            guard.insert(id);
        }
    }

    /// Whether a session's video stream has forwarded at least one packet.
    pub fn is_streaming(&self, id: u64) -> bool {
        self.streaming
            .read()
            .map(|guard| guard.contains(&id))
            .unwrap_or(false)
    }

    pub fn set_phone_screen(&self, id: u64, on: bool) {
        if let Ok(mut guard) = self.phone_screens.write() {
            if on {
                guard.insert(id);
            } else {
                guard.remove(&id);
            }
        }
    }

    pub fn phone_screen_on(&self, id: u64) -> bool {
        self.phone_screens
            .read()
            .map(|guard| guard.contains(&id))
            .unwrap_or(false)
    }

    pub fn session_kind(&self, id: u64) -> Option<MirrorKind> {
        self.sessions
            .read()
            .ok()
            .and_then(|guard| guard.get(&id).map(|handle| handle.info.kind))
    }

    pub fn list_sessions(&self) -> Vec<MirrorSession> {
        self.sessions
            .read()
            .map(|guard| guard.values().map(|handle| handle.info.clone()).collect())
            .unwrap_or_default()
    }

    pub fn set_session(
        &self,
        info: MirrorSession,
        stop: tokio::sync::oneshot::Sender<()>,
        control: ControlChannel,
    ) {
        if let Ok(mut guard) = self.sessions.write() {
            guard.insert(
                info.id,
                SessionHandle {
                    info,
                    stop,
                    control,
                },
            );
        }
    }

    pub fn session_control(&self, id: u64) -> Option<ControlChannel> {
        self.sessions
            .read()
            .ok()
            .and_then(|guard| guard.get(&id).map(|handle| handle.control.clone()))
    }

    pub fn remove_session(&self, id: u64) {
        if let Ok(mut guard) = self.sessions.write() {
            guard.remove(&id);
        }
        if let Ok(mut guard) = self.pending_sessions.write() {
            guard.remove(&id);
        }
        if let Ok(mut guard) = self.aspects.write() {
            guard.remove(&id);
        }
        if let Ok(mut guard) = self.streaming.write() {
            guard.remove(&id);
        }
        if let Ok(mut guard) = self.phone_screens.write() {
            guard.remove(&id);
        }
    }

    pub fn take_session(&self, id: u64) -> Option<SessionHandle> {
        self.sessions
            .write()
            .ok()
            .and_then(|mut guard| guard.remove(&id))
    }

    pub fn drain_sessions(&self) -> Vec<SessionHandle> {
        self.sessions
            .write()
            .map(|mut guard| guard.drain().map(|(_, handle)| handle).collect())
            .unwrap_or_default()
    }

    pub fn selected_file(&self) -> PathBuf {
        self.app_data_dir.join("selected-device.json")
    }

    /// The device the user last selected (mirrored to the tray), if any.
    pub fn selected_serial(&self) -> Option<String> {
        self.selected.read().ok().and_then(|guard| guard.clone())
    }

    pub fn set_selected_serial(&self, serial: Option<String>) -> crate::error::Result<()> {
        {
            let mut guard = self
                .selected
                .write()
                .map_err(|_| crate::error::AppError::Other("state poisoned".into()))?;
            *guard = serial.clone();
        }
        util::write_json(&self.selected_file(), &serial)
    }

    pub fn overrides(&self) -> HashMap<String, String> {
        self.tool_overrides
            .read()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    pub fn set_override(&self, tool: ToolKind, path: Option<String>) -> crate::error::Result<()> {
        {
            let mut guard = self
                .tool_overrides
                .write()
                .map_err(|_| crate::error::AppError::Other("state poisoned".into()))?;
            match path {
                Some(value) => guard.insert(tool.name().to_string(), value),
                None => guard.remove(tool.name()),
            };
        }
        util::write_json(&self.overrides_file(), &self.overrides())
    }

    /// Abort a running device tracker before replacing it.
    pub fn stop_tracker(&self) {
        if let Ok(mut guard) = self.tracker.write() {
            if let Some(handle) = guard.take() {
                handle.abort();
            }
        }
    }

    pub fn set_tracker(&self, handle: tauri::async_runtime::JoinHandle<()>) {
        if let Ok(mut guard) = self.tracker.write() {
            *guard = Some(handle);
        }
    }

    /// Track a device's logcat streamer, aborting any previous one.
    pub fn set_logcat(&self, serial: &str, handle: tauri::async_runtime::JoinHandle<()>) {
        if let Ok(mut guard) = self.logcats.write() {
            if let Some(previous) = guard.insert(serial.to_string(), handle) {
                previous.abort();
            }
        }
    }

    pub fn stop_logcat(&self, serial: &str) {
        if let Ok(mut guard) = self.logcats.write() {
            if let Some(handle) = guard.remove(serial) {
                handle.abort();
            }
        }
    }

    /// Claim the single wireless-pairing slot. Returns a guard that releases it
    /// on drop, or an error when a session is already in progress.
    pub fn begin_pairing(&self) -> crate::error::Result<PairingGuard<'_>> {
        self.pairing
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .map(|_| PairingGuard(&self.pairing))
            .map_err(|_| {
                crate::error::AppError::Other("A pairing session is already in progress".into())
            })
    }

    /// Register a cancel channel for a new QR pairing session, cancelling the
    /// previous session (if any) so the new one can take over the slot.
    pub fn arm_pairing_cancel(&self) -> tokio::sync::oneshot::Receiver<()> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        if let Ok(mut guard) = self.pairing_cancel.write() {
            if let Some(previous) = guard.replace(tx) {
                let _ = previous.send(());
            }
        }
        rx
    }

    pub fn cancel_pairing(&self) {
        if let Ok(mut guard) = self.pairing_cancel.write() {
            if let Some(tx) = guard.take() {
                let _ = tx.send(());
            }
        }
    }
}

/// RAII release for the slot claimed by [`AppState::begin_pairing`].
pub struct PairingGuard<'a>(&'a AtomicBool);

impl Drop for PairingGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

fn load_overrides(app_data_dir: &Path) -> crate::error::Result<HashMap<String, String>> {
    let path = app_data_dir.join("tool-paths.json");
    Ok(util::read_json::<HashMap<String, String>>(&path)?.unwrap_or_default())
}

fn load_selected(app_data_dir: &Path) -> Option<String> {
    let path = app_data_dir.join("selected-device.json");
    util::read_json::<Option<String>>(&path)
        .ok()
        .flatten()
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_device_holds_one_link_and_a_stale_finish_keeps_its_replacement() {
        let links = DeviceLinks::default();
        let (first, first_rx) = tokio::sync::oneshot::channel();
        let id = links.insert("phone", first).unwrap();
        let (duplicate, _) = tokio::sync::oneshot::channel();
        assert!(links.insert("phone", duplicate).is_none());

        links.stop("phone");
        assert!(first_rx.blocking_recv().is_ok());
        let (second, _) = tokio::sync::oneshot::channel();
        let replacement = links.insert("phone", second).unwrap();

        links.finished("phone", id);
        assert!(links.contains("phone"));
        links.finished("phone", replacement);
        assert!(!links.contains("phone"));
    }
}
