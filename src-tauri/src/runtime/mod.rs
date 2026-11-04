//! Tauri-coupled coordinators: long-lived background tasks and window lifecycle.
//! Unlike `services/`, these modules talk to Tauri (managed state and events).

pub mod clipboard_sync;
pub mod launcher;
pub mod logcat;
pub mod mirror;
pub mod mirror_session;
pub mod now_playing;
pub mod phone_audio;
pub mod reconnect;
pub mod recording;
pub mod recording_audio;
pub mod server_link;
pub mod session_window;
pub mod tools;
pub mod tracker;
