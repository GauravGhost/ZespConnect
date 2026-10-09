use std::env;
use std::path::{Path, PathBuf};
use std::time::Duration;

use once_cell::sync::Lazy;
use regex::Regex;

use crate::models::{ToolInfo, ToolKind, ToolSource, ToolStatus};
use crate::platform::EXE_SUFFIX;
use crate::services::updates;
use crate::util::process;

const VERSION_TIMEOUT: Duration = Duration::from_secs(6);
/// How many directory levels to search under a managed version or a winget package folder.
const MANAGED_SEARCH_DEPTH: usize = 3;
/// How many directory levels to search inside a freshly extracted archive.
const ARCHIVE_SEARCH_DEPTH: usize = 4;
/// Stop a binary search after this many matches.
const MAX_SEARCH_MATCHES: usize = 4;

static VERSION_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(\d+\.\d+(?:\.\d+)?)").expect("valid regex"));

/// Pulls the platform-tools release from `adb --version` (e.g. `Version 37.0.1-…`), not the
/// protocol version line; the capital `Version` keeps the protocol line from matching.
static PLATFORM_TOOLS_VERSION_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"Version\s+(\d+\.\d+\.\d+)").expect("valid regex"));

/// Resolve both tools, preferring override, then `PATH`, known locations, then the managed download directory.
pub async fn resolve(
    adb_override: Option<&str>,
    scrcpy_override: Option<&str>,
    managed_dir: &Path,
) -> ToolStatus {
    let adb = resolve_tool(ToolKind::Adb, adb_override, &["version"], managed_dir).await;
    let scrcpy = resolve_tool(
        ToolKind::Scrcpy,
        scrcpy_override,
        &["--version"],
        managed_dir,
    )
    .await;
    ToolStatus {
        adb,
        scrcpy,
        ..ToolStatus::default()
    }
}

async fn resolve_tool(
    tool: ToolKind,
    override_path: Option<&str>,
    version_args: &[&str],
    managed_dir: &Path,
) -> ToolInfo {
    let name = tool.name();
    let override_path = override_path
        .map(PathBuf::from)
        .filter(|candidate| candidate.is_file());

    let (path, source) = if let Some(path) = override_path {
        (path, ToolSource::Override)
    } else if let Some(path) = find_in_path(name) {
        (path, ToolSource::Path)
    } else if let Some(path) = known_locations(tool)
        .into_iter()
        .find(|path| path.is_file())
    {
        (path, ToolSource::Known)
    } else if let Some(path) = managed_binary(managed_dir, tool) {
        // A copy the app downloaded for itself (no override set).
        (path, ToolSource::Known)
    } else {
        return ToolInfo::missing();
    };

    ToolInfo {
        found: true,
        version: version_of(tool, &path, version_args).await,
        path: Some(path.to_string_lossy().to_string()),
        source,
    }
}

/// `<app_data_dir>/tools` - where downloaded adb/scrcpy copies live.
pub fn managed_root(managed_dir: &Path) -> PathBuf {
    managed_dir.join("tools")
}

/// The upstream package name inside the managed root: adb in `platform-tools`, scrcpy in its own directory.
pub fn managed_slot(tool: ToolKind) -> &'static str {
    match tool {
        ToolKind::Adb => "platform-tools",
        ToolKind::Scrcpy => "scrcpy",
    }
}

