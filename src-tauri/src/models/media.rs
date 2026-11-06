use serde::{Deserialize, Serialize};

/// What a phone's foreground media session is playing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NowPlaying {
    pub package: String,
    pub app: String,
    pub title: String,
    pub artist: Option<String>,
    pub playing: bool,
    pub can_previous: bool,
    pub can_next: bool,
    /// The app's own message when its session is in the error state.
    pub error: Option<String>,
}

/// A media button sent to the phone.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MediaKey {
    PlayPause,
    Next,
    Previous,
}

/// Where a phone's sound plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SoundRoute {
    Phone,
    Computer,
    /// The computer and the phone together (Android 13+).
    Both,
}
