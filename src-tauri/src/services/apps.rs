use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::error::{AppError, Result};
use crate::models::{AppInfo, AppListResult, AppListSource, LabelSource, ToolStatus};
use crate::services::{adb, app_icons};
use crate::util;
use crate::util::process::{self, ProcessOutput};
use crate::util::validation::{shell_quote, validate_package};

const SCRCPY_TIMEOUT: Duration = Duration::from_secs(60);
const PM_TIMEOUT: Duration = Duration::from_secs(30);
const ACTION_TIMEOUT: Duration = Duration::from_secs(30);
const INSTALL_TIMEOUT: Duration = Duration::from_secs(300);

/// Everything including system apps; filtering happens on the way out so the cache stays complete.
const LIST_PACKAGES: &str = "pm list packages -f";
const LIST_APPS_MARKER: &str = "List of apps";

/// APK path prefixes that mean the package is part of the system image.
const SYSTEM_PREFIXES: &[&str] = &[
    "/system",
    "/system_ext",
    "/product",
    "/vendor",
    "/apex",
    "/odm",
];

/// serial → apps, persisted as JSON under the app data dir.
type AppCache = HashMap<String, Vec<AppInfo>>;

/// Parse `scrcpy --list-apps` output. After the `List of apps` marker each line is one app: an
/// optional `INFO:` prefix, `*` for system (or `-`/space for user), then label and package in
/// either order - the token matching the package-id grammar is the package.
pub fn parse_scrcpy_apps(text: &str) -> Vec<AppInfo> {
    let mut apps = Vec::new();
    let mut started = false;

    for raw in text.lines() {
        let line = raw.trim();
        if !started {
            if line.contains(LIST_APPS_MARKER) {
                started = true;
            }
            continue;
        }

        let line = line.strip_prefix("INFO:").map(str::trim).unwrap_or(line);
        if line.is_empty() {
            continue;
        }

        let (is_system, rest) = if let Some(rest) = line.strip_prefix('*') {
            (true, rest)
        } else if let Some(rest) = line.strip_prefix('-') {
            (false, rest)
        } else {
            (false, line)
        };

        let tokens: Vec<&str> = rest.split_whitespace().collect();
        let Some(index) = tokens.iter().rposition(|t| validate_package(t).is_ok()) else {
            // Not an app row (headers, summaries, separators).
            continue;
        };

        let package = tokens[index].to_string();
        let label: String = tokens
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != index)
            .map(|(_, token)| *token)
            .collect::<Vec<_>>()
            .join(" ");

        apps.push(AppInfo {
            label: if label.is_empty() {
                prettify(&package)
            } else {
                label
            },
            package,
            is_system,
            version_name: None,
            label_source: LabelSource::Scrcpy,
            icon_path: None,
        });
    }

    apps
}

/// Parse `pm list packages -f` (`package:/path/base.apk=com.example`). Split APKs add extra
/// lines per package; the first (base) line wins and its path decides the system flag.
pub fn parse_pm_packages(text: &str) -> Vec<AppInfo> {
    let mut apps = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for raw in text.lines() {
        let line = raw.trim();
        let Some(rest) = line.strip_prefix("package:") else {
            continue;
        };

        let (path, package) = match rest.rsplit_once('=') {
            Some((path, name)) => (Some(path), name.trim()),
            None => (None, rest.trim()),
        };
        if validate_package(package).is_err() || !seen.insert(package.to_string()) {
            continue;
        }

        apps.push(AppInfo {
            package: package.to_string(),
            label: prettify(package),
            is_system: path.map(is_system_path).unwrap_or(false),
            version_name: None,
            label_source: LabelSource::Derived,
            icon_path: None,
        });
    }

    apps
}

/// A package installed under a system-image path rather than `/data/app`.
fn is_system_path(path: &str) -> bool {
    SYSTEM_PREFIXES
        .iter()
        .any(|prefix| path.starts_with(prefix))
}

/// Turn `com.example.my_cool-app` into a readable `My Cool App`.
fn prettify(package: &str) -> String {
    let last = package.rsplit('.').next().unwrap_or(package);
    let cleaned = last.replace(['_', '-'], " ");

    let mut out = String::with_capacity(cleaned.len());
    for (i, word) in cleaned.split_whitespace().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            out.extend(first.to_uppercase());
            out.push_str(chars.as_str());
        }
    }

    if out.is_empty() {
        package.to_string()
    } else {
        out
    }
}

fn cache_file(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("apps-cache.json")
}

fn read_cache(path: &Path, serial: &str) -> Option<Vec<AppInfo>> {
    util::read_json::<AppCache>(path)
        .ok()
        .flatten()
        .and_then(|cache| cache.get(serial).cloned())
}

fn write_cache(path: &Path, serial: &str, apps: &[AppInfo]) {
    let mut cache = util::read_json::<AppCache>(path)
        .ok()
        .flatten()
        .unwrap_or_default();
    cache.insert(serial.to_string(), apps.to_vec());
    let _ = util::write_json(path, &cache);
}

