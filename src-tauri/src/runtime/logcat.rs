use tauri::ipc::Channel;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};

use crate::error::{AppError, Result};
use crate::models::{LogLine, LogStream, LogcatFilter};
use crate::services::{adb, console};
use crate::state::AppState;
use crate::util::validation::validate_package;

/// Stream `serial`'s logcat into `channel` until [`AppState::stop_logcat`] runs or the window goes
/// away (the emitted lines stop once its channel is dropped). Replaces any stream already running.
pub async fn open(
    state: &AppState,
    serial: &str,
    filter: LogcatFilter,
    channel: Channel<LogLine>,
) -> Result<()> {
    let adb_path = adb::adb_path(&state.tools())?;
    let epoch = state.stop_logcat(serial);

    let min_level = filter.min_level.map(console::validate_level).transpose()?;
    let scope = match filter.package.as_deref() {
        Some(package) => {
            validate_package(package)?;
            let sdk = state
                .device(serial)
                .and_then(|device| device.sdk)
                .and_then(|sdk| sdk.parse().ok())
                .unwrap_or(0);
            Some(console::app_scope(&adb_path, serial, package, sdk).await?)
        }
        None => None,
    };
    let args = console::logcat_args(scope.as_ref(), min_level);

    let mut child = console::spawn_logcat(&adb_path, serial, &args)?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| AppError::Other("no stdout from logcat".into()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| AppError::Other("no stderr from logcat".into()))?;

    let err_task =
        tauri::async_runtime::spawn(forward_lines(stderr, LogStream::Stderr, channel.clone()));
    let handle = tauri::async_runtime::spawn(async move {
        forward_lines(stdout, LogStream::Stdout, channel).await;
        err_task.abort();
        // Dropping the child kills the logcat process (`kill_on_drop`).
        drop(child);
    });

    state.set_logcat(serial, epoch, handle);
    Ok(())
}

async fn forward_lines<R: AsyncRead + Unpin>(
    reader: R,
    stream: LogStream,
    channel: Channel<LogLine>,
) {
    let mut lines = BufReader::new(reader).lines();
    while let Ok(Some(text)) = lines.next_line().await {
        if channel.send(LogLine { stream, text }).is_err() {
            break;
        }
    }
}
