use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}

/// User preferences persisted to `settings.json` in the app data dir.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// Install scrcpy automatically on startup when it is missing and a
    /// supported package manager is available.
    #[serde(default = "default_true")]
    pub auto_install_scrcpy: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            auto_install_scrcpy: true,
        }
    }
}
