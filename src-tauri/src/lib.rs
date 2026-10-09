pub mod commands;
pub mod error;
pub mod models;
pub mod platform;
pub mod runtime;
pub mod services;
pub mod state;
pub mod tray;
pub mod util;

use std::path::PathBuf;

use tauri::{AppHandle, Manager};

use crate::runtime::session_window;
use crate::state::AppState;

/// The label of the app's main window (see `tauri.conf.json`).
pub(crate) const MAIN_WINDOW: &str = "main";

/// Bring the main window to the front, restoring it if hidden or minimized.
pub(crate) fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        platform::show_in_dock(app);
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// Drop the Dock icon once no window other than `closing` is visible, so the app lives only in the tray.
fn hide_from_dock_when_idle(app: &AppHandle, closing: &str) {
    let any_visible = app
        .webview_windows()
        .iter()
        .any(|(label, window)| label != closing && window.is_visible().unwrap_or(false));
    if !any_visible {
        platform::hide_from_dock(app);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_window(app)
        }))
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

            // The main window starts hidden, so start without a Dock icon until it is shown.
            platform::hide_from_dock(app.handle());

            // Resolve tools synchronously so the first paint shows the real status.
            let status =
                tauri::async_runtime::block_on(runtime::tools::resolve(&app.state::<AppState>()));
            app.state::<AppState>().set_tools(status);

            // Start the push-based device tracker.
            let handle = app.handle().clone();
            let tracker = runtime::tracker::spawn(handle.clone());
            handle.state::<AppState>().set_tracker(tracker);

            // Tray icon + menu.
            tray::build(app.handle())?;

            // Check for newer adb/scrcpy in the background; the UI polls the
            // status, so it just fills in once the request returns.
            let updater = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let _ = runtime::tools::check_for_updates(&updater).await;
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            match event {
                // Keep a mirror window's client area on the video's aspect ratio when resizing.
                tauri::WindowEvent::Resized(size) => on_mirror_resized(window, *size),
                // Closing main only hides it (quit via the tray); a mirror window closes for real, stopping its session.
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    if window.label() == MAIN_WINDOW {
                        api.prevent_close();
                        let _ = window.hide();
                        hide_from_dock_when_idle(window.app_handle(), MAIN_WINDOW);
                    } else if let Some(id) = session_window::session_id(window.label()) {
                        platform::forget_aspect(window);
                        runtime::mirror_session::stop(window.app_handle(), id);
                        hide_from_dock_when_idle(window.app_handle(), window.label());
                    }
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::system::get_tools_status,
            commands::system::set_tool_path,
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

/// Sub-pixel slack allowed when correcting a window to its video's aspect ratio.
#[cfg(not(windows))]
const RESIZE_TOLERANCE_PX: f64 = 2.0;

/// Nudge a resized mirror window back onto its video's aspect ratio, keeping the dragged width.
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

    if (width - logical_width).abs() > RESIZE_TOLERANCE_PX
        || (height - logical_height).abs() > RESIZE_TOLERANCE_PX
    {
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
    if let Some(id) = session_window::session_id(window.label()) {
        enforce_aspect(window, size.width, size.height, id);
    }
}
