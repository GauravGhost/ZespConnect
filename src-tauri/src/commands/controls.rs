use tauri::State;

use crate::error::Result;
use crate::models::{DeveloperState, RebootMode};
use crate::services::{developer, toggles};
use crate::state::AppState;
use crate::util::validation::validate_serial;

#[tauri::command]
pub async fn set_toggle(
    serial: String,
    key: String,
    on: bool,
    state: State<'_, AppState>,
) -> Result<()> {
    validate_serial(&serial)?;
    toggles::set_toggle(&state.tools(), &serial, &key, on).await
}

#[tauri::command]
pub async fn set_brightness(
    serial: String,
    percent: i32,
    state: State<'_, AppState>,
) -> Result<()> {
    validate_serial(&serial)?;
    toggles::set_brightness(&state.tools(), &serial, percent).await
}

#[tauri::command]
pub async fn set_volume(serial: String, percent: i32, state: State<'_, AppState>) -> Result<()> {
    validate_serial(&serial)?;
    toggles::set_volume(&state.tools(), &serial, percent).await
}

#[tauri::command]
pub async fn reboot(serial: String, mode: RebootMode, state: State<'_, AppState>) -> Result<()> {
    validate_serial(&serial)?;
    toggles::reboot(&state.tools(), &serial, mode).await
}

#[tauri::command]
pub async fn get_developer_state(
    serial: String,
    state: State<'_, AppState>,
) -> Result<DeveloperState> {
    validate_serial(&serial)?;
    developer::state(&state.tools(), &serial).await
}

#[tauri::command]
pub async fn set_developer_toggle(
    serial: String,
    key: String,
    on: bool,
    state: State<'_, AppState>,
) -> Result<()> {
    validate_serial(&serial)?;
    developer::set_toggle(&state.tools(), &serial, &key, on).await
}

#[tauri::command]
pub async fn set_animation_scale(
    serial: String,
    scale: f32,
    state: State<'_, AppState>,
) -> Result<()> {
    validate_serial(&serial)?;
    developer::set_animation_scale(&state.tools(), &serial, scale).await
}
