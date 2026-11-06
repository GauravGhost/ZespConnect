use tauri::ipc::Channel;
use tauri::State;

use crate::error::Result;
use crate::models::{LogLine, LogcatFilter, ShellResult};
use crate::runtime::logcat;
use crate::services::console as console_service;
use crate::state::AppState;
use crate::util::validation::validate_serial;

#[tauri::command]
pub async fn shell_exec(
    serial: String,
    command: String,
    state: State<'_, AppState>,
) -> Result<ShellResult> {
    validate_serial(&serial)?;
    let tools = state.tools();
    console_service::exec(&tools, &serial, &command).await
}

/// Stream `adb logcat` into the calling window until [`close_logcat`] runs or the window goes away.
#[tauri::command]
pub async fn open_logcat(
    serial: String,
    filter: LogcatFilter,
    channel: Channel<LogLine>,
    state: State<'_, AppState>,
) -> Result<()> {
    validate_serial(&serial)?;
    logcat::open(&state, &serial, filter, channel).await
}

#[tauri::command]
pub fn close_logcat(serial: String, state: State<'_, AppState>) -> Result<()> {
    validate_serial(&serial)?;
    state.stop_logcat(&serial);
    Ok(())
}

/// Write exported log text to a path the user picked in a save dialog.
#[tauri::command]
pub async fn save_log(path: String, text: String) -> Result<()> {
    tokio::fs::write(&path, text).await?;
    Ok(())
}
