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
    Path,
    /// Found at a well-known install location.
    Known,
    NotFound,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub found: bool,
    pub path: Option<String>,
    pub version: Option<String>,
    pub source: ToolSource,
    /// The copy the app downloaded itself, if any, so the UI can switch back to it.
    pub managed_path: Option<String>,
}

impl ToolInfo {
    pub fn missing() -> Self {
        ToolInfo {
            found: false,
            path: None,
            version: None,
            source: ToolSource::NotFound,
            managed_path: None,
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
    pub adb_update: UpdateInfo,
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

/// Streamed to the UI while a tool download and install runs.
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