/// The newest managed binary for `tool`, scanning version subdirectories newest first.
fn managed_binary(managed_dir: &Path, tool: ToolKind) -> Option<PathBuf> {
    let base = managed_root(managed_dir).join(managed_slot(tool));
    let exe = format!("{}{EXE_SUFFIX}", tool.name());

    let mut versions: Vec<PathBuf> = std::fs::read_dir(&base)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    versions.sort_by(|a, b| updates::compare(&dir_version(b), &dir_version(a)));

    for dir in versions {
        let mut found = Vec::new();
        find_named_files(&dir, &exe, MANAGED_SEARCH_DEPTH, &mut found);
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

/// The platform-tools release from `adb --version` (e.g. `37.0.1`), comparable to Google's revision.
pub async fn platform_tools_version(adb_path: &str) -> Option<String> {
    let output = process::run(adb_path, &["version".to_string()], VERSION_TIMEOUT)
        .await
        .ok()?;
    platform_tools_version_of(&output.text())
}

fn platform_tools_version_of(output: &str) -> Option<String> {
    PLATFORM_TOOLS_VERSION_RE
        .captures(output)
        .map(|caps| caps[1].to_string())
}

async fn version_of(tool: ToolKind, path: &Path, args: &[&str]) -> Option<String> {
    let program = path.to_string_lossy().to_string();
    let argv: Vec<String> = args.iter().map(|a| a.to_string()).collect();
    let output = process::run(&program, &argv, VERSION_TIMEOUT).await.ok()?;
    if !output.success {
        return None;
    }
    preferred_version(tool, &output.text())
}

/// The meaningful version from a tool's `--version` banner. adb prints its constant protocol
/// version first and the platform-tools release second; the release is what matches upstream, so
/// prefer it. Everything else uses the first version token.
fn preferred_version(tool: ToolKind, text: &str) -> Option<String> {
    if tool == ToolKind::Adb {
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
fn known_locations(tool: ToolKind) -> Vec<PathBuf> {
    let name = tool.name();
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

/// Search `%LOCALAPPDATA%\Microsoft\WinGet\Packages` for `<name>.exe`; portable winget packages have no shim.
#[cfg(windows)]
fn winget_package_binaries(name: &str) -> Vec<PathBuf> {
    let Some(local) = env::var_os("LOCALAPPDATA").map(PathBuf::from) else {
        return Vec::new();
    };
    let base = local.join("Microsoft").join("WinGet").join("Packages");
    let exe = format!("{name}{EXE_SUFFIX}");

    let mut found = Vec::new();
    find_named_files(&base, &exe, MANAGED_SEARCH_DEPTH, &mut found);
    found
}

/// Find `name` plus the platform suffix under `root`, a few levels deep (for an extracted archive).
pub fn find_binary(root: &Path, name: &str) -> Option<PathBuf> {
    let exe = format!("{name}{EXE_SUFFIX}");
    let mut found = Vec::new();
    find_named_files(root, &exe, ARCHIVE_SEARCH_DEPTH, &mut found);
    found.into_iter().next()
}

/// Shallow recursive search for `file_name` under `dir`, bounded by `depth`.
fn find_named_files(dir: &Path, file_name: &str, depth: usize, out: &mut Vec<PathBuf>) {
    if depth == 0 || out.len() >= MAX_SEARCH_MATCHES {
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
fn known_locations(tool: ToolKind) -> Vec<PathBuf> {
    let name = tool.name();
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
    if tool == ToolKind::Scrcpy {
        paths.push(PathBuf::from(
            "/Applications/scrcpy.app/Contents/MacOS/scrcpy",
        ));
    }
    paths
}

#[cfg(all(unix, not(target_os = "macos")))]
fn known_locations(tool: ToolKind) -> Vec<PathBuf> {
    let name = tool.name();
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
        find_named_files(&root, "scrcpy.exe", MANAGED_SEARCH_DEPTH, &mut found);
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
        assert_eq!(
            preferred_version(ToolKind::Adb, banner).as_deref(),
            Some("36.0.0")
        );
    }

    #[test]
    fn adb_falls_back_to_the_protocol_version_when_unlisted() {
        assert_eq!(
            preferred_version(ToolKind::Adb, "Android Debug Bridge version 1.0.39").as_deref(),
            Some("1.0.39")
        );
    }

    #[test]
    fn other_tools_use_the_first_version_token() {
        let banner = "scrcpy 5.0 <https://github.com/Genymobile/scrcpy>";
        assert_eq!(
            preferred_version(ToolKind::Scrcpy, banner).as_deref(),
            Some("5.0")
        );
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

        let found = managed_binary(&root, ToolKind::Scrcpy).unwrap();
        assert!(found.ends_with(format!("5.0/{exe}")));

        let _ = std::fs::remove_dir_all(&root);
    }
}
