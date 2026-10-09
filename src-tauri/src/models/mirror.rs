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
    /// Android's desktop mode on a decorated virtual display, sized to the computer's screen.
    Desktop,
}

/// A pointer action on a mirrored screen, mirroring the `TouchAction` TS type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TouchAction {
    Down,
    Move,
    Up,
    /// The mouse moved with no button pressed (desktop mode only).
    Hover,
}

/// A key press or release, mirroring the `KeyAction` TS type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KeyAction {
    Down,
    Up,
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

/// A mirror rendered inside a Tauri window the app owns (no external scrcpy
/// window). `id` is a process-local session id, not an OS pid.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorSession {
    pub id: u64,
    pub serial: String,
    pub kind: MirrorKind,
    pub package: Option<String>,
    pub label: Option<String>,
    pub started: u64,
    /// Desktop mode: whether the phone's own screen starts on (the title-bar
    /// toggle) because the phone was in use when the session opened.
    #[serde(default, rename = "phoneScreen")]
    pub phone_screen: bool,
}

/// Everything needed to start one in-app mirror session. Kept in `AppState`
/// between opening the window and the window asking for its stream, so a failed
/// stream can fall back to the external scrcpy window with the same settings.
#[derive(Debug, Clone, Deserialize)]
pub struct MirrorSessionSpec {
    pub serial: String,
    pub kind: MirrorKind,
    pub package: Option<String>,
    pub label: String,
    pub options: MirrorOptions,
    /// The `WxH/dpi` virtual display for a desktop session, sized to the
    /// computer's screen when the window opens. Never sent by the UI.
    #[serde(default)]
    pub desktop_display: Option<String>,
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
    pub always_on_top: bool,
    pub aspect_lock: bool,
    pub background: Option<String>,
    pub fit: Option<String>,
    pub camera_facing: Option<String>,
    pub camera_torch: bool,
    pub camera_zoom: Option<f64>,
}
