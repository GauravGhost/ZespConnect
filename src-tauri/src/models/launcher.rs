use serde::{Deserialize, Serialize};

/// Cmd+Shift+Space on macOS, Ctrl+Shift+Space elsewhere.
pub const DEFAULT_SHORTCUT: &str = "CommandOrControl+Shift+Space";

/// The app launcher's saved settings, kept in `launcher.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherSettings {
    pub enabled: bool,
    pub shortcut: String,
}

impl Default for LauncherSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            shortcut: DEFAULT_SHORTCUT.to_string(),
        }
    }
}

/// The saved settings plus whether the OS accepted the hotkey this run.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherStatus {
    pub enabled: bool,
    pub shortcut: String,
    pub registered: bool,
}

/// Which phone the launcher popup acts on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum LauncherTarget {
    Ready { serial: String, name: String },
    Offline { name: String },
    PickInTray,
    NoPhone,
}
