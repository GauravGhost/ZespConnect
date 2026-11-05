use tauri::menu::{CheckMenuItem, IsMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Manager, Wry};
use tauri_plugin_autostart::ManagerExt;

use crate::models::{Device, DeviceTransport};
use crate::state::AppState;

use super::actions;

pub(super) const ID_DASHBOARD: &str = "dashboard";
pub(super) const ID_MIRROR_SCREEN: &str = "mirror:screen";
pub(super) const ID_MIRROR_CAMERA: &str = "mirror:camera";
pub(super) const ID_STOP_MIRRORS: &str = "mirror:stop-all";
pub(super) const ID_LAUNCHER: &str = "launcher";
pub(super) const ID_MEDIA_PLAY_PAUSE: &str = "media:play-pause";
pub(super) const ID_MEDIA_NEXT: &str = "media:next";
pub(super) const ID_MEDIA_PREVIOUS: &str = "media:previous";
pub(super) const ID_PHONE_SOUND: &str = "phone-sound";
pub(super) const ID_CLIPBOARD_SYNC: &str = "clipboard-sync";
pub(super) const ID_REFRESH: &str = "refresh";
pub(super) const ID_AUTOSTART: &str = "autostart";
pub(super) const ID_EXIT: &str = "exit";

const ID_ACTIVE_DEVICE: &str = "active-device";
const ID_NO_DEVICES: &str = "devices:none";
const DEVICE_PREFIX: &str = "device:";
const NAV_PREFIX: &str = "nav:";

/// The app's tabs as the frontend names them (see `src/app/navigation.ts`), in sidebar order.
const TABS: [(&str, &str); 9] = [
    ("overview", "Overview"),
    ("features", "Features"),
    ("apps", "Apps"),
    ("mirror", "Mirror"),
    ("files", "Files"),
    ("console", "Console"),
    ("wireless", "Wireless"),
    ("emulators", "Emulators"),
    ("settings", "Settings"),
];

/// The tab a Go To item opens.
pub(super) fn tab_for(id: &str) -> Option<&'static str> {
    let tab = id.strip_prefix(NAV_PREFIX)?;
    TABS.iter().map(|(id, _)| *id).find(|known| *known == tab)
}

/// The device a Devices submenu item selects.
pub(super) fn device_serial_for(id: &str) -> Option<&str> {
    id.strip_prefix(DEVICE_PREFIX)
}

pub(super) fn build(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let active = actions::active_device(app);

    let dashboard = MenuItem::with_id(app, ID_DASHBOARD, "Dashboard", true, None::<&str>)?;
    let devices = build_devices_submenu(app)?;
    let active_label = active
        .as_ref()
        .map_or_else(|| "No active device".to_string(), device_label);
    let active_header =
        MenuItem::with_id(app, ID_ACTIVE_DEVICE, active_label, false, None::<&str>)?;

    let has_device = active.is_some();
    let mirror = MenuItem::with_id(
        app,
        ID_MIRROR_SCREEN,
        "Mirror Screen",
        has_device,
        None::<&str>,
    )?;
    let camera = MenuItem::with_id(
        app,
        ID_MIRROR_CAMERA,
        "Open Camera",
        has_device,
        None::<&str>,
    )?;
    let launcher = MenuItem::with_id(app, ID_LAUNCHER, "App Launcher", true, None::<&str>)?;
    let media = build_media_submenu(app, has_device)?;
    let (sound_on_computer, clipboard_synced) = active.as_ref().map_or((false, false), |device| {
        let state = app.state::<AppState>();
        (
            state.phone_audio().contains(&device.serial),
            state.clipboard_sync().contains(&device.serial),
        )
    });
    let phone_sound = CheckMenuItem::with_id(
        app,
        ID_PHONE_SOUND,
        "Play Phone Sound Here",
        has_device,
        sound_on_computer,
        None::<&str>,
    )?;
    let clipboard = CheckMenuItem::with_id(
        app,
        ID_CLIPBOARD_SYNC,
        "Sync Clipboard",
        has_device,
        clipboard_synced,
        None::<&str>,
    )?;
    let stop_mirrors = MenuItem::with_id(
        app,
        ID_STOP_MIRRORS,
        "Close All Mirrors",
        true,
        None::<&str>,
    )?;

    let go_to = build_go_to_submenu(app)?;
    let refresh = MenuItem::with_id(app, ID_REFRESH, "Refresh Devices", true, None::<&str>)?;
    let autostart = CheckMenuItem::with_id(
        app,
        ID_AUTOSTART,
        "Run at Startup",
        true,
        is_autostart_enabled(app),
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, ID_EXIT, "Exit", true, None::<&str>)?;

    let separators = [
        PredefinedMenuItem::separator(app)?,
        PredefinedMenuItem::separator(app)?,
        PredefinedMenuItem::separator(app)?,
        PredefinedMenuItem::separator(app)?,
    ];

    let items: Vec<&dyn IsMenuItem<Wry>> = vec![
        &dashboard,
        &devices,
        &separators[0],
        &active_header,
        &mirror,
        &camera,
        &launcher,
        &media,
        &phone_sound,
        &clipboard,
        &separators[1],
        &stop_mirrors,
        &separators[2],
        &go_to,
        &refresh,
        &autostart,
        &separators[3],
        &quit,
    ];

    Menu::with_items(app, &items)
}

