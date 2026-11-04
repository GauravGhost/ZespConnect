//! Background device tracker: runs `adb track-devices` and re-publishes the device list
//! on change, retrying if the stream ends (the frontend poll is the final safety net).

use std::process::Stdio;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use tokio::io::AsyncReadExt;

use crate::services::{adb, devices};
use crate::state::AppState;
use crate::util::process;

const RETRY_DELAY: Duration = Duration::from_secs(2);
const NO_ADB_DELAY: Duration = Duration::from_secs(3);
const EVENT_DEVICES_CHANGED: &str = "devices:changed";

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
        let _ = app.emit(EVENT_DEVICES_CHANGED, &list);
        crate::tray::sync(app);
        crate::runtime::reconnect::devices_changed();
    }
}

/// Read length-prefixed frames from `adb track-devices`; each frame means the device set changed.
async fn track_stream(app: &AppHandle, adb_path: &str) -> crate::error::Result<()> {
    let mut child = process::base_command(adb_path, &["track-devices".to_string()])
        .stderr(Stdio::null())
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
