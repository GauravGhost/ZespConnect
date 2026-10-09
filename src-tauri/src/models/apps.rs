use serde::{Deserialize, Serialize};

/// Where an app's display label came from, so the UI can explain itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LabelSource {
    /// Real name from `scrcpy --list-apps`.
    Scrcpy,
    /// Served from the on-disk cache.
    Cache,
    /// Derived from the package id (last resort).
    Derived,
}

/// Which pipeline produced a whole app list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AppListSource {
    Scrcpy,
    Cache,
    Pm,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    pub package: String,
    pub label: String,
    pub is_system: bool,
    pub version_name: Option<String>,
    pub label_source: LabelSource,
    pub icon_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppListResult {
    pub apps: Vec<AppInfo>,
    pub source: AppListSource,
    pub stale: bool,
}

/// Progress for an in-flight APK install, streamed to the UI as `install:progress`.
/// `package` is the APK's file stem, since the installer only knows the file path.
#[derive(Debug, Clone, Serialize)]
pub struct AppInstallProgress {
    pub serial: String,
    pub package: String,
    pub progress: u32,
    pub message: String,
}
