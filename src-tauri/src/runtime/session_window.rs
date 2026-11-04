//! App-owned Tauri windows that host in-app mirror sessions, each labelled `mirror-<session id>`.

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::error::{AppError, Result};
use crate::models::{MirrorKind, MirrorSessionSpec};
use crate::runtime::mirror;
use crate::services::{mirror_window, scrcpy};
use crate::state::AppState;

/// The frontend checks this prefix too, to render a mirror viewer instead of the main UI.
const WINDOW_PREFIX: &str = "mirror-";
/// Initial width relative to the default long side: a desktop opens landscape, a phone portrait.
const DESKTOP_WIDTH_RATIO: f64 = 1.5;
const PHONE_WIDTH_RATIO: f64 = 0.5;

pub fn label(id: u64) -> String {
    format!("{WINDOW_PREFIX}{id}")
}

/// The session id in a `mirror-<id>` window label, if the label is one.
pub fn session_id(label: &str) -> Option<u64> {
    label
        .strip_prefix(WINDOW_PREFIX)
        .and_then(|value| value.parse::<u64>().ok())
}

/// Keep `spec` as a pending session and open its window; the window asks for its stream
/// separately so a failure can fall back.
pub fn open(app: &AppHandle, mut spec: MirrorSessionSpec) -> Result<u64> {
    let desktop = spec.kind == MirrorKind::Desktop;
    spec.desktop_display = desktop.then(|| mirror::screen_desktop_display(app));
    let title = mirror::window_title(&mirror::label_or(&spec.label, &spec.serial));

    let id = {
        let state = app.state::<AppState>();
        let id = state.next_session_id();
        state.set_pending_session(id, spec);
        id
    };

    // A desktop can go full screen; a phone-shaped window cannot.
    let width_ratio = if desktop {
        DESKTOP_WIDTH_RATIO
    } else {
        PHONE_WIDTH_RATIO
    };
    // `?desktop` tells the page to draw the desktop's title bar from the start.
    let page = if desktop {
        "index.html?desktop"
    } else {
        "index.html"
    };
    let builder = WebviewWindowBuilder::new(app, label(id), WebviewUrl::App(page.into()))
        .title(title)
        .inner_size(
            mirror_window::DEFAULT_LONG_SIDE * width_ratio,
            mirror_window::DEFAULT_LONG_SIDE,
        )
        .min_inner_size(mirror_window::MIN_SIDE, mirror_window::MIN_SIDE)
        .maximizable(desktop);
    // Every mirror window draws its own title bar, so the native one is dropped; macOS keeps
    // its traffic lights via an overlay. On Windows, dropping the chrome also keeps edges resizable.
    #[cfg(target_os = "macos")]
    let builder = builder
        .title_bar_style(tauri::TitleBarStyle::Overlay)
        .hidden_title(true);
    #[cfg(not(target_os = "macos"))]
    let builder = builder.decorations(false).shadow(false);
    crate::platform::unhide_app(app);
    builder
        .build()
        .map_err(|error| AppError::Other(error.to_string()))?;

    Ok(id)
}

/// A display that follows its window, sized to that window when the session starts.
pub struct FlexDisplay {
    /// The `WxH/dpi` spec for `new_display`.
    pub spec: String,
    /// Display pixels per logical window pixel, kept for later resizes.
    pub pixels_per_point: f64,
}

/// Size a flex display for session `id`'s window. A desktop takes the computer's UI scale; an app
/// takes `phone_dpi`, because apps apply some insets (the status bar height) as raw phone pixels.
pub fn flex_display(
    app: &AppHandle,
    id: u64,
    kind: MirrorKind,
    phone_dpi: Option<u32>,
) -> Option<FlexDisplay> {
    let window = app.get_webview_window(&label(id))?;
    let scale = window.scale_factor().ok()?;
    let size = window.inner_size().ok()?;
    let dpi = if kind == MirrorKind::Desktop {
        scrcpy::desktop_density(scale)
    } else {
        phone_dpi?
    };
    let pixels_per_point = f64::from(dpi) / scrcpy::BASE_DENSITY;
    let inset = mirror_window::vertical_inset(Some(kind));
    let (width, height) =
        mirror_window::display_pixels((size.width, size.height), scale, inset, pixels_per_point)?;
    Some(FlexDisplay {
        spec: format!("{width}x{height}/{dpi}"),
        pixels_per_point,
    })
}

pub fn close(app: &AppHandle, id: u64) {
    if let Some(window) = app.get_webview_window(&label(id)) {
        let _ = window.close();
    }
}
