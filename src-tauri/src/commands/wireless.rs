use std::time::Duration;

use tauri::{AppHandle, State};

use crate::error::{AppError, Result};
use crate::models::{MdnsDevice, QrPayload, ReconnectTarget};
use crate::runtime::{reconnect, tracker};
use crate::services::adb;
use crate::services::wireless::{self, Reconnect};
use crate::state::AppState;
use crate::util::validation::validate_serial;

const PAIRING_SLOT_ATTEMPTS: u32 = 20;
const PAIRING_SLOT_RETRY: Duration = Duration::from_millis(100);
const QR_TIMEOUT_DEFAULT_SECS: u64 = 180;
const QR_TIMEOUT_MIN_SECS: u64 = 30;
const QR_TIMEOUT_MAX_SECS: u64 = 300;

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
    // A newer QR session supersedes an older one (e.g. the popup was reopened).
    let mut cancel = state.arm_pairing_cancel();

    // One session at a time - they would otherwise fight over the adb server.
    // Give a just-cancelled session a moment to release the slot.
    let mut attempts = 0;
    let _guard = loop {
        match state.begin_pairing() {
            Ok(guard) => break guard,
            Err(error) if attempts >= PAIRING_SLOT_ATTEMPTS => return Err(error),
            Err(_) => {
                attempts += 1;
                tokio::time::sleep(PAIRING_SLOT_RETRY).await;
            }
        }
    };

    let adb_path = adb::adb_path(&state.tools())?;
    let secs = timeout
        .unwrap_or(QR_TIMEOUT_DEFAULT_SECS)
        .clamp(QR_TIMEOUT_MIN_SECS, QR_TIMEOUT_MAX_SECS);
    let address = tokio::select! {
        result = wireless::pair_and_connect(&adb_path, &payload, Duration::from_secs(secs)) => result?,
        _ = &mut cancel => return Err(AppError::Cancelled),
    };

    // Reuse the tracker's refresh so the new device reaches the UI immediately.
    tracker::refresh(&app).await;
    Ok(address)
}

/// Stop waiting for a QR scan started by [`pair_wireless_qr`].
#[tauri::command]
pub fn cancel_wireless_qr(state: State<'_, AppState>) {
    state.cancel_pairing();
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

/// `adb connect <addr>` - bring up a phone already enabled for wireless debugging.
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

#[tauri::command]
pub async fn list_mdns_services(state: State<'_, AppState>) -> Result<Vec<MdnsDevice>> {
    let adb_path = adb::adb_path(&state.tools())?;
    wireless::mdns_devices(&adb_path).await
}

/// Replace the saved phones the background watcher reconnects.
#[tauri::command]
pub fn set_reconnect_targets(targets: Vec<ReconnectTarget>, state: State<'_, AppState>) {
    state.set_reconnect_targets(targets);
}

/// Reconnect a saved phone, following it to its current port when it has moved.
#[tauri::command]
pub async fn reconnect_wireless(
    target: ReconnectTarget,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String> {
    let adb_path = adb::adb_path(&state.tools())?;
    let services = wireless::connect_services(&adb_path).await;
    let address = wireless::locate(&services, &target)
        .map(|service| format!("{}:{}", service.host, service.port))
        .or_else(|| target.address.clone())
        .ok_or_else(|| {
            AppError::Other(
                "The phone was not found. Check that Wireless debugging is on and it is on the same Wi-Fi."
                    .into(),
            )
        })?;

    match wireless::reconnect(&adb_path, &address).await {
        Reconnect::Connected(address) => {
            tracker::refresh(&app).await;
            Ok(address)
        }
        Reconnect::Unpaired => {
            reconnect::emit_unpaired(&app, &target);
            Err(AppError::Other(
                "The phone no longer trusts this computer. Pair it again.".into(),
            ))
        }
        Reconnect::Unavailable(message) => Err(AppError::CommandFailed(message)),
    }
}
