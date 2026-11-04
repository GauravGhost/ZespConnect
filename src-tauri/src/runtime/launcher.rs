//! The app launcher: a global hotkey that toggles a search popup window for the active phone.

use tauri::{
    AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

use crate::error::{AppError, Result};
use crate::models::{AppInfo, LauncherAppsChanged, LauncherSettings, LauncherStatus};
use crate::services::{apps, launcher};
use crate::state::AppState;
use crate::util;

/// The popup window's label; the frontend renders the launcher for it.
pub const WINDOW: &str = "launcher";
/// Tells the popup to reset and reload, since it is reused rather than recreated.
const SHOWN_EVENT: &str = "launcher:shown";
const APPS_CHANGED_EVENT: &str = "launcher:apps-changed";
const WIDTH: f64 = 560.0;
const HEIGHT: f64 = 420.0;

/// Register the saved hotkey at startup. A taken hotkey stays enabled and shows up as not
/// registered on the Features page.
pub fn init(app: &AppHandle) {
    let state = app.state::<AppState>();
    let settings = util::read_json::<LauncherSettings>(&state.launcher_file())
        .ok()
        .flatten()
        .unwrap_or_default();
    let registered = register(app, &settings).is_ok();
    state.set_launcher_settings(settings);
    state.set_launcher_registered(registered);
}

pub fn status(app: &AppHandle) -> LauncherStatus {
    let state = app.state::<AppState>();
    let settings = state.launcher_settings();
    LauncherStatus {
        enabled: settings.enabled,
        shortcut: settings.shortcut,
        registered: state.launcher_registered(),
    }
}

/// Apply and save new settings. If the OS refuses the hotkey, the previous one is restored and
/// nothing is saved.
pub fn save_settings(app: &AppHandle, settings: LauncherSettings) -> Result<LauncherStatus> {
    parse_shortcut(&settings.shortcut)?;
    let state = app.state::<AppState>();
    let previous = state.launcher_settings();

    if let Err(error) = register(app, &settings) {
        let restored = register(app, &previous).is_ok();
        state.set_launcher_registered(restored);
        return Err(error);
    }

    util::write_json(&state.launcher_file(), &settings)?;
    state.set_launcher_settings(settings);
    state.set_launcher_registered(true);
    Ok(status(app))
}

fn parse_shortcut(shortcut: &str) -> Result<Shortcut> {
    shortcut
        .parse::<Shortcut>()
        .map_err(|_| AppError::Invalid(format!("Not a valid hotkey: {shortcut}")))
}

/// Replace whatever is registered with `settings`' hotkey, or nothing when disabled.
fn register(app: &AppHandle, settings: &LauncherSettings) -> Result<()> {
    let shortcuts = app.global_shortcut();
    shortcuts
        .unregister_all()
        .map_err(|error| AppError::Other(error.to_string()))?;
    if !settings.enabled {
        return Ok(());
    }
    shortcuts
        .register(parse_shortcut(&settings.shortcut)?)
        .map_err(|_| AppError::Invalid(format!("{} is in use by another app", settings.shortcut)))
}

/// The apps to offer for `serial`, from the cache when there is one. The first call per phone each
/// run also re-reads the phone in the background, so newly installed apps show up.
pub async fn apps(app: &AppHandle, serial: &str) -> Result<Vec<AppInfo>> {
    let state = app.state::<AppState>();
    let data_dir = state.app_data_dir();
    let first_this_run = state.claim_launcher_refresh(serial);

    let Some(cached) = apps::cached(&data_dir, serial) else {
        let listed = apps::list(&state.tools(), serial, true, false, &data_dir).await?;
        return Ok(launcher::launchable(listed.apps));
    };

    if first_this_run {
        let app = app.clone();
        let serial = serial.to_string();
        tauri::async_runtime::spawn(async move {
            let state = app.state::<AppState>();
            // A failed refresh leaves the cached list showing, which is still correct enough.
            if let Ok(listed) = apps::list(&state.tools(), &serial, true, true, &data_dir).await {
                let apps = launcher::launchable(listed.apps);
                let _ = app.emit_to(
                    WINDOW,
                    APPS_CHANGED_EVENT,
                    LauncherAppsChanged { serial, apps },
                );
            }
        });
    }
    Ok(launcher::launchable(cached))
}

/// Show the popup, or hide it when it is already showing.
pub fn toggle(app: &AppHandle) {
    match app.get_webview_window(WINDOW) {
        Some(window) if window.is_visible().unwrap_or(false) => hide(app),
        Some(window) => show(app, &window),
        None => match build(app) {
            Ok(window) => show(app, &window),
            Err(error) => eprintln!("launcher window failed: {error}"),
        },
    }
}

fn build(app: &AppHandle) -> tauri::Result<tauri::WebviewWindow> {
    WebviewWindowBuilder::new(app, WINDOW, WebviewUrl::App("index.html".into()))
        .title("Zesp Connect launcher")
        .inner_size(WIDTH, HEIGHT)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .visible(false)
        .build()
}

fn show(app: &AppHandle, window: &tauri::WebviewWindow) {
    center_on_cursor_monitor(app, window);
    #[cfg(target_os = "macos")]
    let _ = app.show();
    let _ = window.show();
    let _ = window.set_focus();
    let _ = app.emit_to(WINDOW, SHOWN_EVENT, ());
}

/// Center on the screen the mouse is on, so the popup appears where the user is looking.
fn center_on_cursor_monitor(app: &AppHandle, window: &tauri::WebviewWindow) {
    let monitor = app
        .cursor_position()
        .ok()
        .and_then(|cursor| app.monitor_from_point(cursor.x, cursor.y).ok().flatten());
    let Some(monitor) = monitor else {
        let _ = window.center();
        return;
    };
    let size = LogicalSize::new(WIDTH, HEIGHT).to_physical::<f64>(monitor.scale_factor());
    let origin = monitor.position();
    let area = monitor.size();
    let _ = window.set_position(PhysicalPosition::new(
        origin.x as f64 + (area.width as f64 - size.width) / 2.0,
        origin.y as f64 + (area.height as f64 - size.height) / 2.0,
    ));
}

pub fn hide(app: &AppHandle) {
    let Some(window) = app.get_webview_window(WINDOW) else {
        return;
    };
    let _ = window.hide();
    // Hiding the app hands focus back to the app the user was in before the hotkey.
    #[cfg(target_os = "macos")]
    if !other_window_visible(app) {
        let _ = app.hide();
    }
}

#[cfg(target_os = "macos")]
fn other_window_visible(app: &AppHandle) -> bool {
    app.webview_windows()
        .iter()
        .any(|(label, window)| label != WINDOW && window.is_visible().unwrap_or(false))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::DEFAULT_SHORTCUT;

    #[test]
    fn parses_the_default_shortcut() {
        assert!(parse_shortcut(DEFAULT_SHORTCUT).is_ok());
    }

    #[test]
    fn rejects_an_unparseable_shortcut() {
        let error = parse_shortcut("Ctrl+Nope").unwrap_err();
        assert_eq!(error.to_string(), "Not a valid hotkey: Ctrl+Nope");
    }
}
