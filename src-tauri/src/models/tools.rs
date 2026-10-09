use serde::{Deserialize, Serialize};

/// Which external tool a command or install action refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolKind {
    Adb,
    Scrcpy,
}

impl ToolKind {
    pub fn name(self) -> &'static str {
        match self {
            ToolKind::Adb => "adb",
            ToolKind::Scrcpy => "scrcpy",
        }
    }
}

/// Where a resolved tool path came from, so the UI can explain itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolSource {
    /// Explicitly configured by the user (persisted).
    Override,
    /// Found on `PATH`.
    Path,
    /// Found at a well-known install location.
    Known,
    /// Not found anywhere.
    NotFound,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub found: bool,
    pub path: Option<String>,
    pub version: Option<String>,
    pub source: ToolSource,
}

impl ToolInfo {
    pub fn missing() -> Self {
        ToolInfo {
            found: false,
            path: None,
            version: None,
            source: ToolSource::NotFound,
        }
    }
}

/// Comparison of an installed tool against the latest upstream release; a `Default` means "not checked".
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateInfo {
    /// Latest version published upstream, if it could be read.
    pub latest_version: Option<String>,
    /// True when a newer version than the installed one exists.
    pub outdated: bool,
    /// Why the check failed (offline, rate-limited, …), if it did.
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStatus {
    pub adb: ToolInfo,
    pub scrcpy: ToolInfo,
    /// Update check for adb (independent of whether it is installed).
    pub adb_update: UpdateInfo,
    /// Update check for scrcpy.
    pub scrcpy_update: UpdateInfo,
}

impl Default for ToolStatus {
    fn default() -> Self {
        ToolStatus {
            adb: ToolInfo::missing(),
            scrcpy: ToolInfo::missing(),
            adb_update: UpdateInfo::default(),
            scrcpy_update: UpdateInfo::default(),
        }
    }
}

/// Streamed to the UI while a package-manager install runs.
#[derive(Debug, Clone, Serialize)]
pub struct InstallProgress {
    pub tool: String,
    pub line: String,
}

/// Emitted when an install finishes (success or failure).
#[derive(Debug, Clone, Serialize)]
pub struct InstallDone {
    pub tool: String,
    pub ok: bool,
    pub message: String,
}
