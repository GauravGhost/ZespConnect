//! Mirroring commands (thin wrappers over `services::mirror`).

use tauri::{AppHandle, State};

use crate::error::Result;
use crate::models::MirrorProcess;
use crate::services::mirror::{self, AppDisplay};
use crate::state::AppState;
use crate::util::validation::{validate_package, validate_serial};

/// Open an app in its own window on the laptop (main-display fallback on 12–13).
#[tauri::command]
pub async fn start_app_window(
    serial: String,
    package: String,
    label: String,
    app: AppHandle,
) -> Result<MirrorProcess> {
    validate_serial(&serial)?;
    validate_package(&package)?;
    mirror::start_app(&app, &serial, &package, &label, AppDisplay::Window).await
}

/// Mirror the main display with the app in the foreground.
#[tauri::command]
pub async fn start_app_on_screen(
    serial: String,
    package: String,
    label: String,
    app: AppHandle,
) -> Result<MirrorProcess> {
    validate_serial(&serial)?;
    validate_package(&package)?;
    mirror::start_app(&app, &serial, &package, &label, AppDisplay::Screen).await
}

#[tauri::command]
pub async fn stop_mirror(pid: u32, app: AppHandle) -> Result<()> {
    mirror::stop(&app, pid)
}

#[tauri::command]
pub async fn stop_all_mirrors(app: AppHandle) -> Result<()> {
    mirror::stop_all(&app);
    Ok(())
}

/// The scrcpy windows currently open.
#[tauri::command]
pub fn list_mirrors(state: State<'_, AppState>) -> Vec<MirrorProcess> {
    state.list_mirrors()
}
