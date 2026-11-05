use std::path::Path;

use tauri::{AppHandle, Emitter, State};

use crate::error::Result;
use crate::models::{AppInstallProgress, AppListResult};
use crate::services::apps as apps_service;
use crate::state::AppState;
use crate::util::validation::{validate_package, validate_serial};

const EVENT_INSTALL_PROGRESS: &str = "install:progress";

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
pub async fn install_apk(
    serial: String,
    path: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<()> {
    validate_serial(&serial)?;
    let tools = state.tools();

    // `adb install` has no machine-readable progress; report the file being
    // installed at the start and a completion line at the end.
    let name = Path::new(&path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("apk")
        .to_string();
    let _ = app.emit(
        EVENT_INSTALL_PROGRESS,
        AppInstallProgress {
            serial: serial.clone(),
            package: name.clone(),
            progress: 0,
            message: format!("Installing {name}…"),
        },
    );

    apps_service::install_apk(&tools, &serial, &path).await?;

    let _ = app.emit(
        EVENT_INSTALL_PROGRESS,
        AppInstallProgress {
            serial,
            package: name.clone(),
            progress: 100,
            message: format!("Installed {name}"),
        },
    );

    Ok(())
}
