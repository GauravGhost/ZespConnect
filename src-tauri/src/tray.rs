//! System tray integration (Windows, macOS and Linux).
//!
//! Mirrors an Androlaunch-style menu: a Dashboard entry, a live Devices
//! submenu, Run at Startup, quick navigation to the Console/Wireless screens,
//! Refresh Devices, and Exit. Clicks either open the window on a specific tab
//! (through the `tray:navigate` / `tray:select-device` events) or act on the
//! backend directly.

use tauri::menu::{CheckMenuItem, IsMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Wry};
use tauri_plugin_autostart::ManagerExt;

use crate::models::{Device, DeviceTransport};
use crate::state::AppState;

pub const TRAY_ID: &str = "zesp-tray";

const EVENT_NAVIGATE: &str = "tray:navigate";
const EVENT_SELECT_DEVICE: &str = "tray:select-device";

const ID_DASHBOARD: &str = "dashboard";
const ID_AUTOSTART: &str = "autostart";
const ID_CONSOLE: &str = "nav:console";
const ID_WIRELESS: &str = "nav:wireless";
const ID_REFRESH: &str = "refresh";
const ID_EXIT: &str = "exit";
const ID_NO_DEVICES: &str = "device:none";
const DEVICE_PREFIX: &str = "device:";

#[derive(Clone, serde::Serialize)]
struct NavigatePayload {
    tab: &'static str,
}

#[derive(Clone, serde::Serialize)]
struct SelectDevicePayload {
    serial: String,
}

/// Create the tray icon and its menu. Called once during setup.
pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let menu = build_menu(app)?;

    let mut builder: TrayIconBuilder<Wry> = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .tooltip("Zesp Connect")
        // On Windows a left click shows the window; macOS/Linux open the menu
        // on click, so the Dashboard entry is what brings the window forward.
        .show_menu_on_left_click(!cfg!(target_os = "windows"))
        .on_menu_event(|app, event| handle_menu(app, event.id.as_ref()))
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    // macOS menu-bar icons look best when they follow the system theme.
    #[cfg(target_os = "macos")]
    {
        builder = builder.icon_as_template(true);
    }

    builder.build(app)?;
    Ok(())
}

/// Rebuild the menu so the Devices submenu and autostart check reflect reality.
pub fn sync(app: &AppHandle) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        if let Ok(menu) = build_menu(app) {
            let _ = tray.set_menu(Some(menu));
        }
    }
}

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let dashboard = MenuItem::with_id(app, ID_DASHBOARD, "Dashboard", true, None::<&str>)?;
    let devices = build_devices_submenu(app)?;
    let autostart = CheckMenuItem::with_id(
        app,
        ID_AUTOSTART,
        "Run at Startup",
        true,
        is_autostart_enabled(app),
        None::<&str>,
    )?;
    let console = MenuItem::with_id(app, ID_CONSOLE, "Commands", true, None::<&str>)?;
    let wireless = MenuItem::with_id(app, ID_WIRELESS, "Wireless Pairing", true, None::<&str>)?;
    let refresh = MenuItem::with_id(app, ID_REFRESH, "Refresh Devices", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, ID_EXIT, "Exit", true, None::<&str>)?;

    let separator_top = PredefinedMenuItem::separator(app)?;
    let separator_bottom = PredefinedMenuItem::separator(app)?;

    let items: Vec<&dyn IsMenuItem<Wry>> = vec![
        &dashboard,
        &devices,
        &separator_top,
        &autostart,
        &console,
        &wireless,
        &refresh,
        &separator_bottom,
        &quit,
    ];

    Menu::with_items(app, &items)
}

fn build_devices_submenu(app: &AppHandle) -> tauri::Result<Submenu<Wry>> {
    let devices = app.state::<AppState>().devices();
    let connected = devices.iter().filter(|device| device.connected).count();
    let label = match connected {
        0 => "Devices".to_string(),
        count => format!("Devices ({count})"),
    };

    let entries: Vec<MenuItem<Wry>> = if devices.is_empty() {
        vec![MenuItem::with_id(
            app,
            ID_NO_DEVICES,
            "No devices",
            false,
            None::<&str>,
        )?]
    } else {
        devices
            .iter()
            .map(|device| {
                MenuItem::with_id(
                    app,
                    format!("{DEVICE_PREFIX}{}", device.serial),
                    device_label(device),
                    true,
                    None::<&str>,
                )
            })
            .collect::<tauri::Result<Vec<_>>>()?
    };

    let refs: Vec<&dyn IsMenuItem<Wry>> = entries
        .iter()
        .map(|item| item as &dyn IsMenuItem<Wry>)
        .collect();
    Submenu::with_items(app, label, true, &refs)
}

fn device_label(device: &Device) -> String {
    if !device.connected {
        return format!("{} (offline)", device.display_name);
    }
    match device.transport {
        DeviceTransport::Wifi => format!("{} · Wi-Fi", device.display_name),
        DeviceTransport::Usb => format!("{} · USB", device.display_name),
    }
}

fn handle_menu(app: &AppHandle, id: &str) {
    match id {
        ID_DASHBOARD => navigate(app, "overview"),
        ID_CONSOLE => navigate(app, "console"),
        ID_WIRELESS => navigate(app, "wireless"),
        ID_REFRESH => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                crate::services::tracker::refresh(&app).await;
            });
        }
        ID_EXIT => app.exit(0),
        ID_AUTOSTART => toggle_autostart(app),
        other => {
            if let Some(serial) = other.strip_prefix(DEVICE_PREFIX) {
                select_device(app, serial);
            }
        }
    }
}

fn navigate(app: &AppHandle, tab: &'static str) {
    show_window(app);
    let _ = app.emit(EVENT_NAVIGATE, NavigatePayload { tab });
}

fn select_device(app: &AppHandle, serial: &str) {
    show_window(app);
    let _ = app.emit(
        EVENT_SELECT_DEVICE,
        SelectDevicePayload {
            serial: serial.to_string(),
        },
    );
}

fn toggle_autostart(app: &AppHandle) {
    let manager = app.autolaunch();
    let enabled = manager.is_enabled().unwrap_or(false);
    let result = if enabled {
        manager.disable()
    } else {
        manager.enable()
    };
    if result.is_ok() {
        // Rebuild so the checkmark matches the new state.
        sync(app);
    }
}

fn is_autostart_enabled(app: &AppHandle) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

fn show_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
