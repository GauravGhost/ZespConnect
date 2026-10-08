use serde::Serialize;

/// A wireless-debugging pairing QR payload (Android 11+).
///
/// `payload` is the standard `WIFI:T:ADB;S:<name>;P:<password>;;` string the
/// phone's "Pair device with QR code" screen understands.
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
