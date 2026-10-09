//! Tauri-coupled coordinators: long-lived background tasks and window lifecycle.
//! Unlike `services/`, these modules talk to Tauri (managed state and events).

pub mod mirror;
pub mod mirror_session;
pub mod tools;
pub mod tracker;
