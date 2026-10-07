use serde::{Deserialize, Serialize};

/// What a tracked scrcpy process is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MirrorKind {
    /// Full device screen.
    Mirror,
    /// Device camera.
    Camera,
    /// An app on its own virtual display (its own window).
    App,
    /// The main display with an app in the foreground.
    ScreenApp,
}

/// A running scrcpy window tracked in `AppState`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorProcess {
    pub pid: u32,
    pub serial: String,
    pub kind: MirrorKind,
    pub package: Option<String>,
    pub label: Option<String>,
    pub started: u64,
}
