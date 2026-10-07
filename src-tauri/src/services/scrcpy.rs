//! scrcpy argv building and Android API detection for app windows.
//!
//! No Tauri types here — everything is pure or a thin adb call, so it is unit
//! tested directly. The lifecycle lives in `services::mirror`.

use std::time::Duration;

use crate::error::{AppError, Result};
use crate::models::ToolStatus;
use crate::services::{adb, updates};

const API_TIMEOUT: Duration = Duration::from_secs(15);

/// Default `-m` cap when an app is mirrored on the main display.
pub const DEFAULT_MAX_SIZE: u32 = 1024;

/// scrcpy's `--new-display` creates a virtual display. On Android 12, 12L and 13
/// the platform SystemUI crashes on it and the phone ends up locked, so those
/// versions (and any unknown version) must mirror the main display instead.
const UNSAFE_VIRTUAL_DISPLAY_APIS: &[u32] = &[31, 32, 33];

/// scrcpy added `--flex-display` in 4.0; older releases (e.g. 3.3.3) reject it
/// with "unrecognized option", so it must only be sent when supported.
const FLEX_DISPLAY_MIN_VERSION: &str = "4.0";

pub fn scrcpy_path(tools: &ToolStatus) -> Result<String> {
    tools.scrcpy.path.clone().ok_or(AppError::ScrcpyMissing)
}

/// Whether it is safe to give an app its own virtual display on this device.
pub fn virtual_display_safe(api: Option<u32>) -> bool {
    match api {
        Some(value) => !UNSAFE_VIRTUAL_DISPLAY_APIS.contains(&value),
        None => false,
    }
}

/// The device's Android API level (`ro.build.version.sdk`), if readable.
pub async fn android_api(adb_path: &str, serial: &str) -> Option<u32> {
    let output = adb::shell(
        adb_path,
        serial,
        "getprop ro.build.version.sdk",
        API_TIMEOUT,
    )
    .await
    .ok()?;
    output.stdout.trim().parse().ok()
}

/// Whether the installed scrcpy understands `--flex-display` (added in 4.0).
/// An unknown version is treated as unsupported, so we err toward omitting it.
pub fn flex_display_supported(version: Option<&str>) -> bool {
    version
        .map(|version| updates::compare(version, FLEX_DISPLAY_MIN_VERSION).is_ge())
        .unwrap_or(false)
}

/// Build the argv for an app window. With a virtual display the app gets its own
/// resizable window; otherwise the main display is mirrored with the app in front.
///
/// The adb binary is NOT an argument: scrcpy 4.x removed `--adb` and expects the
/// path in the `ADB` environment variable instead (set by the caller).
pub fn app_window_args(
    serial: &str,
    package: &str,
    window_title: &str,
    use_virtual_display: bool,
    flex_display: bool,
    max_size: u32,
) -> Vec<String> {
    let mut args = vec![
        "--serial".to_string(),
        serial.to_string(),
        "--window-title".to_string(),
        window_title.to_string(),
        format!("--start-app={package}"),
    ];

    if use_virtual_display {
        args.push("--new-display".to_string());
        if flex_display {
            args.push("--flex-display".to_string());
        } else {
            // Older scrcpy has no `--flex-display`; cap the virtual display instead.
            args.push("-m".to_string());
            args.push(max_size.to_string());
        }
    } else {
        args.push("-m".to_string());
        args.push(max_size.to_string());
    }

    args.push("--stay-awake".to_string());
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn virtual_display_is_unsafe_on_android_12_to_13_and_unknown() {
        assert!(virtual_display_safe(Some(30)));
        assert!(virtual_display_safe(Some(34)));
        assert!(!virtual_display_safe(Some(31)));
        assert!(!virtual_display_safe(Some(32)));
        assert!(!virtual_display_safe(Some(33)));
        assert!(!virtual_display_safe(None));
    }

    #[test]
    fn flex_display_needs_scrcpy_4() {
        assert!(flex_display_supported(Some("4.0")));
        assert!(flex_display_supported(Some("5.0")));
        assert!(!flex_display_supported(Some("3.3.3")));
        assert!(!flex_display_supported(None));
    }

    #[test]
    fn builds_virtual_display_app_args() {
        let args = app_window_args(
            "SERIAL",
            "com.example.app",
            "Zesp Connect — Example",
            true,
            true,
            1024,
        );

        assert!(args.contains(&"--start-app=com.example.app".to_string()));
        assert!(args.contains(&"--new-display".to_string()));
        assert!(args.contains(&"--flex-display".to_string()));
        assert!(!args.contains(&"-m".to_string()));
        // scrcpy 4.x rejects `--adb`; the path travels in the ADB env var instead.
        assert!(!args.contains(&"--adb".to_string()));
    }

    #[test]
    fn omits_flex_display_on_older_scrcpy() {
        let args = app_window_args("SERIAL", "com.example.app", "title", true, false, 1024);

        assert!(args.contains(&"--new-display".to_string()));
        assert!(!args.contains(&"--flex-display".to_string()));

        let size_index = args.iter().position(|a| a == "-m").expect("-m");
        assert_eq!(args[size_index + 1], "1024");
    }

    #[test]
    fn builds_main_display_app_args() {
        let args = app_window_args("SERIAL", "com.example.app", "title", false, false, 800);

        assert!(args.contains(&"--start-app=com.example.app".to_string()));
        assert!(!args.contains(&"--new-display".to_string()));

        let size_index = args.iter().position(|a| a == "-m").expect("-m");
        assert_eq!(args[size_index + 1], "800");
    }
}
