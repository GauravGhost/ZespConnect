//! Device commands (thin wrappers over `services::devices`).

use tauri::State;

use crate::error::Result;
use crate::models::{Device, DeviceInfo};
use crate::services::devices as devices_service;
use crate::state::AppState;
use crate::util::validation::validate_serial;

#[tauri::command]
pub async fn list_devices(state: State<'_, AppState>) -> Result<Vec<Device>> {
    let tools = state.tools();
    let list = devices_service::list(&tools).await?;
    state.set_devices(list.clone());
    Ok(list)
}

#[tauri::command]
pub async fn get_device_info(serial: String, state: State<'_, AppState>) -> Result<DeviceInfo> {
    validate_serial(&serial)?;
    let tools = state.tools();
    devices_service::info(&tools, &serial).await
}
