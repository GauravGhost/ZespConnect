use tauri::State;

use crate::error::Result;
use crate::models::{PortDirection, PortRule};
use crate::services::{adb, ports};
use crate::state::AppState;
use crate::util::validation::validate_serial;

fn adb_path(serial: &str, state: &AppState) -> Result<String> {
    validate_serial(serial)?;
    adb::adb_path(&state.tools())
}

#[tauri::command]
pub async fn list_port_rules(serial: String, state: State<'_, AppState>) -> Result<Vec<PortRule>> {
    let adb_path = adb_path(&serial, &state)?;
    ports::list(&adb_path, &serial).await
}

/// Changes return the fresh list so the UI needs no second round trip.
#[tauri::command]
pub async fn add_port_rule(
    serial: String,
    direction: PortDirection,
    host_port: u16,
    device_port: u16,
    state: State<'_, AppState>,
) -> Result<Vec<PortRule>> {
    let adb_path = adb_path(&serial, &state)?;
    ports::add(&adb_path, &serial, direction, host_port, device_port).await?;
    ports::list(&adb_path, &serial).await
}

#[tauri::command]
pub async fn remove_port_rule(
    serial: String,
    rule: PortRule,
    state: State<'_, AppState>,
) -> Result<Vec<PortRule>> {
    let adb_path = adb_path(&serial, &state)?;
    ports::remove(&adb_path, &serial, &rule).await?;
    ports::list(&adb_path, &serial).await
}

#[tauri::command]
pub async fn clear_port_rules(serial: String, state: State<'_, AppState>) -> Result<Vec<PortRule>> {
    let adb_path = adb_path(&serial, &state)?;
    ports::clear(&adb_path, &serial).await?;
    ports::list(&adb_path, &serial).await
}
