use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;

use crate::models::{Device, ToolStatus};
use crate::util;

/// Shared, process-wide state. Guards are only ever held for short,
/// synchronous critical sections (never across `.await`).
pub struct AppState {
    tools: RwLock<ToolStatus>,
    tool_overrides: RwLock<HashMap<String, String>>,
    devices: RwLock<Vec<Device>>,
    app_data_dir: PathBuf,
    tracker: RwLock<Option<tauri::async_runtime::JoinHandle<()>>>,
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
            app_data_dir,
            tracker: RwLock::new(None),
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
