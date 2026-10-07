//! Tool discovery commands (thin wrappers over `services::tools`).

use tauri::{AppHandle, State};

use crate::error::{AppError, Result};
use crate::models::ToolStatus;
use crate::services::{tools, tracker};
use crate::state::AppState;

#[tauri::command]
pub fn get_tools_status(state: State<'_, AppState>) -> ToolStatus {
    state.tools()
}

#[tauri::command]
pub async fn set_tool_path(
    tool: String,
    path: Option<String>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<ToolStatus> {
    if tool != "adb" && tool != "scrcpy" {
        return Err(AppError::Invalid(format!("unknown tool: {tool}")));
    }

    state.set_override(&tool, path)?;

    let overrides = state.overrides();
    let status = tools::resolve(
        overrides.get("adb").map(String::as_str),
        overrides.get("scrcpy").map(String::as_str),
    )
    .await;
    state.set_tools(status.clone());

    // The adb path may have changed — restart the tracker against the new one.
    state.stop_tracker();
    state.set_tracker(tracker::spawn(app));

    Ok(status)
}

#[tauri::command]
pub fn get_scrcpy_version(state: State<'_, AppState>) -> Option<String> {
    state.tools().scrcpy.version
}
