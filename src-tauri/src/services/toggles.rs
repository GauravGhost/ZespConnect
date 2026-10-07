//! Quick controls: toggles, brightness, volume and reboot.
//!
//! Brightness span and volume stream range differ per device and are not
//! reported by any single adb call, so each is detected once and cached for the
//! life of the process. The pure helpers at the bottom are unit-tested.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use once_cell::sync::Lazy;
use regex::Regex;

use crate::error::{AppError, Result};
use crate::models::{QuickState, RebootMode, ToolStatus};
use crate::services::{adb, devices};
use crate::util::process::ProcessOutput;

const SET_TIMEOUT: Duration = Duration::from_secs(15);
const READ_TIMEOUT: Duration = Duration::from_secs(10);

/// Android 12+ widened the brightness range to 0–4095; older releases use 0–255.
const SPAN_WIDE: i32 = 4095;
const SPAN_NARROW: i32 = 255;
/// A value above the narrow range that persists only on a wide-range device.
const PROBE_BRIGHTNESS: i32 = 260;

static DIGITS_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(\d+)").expect("valid regex"));
static VOLUME_RANGE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"volume is (\d+) in range \[(\d+)\.\.(\d+)\]").expect("valid regex"));

static BRIGHTNESS_SPAN: Lazy<Mutex<HashMap<String, i32>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static VOLUME_RANGE: Lazy<Mutex<HashMap<String, (i32, i32)>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// The remote shell command for one quick toggle.
pub fn toggle_command(key: &str, on: bool) -> Result<String> {
    let bool01 = if on { "1" } else { "0" };
    let onoff = if on { "enable" } else { "disable" };
    let yesno = if on { "yes" } else { "no" };

    let command = match key {
        "wifi" => format!("svc wifi {onoff}"),
        "bluetooth" => format!("svc bluetooth {onoff}"),
        "darkmode" => format!("cmd uimode night {yesno}"),
        "airplane" => format!(
            "settings put global airplane_mode_on {bool01} && \
             am broadcast -a android.intent.action.AIRPLANE_MODE"
        ),
        "data" => format!("settings put global mobile_data {bool01} && svc data {onoff}"),
        "location" => format!(
            "settings put secure location_mode {}",
            if on { "3" } else { "0" }
        ),
        "dnd" => format!("cmd settings put global zen_mode {bool01}"),
        "rotate" => format!("settings put system accelerometer_rotation {bool01}"),
        _ => return Err(AppError::Invalid(format!("Unknown toggle: {key}"))),
    };
    Ok(command)
}

/// Battery + version + quick state, with the real brightness span folded in.
pub async fn quick_state(tools: &ToolStatus, serial: &str) -> Result<QuickState> {
    let mut info = devices::info(tools, serial).await?;
    let adb = adb::adb_path(tools)?;
    if let Ok(span) = brightness_span(&adb, serial).await {
        info.quick.brightness_max = span;
    }
    Ok(info.quick)
}

pub async fn set_toggle(tools: &ToolStatus, serial: &str, key: &str, on: bool) -> Result<()> {
    let adb = adb::adb_path(tools)?;
    let command = toggle_command(key, on)?;
    let output = adb::shell(&adb, serial, &command, SET_TIMEOUT).await?;
    ensure_ok(&output)
}

pub async fn set_brightness(tools: &ToolStatus, serial: &str, percent: i32) -> Result<()> {
    let adb = adb::adb_path(tools)?;
    let span = brightness_span(&adb, serial).await?;
    let raw = percent_to_raw(percent, span);
    let output = adb::shell(
        &adb,
        serial,
        &format!("settings put system screen_brightness {raw}"),
        SET_TIMEOUT,
    )
    .await?;
    ensure_ok(&output)
}

pub async fn set_volume(tools: &ToolStatus, serial: &str, percent: i32) -> Result<()> {
    let adb = adb::adb_path(tools)?;
    let (low, high) = volume_range(&adb, serial).await?;
    let index = volume_index(percent, low, high);

    let output = adb::shell(
        &adb,
        serial,
        &format!("cmd media_session volume --stream 3 --set {index}"),
        SET_TIMEOUT,
    )
    .await?;
    if output.success {
        return Ok(());
    }

    // Older releases only expose `media volume`.
    let fallback = adb::shell(
        &adb,
        serial,
        &format!("media volume --stream 3 --set {index}"),
        SET_TIMEOUT,
    )
    .await?;
    ensure_ok(&fallback)
}

pub async fn reboot(tools: &ToolStatus, serial: &str, mode: RebootMode) -> Result<()> {
    let adb = adb::adb_path(tools)?;
    let args = vec!["reboot".to_string(), mode.as_arg().to_string()];
    let output = adb::run_scoped(&adb, serial, &args, SET_TIMEOUT).await?;
    ensure_ok(&output)
}

/// Detect (once per device) whether brightness runs 0–255 or 0–4095 by writing a
/// value above the narrow range and seeing whether it sticks, then restoring.
async fn brightness_span(adb: &str, serial: &str) -> Result<i32> {
    if let Some(span) = cache_get(&BRIGHTNESS_SPAN, serial) {
        return Ok(span);
    }

    let original = read_brightness(adb, serial).await?.unwrap_or(128);
    let _ = write_brightness(adb, serial, PROBE_BRIGHTNESS).await;
    let after = read_brightness(adb, serial).await?;
    let span = if after == Some(PROBE_BRIGHTNESS) {
        SPAN_WIDE
    } else {
        SPAN_NARROW
    };
    let _ = write_brightness(adb, serial, original).await;

    cache_set(&BRIGHTNESS_SPAN, serial, span);
    Ok(span)
}

