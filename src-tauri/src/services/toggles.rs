//! Quick controls: toggles, brightness, volume and reboot. The brightness ceiling differs per
//! device and is read from the framework resource, cached for the life of the process.

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

/// Fallback screen-brightness ceiling (AOSP default) for devices without `config_screenBrightnessSettingMaximum`.
const BRIGHTNESS_MAX_FALLBACK: i32 = 255;
/// Framework resource holding the upper bound of `Settings.System.SCREEN_BRIGHTNESS`.
const OVERLAY_BRIGHTNESS_MAX: &str =
    "cmd overlay lookup android android:integer/config_screenBrightnessSettingMaximum";
/// Android volume key codes, used to nudge the stream up/down.
const KEYCODE_VOLUME_UP: &str = "24";
const KEYCODE_VOLUME_DOWN: &str = "25";

static VOLUME_RANGE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"volume is (\d+) in range \[(\d+)\.\.(\d+)\]").expect("valid regex"));

static BRIGHTNESS_MAX: Lazy<Mutex<HashMap<String, i32>>> = Lazy::new(|| Mutex::new(HashMap::new()));

/// The remote shell command for one quick toggle.
pub fn toggle_command(key: &str, on: bool) -> Result<String> {
    let flag = if on { "1" } else { "0" };
    let on_off = if on { "enable" } else { "disable" };
    let yes_no = if on { "yes" } else { "no" };

    let command = match key {
        "wifi" => format!("svc wifi {on_off}"),
        "bluetooth" => format!("svc bluetooth {on_off}"),
        "darkmode" => format!("cmd uimode night {yes_no}"),
        "airplane" => format!(
            "settings put global airplane_mode_on {flag} && \
             am broadcast -a android.intent.action.AIRPLANE_MODE"
        ),
        "data" => format!("settings put global mobile_data {flag} && svc data {on_off}"),
        "location" => format!(
            "settings put secure location_mode {}",
            if on { "3" } else { "0" }
        ),
        "dnd" => format!("cmd settings put global zen_mode {flag}"),
        "rotate" => format!("settings put system accelerometer_rotation {flag}"),
        _ => return Err(AppError::Invalid(format!("Unknown toggle: {key}"))),
    };
    Ok(command)
}

/// Battery + version + quick state, with the real brightness ceiling folded in.
pub async fn quick_state(tools: &ToolStatus, serial: &str) -> Result<QuickState> {
    let mut info = devices::info(tools, serial).await?;
    let adb = adb::adb_path(tools)?;
    info.quick.brightness_max = brightness_max(&adb, serial).await;
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
    let max = brightness_max(&adb, serial).await;
    let raw = percent_to_raw(percent, max);
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
    let (current, low, high) = volume_levels(&adb, serial).await?;
    let target = volume_index(percent, low, high);
    let delta = target - current;
    if delta == 0 {
        return Ok(());
    }

    // `cmd media_session volume --set` is a silent no-op on several builds, so drive the stream with key events.
    let key = if delta > 0 {
        KEYCODE_VOLUME_UP
    } else {
        KEYCODE_VOLUME_DOWN
    };
    let keys = vec![key; delta.unsigned_abs() as usize].join(" ");
    let output = adb::shell(&adb, serial, &format!("input keyevent {keys}"), SET_TIMEOUT).await?;
    ensure_ok(&output)
}

pub async fn reboot(tools: &ToolStatus, serial: &str, mode: RebootMode) -> Result<()> {
    let adb = adb::adb_path(tools)?;
    let args = vec!["reboot".to_string(), mode.as_arg().to_string()];
    let output = adb::run_scoped(&adb, serial, &args, SET_TIMEOUT).await?;
    ensure_ok(&output)
}

/// Read (once per device) the framework's screen-brightness ceiling. The setting accepts any
/// integer, so the configured maximum has to be read directly; older devices fall back to 255.
async fn brightness_max(adb: &str, serial: &str) -> i32 {
    if let Some(max) = cache_get(&BRIGHTNESS_MAX, serial) {
        return max;
    }

    let max = match adb::shell(adb, serial, OVERLAY_BRIGHTNESS_MAX, READ_TIMEOUT).await {
        Ok(output) => parse_brightness_max(&output.text()).unwrap_or(BRIGHTNESS_MAX_FALLBACK),
        Err(_) => BRIGHTNESS_MAX_FALLBACK,
    };

    cache_set(&BRIGHTNESS_MAX, serial, max);
    max
}

/// Current stream-3 (music) `(index, low, high)` from `cmd media_session volume`.
async fn volume_levels(adb: &str, serial: &str) -> Result<(i32, i32, i32)> {
    for command in [
        "cmd media_session volume --stream 3 --get",
        "media volume --stream 3 --get",
    ] {
        if let Ok(output) = adb::shell(adb, serial, command, READ_TIMEOUT).await {
            if let Some(levels) = parse_volume_levels(&output.text()) {
                return Ok(levels);
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

/// `(current, low, high)` from a `volume is X in range [low..high]` line.
pub fn parse_volume_levels(text: &str) -> Option<(i32, i32, i32)> {
    let caps = VOLUME_RANGE_RE.captures(text)?;
    let current = caps[1].parse::<i32>().ok()?;
    let low = caps[2].parse::<i32>().ok()?;
    let high = caps[3].parse::<i32>().ok()?;
    if high > low {
        Some((current, low, high))
    } else {
        None
    }
}

/// The brightness ceiling from `cmd overlay lookup`, if the first line is a sane integer.
pub fn parse_brightness_max(text: &str) -> Option<i32> {
    let value = text.lines().next()?.trim().parse::<i32>().ok()?;
    (1..=65_535).contains(&value).then_some(value)
}

/// Map a UI percentage to a raw brightness value for the given ceiling.
pub fn percent_to_raw(percent: i32, max: i32) -> i32 {
    let pct = percent.clamp(0, 100);
    ((pct as f64 / 100.0) * max as f64).round() as i32
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
    fn parses_volume_level_lines() {
        assert_eq!(
            parse_volume_levels("volume is 10 in range [0..15]"),
            Some((10, 0, 15))
        );
        // The verbose `[V] ...` prefix that `media_session` emits must not matter.
        assert_eq!(
            parse_volume_levels("[V] volume is 17 in range [0..25]"),
            Some((17, 0, 25))
        );
        assert_eq!(
            parse_volume_levels("volume is 3 in range [1..10]"),
            Some((3, 1, 10))
        );
        assert_eq!(parse_volume_levels("garbage"), None);
        assert_eq!(parse_volume_levels("volume is 3 in range [5..5]"), None);
    }

    #[test]
    fn parses_brightness_max_output() {
        assert_eq!(parse_brightness_max("255"), Some(255));
        assert_eq!(parse_brightness_max("4095\n"), Some(4095));
        assert_eq!(parse_brightness_max(""), None);
        assert_eq!(parse_brightness_max("null"), None);
        assert_eq!(
            parse_brightness_max("Error: no such resource (code 0x1)"),
            None
        );
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
}
