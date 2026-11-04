//! System tray integration (Windows, macOS and Linux). The menu offers quick actions on the active
//! phone (mirror, camera, launcher, media keys, sound and clipboard), a Devices submenu, a Go To
//! submenu for every tab, Run at Startup and Exit. Clicks either open the window on a tab (via the
//! `tray:*` events) or act on the backend directly.

mod actions;
mod menu;

use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Wry};

use crate::models::{MediaKey, MirrorKind};
use crate::show_main_window;

const TRAY_ID: &str = "zesp-tray";

/// Create the tray icon and its menu. Called once during setup.
pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let menu = menu::build(app)?;

    let mut builder: TrayIconBuilder<Wry> = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .tooltip("Zesp Connect")
        // On Windows a left click shows the window; macOS/Linux open the menu, so Dashboard brings it forward.
        .show_menu_on_left_click(!cfg!(target_os = "windows"))
        .on_menu_event(|app, event| handle_menu(app, event.id.as_ref()))
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });

    // macOS draws template icons from their alpha channel only, so the menu bar needs a
    // transparent outline glyph; the full-color app icon would render as a solid square.
    #[cfg(target_os = "macos")]
    {
        builder = builder
            .icon(tauri::include_image!("icons/tray.png"))
            .icon_as_template(true);
    }

    #[cfg(not(target_os = "macos"))]
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder.build(app)?;
    Ok(())
}

/// Rebuild the menu so the device entries and check marks reflect reality.
pub fn sync(app: &AppHandle) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        if let Ok(menu) = menu::build(app) {
            let _ = tray.set_menu(Some(menu));
        }
    }
}

fn handle_menu(app: &AppHandle, id: &str) {
    match id {
        menu::ID_DASHBOARD => actions::navigate(app, "overview"),
        menu::ID_MIRROR_SCREEN => actions::open_mirror(app, MirrorKind::Mirror),
        menu::ID_MIRROR_CAMERA => actions::open_mirror(app, MirrorKind::Camera),
        menu::ID_STOP_MIRRORS => actions::stop_all_mirrors(app),
        menu::ID_LAUNCHER => actions::toggle_launcher(app),
        menu::ID_MEDIA_PLAY_PAUSE => actions::press_media_key(app, MediaKey::PlayPause),
        menu::ID_MEDIA_NEXT => actions::press_media_key(app, MediaKey::Next),
        menu::ID_MEDIA_PREVIOUS => actions::press_media_key(app, MediaKey::Previous),
        menu::ID_PHONE_SOUND => actions::toggle_phone_sound(app),
        menu::ID_CLIPBOARD_SYNC => actions::toggle_clipboard_sync(app),
        menu::ID_REFRESH => actions::refresh_devices(app),
        menu::ID_AUTOSTART => actions::toggle_autostart(app),
        menu::ID_EXIT => app.exit(0),
        other => {
            if let Some(tab) = menu::tab_for(other) {
                actions::navigate(app, tab);
            } else if let Some(serial) = menu::device_serial_for(other) {
                actions::select_device(app, serial);
            }
        }
    }
}
