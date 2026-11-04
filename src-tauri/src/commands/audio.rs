//! Choosing where a device's sound plays: on the phone, or on the computer.

use tauri::{AppHandle, State};

use crate::error::Result;
use crate::runtime::phone_audio;
use crate::state::AppState;
use crate::util::validation::validate_serial;

/// Devices whose sound currently plays on the computer.
#[tauri::command]
pub fn list_phone_audio(state: State<'_, AppState>) -> Vec<String> {
    state.phone_audio().serials()
}

#[tauri::command]
pub async fn set_phone_audio(serial: String, on_computer: bool, app: AppHandle) -> Result<()> {
    validate_serial(&serial)?;
    if on_computer {
        phone_audio::start(&app, &serial).await
    } else {
        phone_audio::stop(&app, &serial);
        Ok(())
    }
}
