use tauri::ipc::{Channel, Response};
use tauri::{AppHandle, LogicalSize, Manager, State};

use crate::error::{AppError, Result};
use crate::models::{
    KeyAction, MirrorAction, MirrorKind, MirrorOptions, MirrorProcess, MirrorSession,
    MirrorSessionSpec, TouchAction,
};
use crate::runtime::mirror::{self, AppDisplay};
use crate::runtime::{mirror_session, session_window};
use crate::services::control::{self, Position};
use crate::services::mirror_window;
use crate::state::AppState;
use crate::util::validation::{validate_package, validate_serial};

#[tauri::command]
pub async fn start_mirror(
    serial: String,
    options: MirrorOptions,
    app: AppHandle,
) -> Result<MirrorProcess> {
    validate_serial(&serial)?;
    mirror::start_mirror(&app, &serial, &options).await
}

#[tauri::command]
pub async fn start_camera(
    serial: String,
    options: MirrorOptions,
    app: AppHandle,
) -> Result<MirrorProcess> {
    validate_serial(&serial)?;
    mirror::start_camera(&app, &serial, &options).await
}

/// Open an app in its own window on the laptop (main-display fallback on 12-13).
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

/// Open Android's desktop mode in an external scrcpy window (when the webview cannot render it in-app).
#[tauri::command]
pub async fn start_desktop(serial: String, app: AppHandle) -> Result<MirrorProcess> {
    validate_serial(&serial)?;
    let display = mirror::screen_desktop_display(&app);
    mirror::start_desktop(&app, &serial, &display).await
}

#[tauri::command]
pub async fn stop_mirror(pid: u32, app: AppHandle) -> Result<()> {
    mirror::stop(&app, pid);
    Ok(())
}

#[tauri::command]
pub async fn stop_all_mirrors(app: AppHandle) -> Result<()> {
    mirror::stop_all(&app);
    mirror_session::stop_all(&app);
    Ok(())
}

#[tauri::command]
pub fn list_mirrors(state: State<'_, AppState>) -> Vec<MirrorProcess> {
    state.list_mirrors()
}

/// Open a mirror inside an app-owned Tauri window; the window asks for its stream separately so a failure can fall back.
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

    session_window::open(
        &app,
        MirrorSessionSpec {
            serial,
            kind,
            package,
            label,
            options,
            desktop_display: None,
        },
    )
}

/// Fit a session's window to its video's aspect ratio once the first frame reveals the device
/// size, and remember it so later resizes stay locked to it (see `mirror_window::vertical_inset`).
#[tauri::command]
pub async fn fit_mirror_window(id: u64, width: u32, height: u32, app: AppHandle) -> Result<()> {
    if width == 0 || height == 0 {
        return Err(AppError::Invalid("Invalid video size".into()));
    }

    let state = app.state::<AppState>();
    let inset = mirror_window::vertical_inset(state.session_kind(id));
    let window = app
        .get_webview_window(&session_window::label(id))
        .ok_or_else(|| AppError::Invalid("Unknown mirror window".into()))?;

    // A flex display already takes the window's shape, so fitting would undo the user's resize.
    // It is turned instead when the video flips between portrait and landscape, which is how an
    // app that forces landscape (a game, or a video going full screen) gets a landscape window.
    if state
        .session(id)
        .is_some_and(|session| session.flex_display)
    {
        return turn_window(&window, width > height, inset);
    }

    let aspect = width as f64 / height as f64;
    let long_side = mirror::window_long_side(&window, inset);
    state.set_aspect(id, aspect);

    let (width, height) =
        mirror_window::pad(mirror_window::fit_size(width, height, long_side), inset);
    window
        .set_size(LogicalSize::new(width, height))
        .map_err(|error| AppError::Other(error.to_string()))?;

    Ok(())
}

/// Turn a window whose display follows it so its video area faces `landscape`. Nothing happens
/// when it already does, or when the OS is showing the window maximized or full screen.
fn turn_window(window: &tauri::WebviewWindow, landscape: bool, inset: f64) -> Result<()> {
    if window.is_maximized().unwrap_or(false) || window.is_fullscreen().unwrap_or(false) {
        return Ok(());
    }
    let scale = window.scale_factor().unwrap_or(1.0);
    let size = window
        .inner_size()
        .map_err(|error| AppError::Other(error.to_string()))?;
    let logical = (
        f64::from(size.width) / scale,
        f64::from(size.height) / scale,
    );
    let video = mirror_window::unpad(logical, inset);
    if (video.0 > video.1) == landscape {
        return Ok(());
    }
    let (width, height) = mirror_window::turned_size(logical, inset);
    window
        .set_size(LogicalSize::new(width, height))
        .map_err(|error| AppError::Other(error.to_string()))
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

/// Inject a pointer action; `x`/`y` and `width`/`height` share the video's space. `button`/`buttons` are desktop-only.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn mirror_touch(
    id: u64,
    action: TouchAction,
    x: u32,
    y: u32,
    width: u16,
    height: u16,
    button: Option<u32>,
    buttons: Option<u32>,
    state: State<'_, AppState>,
) -> Result<()> {
    let position = Position {
        x,
        y,
        width,
        height,
    };
    session_channel(&state, id)?.send(control::inject_touch(
        action,
        position,
        button.unwrap_or(0),
        buttons.unwrap_or(0),
    ))
}

