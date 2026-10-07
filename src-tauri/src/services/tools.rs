use std::env;
use std::path::PathBuf;
use std::time::Duration;

use once_cell::sync::Lazy;
use regex::Regex;

use crate::models::{ToolInfo, ToolSource, ToolStatus};
use crate::platform::EXE_SUFFIX;
use crate::util::process;

const VERSION_TIMEOUT: Duration = Duration::from_secs(6);

static VERSION_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(\d+\.\d+(?:\.\d+)?)").expect("valid regex"));

/// Resolve both tools, preferring an explicit override, then `PATH`, then
/// well-known install locations.
pub async fn resolve(adb_override: Option<&str>, scrcpy_override: Option<&str>) -> ToolStatus {
    let adb = resolve_tool("adb", adb_override, &["version"]).await;
    let scrcpy = resolve_tool("scrcpy", scrcpy_override, &["--version"]).await;
    ToolStatus { adb, scrcpy }
}

async fn resolve_tool(name: &str, override_path: Option<&str>, version_args: &[&str]) -> ToolInfo {
    // 1. User override wins.
    if let Some(path) = override_path {
        let candidate = PathBuf::from(path);
        if candidate.is_file() {
            let version = version_of(&candidate, version_args).await;
            return ToolInfo {
                found: true,
                path: Some(path.to_string()),
                version,
                source: ToolSource::Override,
            };
        }
    }

    // 2. On PATH.
    if let Some(path) = find_in_path(name) {
        let version = version_of(&path, version_args).await;
        return ToolInfo {
            found: true,
            path: Some(path.to_string_lossy().to_string()),
            version,
            source: ToolSource::Path,
        };
    }

    // 3. Well-known locations.
    for path in known_locations(name) {
        if path.is_file() {
            let version = version_of(&path, version_args).await;
            return ToolInfo {
                found: true,
                path: Some(path.to_string_lossy().to_string()),
                version,
                source: ToolSource::Known,
            };
        }
    }

    ToolInfo::missing()
}

async fn version_of(path: &std::path::Path, args: &[&str]) -> Option<String> {
    let program = path.to_string_lossy().to_string();
    let argv: Vec<String> = args.iter().map(|a| a.to_string()).collect();
    let output = process::run(&program, &argv, VERSION_TIMEOUT).await.ok()?;
    if !output.success {
        return None;
    }
    let first_line = output.text().lines().next().unwrap_or_default().to_string();
    if let Some(caps) = VERSION_RE.captures(&first_line) {
        return Some(caps[1].to_string());
    }
    if first_line.trim().is_empty() {
        None
    } else {
        Some(first_line)
    }
}

fn find_in_path(name: &str) -> Option<PathBuf> {
    let exe = format!("{name}{EXE_SUFFIX}");
    let path_var = env::var_os("PATH")?;
    env::split_paths(&path_var)
        .map(|dir| dir.join(&exe))
        .find(|candidate| candidate.is_file())
}

#[cfg(windows)]
fn known_locations(name: &str) -> Vec<PathBuf> {
    let home = dirs::home_dir().unwrap_or_default();
    let local = env::var_os("LOCALAPPDATA").map(PathBuf::from);
    let program_data = env::var_os("ProgramData").map(PathBuf::from);
    let exe = format!("{name}{EXE_SUFFIX}");

    let mut paths = vec![
        home.join("scoop").join("shims").join(&exe),
        home.join("scoop")
            .join("apps")
            .join(name)
            .join("current")
            .join(&exe),
    ];
    if let Some(local) = &local {
        paths.push(
            local
                .join("Android")
                .join("Sdk")
                .join("platform-tools")
                .join(&exe),
        );
    }
    if let Some(program_data) = &program_data {
        paths.push(program_data.join("chocolatey").join("bin").join(&exe));
    }
    paths.push(PathBuf::from("C:\\Android\\platform-tools").join(&exe));
    paths
}

#[cfg(target_os = "macos")]
fn known_locations(name: &str) -> Vec<PathBuf> {
    let home = dirs::home_dir().unwrap_or_default();
    let mut paths = vec![
        PathBuf::from("/opt/homebrew/bin").join(name),
        PathBuf::from("/usr/local/bin").join(name),
        home.join(".local").join("bin").join(name),
        home.join("Library")
            .join("Android")
            .join("sdk")
            .join("platform-tools")
            .join(name),
        home.join("Android")
            .join("Sdk")
            .join("platform-tools")
            .join(name),
    ];
    if name == "scrcpy" {
        paths.push(PathBuf::from(
            "/Applications/scrcpy.app/Contents/MacOS/scrcpy",
        ));
    }
    paths
}

#[cfg(all(unix, not(target_os = "macos")))]
fn known_locations(name: &str) -> Vec<PathBuf> {
    let home = dirs::home_dir().unwrap_or_default();
    vec![
        home.join(".local").join("bin").join(name),
        PathBuf::from("/usr/local/bin").join(name),
        PathBuf::from("/usr/bin").join(name),
        PathBuf::from("/usr/lib/android-sdk/platform-tools").join(name),
        PathBuf::from("/opt/android-sdk/platform-tools").join(name),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_version_from_banners() {
        assert_eq!(
            VERSION_RE
                .captures("Android Debug Bridge version 1.0.41")
                .unwrap()[1]
                .to_string(),
            "1.0.41"
        );
        assert_eq!(
            VERSION_RE
                .captures("scrcpy 3.1 <https://github.com>")
                .unwrap()[1]
                .to_string(),
            "3.1"
        );
    }
}
