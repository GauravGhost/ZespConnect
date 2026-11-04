use serde::{Deserialize, Serialize};

/// A wireless-debugging pairing QR payload; `payload` is the standard `WIFI:T:ADB;S:<name>;P:<password>;;` string.
#[derive(Debug, Clone, Serialize)]
pub struct QrPayload {
    pub payload: String,
    pub password: String,
    pub created: u64,
}

/// A service instance discovered by `adb mdns services`, shaped for the UI.
#[derive(Debug, Clone, Serialize)]
pub struct MdnsDevice {
    pub name: String,
    pub service_type: String,
    pub host: String,
    pub port: u16,
    pub address: String,
}

/// A saved phone the reconnect watcher brings back when it reappears on the network.
#[derive(Debug, Clone, Deserialize)]
pub struct ReconnectTarget {
    pub serial: String,
    /// Stable mDNS instance name (`adb-<serialno>-<id>`); the port changes, this does not.
    pub name: Option<String>,
    pub address: Option<String>,
}

/// A saved phone reconnected (or found unpaired) by the watcher.
#[derive(Debug, Clone, Serialize)]
pub struct ReconnectEvent {
    pub serial: String,
    pub address: Option<String>,
}
