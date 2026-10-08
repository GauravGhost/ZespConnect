use serde::{Deserialize, Serialize};

/// An Android Virtual Device known to the local `emulator` binary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Avd {
    pub name: String,
    /// True when a running emulator reports this AVD as its own.
    pub running: bool,
}
