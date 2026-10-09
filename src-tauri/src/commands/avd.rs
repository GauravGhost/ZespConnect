//! AVD (emulator) commands (thin wrappers over `services::avd`).

use tauri::State;

use crate::error::Result;
use crate::models::Avd;
use crate::services::avd;
use crate::state::AppState;

#[tauri::command]
pub async fn list_avds(state: State<'_, AppState>) -> Result<Vec<Avd>> {
    let tools = state.tools();
    let devices = state.devices();
    avd::list(&tools, &devices).await
}

#[tauri::command]
pub async fn start_avd(name: String, state: State<'_, AppState>) -> Result<()> {
    let tools = state.tools();
    let log_path = state.app_data_dir().join("emulator.log");
    avd::start(&tools, &name, &log_path).await
}
