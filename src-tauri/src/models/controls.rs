use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickState {
    pub wifi: bool,
    pub bluetooth: bool,
    pub dark_mode: bool,
    pub airplane_mode: bool,
    pub mobile_data: bool,
    pub location: bool,
    pub dnd: bool,
    pub auto_rotate: bool,
    pub brightness: i32,
    pub brightness_max: i32,
    pub volume: i32,
}

impl Default for QuickState {
    fn default() -> Self {
        QuickState {
            wifi: false,
            bluetooth: false,
            dark_mode: false,
            airplane_mode: false,
            mobile_data: false,
            location: false,
            dnd: false,
            auto_rotate: false,
            brightness: 128,
            brightness_max: 255,
            volume: 50,
        }
    }
}

/// Whitelisted reboot targets; anything else is rejected before it reaches adb.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RebootMode {
    Normal,
    Bootloader,
    Recovery,
}

impl RebootMode {
    pub fn as_arg(self) -> &'static str {
        match self {
            RebootMode::Normal => "normal",
            RebootMode::Bootloader => "bootloader",
            RebootMode::Recovery => "recovery",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub model: String,
    pub android_version: String,
    pub sdk: String,
    pub battery: Option<i32>,
    pub charging: bool,
    pub quick: QuickState,
}
