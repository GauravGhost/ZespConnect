use tauri::ipc::Response;
use tauri::State;

use crate::error::Result;
use crate::services::{adb, layout};
use crate::state::AppState;
use crate::util::validation::validate_serial;

fn adb_path(serial: &str, state: &AppState) -> Result<String> {
    validate_serial(serial)?;
    adb::adb_path(&state.tools())
}

#[tauri::command]
pub async fn dump_layout(serial: String, state: State<'_, AppState>) -> Result<String> {
    let adb_path = adb_path(&serial, &state)?;
    layout::dump(&adb_path, &serial).await
}

/// Raw PNG bytes, which reach the page as an ArrayBuffer instead of a JSON number array.
#[tauri::command]
pub async fn capture_screen(serial: String, state: State<'_, AppState>) -> Result<Response> {
    let adb_path = adb_path(&serial, &state)?;
    Ok(Response::new(layout::screenshot(&adb_path, &serial).await?))
}
