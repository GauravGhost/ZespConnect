//! Tool lifecycle: re-resolve, update-check and install, driving managed state
//! and progress events. Tauri-coupled, unlike the pure `services::tools`.

use std::path::Path;

use tauri::{AppHandle, Emitter, Manager};

use crate::error::{AppError, Result};
use crate::models::{InstallDone, InstallProgress, ToolKind, ToolStatus, UpdateInfo};
use crate::runtime::tracker;
use crate::services::{install, tools, updates};
use crate::state::AppState;

const EVENT_INSTALL_PROGRESS: &str = "tools:install:progress";
const EVENT_INSTALL_DONE: &str = "tools:install:done";

pub async fn set_tool_path(
    app: &AppHandle,
    tool: &str,
    path: Option<String>,
) -> Result<ToolStatus> {
    if tool != "adb" && tool != "scrcpy" {
        return Err(AppError::Invalid(format!("unknown tool: {tool}")));
    }
    app.state::<AppState>().set_override(tool, path)?;
    refresh(app).await
}

/// Compare the installed tools against the latest upstream releases.
pub async fn check_for_updates(app: &AppHandle) -> Result<ToolStatus> {
    let latest = updates::fetch_latest().await;

    let state = app.state::<AppState>();
    let mut status = state.tools();

    // adb's own version string is the ADB protocol version (`1.0.41`); the
    // comparable value is the platform-tools release (`37.0.1`).
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

/// Re-resolve tools and restart the device tracker (the adb path may have
/// changed). The last update check is preserved so badges survive a re-scan.
pub async fn refresh(app: &AppHandle) -> Result<ToolStatus> {
    let state = app.state::<AppState>();
    let overrides = state.overrides();
    let managed_dir = state.app_data_dir();
    let previous = state.tools();

    let mut status = tools::resolve(
        overrides.get("adb").map(String::as_str),
        overrides.get("scrcpy").map(String::as_str),
        &managed_dir,
    )
    .await;
    status.adb_update = previous.adb_update;
    status.scrcpy_update = previous.scrcpy_update;

    state.set_tools(status.clone());
    state.stop_tracker();
    state.set_tracker(tracker::spawn(app.clone()));

    Ok(status)
}

/// Download one tool from upstream, point the override at it, then re-resolve.
/// Streams each progress line to the UI and finishes with a done event.
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
        .set_override(&tool_name, Some(binary.to_string_lossy().to_string()))?;

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