/// A device's cached apps as stored, with each app's original `label_source`.
pub fn cached(app_data_dir: &Path, serial: &str) -> Option<Vec<AppInfo>> {
    read_cache(&cache_file(app_data_dir), serial)
}

/// A device's cached app names by package, for naming an app without scanning the phone.
pub fn cached_labels(app_data_dir: &Path, serial: &str) -> HashMap<String, String> {
    read_cache(&cache_file(app_data_dir), serial)
        .unwrap_or_default()
        .into_iter()
        .map(|app| (app.package, app.label))
        .collect()
}

/// An app's display name: its cached label, else one derived from the package.
pub fn display_label(labels: &HashMap<String, String>, package: &str) -> String {
    labels
        .get(package)
        .cloned()
        .unwrap_or_else(|| prettify(package))
}

fn finish(mut apps: Vec<AppInfo>, source: AppListSource, include_system: bool) -> AppListResult {
    if !include_system {
        apps.retain(|app| !app.is_system);
    }
    apps.sort_by_key(|app| app.label.to_lowercase());
    AppListResult {
        apps,
        source,
        stale: matches!(source, AppListSource::Cache),
    }
}

fn from_cache(mut apps: Vec<AppInfo>, include_system: bool) -> AppListResult {
    for app in &mut apps {
        app.label_source = LabelSource::Cache;
    }
    finish(apps, AppListSource::Cache, include_system)
}

async fn scrcpy_list(scrcpy: &str, serial: &str) -> Result<Vec<AppInfo>> {
    let args = vec![
        "-s".to_string(),
        serial.to_string(),
        "--list-apps".to_string(),
    ];
    let output = process::run(scrcpy, &args, SCRCPY_TIMEOUT).await?;
    if output.stdout.trim().is_empty() {
        return Err(AppError::CommandFailed(output.text()));
    }
    Ok(parse_scrcpy_apps(&output.stdout))
}

async fn pm_list(adb_path: &str, serial: &str) -> Result<Vec<AppInfo>> {
    let output = adb::shell(adb_path, serial, LIST_PACKAGES, PM_TIMEOUT).await?;
    if !output.success && output.stdout.trim().is_empty() {
        return Err(AppError::CommandFailed(output.text()));
    }
    Ok(parse_pm_packages(&output.stdout))
}

/// Installed apps for `serial`, from cache when possible and refreshed on demand.
pub async fn list(
    tools: &ToolStatus,
    serial: &str,
    include_system: bool,
    refresh: bool,
    app_data_dir: &Path,
) -> Result<AppListResult> {
    let path = cache_file(app_data_dir);
    let cached = read_cache(&path, serial);

    let mut result = match cached.clone() {
        Some(apps) if !refresh => from_cache(apps, include_system),
        _ => {
            // Icons are optional, so a failed icon read still returns the list.
            let (listed, _) = tokio::join!(
                read_device(tools, serial, include_system, cached, &path),
                app_icons::refresh(tools, serial, app_data_dir),
            );
            listed?
        }
    };
    app_icons::attach(&mut result.apps, app_data_dir, serial);
    Ok(result)
}

/// Read the list from the phone (scrcpy for real names, else `pm`) and update the cache.
async fn read_device(
    tools: &ToolStatus,
    serial: &str,
    include_system: bool,
    cached: Option<Vec<AppInfo>>,
    path: &Path,
) -> Result<AppListResult> {
    if let Some(scrcpy) = tools.scrcpy.path.as_deref() {
        if let Ok(apps) = scrcpy_list(scrcpy, serial).await {
            if !apps.is_empty() {
                write_cache(path, serial, &apps);
                return Ok(finish(apps, AppListSource::Scrcpy, include_system));
            }
        }
    }

    if let Some(apps) = cached {
        return Ok(from_cache(apps, include_system));
    }

    let adb_path = adb::adb_path(tools)?;
    let apps = pm_list(&adb_path, serial).await?;
    if apps.is_empty() {
        return Err(AppError::CommandFailed(format!(
            "No apps reported for {serial}"
        )));
    }
    write_cache(path, serial, &apps);
    Ok(finish(apps, AppListSource::Pm, include_system))
}

/// Bring a package to the foreground with `monkey`, falling back to `am start`.
pub async fn launch(tools: &ToolStatus, serial: &str, package: &str) -> Result<()> {
    let adb_path = adb::adb_path(tools)?;
    let pkg = shell_quote(package);
    let command = format!(
        "monkey -p {pkg} -c android.intent.category.LAUNCHER 1 2>/dev/null || \
         am start -n \"$(cmd package resolve-activity --brief -c android.intent.category.LAUNCHER {pkg} | tail -n 1)\""
    );
    let output = adb::shell(&adb_path, serial, &command, ACTION_TIMEOUT).await?;
    if !output.success {
        return Err(AppError::CommandFailed(output.text()));
    }
    Ok(())
}