fn build_devices_submenu(app: &AppHandle) -> tauri::Result<Submenu<Wry>> {
    let devices = app.state::<AppState>().devices();
    let selected = app.state::<AppState>().selected_serial();
    let label = match devices.len() {
        0 => "Devices".to_string(),
        count => format!("Devices ({count})"),
    };

    let entries: Vec<CheckMenuItem<Wry>> = if devices.is_empty() {
        vec![CheckMenuItem::with_id(
            app,
            ID_NO_DEVICES,
            "No devices",
            false,
            false,
            None::<&str>,
        )?]
    } else {
        devices
            .iter()
            .map(|device| {
                CheckMenuItem::with_id(
                    app,
                    format!("{DEVICE_PREFIX}{}", device.serial),
                    device_label(device),
                    true,
                    Some(device.serial.clone()) == selected,
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

fn build_media_submenu(app: &AppHandle, enabled: bool) -> tauri::Result<Submenu<Wry>> {
    let play_pause =
        MenuItem::with_id(app, ID_MEDIA_PLAY_PAUSE, "Play / Pause", true, None::<&str>)?;
    let next = MenuItem::with_id(app, ID_MEDIA_NEXT, "Next Track", true, None::<&str>)?;
    let previous = MenuItem::with_id(app, ID_MEDIA_PREVIOUS, "Previous Track", true, None::<&str>)?;
    Submenu::with_items(app, "Media", enabled, &[&play_pause, &next, &previous])
}

fn build_go_to_submenu(app: &AppHandle) -> tauri::Result<Submenu<Wry>> {
    let entries = TABS
        .iter()
        .map(|(tab, label)| {
            MenuItem::with_id(
                app,
                format!("{NAV_PREFIX}{tab}"),
                *label,
                true,
                None::<&str>,
            )
        })
        .collect::<tauri::Result<Vec<_>>>()?;
    let refs: Vec<&dyn IsMenuItem<Wry>> = entries
        .iter()
        .map(|item| item as &dyn IsMenuItem<Wry>)
        .collect();
    Submenu::with_items(app, "Go To", true, &refs)
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

fn is_autostart_enabled(app: &AppHandle) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn go_to_ids_map_to_known_tabs_only() {
        assert_eq!(tab_for("nav:files"), Some("files"));
        assert_eq!(tab_for("nav:unknown"), None);
        assert_eq!(tab_for("files"), None);
    }

    #[test]
    fn device_ids_yield_their_serial() {
        assert_eq!(
            device_serial_for("device:emulator-5554"),
            Some("emulator-5554")
        );
        assert_eq!(
            device_serial_for("device:192.168.1.5:5555"),
            Some("192.168.1.5:5555")
        );
        assert_eq!(device_serial_for(ID_NO_DEVICES), None);
        assert_eq!(device_serial_for("mirror:screen"), None);
    }
}
