//! Tool lifecycle: re-resolve, update-check and install, driving managed state and progress events.

use std::path::Path;

use tauri::{AppHandle, Emitter, Manager};

use crate::error::Result;
use crate::models::{InstallDone, InstallProgress, ToolKind, ToolStatus, UpdateInfo};
use crate::runtime::tracker;
use crate::services::{install, tools, updates};
use crate::state::AppState;

const EVENT_INSTALL_PROGRESS: &str = "tools:install:progress";
const EVENT_INSTALL_DONE: &str = "tools:install:done";

pub async fn set_tool_path(
    app: &AppHandle,
    tool: ToolKind,
    path: Option<String>,
) -> Result<ToolStatus> {
    app.state::<AppState>().set_override(tool, path)?;
    refresh(app).await
}

/// Compare the installed tools against the latest upstream releases.
pub async fn check_for_updates(app: &AppHandle) -> Result<ToolStatus> {
    let latest = updates::fetch_latest().await;

    let state = app.state::<AppState>();
    let mut status = state.tools();

    // adb reports its protocol version (`1.0.41`); compare platform-tools releases (`37.0.1`).
    let installed_adb = match state.adb_path() {
        Some(path) => tools::platform_tools_version(&path).await,
        None => None,
    };

    status.adb_update = UpdateInfo {
        outdated: latest
            .adb
            .as_deref()
            .map(|latest| updates::is_outdated(installed_adb.as_deref(), latest))
            .unwrap_or(false),
        latest_version: latest.adb,
        error: latest.adb_error,
    };
    status.scrcpy_update = UpdateInfo {
        outdated: latest
            .scrcpy
            .as_deref()
            .map(|latest| updates::is_outdated(status.scrcpy.version.as_deref(), latest))
            .unwrap_or(false),
        latest_version: latest.scrcpy,
        error: latest.scrcpy_error,
    };

    state.set_tools(status.clone());
    Ok(status)
}

/// Resolve both tools, honouring the user's saved path overrides.
pub async fn resolve(state: &AppState) -> ToolStatus {
    let overrides = state.overrides();
    let path_for = |tool: ToolKind| overrides.get(tool.name()).map(String::as_str);
    tools::resolve(
        path_for(ToolKind::Adb),
        path_for(ToolKind::Scrcpy),
        &state.app_data_dir(),
    )
    .await
}

/// Re-resolve tools and restart the device tracker; the last update check is preserved.
pub async fn refresh(app: &AppHandle) -> Result<ToolStatus> {
    let state = app.state::<AppState>();
    let previous = state.tools();

    let mut status = resolve(&state).await;
    status.adb_update = previous.adb_update;
    status.scrcpy_update = previous.scrcpy_update;

    state.set_tools(status.clone());
    state.stop_tracker();
    state.set_tracker(tracker::spawn(app.clone()));

    Ok(status)
}

/// Download one tool, point the override at it, re-resolve, and stream progress to the UI.
pub async fn perform_install(app: &AppHandle, tool: ToolKind) -> Result<ToolStatus> {
    let managed_dir = app.state::<AppState>().app_data_dir();
    let tool_name = tool.name().to_string();

    let emitter = app.clone();
    let name = tool_name.clone();
    let outcome = install::install(&managed_dir, tool, move |line| {
        let _ = emitter.emit(
            EVENT_INSTALL_PROGRESS,
            InstallProgress {
                tool: name.clone(),
                line,
            },
        );
    })
    .await;

    let binary = match outcome {
        Ok(binary) => binary,
        Err(error) => {
            let message = error.to_string();
            let _ = app.emit(
                EVENT_INSTALL_DONE,
                InstallDone {
                    tool: tool_name,
                    ok: false,
                    message: message.clone(),
                },
            );
            return Err(error);
        }
    };

    // Remember the downloaded binary so it wins over a system install.
    app.state::<AppState>()
        .set_override(tool, Some(binary.to_string_lossy().to_string()))?;

    let status = settle_after_install(app, tool, &binary).await?;
    let _ = app.emit(
        EVENT_INSTALL_DONE,
        InstallDone {
            tool: tool_name.clone(),
            ok: true,
            message: format!("{tool_name} is ready"),
        },
    );
    Ok(status)
}

/// Re-resolve after a download and mark the freshly installed tool as current.
async fn settle_after_install(
    app: &AppHandle,
    tool: ToolKind,
    binary: &Path,
) -> Result<ToolStatus> {
    let mut status = refresh(app).await?;

    let latest_version = match tool {
        ToolKind::Adb => tools::platform_tools_version(&binary.to_string_lossy()).await,
        ToolKind::Scrcpy => status.scrcpy.version.clone(),
    };
    let update = UpdateInfo {
        latest_version,
        outdated: false,
        error: None,
    };
    match tool {
        ToolKind::Adb => status.adb_update = update,
        ToolKind::Scrcpy => status.scrcpy_update = update,
    }

    app.state::<AppState>().set_tools(status.clone());
    Ok(status)
}
