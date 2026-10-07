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

/// Options for a screen-mirror or camera session, mirroring the `MirrorOptions`
/// TypeScript interface (`src/types/index.ts`). Unknown fields are rejected so a
/// typo in the UI fails loudly instead of silently being ignored.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MirrorOptions {
    pub resolution: Option<u32>,
    pub fps: Option<u32>,
    pub bitrate: Option<u32>,
    pub audio: bool,
    pub borderless: bool,
    pub stay_awake: bool,
    pub virtual_display: bool,
    pub always_on_top: bool,
    pub aspect_lock: bool,
    pub background: Option<String>,
    pub fit: Option<String>,
    pub camera_facing: Option<String>,
    pub camera_torch: bool,
    pub camera_zoom: Option<f64>,
}
