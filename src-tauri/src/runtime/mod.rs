//! Tauri-coupled coordinators: long-lived background tasks and window lifecycle.
//! Unlike `services/`, these modules talk to Tauri (managed state and events).

pub mod clipboard_sync;
pub mod logcat;
pub mod mirror;
pub mod mirror_session;
pub mod phone_audio;
pub mod reconnect;
pub mod server_link;
pub mod session_window;
pub mod tools;
pub mod tracker;
