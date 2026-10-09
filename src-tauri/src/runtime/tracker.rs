//! Background device tracker.
//!
//! Runs `adb track-devices` (a long-lived, push-based stream) and re-publishes
//! the device list whenever it changes. Falls back to periodic refresh if the
//! stream ends. The frontend query poll is a final safety net.

use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use tokio::io::AsyncReadExt;

use crate::platform;
use crate::services::{adb, devices};
use crate::state::AppState;

const RETRY_DELAY: Duration = Duration::from_secs(2);
const NO_ADB_DELAY: Duration = Duration::from_secs(3);

pub fn spawn(app: AppHandle) -> tauri::async_runtime::JoinHandle<()> {
    tauri::async_runtime::spawn(run(app))
}

async fn run(app: AppHandle) {
    loop {
        let adb_path = app.state::<AppState>().adb_path();

        let Some(adb_path) = adb_path else {
            tokio::time::sleep(NO_ADB_DELAY).await;
            continue;
        };

        let _ = adb::start_server(&adb_path).await;
        refresh(&app).await;

        // Track until the stream ends, then retry.
        let _ = track_stream(&app, &adb_path).await;
        tokio::time::sleep(RETRY_DELAY).await;
    }
}

/// Fetch the full list, store it, notify the UI, and refresh the tray menu.
pub async fn refresh(app: &AppHandle) {
    let tools = app.state::<AppState>().tools();
    if let Ok(list) = devices::list(&tools).await {
        app.state::<AppState>().set_devices(list.clone());
        let _ = app.emit("devices:changed", &list);
        crate::tray::sync(app);
    }
}

/// Read length-prefixed frames from `adb track-devices`; on each frame the
/// device set changed, so refresh and publish. Returns when the stream ends.
async fn track_stream(app: &AppHandle, adb_path: &str) -> crate::error::Result<()> {
    use std::process::{Command as StdCommand, Stdio};

    let (program, args) = platform::prepare(adb_path, &["track-devices".to_string()]);

    let mut std_cmd = StdCommand::new(program);
    std_cmd
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    platform::configure_command(&mut std_cmd);

    let mut child = tokio::process::Command::from(std_cmd)
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| crate::error::AppError::CommandFailed(e.to_string()))?;

    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| crate::error::AppError::Other("no stdout from track-devices".into()))?;

    loop {
        // Each frame is a 4-character hex length followed by that many bytes.
        let mut length_bytes = [0u8; 4];
        stdout
            .read_exact(&mut length_bytes)
            .await
            .map_err(|e| crate::error::AppError::Io(e.to_string()))?;

        let length_text = String::from_utf8_lossy(&length_bytes);
        let length = usize::from_str_radix(length_text.trim(), 16).map_err(|_| {
            crate::error::AppError::Parse(format!("bad frame length: {length_text:?}"))
        })?;

        let mut payload = vec![0u8; length];
        stdout
            .read_exact(&mut payload)
            .await
            .map_err(|e| crate::error::AppError::Io(e.to_string()))?;

        refresh(app).await;
    }
}
