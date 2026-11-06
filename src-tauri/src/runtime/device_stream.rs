use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Child;

use crate::error::{AppError, Result};
use crate::state::{AppState, DeviceLinks};

/// Which of [`AppState`]'s link sets a stream registers in.
pub type Links = fn(&AppState) -> &DeviceLinks;

/// Turn a long-running `adb shell` child's lines into values sent to `channel`, until the stream
/// is stopped through `links`, the phone loop ends, or the window drops its channel.
/// `on_line` returns a value whenever a line completes one. Replaces any stream already running.
pub fn open<T, F>(
    app: &AppHandle,
    links: Links,
    serial: &str,
    mut child: Child,
    channel: Channel<T>,
    mut on_line: F,
) -> Result<()>
where
    T: Serialize + Clone + Send + 'static,
    F: FnMut(&str) -> Option<T> + Send + 'static,
{
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| AppError::Other("no stdout from adb shell".into()))?;
    let state = app.state::<AppState>();
    links(&state).stop(serial);
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
    let Some(id) = links(&state).insert(serial, stop_tx) else {
        return Ok(());
    };

    let handle = app.clone();
    let serial = serial.to_string();
    tauri::async_runtime::spawn(async move {
        let forward = async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if let Some(value) = on_line(&line) {
                    if channel.send(value).is_err() {
                        break;
                    }
                }
            }
        };
        tokio::select! {
            _ = forward => {}
            _ = stop_rx => {}
        }
        // Dropping the child kills adb, which ends the loop on the phone.
        drop(child);
        links(&handle.state::<AppState>()).finished(&serial, id);
    });
    Ok(())
}
