//! scrcpy app-window lifecycle: spawn, track, stop, report exit. Tauri-coupled like the
//! other runtime coordinators; the pure argv logic is in `services::scrcpy`.

use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager, WebviewWindow};

use crate::error::{AppError, Result};
use crate::models::{MirrorKind, MirrorOptions, MirrorProcess};
use crate::services::{adb, mirror_window, scrcpy};
use crate::state::AppState;
use crate::util::{self, process};

const EVENT_MIRROR_STARTED: &str = "mirror:started";
const EVENT_MIRROR_EXITED: &str = "mirror:exited";
/// scrcpy aborts almost immediately on an unreachable device; wait a moment before trusting the window.
const STARTUP_GRACE: Duration = Duration::from_millis(1500);
const LOG_TAIL_LINES: usize = 4;
/// Fallback desktop size when no monitor can be read (e.g. a headless session).
const FALLBACK_DESKTOP_WIDTH: u32 = 1920;
const FALLBACK_DESKTOP_HEIGHT: u32 = 1080;
const FALLBACK_SCALE: f64 = 1.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppDisplay {
    /// The app gets its own window on a virtual display (when it is safe).
    Window,
    /// Mirror the main display with the app brought to the foreground.
    Screen,
}

/// Open an app; falls back to the main display on Android versions where a virtual display locks the phone.
pub async fn start_app(
    app: &AppHandle,
    serial: &str,
    package: &str,
    label: &str,
    display: AppDisplay,
) -> Result<MirrorProcess> {
    let tools = app.state::<AppState>().tools();
    let scrcpy_bin = scrcpy::scrcpy_path(&tools)?;
    let adb_bin = adb::adb_path(&tools)?;

    let use_virtual = match display {
        AppDisplay::Screen => false,
        AppDisplay::Window => {
            scrcpy::virtual_display_safe(scrcpy::android_api(&adb_bin, serial).await)
        }
    };
    // `--flex-display` only exists in scrcpy 4.0+; omit it on older releases.
    let flex = use_virtual && scrcpy::flex_display_supported(tools.scrcpy.version.as_deref());
    let no_decorations =
        use_virtual && scrcpy::vd_decorations_option_supported(tools.scrcpy.version.as_deref());
    let kind = if use_virtual {
        MirrorKind::App
    } else {
        MirrorKind::ScreenApp
    };

    let label = if label.trim().is_empty() {
        package.to_string()
    } else {
        label.to_string()
    };
    let title = format!("Zesp Connect - {label}");
    let args = scrcpy::app_window_args(
        serial,
        package,
        &title,
        use_virtual,
        flex,
        no_decorations,
        scrcpy::DEFAULT_MAX_SIZE,
    );

    spawn_window(
        app,
        WindowSpec {
            serial,
            kind,
            package: Some(package.to_string()),
            label,
            scrcpy_bin: &scrcpy_bin,
            adb_bin: &adb_bin,
            args: &args,
        },
    )
    .await
}

pub async fn start_mirror(
    app: &AppHandle,
    serial: &str,
    options: &MirrorOptions,
) -> Result<MirrorProcess> {
    let tools = app.state::<AppState>().tools();
    let scrcpy_bin = scrcpy::scrcpy_path(&tools)?;
    let adb_bin = adb::adb_path(&tools)?;

    let label = device_label(app, serial);
    let title = format!("Zesp Connect - {label}");
    let args = scrcpy::mirror_args(serial, &title, options);

    spawn_window(
        app,
        WindowSpec {
            serial,
            kind: MirrorKind::Mirror,
            package: None,
            label,
            scrcpy_bin: &scrcpy_bin,
            adb_bin: &adb_bin,
            args: &args,
        },
    )
    .await
}

/// Open Android's desktop mode on a virtual display of `display` (`WxH/dpi`) in a new window.
pub async fn start_desktop(app: &AppHandle, serial: &str, display: &str) -> Result<MirrorProcess> {
    let tools = app.state::<AppState>().tools();
    let scrcpy_bin = scrcpy::scrcpy_path(&tools)?;
    let adb_bin = adb::adb_path(&tools)?;
    scrcpy::prepare_desktop(&adb_bin, serial).await?;

    let label = format!("{} desktop", device_label(app, serial));
    let title = format!("Zesp Connect - {label}");
    let args = scrcpy::desktop_args(serial, &title, display);

    spawn_window(
        app,
        WindowSpec {
            serial,
            kind: MirrorKind::Desktop,
            package: None,
            label,
            scrcpy_bin: &scrcpy_bin,
            adb_bin: &adb_bin,
            args: &args,
        },
    )
    .await
}

