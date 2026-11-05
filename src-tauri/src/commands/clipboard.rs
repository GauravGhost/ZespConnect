use tauri::{AppHandle, State};

use crate::error::Result;
use crate::runtime::clipboard_sync;
use crate::state::AppState;
use crate::util::validation::validate_serial;

#[tauri::command]
pub fn list_clipboard_sync(state: State<'_, AppState>) -> Vec<String> {
    state.clipboard_sync().serials()
}

#[tauri::command]
pub async fn set_clipboard_sync(serial: String, on: bool, app: AppHandle) -> Result<()> {
    validate_serial(&serial)?;
    if on {
        clipboard_sync::enable(&app, &serial).await
    } else {
        clipboard_sync::disable(&app, &serial)
    }
}