async fn read_brightness(adb: &str, serial: &str) -> Result<Option<i32>> {
    let output = adb::shell(
        adb,
        serial,
        "settings get system screen_brightness",
        READ_TIMEOUT,
    )
    .await?;
    Ok(first_int(&output.text()))
}

async fn write_brightness(adb: &str, serial: &str, raw: i32) -> Result<()> {
    let output = adb::shell(
        adb,
        serial,
        &format!("settings put system screen_brightness {raw}"),
        READ_TIMEOUT,
    )
    .await?;
    ensure_ok(&output)
}

/// Cache the `[low..high]` volume range of stream 3 (music).
async fn volume_range(adb: &str, serial: &str) -> Result<(i32, i32)> {
    if let Some(range) = cache_get(&VOLUME_RANGE, serial) {
        return Ok(range);
    }

    for command in [
        "cmd media_session volume --stream 3 --get",
        "media volume --stream 3 --get",
    ] {
        if let Ok(output) = adb::shell(adb, serial, command, READ_TIMEOUT).await {
            if let Some(range) = parse_volume_range(&output.text()) {
                cache_set(&VOLUME_RANGE, serial, range);
                return Ok(range);
            }
        }
    }

    Err(AppError::Parse(
        "Could not determine the device volume range".into(),
    ))
}

fn ensure_ok(output: &ProcessOutput) -> Result<()> {
    if output.success || output.text().is_empty() {
        Ok(())
    } else {
        Err(AppError::CommandFailed(output.text()))
    }
}

fn cache_get<T: Copy>(cache: &Lazy<Mutex<HashMap<String, T>>>, serial: &str) -> Option<T> {
    cache
        .lock()
        .ok()
        .and_then(|guard| guard.get(serial).copied())
}

fn cache_set<T>(cache: &Lazy<Mutex<HashMap<String, T>>>, serial: &str, value: T) {
    if let Ok(mut guard) = cache.lock() {
        guard.insert(serial.to_string(), value);
    }
}

/// First run of digits in `text`, if any.
pub fn first_int(text: &str) -> Option<i32> {
    DIGITS_RE
        .captures(text)
        .and_then(|caps| caps[1].parse::<i32>().ok())
}

/// `(low, high)` from a `volume is X in range [low..high]` line.
pub fn parse_volume_range(text: &str) -> Option<(i32, i32)> {
    let caps = VOLUME_RANGE_RE.captures(text)?;
    let low = caps[2].parse::<i32>().ok()?;
    let high = caps[3].parse::<i32>().ok()?;
    if high > low {
        Some((low, high))
    } else {
        None
    }
}

/// Map a UI percentage to a raw brightness value for the given span.
pub fn percent_to_raw(percent: i32, span: i32) -> i32 {
    let pct = percent.clamp(0, 100);
    ((pct as f64 / 100.0) * span as f64).round() as i32
}

/// Map a UI percentage to a stream volume index within `[low..high]`.
pub fn volume_index(percent: i32, low: i32, high: i32) -> i32 {
    let pct = percent.clamp(0, 100);
    low + ((pct as f64 / 100.0) * (high - low) as f64).round() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_toggle_commands() {
        assert_eq!(toggle_command("wifi", true).unwrap(), "svc wifi enable");
        assert_eq!(toggle_command("wifi", false).unwrap(), "svc wifi disable");
        assert_eq!(
            toggle_command("darkmode", true).unwrap(),
            "cmd uimode night yes"
        );
        assert_eq!(
            toggle_command("airplane", false).unwrap(),
            "settings put global airplane_mode_on 0 && \
             am broadcast -a android.intent.action.AIRPLANE_MODE"
        );
        assert_eq!(
            toggle_command("location", true).unwrap(),
            "settings put secure location_mode 3"
        );
        assert_eq!(
            toggle_command("rotate", false).unwrap(),
            "settings put system accelerometer_rotation 0"
        );
        assert_eq!(
            toggle_command("dnd", true).unwrap(),
            "cmd settings put global zen_mode 1"
        );
        assert_eq!(
            toggle_command("data", true).unwrap(),
            "settings put global mobile_data 1 && svc data enable"
        );
    }

    #[test]
    fn rejects_unknown_toggles() {
        assert!(toggle_command("nope", true).is_err());
    }

    #[test]
    fn parses_volume_range_lines() {
        assert_eq!(
            parse_volume_range("volume is 10 in range [0..15]"),
            Some((0, 15))
        );
        assert_eq!(
            parse_volume_range("volume is 3 in range [1..10]"),
            Some((1, 10))
        );
        assert_eq!(parse_volume_range("garbage"), None);
        assert_eq!(parse_volume_range("volume is 3 in range [5..5]"), None);
    }

    #[test]
    fn maps_percent_to_raw_brightness() {
        assert_eq!(percent_to_raw(0, 255), 0);
        assert_eq!(percent_to_raw(100, 255), 255);
        assert_eq!(percent_to_raw(50, 255), 128);
        assert_eq!(percent_to_raw(100, 4095), 4095);
        assert_eq!(percent_to_raw(50, 4095), 2048);
        // Out-of-range input is clamped.
        assert_eq!(percent_to_raw(150, 255), 255);
        assert_eq!(percent_to_raw(-10, 255), 0);
    }

    #[test]
    fn maps_percent_to_volume_index() {
        assert_eq!(volume_index(0, 0, 15), 0);
        assert_eq!(volume_index(100, 0, 15), 15);
        assert_eq!(volume_index(50, 0, 15), 8);
        assert_eq!(volume_index(100, 1, 10), 10);
        assert_eq!(volume_index(0, 1, 10), 1);
    }

    #[test]
    fn reads_first_integer() {
        assert_eq!(first_int("1024"), Some(1024));
        assert_eq!(first_int("null"), None);
        assert_eq!(first_int(" 260\n"), Some(260));
    }
}
