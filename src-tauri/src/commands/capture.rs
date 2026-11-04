//! Screenshots and recordings from mirror windows.

use tauri_plugin_opener::OpenerExt;

use crate::error::{AppError, Result};
use crate::services::capture::{self, CaptureDirs};

#[tauri::command]
pub fn capture_default_dirs() -> Result<CaptureDirs> {
    capture::default_dirs()
}

#[tauri::command]
pub fn capture_open_folder(dir: String, app: tauri::AppHandle) -> Result<()> {
    std::fs::create_dir_all(&dir)
        .map_err(|error| AppError::Io(format!("Could not open {dir}: {error}")))?;
    app.opener()
        .open_path(&dir, None::<&str>)
        .map_err(|error| AppError::Other(error.to_string()))
}
