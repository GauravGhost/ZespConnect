//! Mirroring commands (thin wrappers over `services::mirror` and
//! `services::mirror_session`).

use tauri::ipc::{Channel, Response};
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};

use crate::error::{AppError, Result};
use crate::models::{MirrorKind, MirrorOptions, MirrorProcess, MirrorSession, MirrorSessionSpec};
use crate::services::mirror::{self, AppDisplay};
use crate::services::mirror_session;
use crate::state::AppState;
use crate::util::validation::{validate_package, validate_serial};

/// Mirror the full device screen in a new window.
#[tauri::command]
pub async fn start_mirror(
    serial: String,
    options: MirrorOptions,
    app: AppHandle,
) -> Result<MirrorProcess> {
    validate_serial(&serial)?;
    mirror::start_mirror(&app, &serial, &options).await
}

/// Stream the device camera in a new window.
#[tauri::command]
pub async fn start_camera(
    serial: String,
    options: MirrorOptions,
    app: AppHandle,
) -> Result<MirrorProcess> {
    validate_serial(&serial)?;
    mirror::start_camera(&app, &serial, &options).await
}

/// Open an app in its own window on the laptop (main-display fallback on 12–13).
#[tauri::command]
pub async fn start_app_window(
    serial: String,
    package: String,
    label: String,
    app: AppHandle,
) -> Result<MirrorProcess> {
    validate_serial(&serial)?;
    validate_package(&package)?;
    mirror::start_app(&app, &serial, &package, &label, AppDisplay::Window).await
}

/// Mirror the main display with the app in the foreground.
#[tauri::command]
pub async fn start_app_on_screen(
    serial: String,
    package: String,
    label: String,
    app: AppHandle,
) -> Result<MirrorProcess> {
    validate_serial(&serial)?;
    validate_package(&package)?;
    mirror::start_app(&app, &serial, &package, &label, AppDisplay::Screen).await
}

#[tauri::command]
pub async fn stop_mirror(pid: u32, app: AppHandle) -> Result<()> {
    mirror::stop(&app, pid)
}

#[tauri::command]
pub async fn stop_all_mirrors(app: AppHandle) -> Result<()> {
    mirror::stop_all(&app);
    mirror_session::stop_all(&app);
    Ok(())
}

/// The scrcpy windows currently open.
#[tauri::command]
pub fn list_mirrors(state: State<'_, AppState>) -> Vec<MirrorProcess> {
    state.list_mirrors()
}

/// Open a mirror inside a Tauri window the app owns. The window asks for its
/// stream separately, so a failed stream can fall back to an external window.
#[tauri::command]
pub async fn open_mirror_window(
    serial: String,
    kind: MirrorKind,
    package: Option<String>,
    label: String,
    options: MirrorOptions,
    app: AppHandle,
) -> Result<u64> {
    validate_serial(&serial)?;
    if let Some(package) = package.as_deref() {
        validate_package(package)?;
    }

    let id = {
        let state = app.state::<AppState>();
        let id = state.next_session_id();
        state.set_pending_session(
            id,
            MirrorSessionSpec {
                serial: serial.clone(),
                kind,
                package,
                label: label.clone(),
                options,
            },
        );
        id
    };

    let title = if label.trim().is_empty() {
        serial.clone()
    } else {
        label.clone()
    };
    WebviewWindowBuilder::new(&app, window_label(id), WebviewUrl::App("index.html".into()))
        .title(format!("Zesp Connect — {title}"))
        .inner_size(420.0, 820.0)
        .min_inner_size(260.0, 320.0)
        .build()
        .map_err(|error| AppError::Other(error.to_string()))?;

    Ok(id)
}

/// Stream a previously opened session's video into the calling window.
#[tauri::command]
pub async fn start_mirror_stream(
    id: u64,
    channel: Channel<Response>,
    app: AppHandle,
) -> Result<MirrorSession> {
    let spec = app
        .state::<AppState>()
        .take_pending_session(id)
        .ok_or_else(|| AppError::Invalid("Unknown mirror session".into()))?;
    mirror_session::start(&app, id, spec, channel).await
}

/// Close one in-app session and its window.
#[tauri::command]
pub async fn stop_mirror_session(id: u64, app: AppHandle) -> Result<()> {
    mirror_session::stop(&app, id);
    close_window(&app, id);
    Ok(())
}

/// The in-app mirror sessions currently running.
#[tauri::command]
pub fn list_mirror_sessions(state: State<'_, AppState>) -> Vec<MirrorSession> {
    state.list_sessions()
}

/// Reopen a session as an external scrcpy window when in-app rendering is not
/// available, reusing the settings captured when the window was opened.
#[tauri::command]
pub async fn mirror_session_fallback(id: u64, app: AppHandle) -> Result<()> {
    let spec = app
        .state::<AppState>()
        .take_pending_session(id)
        .ok_or_else(|| AppError::Invalid("Unknown mirror session".into()))?;

    match spec.kind {
        MirrorKind::Mirror => {
            mirror::start_mirror(&app, &spec.serial, &spec.options).await?;
        }
        MirrorKind::Camera => {
            mirror::start_camera(&app, &spec.serial, &spec.options).await?;
        }
        MirrorKind::App | MirrorKind::ScreenApp => {
            let package = spec
                .package
                .clone()
                .ok_or_else(|| AppError::Invalid("Missing package".into()))?;
            let display = if spec.kind == MirrorKind::App {
                AppDisplay::Window
            } else {
                AppDisplay::Screen
            };
            mirror::start_app(&app, &spec.serial, &package, &spec.label, display).await?;
        }
    }

    close_window(&app, id);
    Ok(())
}

fn window_label(id: u64) -> String {
    format!("mirror-{id}")
}

fn close_window(app: &AppHandle, id: u64) {
    if let Some(window) = app.get_webview_window(&window_label(id)) {
        let _ = window.close();
    }
}
