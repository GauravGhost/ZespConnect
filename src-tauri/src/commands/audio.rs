use std::collections::HashMap;

use tauri::{AppHandle, State};

use crate::error::Result;
use crate::models::SoundRoute;
use crate::runtime::phone_audio;
use crate::state::AppState;
use crate::util::validation::validate_serial;

/// Devices whose sound plays on the computer, with or without the phone too.
#[tauri::command]
pub fn list_phone_audio(state: State<'_, AppState>) -> HashMap<String, SoundRoute> {
    state
        .phone_audio()
        .serials()
        .into_iter()
        .map(|serial| {
            let route = state.phone_audio_route(&serial);
            (serial, route)
        })
        .collect()
}

#[tauri::command]
pub async fn set_phone_audio(serial: String, route: SoundRoute, app: AppHandle) -> Result<()> {
    validate_serial(&serial)?;
    phone_audio::set_route(&app, &serial, route).await
}
