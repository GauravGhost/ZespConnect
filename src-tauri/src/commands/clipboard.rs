//! Clipboard commands (host ⇄ device, via the AdbClipboard helper app).

use tauri::State;

use crate::error::Result;
use crate::services::clipboard as clipboard_service;
use crate::state::AppState;
use crate::util::validation::validate_serial;

#[tauri::command]
pub async fn clipboard_send(
    serial: String,
    text: String,
    state: State<'_, AppState>,
) -> Result<()> {
    validate_serial(&serial)?;
    let tools = state.tools();
    clipboard_service::send(&tools, &serial, &text).await
}

#[tauri::command]
pub async fn clipboard_read(serial: String, state: State<'_, AppState>) -> Result<String> {
    validate_serial(&serial)?;
    let tools = state.tools();
    let text = clipboard_service::read(&tools, &serial).await?;
    clipboard_service::copy_to_host(&text)?;
    Ok(text)
}
