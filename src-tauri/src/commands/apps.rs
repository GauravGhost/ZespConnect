//! App commands (thin wrappers over `services::apps`).

use tauri::State;

use crate::error::Result;
use crate::models::AppListResult;
use crate::services::apps as apps_service;
use crate::state::AppState;
use crate::util::validation::{validate_package, validate_serial};

#[tauri::command]
pub async fn list_apps(
    serial: String,
    include_system: bool,
    refresh: bool,
    state: State<'_, AppState>,
) -> Result<AppListResult> {
    validate_serial(&serial)?;
    let tools = state.tools();
    apps_service::list(
        &tools,
        &serial,
        include_system,
        refresh,
        &state.app_data_dir(),
    )
    .await
}

#[tauri::command]
pub async fn launch_app(serial: String, package: String, state: State<'_, AppState>) -> Result<()> {
    validate_serial(&serial)?;
    validate_package(&package)?;
    let tools = state.tools();
    apps_service::launch(&tools, &serial, &package).await
}

#[tauri::command]
pub async fn uninstall_app(
    serial: String,
    package: String,
    state: State<'_, AppState>,
) -> Result<()> {
    validate_serial(&serial)?;
    validate_package(&package)?;
    let tools = state.tools();
    apps_service::uninstall(&tools, &serial, &package).await
}

#[tauri::command]
pub async fn clear_app_data(
    serial: String,
    package: String,
    state: State<'_, AppState>,
) -> Result<()> {
    validate_serial(&serial)?;
    validate_package(&package)?;
    let tools = state.tools();
    apps_service::clear_data(&tools, &serial, &package).await
}

#[tauri::command]
pub async fn install_apk(serial: String, path: String, state: State<'_, AppState>) -> Result<()> {
    validate_serial(&serial)?;
    let tools = state.tools();
    apps_service::install_apk(&tools, &serial, &path).await
}
