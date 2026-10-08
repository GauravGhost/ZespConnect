use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::RwLock;

use crate::models::{Device, MirrorProcess, MirrorSession, MirrorSessionSpec, ToolStatus};
use crate::services::mirror_session::ControlChannel;
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
    next_session: AtomicU64,
    app_data_dir: PathBuf,
    tracker: RwLock<Option<tauri::async_runtime::JoinHandle<()>>>,
    logcats: RwLock<HashMap<String, tauri::async_runtime::JoinHandle<()>>>,
    pairing: AtomicBool,
    selected: RwLock<Option<String>>,
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
            next_session: AtomicU64::new(1),
            app_data_dir,
            tracker: RwLock::new(None),
            logcats: RwLock::new(HashMap::new()),
            pairing: AtomicBool::new(false),
            selected: RwLock::new(selected),
        }
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

    /// The scrcpy windows currently tracked.
    pub fn list_mirrors(&self) -> Vec<MirrorProcess> {
        self.mirrors
            .read()
            .map(|guard| guard.values().map(|handle| handle.info.clone()).collect())
            .unwrap_or_default()
    }

    /// Register a running scrcpy window and its stop channel.
    pub fn set_mirror(&self, info: MirrorProcess, stop: tokio::sync::oneshot::Sender<()>) {
        if let Ok(mut guard) = self.mirrors.write() {
            guard.insert(info.pid, MirrorHandle { info, stop });
        }
    }

    /// Drop a window's record (used by its watcher once the process ends).
    pub fn remove_mirror(&self, pid: u32) {
        if let Ok(mut guard) = self.mirrors.write() {
            guard.remove(&pid);
        }
    }

    /// Take a window's stop channel, if it is still tracked.
    pub fn take_mirror(&self, pid: u32) -> Option<MirrorHandle> {
        self.mirrors
            .write()
            .ok()
            .and_then(|mut guard| guard.remove(&pid))
    }

    /// Take every tracked window (used to stop them all).
    pub fn drain_mirrors(&self) -> Vec<MirrorHandle> {
        self.mirrors
            .write()
            .map(|mut guard| guard.drain().map(|(_, handle)| handle).collect())
            .unwrap_or_default()
    }

    /// Allocate the next in-app session id.
    pub fn next_session_id(&self) -> u64 {
        self.next_session.fetch_add(1, Ordering::SeqCst)
    }

    /// Remember a session's settings until its window asks for the stream.
    pub fn set_pending_session(&self, id: u64, spec: MirrorSessionSpec) {
        if let Ok(mut guard) = self.pending_sessions.write() {
            guard.insert(id, spec);
        }
    }

    /// Take a pending session's settings, if it has not been started yet.
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

    /// The in-app mirror sessions currently running.
    pub fn list_sessions(&self) -> Vec<MirrorSession> {
        self.sessions
            .read()
            .map(|guard| guard.values().map(|handle| handle.info.clone()).collect())
            .unwrap_or_default()
    }

    /// Register a running session, its stop channel, and its control channel.
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

    /// A handle that injects input into a running session, if it is still alive.
    pub fn session_control(&self, id: u64) -> Option<ControlChannel> {
        self.sessions
            .read()
            .ok()
            .and_then(|guard| guard.get(&id).map(|handle| handle.control.clone()))
    }

    /// Drop a session's record (used by its supervisor once it ends).
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
    }

    /// Take a session's stop channel, if it is still tracked.
    pub fn take_session(&self, id: u64) -> Option<SessionHandle> {
        self.sessions
            .write()
            .ok()
            .and_then(|mut guard| guard.remove(&id))
    }

    /// Take every tracked session (used to stop them all).
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

    pub fn override_for(&self, tool: &str) -> Option<String> {
        self.tool_overrides
            .read()
            .ok()
            .and_then(|guard| guard.get(tool).cloned())
    }

    pub fn overrides(&self) -> HashMap<String, String> {
        self.tool_overrides
            .read()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    pub fn set_override(&self, tool: &str, path: Option<String>) -> crate::error::Result<()> {
        {
            let mut guard = self
                .tool_overrides
                .write()
                .map_err(|_| crate::error::AppError::Other("state poisoned".into()))?;
            match path {
                Some(value) => guard.insert(tool.to_string(), value),
                None => guard.remove(tool),
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

    /// Stop a device's logcat streamer, if one is running.
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
