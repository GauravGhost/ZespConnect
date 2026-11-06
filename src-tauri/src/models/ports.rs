use serde::{Deserialize, Serialize};

/// `Forward`: the computer's port reaches the phone. `Reverse`: the phone's port reaches the computer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PortDirection {
    Forward,
    Reverse,
}

/// One `adb forward` / `adb reverse` rule as adb lists it, e.g. `tcp:3000` on each side.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortRule {
    pub direction: PortDirection,
    pub host: String,
    pub device: String,
}
