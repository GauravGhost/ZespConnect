//! Device file-browser commands (thin wrappers over `services::files`).

use tauri::State;

use crate::error::Result;
use crate::models::FileEntry;
use crate::services::files as files_service;
use crate::state::AppState;
use crate::util::validation::validate_serial;

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
