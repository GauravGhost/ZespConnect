use serde::{Deserialize, Serialize};

/// Where an app's display label came from, so the UI can explain itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LabelSource {
    /// Real name from `scrcpy --list-apps`.
    Scrcpy,
    /// Served from the on-disk cache.
    Cache,
    /// Prettified from `pm list packages`.
    Pm,
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
