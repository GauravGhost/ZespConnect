//! scrcpy app-window lifecycle: spawn, track, stop, and report exit.
//!
//! Like `services::tracker`, this is a long-lived background coordinator, so it
//! talks to Tauri for state and events. The pure argv logic is in
//! `services::scrcpy`.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter, Manager};

use crate::error::{AppError, Result};
use crate::models::{MirrorKind, MirrorOptions, MirrorProcess};
use crate::services::{adb, scrcpy};
use crate::state::AppState;
use crate::util::{self, process};

const EVENT_MIRROR_STARTED: &str = "mirror:started";
const EVENT_MIRROR_EXITED: &str = "mirror:exited";
/// scrcpy aborts almost immediately when the device is unreachable or the serial
/// is ambiguous; give it a moment before trusting the window is up.
const STARTUP_GRACE: Duration = Duration::from_millis(1500);
const LOG_TAIL_LINES: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppDisplay {
    /// The app gets its own window on a virtual display (when it is safe).
    Window,
    /// Mirror the main display with the app brought to the foreground.
    Screen,
}

/// Open an app on the laptop. Falls back to the main display automatically on
/// Android 12–13 (and unknown versions), where a virtual display locks the phone.
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
    let title = format!("Zesp Connect — {label}");
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

/// Mirror the full device screen in a new window.
pub async fn start_mirror(
    app: &AppHandle,
    serial: &str,
    options: &MirrorOptions,
) -> Result<MirrorProcess> {
    let tools = app.state::<AppState>().tools();
    let scrcpy_bin = scrcpy::scrcpy_path(&tools)?;
    let adb_bin = adb::adb_path(&tools)?;

    let label = device_label(app, serial);
    let title = format!("Zesp Connect — {label}");
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

/// Stream the device camera in a new window.
pub async fn start_camera(
    app: &AppHandle,
    serial: &str,
    options: &MirrorOptions,
) -> Result<MirrorProcess> {
    let tools = app.state::<AppState>().tools();
    let scrcpy_bin = scrcpy::scrcpy_path(&tools)?;
    let adb_bin = adb::adb_path(&tools)?;

    let label = format!("{} camera", device_label(app, serial));
    let title = format!("Zesp Connect — {label}");
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

/// Spawn a scrcpy window, confirm it came up, register it in `AppState`, and
/// watch it until it exits (or is stopped) — then clean up and emit
/// `mirror:exited`. Shared by app windows, screen mirrors and the camera.
async fn spawn_window(app: &AppHandle, spec: WindowSpec<'_>) -> Result<MirrorProcess> {
    let log_dir = util::ensure_dir(app.state::<AppState>().app_data_dir().join("mirror-logs"))?;
    let log_path = log_dir.join(format!("{}.log", uuid::Uuid::new_v4()));
    // scrcpy finds adb on PATH, but pass our resolved binary through the ADB env
    // variable so a custom/fake path is honoured (scrcpy 4.x has no `--adb` option).
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
        started: unix_now(),
    };

    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
    app.state::<AppState>().set_mirror(info.clone(), stop_tx);

    // Notify observers the window is up (also covers mirrors opened from the
    // fallback path, which has no mutation of its own to refresh the list).
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

/// Close one tracked window.
pub fn stop(app: &AppHandle, pid: u32) -> Result<()> {
    if let Some(handle) = app.state::<AppState>().take_mirror(pid) {
        let _ = handle.stop.send(());
    }
    Ok(())
}

/// Close every tracked window.
pub fn stop_all(app: &AppHandle) {
    for handle in app.state::<AppState>().drain_mirrors() {
        let _ = handle.stop.send(());
    }
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}
