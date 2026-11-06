use tauri::ipc::Channel;
use tauri::{AppHandle, State};

use crate::error::Result;
use crate::models::{LogLine, LogcatFilter, ShellEvent};
use crate::runtime::{logcat, shell};
use crate::state::AppState;
use crate::util::validation::validate_serial;

/// Start an interactive shell streaming into `channel`; returns the id later calls address it by.
#[tauri::command]
pub fn open_shell(
    serial: String,
    channel: Channel<ShellEvent>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<u64> {
    validate_serial(&serial)?;
    shell::open(&app, &state, &serial, channel)
}

/// Send raw input to a shell: a command line ends with a newline, Ctrl+C is the byte 0x03.
#[tauri::command]
pub fn write_shell(id: u64, input: String, state: State<'_, AppState>) -> Result<()> {
    state.shells().write(id, input)
}

#[tauri::command]
pub fn close_shell(id: u64, state: State<'_, AppState>) {
    state.shells().close(id);
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
