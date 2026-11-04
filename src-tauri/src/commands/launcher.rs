//! The app launcher's settings and popup window.

use tauri::{AppHandle, State};

use crate::error::Result;
use crate::models::{AppInfo, LauncherSettings, LauncherStatus, LauncherTarget};
use crate::runtime::launcher;
use crate::services;
use crate::state::AppState;
use crate::util::validation::validate_serial;

#[tauri::command]
pub fn get_launcher_status(app: AppHandle) -> LauncherStatus {
    launcher::status(&app)
}

#[tauri::command]
pub fn set_launcher_settings(settings: LauncherSettings, app: AppHandle) -> Result<LauncherStatus> {
    launcher::save_settings(&app, settings)
}

/// The phone the popup acts on: the one selected in the dashboard and tray.
#[tauri::command]
pub fn launcher_target(state: State<'_, AppState>) -> LauncherTarget {
    services::launcher::resolve_target(state.selected_serial().as_deref(), &state.devices())
}

#[tauri::command]
pub async fn launcher_apps(serial: String, app: AppHandle) -> Result<Vec<AppInfo>> {
    validate_serial(&serial)?;
    launcher::apps(&app, &serial).await
}

#[tauri::command]
pub fn hide_launcher(app: AppHandle) {
    launcher::hide(&app);
}
