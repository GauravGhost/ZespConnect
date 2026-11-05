use tauri::AppHandle;

use crate::error::Result;
use crate::models::MediaKey;
use crate::runtime::now_playing;
use crate::util::validation::validate_serial;

/// Report what `serial` plays through `now-playing:changed` until called again; `None` stops.
/// Async because spawning the watch needs the Tokio runtime; callers send these in order.
#[tauri::command]
pub async fn watch_now_playing(serial: Option<String>, app: AppHandle) -> Result<()> {
    if let Some(value) = &serial {
        validate_serial(value)?;
    }
    now_playing::watch(&app, serial.as_deref())
}

#[tauri::command]
pub async fn send_media_key(serial: String, key: MediaKey, app: AppHandle) -> Result<()> {
    validate_serial(&serial)?;
    now_playing::press(&app, &serial, key).await
}