pub async fn start_camera(
    app: &AppHandle,
    serial: &str,
    options: &MirrorOptions,
) -> Result<MirrorProcess> {
    let tools = app.state::<AppState>().tools();
    let scrcpy_bin = scrcpy::scrcpy_path(&tools)?;
    let adb_bin = adb::adb_path(&tools)?;

    let label = format!("{} camera", device_label(app, serial));
    let title = format!("Zesp Connect - {label}");
    let args = scrcpy::camera_args(serial, &title, options);

    spawn_window(
        app,
        WindowSpec {
            serial,
            kind: MirrorKind::Camera,
            package: None,
            label,
            scrcpy_bin: &scrcpy_bin,
            adb_bin: &adb_bin,
            args: &args,
        },
    )
    .await
}

/// The `WxH/dpi` desktop display for the main window's screen, falling back to the primary monitor.
pub fn screen_desktop_display(app: &AppHandle) -> String {
    let monitor = app
        .get_webview_window("main")
        .and_then(|window| window.current_monitor().ok().flatten())
        .or_else(|| app.primary_monitor().ok().flatten());
    match monitor {
        Some(monitor) => scrcpy::desktop_display(
            monitor.size().width,
            monitor.size().height,
            monitor.scale_factor(),
        ),
        None => scrcpy::desktop_display(
            FALLBACK_DESKTOP_WIDTH,
            FALLBACK_DESKTOP_HEIGHT,
            FALLBACK_SCALE,
        ),
    }
}

/// The longer side of a window's video area in logical pixels, kept steady across rotation.
pub fn window_long_side(window: &WebviewWindow, inset: f64) -> f64 {
    let scale = window.scale_factor().unwrap_or(FALLBACK_SCALE);
    let Ok(size) = window.inner_size() else {
        return mirror_window::DEFAULT_LONG_SIDE;
    };
    let (width, height) = mirror_window::unpad(
        (size.width as f64 / scale, size.height as f64 / scale),
        inset,
    );
    width.max(height)
}

/// A human name for a connected device, falling back to its serial.
fn device_label(app: &AppHandle, serial: &str) -> String {
    app.state::<AppState>()
        .devices()
        .into_iter()
        .find(|device| device.serial == serial)
        .map(|device| device.display_name)
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| serial.to_string())
}

/// Everything needed to launch and track one scrcpy window.
struct WindowSpec<'a> {
    serial: &'a str,
    kind: MirrorKind,
    package: Option<String>,
    label: String,
    scrcpy_bin: &'a str,
    adb_bin: &'a str,
    args: &'a [String],
}

/// Spawn a scrcpy window, register it, and watch it until it exits; then clean up and emit `mirror:exited`.
async fn spawn_window(app: &AppHandle, spec: WindowSpec<'_>) -> Result<MirrorProcess> {
    let log_dir = util::ensure_dir(app.state::<AppState>().app_data_dir().join("mirror-logs"))?;
    let log_path = log_dir.join(format!("{}.log", uuid::Uuid::new_v4()));
    // Pass our resolved adb via the ADB env var (scrcpy 4.x has no `--adb` option).
    let child_env = [("ADB", spec.adb_bin)];
    let mut child = process::spawn_captured(spec.scrcpy_bin, spec.args, &log_path, &child_env)?;

    tokio::time::sleep(STARTUP_GRACE).await;
    if let Some(status) = child.try_wait()? {
        let tail = process::read_log_tail(&log_path, LOG_TAIL_LINES);
        let _ = std::fs::remove_file(&log_path);
        let message = if tail.is_empty() {
            format!("scrcpy exited ({status})")
        } else {
            tail
        };
        return Err(AppError::CommandFailed(message));
    }

    let pid = child.id().unwrap_or_default();
    let info = MirrorProcess {
        pid,
        serial: spec.serial.to_string(),
        kind: spec.kind,
        package: spec.package,
        label: Some(spec.label),
        started: util::time::unix_now(),
    };

    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
    app.state::<AppState>().set_mirror(info.clone(), stop_tx);

    // Notify observers the window is up (also covers mirrors opened from the fallback path).
    let _ = app.emit(EVENT_MIRROR_STARTED, &info);

    // Watch until the window closes (or we ask it to), then clean up and notify.
    let handle = app.clone();
    let event_info = info.clone();
    tauri::async_runtime::spawn(async move {
        tokio::select! {
            _ = child.wait() => {}
            _ = stop_rx => {
                let _ = child.kill().await;
                let _ = child.wait().await;
            }
        }
        handle.state::<AppState>().remove_mirror(pid);
        let _ = std::fs::remove_file(&log_path);
        let _ = handle.emit(EVENT_MIRROR_EXITED, &event_info);
    });

    Ok(info)
}

pub fn stop(app: &AppHandle, pid: u32) -> Result<()> {
    if let Some(handle) = app.state::<AppState>().take_mirror(pid) {
        let _ = handle.stop.send(());
    }
    Ok(())
}

pub fn stop_all(app: &AppHandle) {
    for handle in app.state::<AppState>().drain_mirrors() {
        let _ = handle.stop.send(());
    }
}
