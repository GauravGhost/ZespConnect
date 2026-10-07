//! Tool discovery, installation and path commands.

use tauri::{AppHandle, Emitter, Manager, State};

use crate::error::{AppError, Result};
use crate::models::{InstallDone, InstallProgress, ToolKind, ToolStatus};
use crate::services::{install, tools, tracker};
use crate::state::AppState;

const EVENT_INSTALL_PROGRESS: &str = "tools:install:progress";
const EVENT_INSTALL_DONE: &str = "tools:install:done";

#[tauri::command]
pub fn get_tools_status(state: State<'_, AppState>) -> ToolStatus {
    state.tools()
}

#[tauri::command]
pub fn get_scrcpy_version(state: State<'_, AppState>) -> Option<String> {
    state.tools().scrcpy.version
}

#[tauri::command]
pub async fn set_tool_path(
    tool: String,
    path: Option<String>,
    app: AppHandle,
) -> Result<ToolStatus> {
    if tool != "adb" && tool != "scrcpy" {
        return Err(AppError::Invalid(format!("unknown tool: {tool}")));
    }
    app.state::<AppState>().set_override(&tool, path)?;
    refresh_tools(&app).await
}

#[tauri::command]
pub async fn install_tool(tool: ToolKind, app: AppHandle) -> Result<ToolStatus> {
    perform_install(&app, tool).await
}

/// Re-resolve tools and restart the device tracker (the adb path may have changed).
pub async fn refresh_tools(app: &AppHandle) -> Result<ToolStatus> {
    let overrides = app.state::<AppState>().overrides();
    let status = tools::resolve(
        overrides.get("adb").map(String::as_str),
        overrides.get("scrcpy").map(String::as_str),
    )
    .await;

    let state = app.state::<AppState>();
    state.set_tools(status.clone());
    state.stop_tracker();
    state.set_tracker(tracker::spawn(app.clone()));

    Ok(status)
}

/// Install one tool through the OS package manager, then re-resolve.
/// Streams each installer line to the UI and finishes with a done event.
pub async fn perform_install(app: &AppHandle, tool: ToolKind) -> Result<ToolStatus> {
    let plan = install::select(tool).ok_or_else(|| {
        AppError::Other(
            "No supported package manager was found (winget/scoop/choco, Homebrew, or \
             apt/dnf/pacman). Install the tool manually, then set its path in Settings."
                .into(),
        )
    })?;

    let tool_name = tool.name().to_string();
    let emitter = app.clone();
    let name = tool_name.clone();

    let outcome = install::run(&plan, move |line| {
        let _ = emitter.emit(
            EVENT_INSTALL_PROGRESS,
            InstallProgress {
                tool: name.clone(),
                line,
            },
        );
    })
    .await;

    // Some managers (notably winget) return a non-zero code for "already
    // installed / no upgrade", so trust the re-scan rather than the exit code.
    let install_error = outcome.err();
    let status = refresh_tools(app).await?;
    let found = match tool {
        ToolKind::Adb => status.adb.found,
        ToolKind::Scrcpy => status.scrcpy.found,
    };

    if found {
        let message = format!("{tool_name} is ready");
        let _ = app.emit(
            EVENT_INSTALL_DONE,
            InstallDone {
                tool: tool_name,
                ok: true,
                message,
            },
        );
        return Ok(status);
    }

    let message = install_error
        .map(|error| error.to_string())
        .unwrap_or_else(|| {
            format!("{tool_name} install finished, but it was not found — set its path in Settings")
        });
    let _ = app.emit(
        EVENT_INSTALL_DONE,
        InstallDone {
            tool: tool_name,
            ok: false,
            message: message.clone(),
        },
    );
    Err(AppError::Other(message))
}
