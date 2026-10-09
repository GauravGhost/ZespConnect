//! Tauri-coupled coordinators: long-lived background tasks and window lifecycle.
//!
//! These modules talk to Tauri (managed state and events), unlike `services/`,
//! which is pure domain logic with no Tauri imports.

pub mod mirror;
pub mod mirror_session;
pub mod tools;
pub mod tracker;
