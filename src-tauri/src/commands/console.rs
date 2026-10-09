//! Console commands: `adb shell` execution and the in-app logcat streamer.

use tauri::ipc::Channel;
use tauri::State;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::error::{AppError, Result};
use crate::models::{LogLine, LogStream, ShellResult};
use crate::services::{adb, console as console_service};
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

/// Stream `adb logcat` into the calling window until [`close_logcat`] runs or the
/// window goes away (the emitted lines stop once its channel is dropped).
#[tauri::command]
pub async fn open_logcat(
    serial: String,
    channel: Channel<LogLine>,
    state: State<'_, AppState>,
) -> Result<()> {
    validate_serial(&serial)?;
    let tools = state.tools();
    let adb_path = adb::adb_path(&tools)?;

    // Only one stream per device; replace whatever was running.
    state.stop_logcat(&serial);

    let mut child = console_service::spawn_logcat(&adb_path, &serial)?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| AppError::Other("no stdout from logcat".into()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| AppError::Other("no stderr from logcat".into()))?;

    let err_channel = channel.clone();
    let err_task = tauri::async_runtime::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let event = LogLine {
                stream: LogStream::Stderr,
                text: line,
            };
            if err_channel.send(event).is_err() {
                break;
            }
        }
    });

    let handle = tauri::async_runtime::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let event = LogLine {
                stream: LogStream::Stdout,
                text: line,
            };
            if channel.send(event).is_err() {
                break;
            }
        }
        err_task.abort();
        // Dropping the child kills the logcat process (`kill_on_drop`).
        drop(child);
    });

    state.set_logcat(&serial, handle);
    Ok(())
}

#[tauri::command]
pub fn close_logcat(serial: String, state: State<'_, AppState>) -> Result<()> {
    validate_serial(&serial)?;
    state.stop_logcat(&serial);
    Ok(())
}
