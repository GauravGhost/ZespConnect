use serde::{Deserialize, Serialize};

use super::GameOptions;

/// What a tracked scrcpy process is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MirrorKind {
    Mirror,
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

/// A one-shot device control from a mirror window's menu, mirroring the `MirrorAction` TS type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MirrorAction {
    Back,
    Notifications,
    QuickSettings,
    CollapsePanels,
    Rotate,
    TorchOn,
    TorchOff,
    ZoomIn,
    ZoomOut,
}

impl MirrorAction {
    /// Camera controls exist only on a scrcpy 4.0+ camera session.
    pub fn is_camera(self) -> bool {
        matches!(
            self,
            Self::TorchOn | Self::TorchOff | Self::ZoomIn | Self::ZoomOut
        )
    }
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

/// A mirror rendered inside an app-owned Tauri window; `id` is a process-local session id, not an OS pid.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorSession {
    pub id: u64,
    pub serial: String,
    pub kind: MirrorKind,
    pub package: Option<String>,
    pub label: Option<String>,
    pub started: u64,
    /// Desktop mode: whether the phone's own screen starts on because it was in use when the session opened.
    #[serde(default, rename = "phoneScreen")]
    pub phone_screen: bool,
    /// The session's virtual display follows its window's size (scrcpy 4.0+ `flex_display`).
    #[serde(default, rename = "flexDisplay")]
    pub flex_display: bool,
    /// Flex display pixels per logical window pixel; resizes keep the display's density.
    #[serde(skip)]
    pub display_scale: f64,
    /// A camera session whose server takes torch and zoom controls (scrcpy 4.0+).
    #[serde(default, rename = "cameraControls")]
    pub camera_controls: bool,
    /// Camera: whether the torch started on.
    #[serde(default)]
    pub torch: bool,
    /// A Game Mode session's current toggles.
    #[serde(default)]
    pub game: Option<GameOptions>,
}

/// Everything needed to start one in-app session, kept in `AppState` so a failed stream can fall back with the same settings.
#[derive(Debug, Clone, Deserialize)]
pub struct MirrorSessionSpec {
    pub serial: String,
    pub kind: MirrorKind,
    pub package: Option<String>,
    pub label: String,
    pub options: MirrorOptions,
    /// The `WxH/dpi` virtual display for a desktop session, sized to the computer's screen. Never sent by the UI.
    #[serde(default)]
    pub desktop_display: Option<String>,
}

/// Options for a mirror or camera session, mirroring the TS `MirrorOptions`; unknown fields are rejected.
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
    /// Present only for a Game Mode session.
    #[serde(default)]
    pub game: Option<GameOptions>,
}
