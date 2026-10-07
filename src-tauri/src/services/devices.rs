//! Device list parsing, mDNS/ip duplicate merging, and per-device info.

use std::time::Duration;

use once_cell::sync::Lazy;
use regex::Regex;

use crate::error::{AppError, Result};
use crate::models::{Device, DeviceInfo, DeviceState, DeviceTransport, QuickState, ToolStatus};
use crate::services::adb;

/// Single round-trip script that reports model, version, battery and every
/// quick-setting we care about. Ported from the reference backend.
const INFO_COMMAND: &str = concat!(
    r#"echo "MODEL:$(getprop ro.product.model)"; "#,
    r#"echo "VER:$(getprop ro.build.version.release)"; "#,
    r#"echo "SDK:$(getprop ro.build.version.sdk)"; "#,
    r#"dumpsys battery | grep -E "level:|powered:"; "#,
    r#"echo "WIFI:$(settings get global wifi_on 2>/dev/null)"; "#,
    r#"echo "BT:$(settings get global bluetooth_on 2>/dev/null)"; "#,
    r#"echo "DARK:$(cmd uimode night 2>/dev/null)"; "#,
    r#"echo "AIR:$(settings get global airplane_mode_on 2>/dev/null)"; "#,
    r#"echo "DATA:$(settings get global mobile_data 2>/dev/null)"; "#,
    r#"echo "LOC:$(settings get secure location_mode 2>/dev/null)"; "#,
    r#"echo "DND:$(cmd settings get global zen_mode 2>/dev/null)"; "#,
    r#"echo "ROT:$(settings get system accelerometer_rotation 2>/dev/null)"; "#,
    r#"echo "BRI:$(settings get system screen_brightness 2>/dev/null)"; "#,
    r#"echo "VOL:$(cmd media_session volume --stream 3 --get 2>/dev/null)";"#,
);

static DIGITS_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(\d+)").expect("valid regex"));
static VOLUME_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"volume is (\d+) in range \[(\d+)\.\.(\d+)\]").expect("valid regex"));

/// A serial that is an mDNS alias rather than a host:port address.
pub fn is_mdns(serial: &str) -> bool {
    serial.contains("_adb-tls-") || serial.contains("._tcp")
}

fn is_wireless(serial: &str) -> bool {
    serial.contains(':') || is_mdns(serial)
}

/// Parse `adb devices -l` output into devices.
pub fn parse_devices(text: &str) -> Vec<Device> {
    let mut devices = Vec::new();

    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.to_lowercase().starts_with("list of devices") {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }
        let serial = parts[0].to_string();
        let state = DeviceState::from_adb(parts[1]);

        let mut model = String::new();
        let mut product = String::new();
        for token in &parts[2..] {
            if let Some(value) = token.strip_prefix("model:") {
                model = value.replace('_', " ");
            } else if let Some(value) = token.strip_prefix("device:") {
                product = value.to_string();
            }
        }

        let display_name = if !model.is_empty() {
            model.clone()
        } else if !product.is_empty() {
            product.clone()
        } else {
            serial.clone()
        };

        devices.push(Device {
            serial: serial.clone(),
            display_name,
            model: if model.is_empty() { product } else { model },
            android_version: None,
            sdk: None,
            transport: if is_wireless(&serial) {
                DeviceTransport::Wifi
            } else {
                DeviceTransport::Usb
            },
            state,
            connected: state.is_connected(),
            battery: None,
            charging: None,
            address: if serial.contains(':') {
                Some(serial.clone())
            } else {
                None
            },
        });
    }

    dedupe(devices)
}

/// Collapse the Android 11+ duplicate transports for one phone.
///
/// A wireless device shows up twice — as `ip:port` and as an
/// `adb-<hash>-<name>._adb-tls-connect._tcp` alias. Keep the mDNS alias (scrcpy
/// refuses the `ip:port` serial while the alias exists) and carry the address
/// for display and reconnecting.
pub fn dedupe(devices: Vec<Device>) -> Vec<Device> {
    let mut result: Vec<Device> = Vec::new();

    for device in devices {
        let device_mdns = is_mdns(&device.serial);
        let twin_index = result.iter().position(|other| {
            is_mdns(&other.serial) != device_mdns
                && !other.model.is_empty()
                && other.model == device.model
        });

        match twin_index {
            None => {
                let mut entry = device;
                if !device_mdns && entry.serial.contains(':') {
                    entry.address = Some(entry.serial.clone());
                }
                result.push(entry);
            }
            Some(index) => {
                let twin = result[index].clone();
                let (mut mdns_entry, address_entry) = if device_mdns {
                    (device, twin)
                } else {
                    (twin, device)
                };

                mdns_entry.address = if address_entry.serial.contains(':') {
                    Some(address_entry.serial.clone())
                } else {
                    mdns_entry.address.clone()
                };
                mdns_entry.transport = DeviceTransport::Wifi;

                if address_entry.connected {
                    mdns_entry.connected = true;
                    mdns_entry.state = DeviceState::Device;
                }

                result[index] = mdns_entry;
            }
        }
    }

    result
}

