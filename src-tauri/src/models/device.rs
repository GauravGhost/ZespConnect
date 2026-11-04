use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeviceTransport {
    Usb,
    Wifi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeviceState {
    Device,
    Unauthorized,
    Offline,
    Connecting,
    Unknown,
}

impl DeviceState {
    pub fn from_adb(state: &str) -> Self {
        match state {
            "device" => DeviceState::Device,
            "unauthorized" => DeviceState::Unauthorized,
            "offline" => DeviceState::Offline,
            "connecting" => DeviceState::Connecting,
            _ => DeviceState::Unknown,
        }
    }

    pub fn is_connected(self) -> bool {
        matches!(self, DeviceState::Device)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub serial: String,
    pub display_name: String,
    pub model: String,
    pub android_version: Option<String>,
    pub sdk: Option<String>,
    pub transport: DeviceTransport,
    pub state: DeviceState,
    pub connected: bool,
    pub battery: Option<i32>,
    pub charging: Option<bool>,
    pub address: Option<String>,
    /// The phone's stable wireless debugging name, when it is announced over mDNS.
    pub mdns_name: Option<String>,
}
