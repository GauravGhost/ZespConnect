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
            // Closing the window keeps the app running in the tray; quit from
            // the tray's Exit entry.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
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
            commands::mirror::start_mirror,
            commands::mirror::start_camera,
            commands::mirror::start_app_window,
            commands::mirror::start_app_on_screen,
            commands::mirror::stop_mirror,
            commands::mirror::stop_all_mirrors,
            commands::mirror::list_mirrors,
            commands::wireless::get_qr_payload,
            commands::wireless::pair_wireless_qr,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
