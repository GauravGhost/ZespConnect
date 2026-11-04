//! Device file-browser commands (thin wrappers over `services::files`).

use tauri::{Manager, State};
use tauri_plugin_opener::OpenerExt;

use crate::error::{AppError, Result};
use crate::models::{FileEntry, SendOutcome};
use crate::services::{apps as apps_service, files as files_service};
use crate::state::AppState;
use crate::util::validation::validate_serial;

/// Cache subfolder holding device files copied over to open on the computer.
const PREVIEW_DIR: &str = "previews";

#[tauri::command]
pub async fn list_dir(
    serial: String,
    path: String,
    state: State<'_, AppState>,
) -> Result<Vec<FileEntry>> {
    validate_serial(&serial)?;
    let tools = state.tools();
    files_service::list(&tools, &serial, &path).await
}

#[tauri::command]
pub async fn pull(
    serial: String,
    remote: String,
    local: String,
    state: State<'_, AppState>,
) -> Result<()> {
    validate_serial(&serial)?;
    let tools = state.tools();
    files_service::pull(&tools, &serial, &remote, &local).await
}

#[tauri::command]
pub async fn push(
    serial: String,
    local: String,
    remote: String,
    state: State<'_, AppState>,
) -> Result<()> {
    validate_serial(&serial)?;
    let tools = state.tools();
    files_service::push(&tools, &serial, &local, &remote).await
}

/// Copy a device file into the app cache and open it with the computer's default app for its type.
#[tauri::command]
pub async fn open_on_computer(
    serial: String,
    remote: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<()> {
    validate_serial(&serial)?;
    let cache_root = preview_cache_root(&app)?;
    let tools = state.tools();
    let local = files_service::pull_for_preview(&tools, &serial, &remote, &cache_root).await?;
    app.opener()
        .open_path(local.to_string_lossy(), None::<&str>)
        .map_err(|error| AppError::Other(error.to_string()))
}

/// Where "open with default app" copies device files; cleared when the app starts and quits.
pub fn preview_cache_root(app: &tauri::AppHandle) -> Result<std::path::PathBuf> {
    app.path()
        .app_cache_dir()
        .map(|dir| dir.join(PREVIEW_DIR))
        .map_err(|error| AppError::Io(format!("No cache folder: {error}")))
}

/// A file dropped on a mirror window: an APK is installed, anything else copied to the drop folder.
#[tauri::command]
pub async fn send_to_phone(
    serial: String,
    path: String,
    state: State<'_, AppState>,
) -> Result<SendOutcome> {
    validate_serial(&serial)?;
    let tools = state.tools();
    if files_service::is_apk(&path) {
        apps_service::install_apk(&tools, &serial, &path).await?;
        Ok(SendOutcome::Installed)
    } else {
        files_service::send_to_drop_dir(&tools, &serial, &path).await?;
        Ok(SendOutcome::Copied)
    }
}

#[tauri::command]
pub async fn mkdir(serial: String, path: String, state: State<'_, AppState>) -> Result<()> {
    validate_serial(&serial)?;
    let tools = state.tools();
    files_service::create_dir(&tools, &serial, &path).await
}

#[tauri::command]
pub async fn delete(serial: String, path: String, state: State<'_, AppState>) -> Result<()> {
    validate_serial(&serial)?;
    let tools = state.tools();
    files_service::delete(&tools, &serial, &path).await
}
