use tauri::{AppHandle, State};

use crate::error::Result;
use crate::models::{Device, DeviceInfo};
use crate::services::devices as devices_service;
use crate::state::AppState;
use crate::tray;
use crate::util::validation::validate_serial;

#[tauri::command]
pub async fn list_devices(state: State<'_, AppState>) -> Result<Vec<Device>> {
    let list = devices_service::list(&state.tools(), &state.devices()).await?;
    state.set_devices(list.clone());
    Ok(list)
}

#[tauri::command]
pub async fn get_device_info(serial: String, state: State<'_, AppState>) -> Result<DeviceInfo> {
    validate_serial(&serial)?;
    let tools = state.tools();
    devices_service::info(&tools, &serial).await
}

/// Remember the active device and reflect it in the tray menu.
#[tauri::command]
pub fn set_selected_device(
    serial: Option<String>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<()> {
    if let Some(value) = &serial {
        validate_serial(value)?;
    }
    state.set_selected_serial(serial)?;
    tray::sync(&app);
    Ok(())
}

/// The device the user last selected, used to seed the UI on startup.
#[tauri::command]
pub fn get_selected_device(state: State<'_, AppState>) -> Option<String> {
    state.selected_serial()
}