/// Remove an app: `adb uninstall` for user apps, else `pm uninstall --user 0` for system packages.
pub async fn uninstall(tools: &ToolStatus, serial: &str, package: &str) -> Result<()> {
    let adb_path = adb::adb_path(tools)?;

    let args = vec!["uninstall".to_string(), package.to_string()];
    let output = adb::run_scoped(&adb_path, serial, &args, ACTION_TIMEOUT).await?;
    if succeeded(&output) {
        return Ok(());
    }

    let command = format!("pm uninstall --user 0 {}", shell_quote(package));
    let output = adb::shell(&adb_path, serial, &command, ACTION_TIMEOUT).await?;
    if succeeded(&output) {
        return Ok(());
    }
    Err(AppError::CommandFailed(output.text()))
}

pub async fn clear_data(tools: &ToolStatus, serial: &str, package: &str) -> Result<()> {
    let adb_path = adb::adb_path(tools)?;
    let command = format!("pm clear {}", shell_quote(package));
    let output = adb::shell(&adb_path, serial, &command, ACTION_TIMEOUT).await?;
    if !succeeded(&output) {
        return Err(AppError::CommandFailed(output.text()));
    }
    Ok(())
}

pub async fn install_apk(tools: &ToolStatus, serial: &str, path: &str) -> Result<()> {
    if !Path::new(path).is_file() {
        return Err(AppError::Invalid(format!("APK not found: {path}")));
    }
    let adb_path = adb::adb_path(tools)?;
    let args = vec!["install".to_string(), "-r".to_string(), path.to_string()];
    let output = adb::run_scoped(&adb_path, serial, &args, INSTALL_TIMEOUT).await?;
    if !succeeded(&output) {
        return Err(AppError::CommandFailed(output.text()));
    }
    Ok(())
}

/// The exit status alone is not trusted; a package-manager command must also print "Success".
fn succeeded(output: &ProcessOutput) -> bool {
    output.success && output.text().contains("Success")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCRCPY: &str = include_str!("../../tests/fixtures/scrcpy_list_apps.txt");
    const PM: &str = include_str!("../../tests/fixtures/pm_list_packages.txt");

    #[test]
    fn parses_scrcpy_app_list() {
        let apps = parse_scrcpy_apps(SCRCPY);
        assert_eq!(apps.len(), 3);

        assert!(apps
            .iter()
            .any(|a| a.package == "com.android.systemui" && a.is_system));
        assert!(apps
            .iter()
            .any(|a| a.package == "com.example.notes" && !a.is_system));

        let maps = apps
            .iter()
            .find(|a| a.package == "com.google.android.apps.maps")
            .expect("maps present");
        assert_eq!(maps.label, "Google Maps");
        assert!(!maps.is_system);
        assert_eq!(maps.label_source, LabelSource::Scrcpy);
    }

    #[test]
    fn scrcpy_parser_tolerates_info_prefix() {
        let text = "\
INFO: List of apps:
INFO:  * com.android.settings    Settings
INFO:  - com.example.app         Example
";
        let apps = parse_scrcpy_apps(text);
        assert_eq!(apps.len(), 2);
        assert_eq!(apps[0].package, "com.android.settings");
        assert!(apps[0].is_system);
        assert_eq!(apps[1].label, "Example");
    }

    #[test]
    fn parses_pm_packages_and_classifies_system() {
        let apps = parse_pm_packages(PM);
        assert_eq!(apps.len(), 4, "split APKs for one package must dedupe");

        let settings = apps
            .iter()
            .find(|a| a.package == "com.android.settings")
            .expect("settings present");
        assert!(settings.is_system);

        let notes = apps
            .iter()
            .find(|a| a.package == "com.example.notes")
            .expect("notes present");
        assert!(!notes.is_system);
        assert_eq!(notes.label, "Notes");
        assert_eq!(notes.label_source, LabelSource::Derived);
    }

    #[test]
    fn prettifies_package_ids() {
        assert_eq!(prettify("com.example.my_cool-app"), "My Cool App");
        assert_eq!(prettify("com.android.settings"), "Settings");
        assert_eq!(prettify("org.single"), "Single");
    }

    #[test]
    fn caches_apps_per_serial() {
        let dir = std::env::temp_dir().join("zesp-apps-cache-test");
        let _ = std::fs::remove_dir_all(&dir);
        let path = cache_file(&dir);

        let apps = parse_pm_packages(PM);
        write_cache(&path, "serial-a", &apps);
        write_cache(&path, "serial-b", &[]);

        let loaded = read_cache(&path, "serial-a").expect("serial-a cached");
        assert_eq!(loaded.len(), apps.len());
        assert_eq!(read_cache(&path, "serial-b").map(|a| a.len()), Some(0));
        assert!(read_cache(&path, "serial-missing").is_none());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