/// Scroll a running session at a point; amounts are in wheel notches.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn mirror_scroll(
    id: u64,
    x: u32,
    y: u32,
    width: u16,
    height: u16,
    hscroll: f32,
    vscroll: f32,
    state: State<'_, AppState>,
) -> Result<()> {
    let position = Position {
        x,
        y,
        width,
        height,
    };
    session_channel(&state, id)?.send(control::inject_scroll(position, hscroll, vscroll))
}

/// Press or release an Android `KEYCODE_*` with a `META_*` mask.
#[tauri::command]
pub async fn mirror_key(
    id: u64,
    action: KeyAction,
    keycode: u32,
    repeat: u32,
    meta_state: u32,
    state: State<'_, AppState>,
) -> Result<()> {
    session_channel(&state, id)?.send(control::inject_keycode(action, keycode, repeat, meta_state))
}

/// Ask a running session for a fresh key frame (after the decoder dropped frames).
#[tauri::command]
pub async fn mirror_reset_video(id: u64, state: State<'_, AppState>) -> Result<()> {
    session_channel(&state, id)?.send(control::reset_video())
}

/// Turn a desktop session's phone screen on or off; the phone stays awake and unlocked either way.
#[tauri::command]
pub async fn mirror_phone_screen(id: u64, on: bool, state: State<'_, AppState>) -> Result<()> {
    let channel = session_channel(&state, id)?;
    state.set_phone_screen(id, on);
    channel.send(control::set_display_power(on))
}

/// Run a one-shot control from the window's menu (Back, panels, rotate, camera torch/zoom).
#[tauri::command]
pub async fn mirror_action(
    id: u64,
    action: MirrorAction,
    state: State<'_, AppState>,
) -> Result<()> {
    // An unknown message type ends an older server's session, so camera controls are gated.
    if action.is_camera()
        && !state
            .session(id)
            .is_some_and(|session| session.camera_controls)
    {
        return Err(AppError::Invalid(
            "Camera controls need a scrcpy 4.0+ camera session".into(),
        ));
    }

    // A recording keeps one picture size (see `runtime::recording`).
    if action == MirrorAction::Rotate && state.recordings().contains(id) {
        return Err(AppError::Invalid("Rotating is off while recording".into()));
    }

    let channel = session_channel(&state, id)?;
    match action {
        MirrorAction::Back => {
            channel.send(control::back_or_screen_on(KeyAction::Down))?;
            channel.send(control::back_or_screen_on(KeyAction::Up))?;
            match state.session(id) {
                Some(MirrorSession {
                    kind: MirrorKind::App,
                    package: Some(package),
                    ..
                }) => mirror_session::reopen_after_back(channel, &package),
                _ => Ok(()),
            }
        }
        MirrorAction::Notifications => channel.send(control::expand_notification_panel()),
        MirrorAction::QuickSettings => channel.send(control::expand_settings_panel()),
        MirrorAction::CollapsePanels => channel.send(control::collapse_panels()),
        MirrorAction::Rotate => channel.send(control::rotate_device()),
        MirrorAction::TorchOn => channel.send(control::camera_set_torch(true)),
        MirrorAction::TorchOff => channel.send(control::camera_set_torch(false)),
        MirrorAction::ZoomIn => channel.send(control::camera_zoom_in()),
        MirrorAction::ZoomOut => channel.send(control::camera_zoom_out()),
    }
}

#[tauri::command]
pub async fn mirror_text(id: u64, text: String, state: State<'_, AppState>) -> Result<()> {
    session_channel(&state, id)?.send(control::inject_text(&text)?)
}

fn session_channel(state: &State<'_, AppState>, id: u64) -> Result<mirror_session::ControlChannel> {
    state
        .session_control(id)
        .ok_or_else(|| AppError::Invalid("Unknown mirror session".into()))
}

#[tauri::command]
pub async fn stop_mirror_session(id: u64, app: AppHandle) -> Result<()> {
    mirror_session::stop(&app, id);
    session_window::close(&app, id);
    Ok(())
}

#[tauri::command]
pub fn list_mirror_sessions(state: State<'_, AppState>) -> Vec<MirrorSession> {
    state.list_sessions()
}

/// Reopen a session as an external scrcpy window when in-app rendering is unavailable.
#[tauri::command]
pub async fn mirror_session_fallback(id: u64, app: AppHandle) -> Result<()> {
    let spec = app
        .state::<AppState>()
        .take_pending_session(id)
        .ok_or_else(|| AppError::Invalid("Unknown mirror session".into()))?;

    mirror::start_fallback(&app, &spec).await?;
    session_window::close(&app, id);
    Ok(())
}
