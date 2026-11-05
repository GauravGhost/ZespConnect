use std::collections::HashSet;
use std::env;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::error::{AppError, Result};
use crate::models::{Avd, Device, ToolStatus};
use crate::platform::EXE_SUFFIX;
use crate::services::{adb, tools as tools_service};
use crate::util::process;

const LIST_TIMEOUT: Duration = Duration::from_secs(15);
const NAME_TIMEOUT: Duration = Duration::from_secs(6);

/// Locate the SDK `emulator` binary: on `PATH`, from the Android SDK env vars,
/// beside the resolved adb (`<sdk>/platform-tools/adb`), or at a known path.
pub fn emulator_path(adb_path: Option<&str>) -> Option<String> {
    let exe = format!("emulator{EXE_SUFFIX}");

    if let Some(path) = tools_service::find_in_path("emulator") {
        return Some(path.to_string_lossy().to_string());
    }

    for var in ["ANDROID_HOME", "ANDROID_SDK_ROOT", "ANDROID_SDK_HOME"] {
        if let Some(dir) = env::var_os(var) {
            let candidate = PathBuf::from(dir).join("emulator").join(&exe);
            if candidate.is_file() {
                return Some(candidate.to_string_lossy().to_string());
            }
        }
    }

    if let Some(adb_path) = adb_path {
        if let Some(sdk) = Path::new(adb_path).parent().and_then(Path::parent) {
            let candidate = sdk.join("emulator").join(&exe);
            if candidate.is_file() {
                return Some(candidate.to_string_lossy().to_string());
            }
        }
    }

    known_locations(&exe)
        .into_iter()
        .find(|candidate| candidate.is_file())
        .map(|candidate| candidate.to_string_lossy().to_string())
}

#[cfg(windows)]
fn known_locations(exe: &str) -> Vec<PathBuf> {
    let home = dirs::home_dir().unwrap_or_default();
    let mut paths = Vec::new();
    if let Some(local) = env::var_os("LOCALAPPDATA").map(PathBuf::from) {
        paths.push(local.join("Android").join("Sdk").join("emulator").join(exe));
    }
    paths.push(PathBuf::from("C:\\Android").join("emulator").join(exe));
    paths.push(
        home.join("AppData")
            .join("Local")
            .join("Android")
            .join("Sdk")
            .join("emulator")
            .join(exe),
    );
    paths
}

#[cfg(target_os = "macos")]
fn known_locations(exe: &str) -> Vec<PathBuf> {
    let home = dirs::home_dir().unwrap_or_default();
    vec![
        home.join("Library")
            .join("Android")
            .join("sdk")
            .join("emulator")
            .join(exe),
        home.join("Android").join("Sdk").join("emulator").join(exe),
    ]
}

#[cfg(all(unix, not(target_os = "macos")))]
fn known_locations(exe: &str) -> Vec<PathBuf> {
    let home = dirs::home_dir().unwrap_or_default();
    vec![
        home.join("Android").join("Sdk").join("emulator").join(exe),
        home.join(".android").join("sdk").join("emulator").join(exe),
        PathBuf::from("/usr/lib/android-sdk/emulator").join(exe),
        PathBuf::from("/opt/android-sdk/emulator").join(exe),
    ]
}

fn require_emulator(tools: &ToolStatus) -> Result<String> {
    emulator_path(tools.adb.path.as_deref()).ok_or_else(|| {
        AppError::Other(
            "The Android emulator was not found. Install the Android SDK emulator.".into(),
        )
    })
}

/// The AVD names reported by `emulator -list-avds`, skipping banners/log lines.
pub fn parse_list(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#') && !line.starts_with("INFO"))
        .map(str::to_string)
        .collect()
}

fn validate_name(name: &str) -> Result<()> {
    let invalid = name.is_empty()
        || name
            .chars()
            .any(|c| c.is_whitespace() || c == '\0' || c == '/' || c == '\\');
    if invalid {
        return Err(AppError::Invalid(format!("Invalid AVD name: {name}")));
    }
    Ok(())
}

/// The AVDs on this machine, flagging the ones a running emulator owns.
pub async fn list(tools: &ToolStatus, devices: &[Device]) -> Result<Vec<Avd>> {
    let emulator = require_emulator(tools)?;
    let output = process::run(&emulator, &["-list-avds".to_string()], LIST_TIMEOUT).await?;
    let running = running_names(tools, devices).await;

    Ok(parse_list(&output.stdout)
        .into_iter()
        .map(|name| Avd {
            running: running.contains(&name),
            name,
        })
        .collect())
}

/// Ask each running emulator which AVD it booted (`adb emu avd name`).
async fn running_names(tools: &ToolStatus, devices: &[Device]) -> HashSet<String> {
    let mut names = HashSet::new();
    let Ok(adb_path) = adb::adb_path(tools) else {
        return names;
    };

    for device in devices
        .iter()
        .filter(|device| device.serial.starts_with("emulator-"))
    {
        let args = vec!["emu".to_string(), "avd".to_string(), "name".to_string()];
        let Ok(output) = adb::run_scoped(&adb_path, &device.serial, &args, NAME_TIMEOUT).await
        else {
            continue;
        };
        if let Some(name) = output
            .stdout
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty() && *line != "OK")
        {
            names.insert(name.to_string());
        }
    }
    names
}

/// Boot an AVD in the background. The emulator window is the user's to close.
pub async fn start(tools: &ToolStatus, name: &str, log_path: &Path) -> Result<()> {
    validate_name(name)?;
    let emulator = require_emulator(tools)?;
    let args = vec!["-avd".to_string(), name.to_string()];
    process::spawn_detached(&emulator, &args, log_path).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_avd_names_and_skips_noise() {
        let text = "\
INFO | Storing crashdata in: /tmp
# comment
Pixel_8_API_35
Nexus_5X_API_30

";
        assert_eq!(parse_list(text), vec!["Pixel_8_API_35", "Nexus_5X_API_30"]);
    }

    #[test]
    fn rejects_unsafe_avd_names() {
        assert!(validate_name("Pixel_8_API_35").is_ok());
        assert!(validate_name("").is_err());
        assert!(validate_name("My AVD").is_err());
        assert!(validate_name("../escape").is_err());
    }
}
