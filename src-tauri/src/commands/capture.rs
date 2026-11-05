use std::path::PathBuf;

use tauri::Manager;
use tauri_plugin_opener::OpenerExt;

use crate::error::{AppError, Result};
use crate::runtime::recording::{self, RecordingOutcome, RecordingTarget};
use crate::services::capture::{self, CaptureDirs};
use crate::services::files;
use crate::state::AppState;
use crate::util::validation::validate_serial;

#[tauri::command]
pub fn capture_default_dirs() -> Result<CaptureDirs> {
    capture::default_dirs()
}

#[tauri::command]
pub fn capture_open_folder(dir: String, app: tauri::AppHandle) -> Result<()> {
    std::fs::create_dir_all(&dir)
        .map_err(|error| AppError::Io(format!("Could not open {dir}: {error}")))?;
    app.opener()
        .open_path(&dir, None::<&str>)
        .map_err(|error| AppError::Other(error.to_string()))
}

const PHONE_SCREENSHOTS: &str = "/sdcard/Pictures/Screenshots";

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedCapture {
    /// A computer path, or the device path when `on_phone`.
    pub location: String,
    pub on_phone: bool,
}

fn header<'a>(request: &'a tauri::ipc::Request<'_>, name: &str) -> Option<&'a str> {
    request
        .headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
}

/// Save a PNG sent as the raw request body. Headers: `x-serial`, `x-target` (`folder` or
/// `phone`), `x-stamp` (local time for the file name) and an optional URI-encoded `x-dir`.
#[tauri::command]
pub async fn save_screenshot(
    request: tauri::ipc::Request<'_>,
    app: tauri::AppHandle,
) -> Result<SavedCapture> {
    let tauri::ipc::InvokeBody::Raw(png) = request.body() else {
        return Err(AppError::Invalid(
            "Expected the screenshot as raw bytes".into(),
        ));
    };
    let serial = header(&request, "x-serial").unwrap_or_default().to_string();
    validate_serial(&serial)?;
    let stamp = header(&request, "x-stamp").unwrap_or_default();
    let state = app.state::<AppState>();
    let model = state
        .device(&serial)
        .map(|device| device.model)
        .unwrap_or_default();
    let name = capture::file_name(&model, &serial, stamp, "png");

    match header(&request, "x-target") {
        Some("folder") => {
            let dir = match header(&request, "x-dir").filter(|dir| !dir.is_empty()) {
                Some(dir) => PathBuf::from(capture::decode_component(dir)?),
                None => PathBuf::from(capture::default_dirs()?.pictures),
            };
            let path = capture::save_to_folder(&dir, &name, png)?;
            Ok(SavedCapture {
                location: path.to_string_lossy().into_owned(),
                on_phone: false,
            })
        }
        Some("phone") => {
            let local = capture::save_to_folder(&std::env::temp_dir(), &name, png)?;
            let pushed = files::send_to_media_dir(
                &state.tools(),
                &serial,
                &local.to_string_lossy(),
                PHONE_SCREENSHOTS,
            )
            .await;
            let _ = std::fs::remove_file(&local);
            Ok(SavedCapture {
                location: pushed?,
                on_phone: true,
            })
        }
        _ => Err(AppError::Invalid("Unknown screenshot target".into())),
    }
}

/// Start recording mirror session `id`; returns when it started, in Unix milliseconds.
#[tauri::command]
pub async fn start_recording(
    id: u64,
    target: RecordingTarget,
    dir: Option<String>,
    stamp: String,
    app: tauri::AppHandle,
) -> Result<u64> {
    recording::start(&app, id, target, dir, &stamp).await
}

#[tauri::command]
pub async fn stop_recording(id: u64, app: tauri::AppHandle) -> Result<RecordingOutcome> {
    recording::stop(&app, id).await
}

/// When session `id`'s recording started (Unix milliseconds), if it is recording.
#[tauri::command]
pub fn recording_state(id: u64, state: tauri::State<'_, AppState>) -> Option<u64> {
    state.recordings().started_at(id)
}
