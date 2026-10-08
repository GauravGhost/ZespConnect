//! Wireless debugging commands (Android 11+ pairing and Wi-Fi transport).

use std::time::Duration;

use tauri::{AppHandle, State};

use crate::error::Result;
use crate::models::{MdnsDevice, QrPayload};
use crate::services::{adb, tracker, wireless};
use crate::state::AppState;
use crate::util::validation::validate_serial;

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

/// Pair using an explicit `host:port` and the code shown on the phone.
#[tauri::command]
pub async fn pair_wireless_code(
    addr: String,
    code: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String> {
    let _guard = state.begin_pairing()?;

    let adb_path = adb::adb_path(&state.tools())?;
    let address = wireless::pair_code(&adb_path, &addr, &code).await?;
    tracker::refresh(&app).await;
    Ok(address)
}

/// `adb connect <addr>` — bring up a phone already enabled for wireless debugging.
#[tauri::command]
pub async fn connect_wireless(
    addr: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String> {
    let adb_path = adb::adb_path(&state.tools())?;
    let address = wireless::connect(&adb_path, &addr).await?;
    tracker::refresh(&app).await;
    Ok(address)
}

/// `adb disconnect <addr>`.
#[tauri::command]
pub async fn disconnect_wireless(
    addr: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<()> {
    let adb_path = adb::adb_path(&state.tools())?;
    wireless::disconnect(&adb_path, &addr).await?;
    tracker::refresh(&app).await;
    Ok(())
}

/// Switch a USB-connected device to TCP mode and connect to it over Wi-Fi.
/// Returns the connected `ip:port`.
#[tauri::command]
pub async fn enable_wireless_tcpip(
    serial: String,
    port: Option<u16>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String> {
    validate_serial(&serial)?;

    let adb_path = adb::adb_path(&state.tools())?;
    let port = port.unwrap_or(wireless::DEFAULT_TCPIP_PORT);
    let address = wireless::enable_tcpip(&adb_path, &serial, port).await?;
    tracker::refresh(&app).await;
    Ok(address)
}

/// The adb pairing/connect services currently advertised on the network.
#[tauri::command]
pub async fn list_mdns_services(state: State<'_, AppState>) -> Result<Vec<MdnsDevice>> {
    let adb_path = adb::adb_path(&state.tools())?;
    wireless::mdns_devices(&adb_path).await
}
