use tauri::{AppHandle, State};

use crate::error::{AppError, Result};
use crate::models::{Controller, GameOptions};
use crate::runtime::{controller_passthrough, controllers, game_mode};
use crate::services::audio_out;
use crate::state::AppState;
use crate::util::validation::validate_serial;

/// Change a running Game Mode session's toggles.
#[tauri::command]
pub async fn update_game_mode(id: u64, options: GameOptions, app: AppHandle) -> Result<()> {
    game_mode::update(&app, id, options).await
}

#[tauri::command]
pub async fn list_audio_outputs() -> Result<Vec<String>> {
    blocking(audio_out::output_names).await
}

#[tauri::command]
pub async fn list_controllers() -> Result<Vec<Controller>> {
    blocking(controllers::list).await
}

/// Phones whose regular mirror windows take this computer's controllers.
#[tauri::command]
pub fn list_controller_passthrough(state: State<'_, AppState>) -> Vec<String> {
    state.controller_passthrough().serials()
}

#[tauri::command]
pub fn set_controller_passthrough(serial: String, on: bool, app: AppHandle) -> Result<()> {
    validate_serial(&serial)?;
    controller_passthrough::set(&app, &serial, on)
}

async fn blocking<T: Send + 'static>(task: fn() -> T) -> Result<T> {
    tauri::async_runtime::spawn_blocking(task)
        .await
        .map_err(|error| AppError::Other(error.to_string()))
}
