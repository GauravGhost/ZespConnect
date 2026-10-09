//! Mirroring commands (thin wrappers over `runtime::mirror` and
//! `runtime::mirror_session`).

use tauri::ipc::{Channel, Response};
use tauri::{AppHandle, LogicalSize, Manager, State, WebviewUrl, WebviewWindowBuilder};

use crate::error::{AppError, Result};
use crate::models::{
    KeyAction, MirrorKind, MirrorOptions, MirrorProcess, MirrorSession, MirrorSessionSpec,
    TouchAction,
};
use crate::platform;
use crate::runtime::mirror::{self, AppDisplay};
use crate::runtime::mirror_session;
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

/// Open Android's desktop mode in an external scrcpy window (used when the
/// webview cannot render it in-app).
#[tauri::command]
pub async fn start_desktop(serial: String, app: AppHandle) -> Result<MirrorProcess> {
    validate_serial(&serial)?;
    let display = mirror::screen_desktop_display(&app);
    mirror::start_desktop(&app, &serial, &display).await
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

    let desktop = kind == MirrorKind::Desktop;
    let desktop_display = desktop.then(|| mirror::screen_desktop_display(&app));

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
                desktop_display,
            },
        );
        id
    };

    let title = if label.trim().is_empty() {
        serial.clone()
    } else {
        label.clone()
    };
    // A desktop opens landscape and, unlike a phone-shaped window, can go full
    // screen (the green button on macOS) - it is sized to fill the screen exactly.
    let (width, height) = if desktop {
        (
            mirror_window::DEFAULT_LONG_SIDE * 1.5,
            mirror_window::DEFAULT_LONG_SIDE,
        )
    } else {
        (
            mirror_window::DEFAULT_LONG_SIDE * 0.5,
            mirror_window::DEFAULT_LONG_SIDE,
        )
    };
    // `?desktop` tells the page to draw the desktop's title bar from the start.
    let page = if desktop {
        "index.html?desktop"
    } else {
        "index.html"
    };
    let builder = WebviewWindowBuilder::new(&app, window_label(id), WebviewUrl::App(page.into()))
        .title(format!("Zesp Connect - {title}"))
        .inner_size(width, height)
        .min_inner_size(mirror_window::MIN_SIDE, mirror_window::MIN_SIDE)
        .maximizable(desktop);
    // Every mirror window draws its own title bar (the title, the always-on-top
    // pin and, on a desktop, the phone-screen toggle and window buttons; see
    // `MirrorTitleBar`/`DesktopTitleBar`), so the native bar is dropped rather
    // than sitting above a second, empty one. macOS keeps its traffic lights via
    // an overlay bar; elsewhere dropping the chrome (and the shadow) also lets
    // tao's hit-test leave every edge resizable on Windows.
    #[cfg(target_os = "macos")]
    let builder = builder
        .title_bar_style(tauri::TitleBarStyle::Overlay)
        .hidden_title(true);
    #[cfg(not(target_os = "macos"))]
    let builder = builder.decorations(false).shadow(false);
    builder
        .build()
        .map_err(|error| AppError::Other(error.to_string()))?;

    Ok(id)
}

/// Fit a session's window to its video's aspect ratio once the first frame reveals
/// the device's dimensions, and remember the ratio so later resizes stay locked to
/// it. The video area then matches the frame exactly, so the canvas fills the
/// window (below its title bar and, for a phone-shaped window, above its bottom
/// gap - see `mirror_window::vertical_inset`) instead of being letterboxed.
#[tauri::command]
pub async fn fit_mirror_window(id: u64, width: u32, height: u32, app: AppHandle) -> Result<()> {
    if width == 0 || height == 0 {
        return Err(AppError::Invalid("Invalid video size".into()));
    }

    let window = app
        .get_webview_window(&window_label(id))
        .ok_or_else(|| AppError::Invalid("Unknown mirror window".into()))?;

    let aspect = width as f64 / height as f64;
    let state = app.state::<AppState>();
    let inset = mirror_window::vertical_inset(state.session_kind(id));
    let long_side = mirror::window_long_side(&window, inset);
    state.set_aspect(id, aspect);
    // On Windows this locks the ratio through the native resize hook so the
    // dragged edge tracks the cursor; elsewhere it is a no-op and the window's
    // resize handler corrects the size instead.
    platform::watch_aspect(&window, aspect, inset);

    let (width, height) =
        mirror_window::pad(mirror_window::fit_size(width, height, long_side), inset);
    window
        .set_size(LogicalSize::new(width, height))
        .map_err(|error| AppError::Other(error.to_string()))?;

    Ok(())
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

/// Inject a pointer action into a running session. `x`/`y` and `width`/`height`
/// share the video's coordinate space, so the server can map them to the device.
/// `button`/`buttons` (Android `BUTTON_*` masks: the one that changed, the ones
/// held) are only sent by desktop windows, for right-click and hover.
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

/// Turn a desktop session's phone screen on (to use the phone alongside the
/// desktop) or off. The phone stays awake and unlocked either way, so the desktop
/// keeps running; the choice is remembered for when the phone is next unlocked.
#[tauri::command]
pub async fn mirror_phone_screen(id: u64, on: bool, state: State<'_, AppState>) -> Result<()> {
    let channel = session_channel(&state, id)?;
    state.set_phone_screen(id, on);
    channel.send(control::set_display_power(on))
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
    close_window(&app, id);
    Ok(())
}

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
        MirrorKind::Desktop => {
            let display = spec
                .desktop_display
                .clone()
                .unwrap_or_else(|| mirror::screen_desktop_display(&app));
            mirror::start_desktop(&app, &spec.serial, &display).await?;
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
