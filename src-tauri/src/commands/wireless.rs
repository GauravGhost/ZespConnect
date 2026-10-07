//! Wireless debugging commands (Android 11+ QR pairing).

use std::time::Duration;

use tauri::{AppHandle, State};

use crate::error::Result;
use crate::models::QrPayload;
use crate::services::{adb, tracker, wireless};
use crate::state::AppState;

/// Generate a fresh pairing QR payload to render in the UI.
#[tauri::command]
pub fn get_qr_payload() -> QrPayload {
    wireless::generate()
}

/// Wait for the phone to scan `payload`, pair it, and connect it over Wi-Fi.
/// Returns the connected `host:port`.
#[tauri::command]
pub async fn pair_wireless_qr(
    payload: String,
    timeout: Option<u64>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String> {
    // One session at a time — they would otherwise fight over the adb server.
    let _guard = state.begin_pairing()?;

    let adb_path = adb::adb_path(&state.tools())?;
    let secs = timeout.unwrap_or(180).clamp(30, 300);
    let address =
        wireless::pair_and_connect(&adb_path, &payload, Duration::from_secs(secs)).await?;

    // Reuse the tracker's refresh so the new device reaches the UI immediately.
    tracker::refresh(&app).await;

    Ok(address)
}
