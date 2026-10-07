use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use crate::models::{AppSettings, Device, ToolStatus};
use crate::util;

/// Shared, process-wide state. Guards are only ever held for short,
/// synchronous critical sections (never across `.await`).
pub struct AppState {
    tools: RwLock<ToolStatus>,
    tool_overrides: RwLock<HashMap<String, String>>,
    settings: RwLock<AppSettings>,
    devices: RwLock<Vec<Device>>,
    app_data_dir: PathBuf,
    tracker: RwLock<Option<tauri::async_runtime::JoinHandle<()>>>,
}

impl AppState {
    pub fn new(app_data_dir: PathBuf) -> Self {
        let overrides = load_overrides(&app_data_dir).unwrap_or_default();
        let settings = load_settings(&app_data_dir).unwrap_or_default();
        AppState {
            tools: RwLock::new(ToolStatus::default()),
            tool_overrides: RwLock::new(overrides),
            settings: RwLock::new(settings),
            devices: RwLock::new(Vec::new()),
            app_data_dir,
            tracker: RwLock::new(None),
        }
    }

    pub fn app_data_dir(&self) -> PathBuf {
        self.app_data_dir.clone()
    }

    pub fn overrides_file(&self) -> PathBuf {
        self.app_data_dir.join("tool-paths.json")
    }

    pub fn settings_file(&self) -> PathBuf {
        self.app_data_dir.join("settings.json")
    }

    pub fn settings(&self) -> AppSettings {
        self.settings
            .read()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    pub fn set_settings(&self, settings: AppSettings) -> crate::error::Result<()> {
        {
            let mut guard = self
                .settings
                .write()
                .map_err(|_| crate::error::AppError::Other("state poisoned".into()))?;
            *guard = settings;
        }
        util::write_json(&self.settings_file(), &self.settings())
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
}

fn load_overrides(app_data_dir: &Path) -> crate::error::Result<HashMap<String, String>> {
    let path = app_data_dir.join("tool-paths.json");
    Ok(util::read_json::<HashMap<String, String>>(&path)?.unwrap_or_default())
}

fn load_settings(app_data_dir: &Path) -> crate::error::Result<AppSettings> {
    let path = app_data_dir.join("settings.json");
    Ok(util::read_json::<AppSettings>(&path)?.unwrap_or_default())
}
