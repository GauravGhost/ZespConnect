use serde::{Deserialize, Serialize};

/// How a Game Mode session holds the phone's rotation, mirroring the TS `GameOrientation`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GameOrientation {
    /// Follow the game.
    #[default]
    Auto,
    Portrait,
    Landscape,
}

/// Game Mode's toggles, mirroring the TS `GameOptions`; sent with a session and updated live.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GameOptions {
    pub orientation: GameOrientation,
    pub do_not_disturb: bool,
    pub screen_off: bool,
    /// The window hides its bars and cursor so OBS captures only the game.
    pub clean_capture: bool,
    pub always_on_top: bool,
    pub stats: bool,
    pub sound_on_computer: bool,
    /// The phone keeps playing its sound too (Android 13+).
    pub sound_on_phone: bool,
    pub low_latency_audio: bool,
    /// An output device name from `list_audio_outputs`; `None` is the system default.
    pub audio_device: Option<String>,
    pub controllers: bool,
}

/// A game controller the computer sees.
#[derive(Debug, Clone, Serialize)]
pub struct Controller {
    pub id: usize,
    pub name: String,
}
