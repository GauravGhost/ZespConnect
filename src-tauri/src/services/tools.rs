use std::env;
use std::path::{Path, PathBuf};
use std::time::Duration;

use once_cell::sync::Lazy;
use regex::Regex;

use crate::models::{ToolInfo, ToolSource, ToolStatus};
use crate::platform::EXE_SUFFIX;
use crate::services::updates;
use crate::util::process;

const VERSION_TIMEOUT: Duration = Duration::from_secs(6);

static VERSION_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(\d+\.\d+(?:\.\d+)?)").expect("valid regex"));

/// `adb --version` prints the ADB protocol version first (`Android Debug Bridge
/// version 1.0.41`) and the platform-tools release second (`Version 37.0.1-…`).
/// Only the latter is comparable to the upstream revision, so pull it out
/// separately (capital `Version`, so the protocol line does not match).
static PLATFORM_TOOLS_VERSION_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"Version\s+(\d+\.\d+\.\d+)").expect("valid regex"));

/// Resolve both tools, preferring an explicit override, then `PATH`, then
/// well-known install locations, and finally the app-managed download directory.
pub async fn resolve(
    adb_override: Option<&str>,
    scrcpy_override: Option<&str>,
    managed_dir: &Path,
) -> ToolStatus {
    let adb = resolve_tool("adb", adb_override, &["version"], managed_dir).await;
    let scrcpy = resolve_tool("scrcpy", scrcpy_override, &["--version"], managed_dir).await;
    ToolStatus {
        adb,
        scrcpy,
        ..ToolStatus::default()
    }
}

async fn resolve_tool(
    name: &str,
    override_path: Option<&str>,
    version_args: &[&str],
    managed_dir: &Path,
) -> ToolInfo {
    // 1. User override wins.
    if let Some(path) = override_path {
        let candidate = PathBuf::from(path);
        if candidate.is_file() {
            let version = version_of(name, &candidate, version_args).await;
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
        let version = version_of(name, &path, version_args).await;
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
            let version = version_of(name, &path, version_args).await;
            return ToolInfo {
                found: true,
                path: Some(path.to_string_lossy().to_string()),
                version,
                source: ToolSource::Known,
            };
        }
    }

    // 4. A copy the app downloaded for itself (no override set).
    if let Some(path) = managed_binary(managed_dir, name) {
        let version = version_of(name, &path, version_args).await;
        return ToolInfo {
            found: true,
            path: Some(path.to_string_lossy().to_string()),
            version,
            source: ToolSource::Known,
        };
    }

    ToolInfo::missing()
}

/// `<app_data_dir>/tools` — where downloaded adb/scrcpy copies live.
pub fn managed_root(managed_dir: &Path) -> PathBuf {
    managed_dir.join("tools")
}

/// The upstream package name for a tool inside the managed root. adb ships in
/// Google's `platform-tools`, scrcpy in its own directory.
pub fn managed_slot(name: &str) -> &'static str {
    if name == "adb" {
        "platform-tools"
    } else {
        "scrcpy"
    }
}

/// The newest managed binary for `name`, scanning version subdirectories newest
/// first. Returns `None` when the app has never downloaded this tool.
pub fn managed_binary(managed_dir: &Path, name: &str) -> Option<PathBuf> {
    let base = managed_root(managed_dir).join(managed_slot(name));
    let exe = format!("{name}{EXE_SUFFIX}");

    let mut versions: Vec<PathBuf> = std::fs::read_dir(&base)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    versions.sort_by(|a, b| updates::compare(&dir_version(b), &dir_version(a)));

    for dir in versions {
        let mut found = Vec::new();
        find_named_files(&dir, &exe, 3, &mut found);
        if let Some(path) = found.into_iter().next() {
            return Some(path);
        }
    }
    None
}

fn dir_version(dir: &Path) -> String {
    dir.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_string()
}

/// The platform-tools release version reported by `adb --version` (e.g. `37.0.1`),
/// used to compare against Google's published revision.
pub async fn platform_tools_version(adb_path: &str) -> Option<String> {
    let output = process::run(adb_path, &["version".to_string()], VERSION_TIMEOUT)
        .await
        .ok()?;
    platform_tools_version_of(&output.text())
}

pub fn platform_tools_version_of(output: &str) -> Option<String> {
    PLATFORM_TOOLS_VERSION_RE
        .captures(output)
        .map(|caps| caps[1].to_string())
}

async fn version_of(name: &str, path: &std::path::Path, args: &[&str]) -> Option<String> {
    let program = path.to_string_lossy().to_string();
    let argv: Vec<String> = args.iter().map(|a| a.to_string()).collect();
    let output = process::run(&program, &argv, VERSION_TIMEOUT).await.ok()?;
    if !output.success {
        return None;
    }
    preferred_version(name, &output.text())
}

