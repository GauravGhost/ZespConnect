use tauri::{AppHandle, State};

use crate::error::Result;
use crate::models::{ToolKind, ToolStatus};
use crate::runtime::tools;
use crate::state::AppState;

#[tauri::command]
pub fn get_tools_status(state: State<'_, AppState>) -> ToolStatus {
    state.tools()
}

#[tauri::command]
pub async fn set_tool_path(
    tool: ToolKind,
    path: Option<String>,
    app: AppHandle,
) -> Result<ToolStatus> {
    tools::set_tool_path(&app, tool, path).await
}

#[tauri::command]
pub async fn install_tool(tool: ToolKind, app: AppHandle) -> Result<ToolStatus> {
    tools::perform_install(&app, tool).await
}

#[tauri::command]
pub async fn check_for_updates(app: AppHandle) -> Result<ToolStatus> {
    tools::check_for_updates(&app).await
}
