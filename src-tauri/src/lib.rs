pub mod commands;
pub mod error;
pub mod models;
pub mod platform;
pub mod services;
pub mod state;
pub mod tray;
pub mod util;

use std::path::PathBuf;

use tauri::Manager;

use crate::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| PathBuf::from("."));
            let _ = std::fs::create_dir_all(&data_dir);
            app.manage(AppState::new(data_dir));

            // Resolve tools synchronously so the first paint shows the real status.
            let status = {
                let state = app.state::<AppState>();
                let overrides = state.overrides();
                let managed_dir = state.app_data_dir();
                tauri::async_runtime::block_on(services::tools::resolve(
                    overrides.get("adb").map(String::as_str),
                    overrides.get("scrcpy").map(String::as_str),
                    &managed_dir,
                ))
            };
            app.state::<AppState>().set_tools(status);

            // Start the push-based device tracker.
            let handle = app.handle().clone();
            let tracker = services::tracker::spawn(handle.clone());
            handle.state::<AppState>().set_tracker(tracker);

            // Tray icon + menu.
            tray::build(app.handle())?;

            // Check for newer adb/scrcpy in the background; the UI polls the
            // status, so it just fills in once the request returns.
            let updater = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let _ = commands::system::check_for_updates(updater).await;
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            match event {
                // Keep a mirror window's client area on the video's aspect ratio, so
                // resizing scales both dimensions together instead of adding padding.
                tauri::WindowEvent::Resized(size) => on_mirror_resized(window, *size),
                // Closing the main window keeps the app running in the tray; quit from
                // the tray's Exit entry. A mirror window closes for real, stopping the
                // session it is rendering.
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    if window.label() == "main" {
                        api.prevent_close();
                        let _ = window.hide();
                    } else if let Some(id) = mirror_id(window.label()) {
                        platform::forget_aspect(window);
                        services::mirror_session::stop(window.app_handle(), id);
                    }
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::system::get_tools_status,
            commands::system::set_tool_path,
            commands::system::get_scrcpy_version,
            commands::system::install_tool,
            commands::system::check_for_updates,
            commands::devices::list_devices,
            commands::devices::get_device_info,
            commands::devices::set_selected_device,
            commands::devices::get_selected_device,
            commands::apps::list_apps,
            commands::apps::launch_app,
            commands::apps::uninstall_app,
            commands::apps::clear_app_data,
            commands::apps::install_apk,
            commands::controls::get_quick_state,
            commands::controls::set_toggle,
            commands::controls::set_brightness,
            commands::controls::set_volume,
            commands::controls::reboot,
            commands::mirror::start_mirror,
            commands::mirror::start_camera,
            commands::mirror::start_app_window,
            commands::mirror::start_app_on_screen,
            commands::mirror::start_desktop,
            commands::mirror::stop_mirror,
            commands::mirror::stop_all_mirrors,
            commands::mirror::list_mirrors,
            commands::mirror::open_mirror_window,
            commands::mirror::start_mirror_stream,
            commands::mirror::fit_mirror_window,
            commands::mirror::stop_mirror_session,
            commands::mirror::mirror_touch,
            commands::mirror::mirror_scroll,
            commands::mirror::mirror_key,
            commands::mirror::mirror_text,
            commands::mirror::mirror_reset_video,
            commands::mirror::mirror_phone_screen,
            commands::mirror::list_mirror_sessions,
            commands::mirror::mirror_session_fallback,
            commands::wireless::get_qr_payload,
            commands::wireless::pair_wireless_qr,
            commands::wireless::cancel_wireless_qr,
            commands::wireless::pair_wireless_code,
            commands::wireless::connect_wireless,
            commands::wireless::disconnect_wireless,
            commands::wireless::enable_wireless_tcpip,
            commands::wireless::list_mdns_services,
            commands::files::list_dir,
            commands::files::pull,
            commands::files::push,
            commands::files::mkdir,
            commands::files::delete,
            commands::console::shell_exec,
            commands::console::open_logcat,
            commands::console::close_logcat,
            commands::clipboard::clipboard_send,
            commands::clipboard::clipboard_read,
            commands::avd::list_avds,
            commands::avd::start_avd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// The session id in a `mirror-<id>` window label, if the label is one.
fn mirror_id(label: &str) -> Option<u64> {
    label
        .strip_prefix("mirror-")
        .and_then(|value| value.parse::<u64>().ok())
}

/// Nudge a resized mirror window back onto its video's aspect ratio, keeping the
/// width the user dragged and letting the height follow.
#[cfg(not(windows))]
fn enforce_aspect<R: tauri::Runtime>(window: &tauri::Window<R>, width: u32, height: u32, id: u64) {
    let Some(aspect) = window.app_handle().state::<AppState>().aspect(id) else {
        return;
    };
    // A maximized or fullscreen window is not the user dragging an edge.
    if window.is_maximized().unwrap_or(false) || window.is_fullscreen().unwrap_or(false) {
        return;
    }

    let scale = window.scale_factor().unwrap_or(1.0);
    let logical_width = width as f64 / scale;
    let logical_height = height as f64 / scale;
    let inset = services::mirror_window::vertical_inset(
        window.app_handle().state::<AppState>().session_kind(id),
    );
    let (video_width, _) = services::mirror_window::unpad((logical_width, logical_height), inset);
    let (width, height) = services::mirror_window::pad(
        services::mirror_window::lock_size(video_width, aspect),
        inset,
    );

    if (width - logical_width).abs() > 2.0 || (height - logical_height).abs() > 2.0 {
        let _ = window.set_size(tauri::LogicalSize::new(width, height));
    }
}

/// On Windows the aspect ratio is enforced by the native `WM_SIZING` hook (see
/// `platform::watch_aspect`), so the rectangle is already correct here.
#[cfg(windows)]
fn on_mirror_resized<R: tauri::Runtime>(
    _window: &tauri::Window<R>,
    _size: tauri::PhysicalSize<u32>,
) {
}

/// Elsewhere there is no resize hook, so correct the size after the fact.
#[cfg(not(windows))]
fn on_mirror_resized<R: tauri::Runtime>(window: &tauri::Window<R>, size: tauri::PhysicalSize<u32>) {
    if let Some(id) = mirror_id(window.label()) {
        enforce_aspect(window, size.width, size.height, id);
    }
}