/// The human-meaningful version from a tool's `--version` banner.
///
/// `adb` prints its ADB **protocol** version first (`Android Debug Bridge
/// version 1.0.41` — a constant) and the **platform-tools release** second
/// (`Version 36.0.0-…`). The release is what matches upstream and the update
/// check, so prefer it. Everything else uses the first version token.
fn preferred_version(name: &str, text: &str) -> Option<String> {
    if name == "adb" {
        if let Some(version) = platform_tools_version_of(text) {
            return Some(version);
        }
    }
    let first_line = text.lines().next().unwrap_or_default().to_string();
    if let Some(caps) = VERSION_RE.captures(&first_line) {
        return Some(caps[1].to_string());
    }
    if first_line.trim().is_empty() {
        None
    } else {
        Some(first_line)
    }
}

/// Look up an executable on `PATH` (with the platform's `.exe` suffix).
pub fn find_in_path(name: &str) -> Option<PathBuf> {
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
        // winget installs shims here (e.g. scrcpy after `winget install`).
        paths.push(
            local
                .join("Microsoft")
                .join("WinGet")
                .join("Links")
                .join(&exe),
        );
    }
    if let Some(program_data) = &program_data {
        paths.push(program_data.join("chocolatey").join("bin").join(&exe));
    }
    paths.push(PathBuf::from("C:\\Android\\platform-tools").join(&exe));
    // Portable packages (e.g. scrcpy) land under WinGet\Packages with no shim.
    paths.extend(winget_package_binaries(name));
    paths
}

/// Search `%LOCALAPPDATA%\Microsoft\WinGet\Packages` for `<name>.exe`.
///
/// winget extracts portable packages into a versioned subfolder without a
/// `Links` shim, so the binary is only reachable by scanning.
#[cfg(windows)]
fn winget_package_binaries(name: &str) -> Vec<PathBuf> {
    let Some(local) = env::var_os("LOCALAPPDATA").map(PathBuf::from) else {
        return Vec::new();
    };
    let base = local.join("Microsoft").join("WinGet").join("Packages");
    let exe = format!("{name}{EXE_SUFFIX}");

    let mut found = Vec::new();
    find_named_files(&base, &exe, 3, &mut found);
    found
}

/// Find `name` (plus the platform executable suffix) under `root`, searching a
/// few levels deep. Used to locate the binary inside an extracted archive.
pub fn find_binary(root: &Path, name: &str) -> Option<PathBuf> {
    let exe = format!("{name}{EXE_SUFFIX}");
    let mut found = Vec::new();
    find_named_files(root, &exe, 4, &mut found);
    found.into_iter().next()
}

/// Shallow recursive search for `file_name` under `dir` (bounded by `depth`).
/// Used for winget's un-shimmed portable packages and the app-managed folder.
fn find_named_files(dir: &Path, file_name: &str, depth: usize, out: &mut Vec<PathBuf>) {
    if depth == 0 || out.len() >= 4 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            find_named_files(&path, file_name, depth - 1, out);
        } else if path.file_name().and_then(|n| n.to_str()) == Some(file_name) {
            out.push(path);
        }
    }
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

    #[cfg(windows)]
    #[test]
    fn finds_a_nested_portable_binary() {
        let root = std::env::temp_dir().join("zesp-winget-scan-test");
        let nested = root
            .join("Genymobile.scrcpy_source_hash")
            .join("scrcpy-win64-v4.1");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join("scrcpy.exe"), b"stub").unwrap();

        let mut found = Vec::new();
        find_named_files(&root, "scrcpy.exe", 3, &mut found);
        assert_eq!(found, vec![nested.join("scrcpy.exe")]);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn reads_the_platform_tools_release_not_the_protocol_version() {
        let banner = "Android Debug Bridge version 1.0.41\nVersion 37.0.1-android-tools\nInstalled as /x/adb";
        assert_eq!(platform_tools_version_of(banner).as_deref(), Some("37.0.1"));
    }

    #[test]
    fn adb_display_version_prefers_the_platform_tools_release() {
        let banner =
            "Android Debug Bridge version 1.0.41\nVersion 36.0.0-13206524\nInstalled as /x/adb";
        assert_eq!(preferred_version("adb", banner).as_deref(), Some("36.0.0"));
    }

    #[test]
    fn adb_falls_back_to_the_protocol_version_when_unlisted() {
        assert_eq!(
            preferred_version("adb", "Android Debug Bridge version 1.0.39").as_deref(),
            Some("1.0.39")
        );
    }

    #[test]
    fn other_tools_use_the_first_version_token() {
        let banner = "scrcpy 5.0 <https://github.com/Genymobile/scrcpy>";
        assert_eq!(preferred_version("scrcpy", banner).as_deref(), Some("5.0"));
    }

    #[test]
    fn picks_the_newest_managed_version() {
        let root = std::env::temp_dir().join("zesp-managed-scan-test");
        let _ = std::fs::remove_dir_all(&root);
        let exe = format!("scrcpy{EXE_SUFFIX}");
        for version in ["4.1", "5.0"] {
            let dir = root.join("tools").join("scrcpy").join(version);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join(&exe), b"stub").unwrap();
        }

        let found = managed_binary(&root, "scrcpy").unwrap();
        assert!(found.ends_with(format!("5.0/{exe}")));

        let _ = std::fs::remove_dir_all(&root);
    }
}
