use tauri::State;

use crate::error::Result;
use crate::models::CrashReport;
use crate::services::{adb, crashes};
use crate::state::AppState;
use crate::util::validation::validate_serial;

fn adb_path(serial: &str, state: &AppState) -> Result<String> {
    validate_serial(serial)?;
    adb::adb_path(&state.tools())
}

#[tauri::command]
pub async fn list_crashes(serial: String, state: State<'_, AppState>) -> Result<Vec<CrashReport>> {
    let adb_path = adb_path(&serial, &state)?;
    crashes::list(&adb_path, &serial).await
}

/// Writes the zip to a path the user picked in a save dialog. Takes minutes.
#[tauri::command]
pub async fn save_bug_report(
    serial: String,
    path: String,
    state: State<'_, AppState>,
) -> Result<()> {
    let adb_path = adb_path(&serial, &state)?;
    crashes::bug_report(&adb_path, &serial, &path).await
}
