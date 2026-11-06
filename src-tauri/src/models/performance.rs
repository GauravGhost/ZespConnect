use serde::Serialize;

/// One second of a phone's vitals, streamed to the Performance page.
/// A field is `None` when the phone does not expose it (or this is the first sample of a delta).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PerfSample {
    /// Busy share of all cores, 0-100.
    pub cpu: Option<f32>,
    /// Busy share per core, in core order; an offline core is `None`.
    pub cores: Vec<Option<f32>>,
    pub mem_used_kb: Option<u64>,
    pub mem_total_kb: Option<u64>,
    /// Frames the foreground app drew per second (HWUI frames, so games on a raw surface read low).
    pub fps: Option<f32>,
    /// Share of the frames drawn since the last sample that missed their deadline, 0-100.
    pub jank: Option<f32>,
    pub refresh_hz: Option<f32>,
    pub foreground: Option<ForegroundApp>,
    pub battery_level: Option<u8>,
    pub battery_temp_c: Option<f32>,
    /// Current flowing out of (or into, when charging) the battery, in milliamps.
    pub battery_ma: Option<f32>,
    pub battery_watts: Option<f32>,
    pub charging: Option<bool>,
    pub net_rx_bps: Option<u64>,
    pub net_tx_bps: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForegroundApp {
    pub package: String,
    pub label: String,
}