/// Parse the tagged output of [`INFO_COMMAND`].
pub fn parse_info(text: &str) -> DeviceInfo {
    let mut info = DeviceInfo {
        model: String::new(),
        android_version: String::new(),
        sdk: String::new(),
        battery: None,
        charging: false,
        quick: QuickState::default(),
    };

    for raw in text.lines() {
        let line = raw.trim();
        if let Some(value) = line.strip_prefix("MODEL:") {
            info.model = value.trim().to_string();
        } else if let Some(value) = line.strip_prefix("VER:") {
            info.android_version = value.trim().to_string();
        } else if let Some(value) = line.strip_prefix("SDK:") {
            info.sdk = value.trim().to_string();
        } else if let Some(value) = line.strip_prefix("WIFI:") {
            info.quick.wifi = value.trim().ends_with('1');
        } else if let Some(value) = line.strip_prefix("BT:") {
            info.quick.bluetooth = value.trim().ends_with('1');
        } else if let Some(value) = line.strip_prefix("DARK:") {
            info.quick.dark_mode = value.to_lowercase().contains("yes");
        } else if let Some(value) = line.strip_prefix("AIR:") {
            info.quick.airplane_mode = value.trim().ends_with('1');
        } else if let Some(value) = line.strip_prefix("DATA:") {
            info.quick.mobile_data = value.trim().ends_with('1');
        } else if let Some(value) = line.strip_prefix("LOC:") {
            info.quick.location = value.trim().parse::<i32>().map(|v| v > 0).unwrap_or(false);
        } else if let Some(value) = line.strip_prefix("DND:") {
            info.quick.dnd = DIGITS_RE
                .captures(value)
                .and_then(|c| c[1].parse::<i32>().ok())
                .map(|v| v > 0)
                .unwrap_or(false);
        } else if let Some(value) = line.strip_prefix("ROT:") {
            info.quick.auto_rotate = value.trim().ends_with('1');
        } else if let Some(value) = line.strip_prefix("BRI:") {
            if let Some(caps) = DIGITS_RE.captures(value) {
                if let Ok(brightness) = caps[1].parse::<i32>() {
                    info.quick.brightness = brightness;
                    // Android 12+ widened the range to 0-4095; older releases use 0-255.
                    info.quick.brightness_max = if brightness > 255 { 4095 } else { 255 };
                }
            }
        } else if line.starts_with("level:") {
            if let Some(caps) = DIGITS_RE.captures(line) {
                info.battery = caps[1].parse::<i32>().ok();
            }
        } else if line.contains("powered: true") {
            info.charging = true;
        } else if line.starts_with("VOL:") {
            if let Some(caps) = VOLUME_RE.captures(line) {
                let current: i32 = caps[1].parse().unwrap_or(0);
                let low: i32 = caps[2].parse().unwrap_or(0);
                let high: i32 = caps[3].parse().unwrap_or(0);
                if high > low {
                    info.quick.volume =
                        ((current - low) as f64 / (high - low) as f64 * 100.0).round() as i32;
                }
            }
        }
    }

    if info.quick.volume < 0 {
        info.quick.volume = 50;
    }

    info
}

/// Fresh device list from `adb devices -l`.
pub async fn list(tools: &ToolStatus) -> Result<Vec<Device>> {
    let adb = adb::adb_path(tools)?;
    let output = adb::list_devices_raw(&adb).await?;
    if !output.success {
        return Err(AppError::CommandFailed(output.text()));
    }
    Ok(parse_devices(&output.stdout))
}

/// Battery + version + quick state for one device.
pub async fn info(tools: &ToolStatus, serial: &str) -> Result<DeviceInfo> {
    let adb = adb::adb_path(tools)?;
    let output = adb::shell(&adb, serial, INFO_COMMAND, Duration::from_secs(25)).await?;
    if !output.success && output.text().is_empty() {
        return Err(AppError::CommandFailed(format!(
            "Could not read info for {serial}"
        )));
    }
    Ok(parse_info(&output.text()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEVICES_L: &str = include_str!("../../tests/fixtures/adb_devices_l.txt");
    const WIRELESS: &str = include_str!("../../tests/fixtures/adb_devices_wireless.txt");
    const INFO: &str = include_str!("../../tests/fixtures/dumpsys_info.txt");

    #[test]
    fn parses_usb_devices() {
        let devices = parse_devices(DEVICES_L);
        assert_eq!(devices.len(), 2);

        let pixel = &devices[0];
        assert_eq!(pixel.serial, "0123456789ABCDEF");
        assert_eq!(pixel.model, "Pixel 8");
        assert_eq!(pixel.display_name, "Pixel 8");
        assert_eq!(pixel.transport, DeviceTransport::Usb);
        assert_eq!(pixel.state, DeviceState::Device);
        assert!(pixel.connected);

        let unauthorized = &devices[1];
        assert_eq!(unauthorized.state, DeviceState::Unauthorized);
        assert!(!unauthorized.connected);
    }

    #[test]
    fn merges_wireless_duplicate_transports() {
        let devices = parse_devices(WIRELESS);
        assert_eq!(
            devices.len(),
            1,
            "ip:port and mDNS alias must collapse to one"
        );

        let phone = &devices[0];
        assert!(is_mdns(&phone.serial), "mDNS alias should be kept");
        assert_eq!(phone.transport, DeviceTransport::Wifi);
        assert_eq!(phone.address.as_deref(), Some("192.168.1.42:5555"));
        assert!(phone.connected);
    }

    #[test]
    fn parses_info_block() {
        let info = parse_info(INFO);
        assert_eq!(info.model, "Pixel 8");
        assert_eq!(info.android_version, "15");
        assert_eq!(info.sdk, "35");
        assert_eq!(info.battery, Some(82));
        assert!(info.charging);
        assert!(info.quick.wifi);
        assert!(info.quick.bluetooth);
        assert!(info.quick.dark_mode);
        assert!(!info.quick.airplane_mode);
        assert!(info.quick.mobile_data);
        assert!(info.quick.location);
        assert!(info.quick.dnd);
        assert!(info.quick.auto_rotate);
        assert_eq!(info.quick.brightness, 1024);
        assert_eq!(info.quick.brightness_max, 4095);
        assert_eq!(info.quick.volume, 53);
    }
}
