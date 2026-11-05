use std::future::Future;

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;

use crate::error::Result;
use crate::models::{
    Device, LauncherTarget, MediaKey, MirrorKind, MirrorOptions, MirrorSessionSpec,
};
use crate::runtime::{
    clipboard_sync, launcher, mirror, mirror_session, now_playing, phone_audio, session_window,
    tracker,
};
use crate::services::launcher::resolve_target;
use crate::show_main_window;
use crate::state::AppState;

const EVENT_NAVIGATE: &str = "tray:navigate";
const EVENT_SELECT_DEVICE: &str = "tray:select-device";
const EVENT_ERROR: &str = "tray:error";

/// Matches the frontend's `DEFAULT_MIRROR_OPTIONS`, so a tray mirror looks like one started in-app.
const MIRROR_BITRATE_MBPS: u32 = 8;

#[derive(Clone, serde::Serialize)]
struct NavigatePayload {
    tab: &'static str,
}

#[derive(Clone, serde::Serialize)]
struct SelectDevicePayload {
    serial: String,
}

#[derive(Clone, serde::Serialize)]
struct ErrorPayload {
    message: String,
}

/// The connected phone that device actions apply to, if there is one.
pub(super) fn active_device(app: &AppHandle) -> Option<Device> {
    let state = app.state::<AppState>();
    match resolve_target(state.selected_serial().as_deref(), &state.devices()) {
        LauncherTarget::Ready { serial, .. } => state.device(&serial),
        _ => None,
    }
}

pub(super) fn navigate(app: &AppHandle, tab: &'static str) {
    show_main_window(app);
    let _ = app.emit(EVENT_NAVIGATE, NavigatePayload { tab });
}

pub(super) fn select_device(app: &AppHandle, serial: &str) {
    // Persist the choice first so the tray's checkmark moves immediately.
    let _ = app
        .state::<AppState>()
        .set_selected_serial(Some(serial.to_string()));
    super::sync(app);

    show_main_window(app);
    let _ = app.emit(
        EVENT_SELECT_DEVICE,
        SelectDevicePayload {
            serial: serial.to_string(),
        },
    );
}

pub(super) fn open_mirror(app: &AppHandle, kind: MirrorKind) {
    let Some(device) = active_device(app) else {
        return;
    };
    let spec = MirrorSessionSpec {
        serial: device.serial,
        kind,
        package: None,
        label: device.display_name,
        options: mirror_options(),
        desktop_display: None,
    };
    // Building a window inside a menu event handler deadlocks on Windows.
    run_reporting_errors(app, |app| async move {
        session_window::open(&app, spec).map(drop)
    });
}

fn mirror_options() -> MirrorOptions {
    MirrorOptions {
        bitrate: Some(MIRROR_BITRATE_MBPS),
        audio: true,
        stay_awake: true,
        aspect_lock: true,
        camera_facing: Some("back".to_string()),
        ..MirrorOptions::default()
    }
}

pub(super) fn stop_all_mirrors(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        mirror::stop_all(&app);
        mirror_session::stop_all(&app);
    });
}

pub(super) fn toggle_launcher(app: &AppHandle) {
    // Building a window inside a menu event handler deadlocks on Windows.
    let app = app.clone();
    tauri::async_runtime::spawn(async move { launcher::toggle(&app) });
}

pub(super) fn press_media_key(app: &AppHandle, key: MediaKey) {
    let Some(serial) = active_device(app).map(|device| device.serial) else {
        return;
    };
    run_reporting_errors(app, move |app| async move {
        now_playing::press(&app, &serial, key).await
    });
}

/// Route the phone's sound to the computer, or give it back to the phone.
pub(super) fn toggle_phone_sound(app: &AppHandle) {
    let Some(serial) = active_device(app).map(|device| device.serial) else {
        return;
    };
    run_reporting_errors(app, move |app| async move {
        let result = if app.state::<AppState>().phone_audio().contains(&serial) {
            phone_audio::stop(&app, &serial);
            Ok(())
        } else {
            phone_audio::start(&app, &serial).await
        };
        // The OS flips the check mark on click; a failed start must flip it back.
        super::sync(&app);
        result
    });
}

pub(super) fn toggle_clipboard_sync(app: &AppHandle) {
    let Some(serial) = active_device(app).map(|device| device.serial) else {
        return;
    };
    run_reporting_errors(app, move |app| async move {
        let result = if app.state::<AppState>().clipboard_sync().contains(&serial) {
            clipboard_sync::disable(&app, &serial)
        } else {
            clipboard_sync::enable(&app, &serial).await
        };
        // The OS flips the check mark on click; a failed start must flip it back.
        super::sync(&app);
        result
    });
}

pub(super) fn refresh_devices(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move { tracker::refresh(&app).await });
}

pub(super) fn toggle_autostart(app: &AppHandle) {
    let manager = app.autolaunch();
    let enabled = manager.is_enabled().unwrap_or(false);
    let result = if enabled {
        manager.disable()
    } else {
        manager.enable()
    };
    if result.is_ok() {
        // Rebuild so the checkmark matches the new state.
        super::sync(app);
    }
}

/// Run `action` off the menu thread; if it fails, show the window with the error.
fn run_reporting_errors<F, Fut>(app: &AppHandle, action: F)
where
    F: FnOnce(AppHandle) -> Fut + Send + 'static,
    Fut: Future<Output = Result<()>> + Send,
{
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(error) = action(app.clone()).await {
            show_main_window(&app);
            let _ = app.emit(
                EVENT_ERROR,
                ErrorPayload {
                    message: error.to_string(),
                },
            );
        }
    });
}
