use std::time::Duration;

use crate::error::{AppError, Result};
use crate::models::{DeveloperState, ToolStatus};
use crate::services::adb;
use crate::services::toggles::ensure_ok;

const TIMEOUT: Duration = Duration::from_secs(10);
const MAX_ANIMATION_SCALE: f32 = 10.0;

const SCALE_SETTINGS: [&str; 3] = [
    "window_animation_scale",
    "transition_animation_scale",
    "animator_duration_scale",
];

/// One round-trip read of every developer setting shown in the app.
const STATE_COMMAND: &str = concat!(
    r#"echo "TAPS:$(settings get system show_touches)"; "#,
    r#"echo "PTR:$(settings get system pointer_location)"; "#,
    r#"echo "BOUNDS:$(getprop debug.layout)"; "#,
    r#"echo "OVERDRAW:$(getprop debug.hwui.overdraw)"; "#,
    r#"echo "ANIM:$(settings get global window_animation_scale) "#,
    r#"$(settings get global transition_animation_scale) "#,
    r#"$(settings get global animator_duration_scale)""#,
);

/// Pokes every service to re-read system properties, so view debug flags apply without
/// restarting apps. It prints an error parcel on recent Android but still works.
const REFRESH_PROPS: &str = "service call activity 1599295570 >/dev/null 2>&1";

pub async fn state(tools: &ToolStatus, serial: &str) -> Result<DeveloperState> {
    let adb = adb::adb_path(tools)?;
    let output = adb::shell(&adb, serial, STATE_COMMAND, TIMEOUT).await?;
    if !output.success {
        return Err(AppError::CommandFailed(output.text()));
    }
    Ok(parse_state(&output.text()))
}

pub async fn set_toggle(tools: &ToolStatus, serial: &str, key: &str, on: bool) -> Result<()> {
    let adb = adb::adb_path(tools)?;
    let output = adb::shell(&adb, serial, &toggle_command(key, on)?, TIMEOUT).await?;
    ensure_ok(&output)
}

pub async fn set_animation_scale(tools: &ToolStatus, serial: &str, scale: f32) -> Result<()> {
    let adb = adb::adb_path(tools)?;
    let output = adb::shell(&adb, serial, &animation_scale_command(scale)?, TIMEOUT).await?;
    ensure_ok(&output)
}

fn toggle_command(key: &str, on: bool) -> Result<String> {
    let flag = u8::from(on);
    let command = match key {
        "show_taps" => format!("settings put system show_touches {flag}"),
        "pointer_location" => format!("settings put system pointer_location {flag}"),
        "layout_bounds" => format!("setprop debug.layout {on}; {REFRESH_PROPS}"),
        "gpu_overdraw" => format!(
            "setprop debug.hwui.overdraw {}; {REFRESH_PROPS}",
            if on { "show" } else { "false" }
        ),
        _ => {
            return Err(AppError::Invalid(format!(
                "Unknown developer toggle: {key}"
            )))
        }
    };
    Ok(command)
}

fn animation_scale_command(scale: f32) -> Result<String> {
    if !(0.0..=MAX_ANIMATION_SCALE).contains(&scale) {
        return Err(AppError::Invalid(format!(
            "Animation scale must be between 0 and {MAX_ANIMATION_SCALE}"
        )));
    }
    Ok(SCALE_SETTINGS
        .iter()
        .map(|name| format!("settings put global {name} {scale}"))
        .collect::<Vec<_>>()
        .join("; "))
}

fn parse_state(text: &str) -> DeveloperState {
    let mut state = DeveloperState {
        show_taps: false,
        pointer_location: false,
        layout_bounds: false,
        gpu_overdraw: false,
        animation_scale: Some(1.0),
    };

    for line in text.lines().map(str::trim) {
        if let Some(value) = line.strip_prefix("TAPS:") {
            state.show_taps = value.trim() == "1";
        } else if let Some(value) = line.strip_prefix("PTR:") {
            state.pointer_location = value.trim() == "1";
        } else if let Some(value) = line.strip_prefix("BOUNDS:") {
            state.layout_bounds = value.trim() == "true";
        } else if let Some(value) = line.strip_prefix("OVERDRAW:") {
            state.gpu_overdraw = value.trim() == "show";
        } else if let Some(value) = line.strip_prefix("ANIM:") {
            state.animation_scale = shared_scale(value);
        }
    }

    state
}

/// The common value of the three scales; an unset scale ("null") is Android's default 1.
fn shared_scale(value: &str) -> Option<f32> {
    let scales: Vec<f32> = value
        .split_whitespace()
        .map(|raw| {
            if raw == "null" {
                Some(1.0)
            } else {
                raw.parse().ok()
            }
        })
        .collect::<Option<_>>()?;
    let first = *scales.first()?;
    scales.iter().all(|&scale| scale == first).then_some(first)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_toggle_commands() {
        assert_eq!(
            toggle_command("show_taps", true).unwrap(),
            "settings put system show_touches 1"
        );
        assert_eq!(
            toggle_command("pointer_location", false).unwrap(),
            "settings put system pointer_location 0"
        );
        assert_eq!(
            toggle_command("layout_bounds", true).unwrap(),
            format!("setprop debug.layout true; {REFRESH_PROPS}")
        );
        assert_eq!(
            toggle_command("gpu_overdraw", false).unwrap(),
            format!("setprop debug.hwui.overdraw false; {REFRESH_PROPS}")
        );
        assert!(toggle_command("wifi", true).is_err());
    }

    #[test]
    fn builds_animation_scale_command() {
        assert_eq!(
            animation_scale_command(0.5).unwrap(),
            "settings put global window_animation_scale 0.5; \
             settings put global transition_animation_scale 0.5; \
             settings put global animator_duration_scale 0.5"
        );
        assert!(animation_scale_command(0.0)
            .unwrap()
            .ends_with("animator_duration_scale 0"));
        assert!(animation_scale_command(-1.0).is_err());
        assert!(animation_scale_command(11.0).is_err());
    }

    #[test]
    fn parses_state() {
        let state = parse_state("TAPS:1\nPTR:0\nBOUNDS:true\nOVERDRAW:show\nANIM:0.5 0.5 0.5\n");
        assert_eq!(
            state,
            DeveloperState {
                show_taps: true,
                pointer_location: false,
                layout_bounds: true,
                gpu_overdraw: true,
                animation_scale: Some(0.5),
            }
        );
    }

    #[test]
    fn parses_unset_and_mixed_scales() {
        let state = parse_state("TAPS:0\nPTR:0\nBOUNDS:\nOVERDRAW:\nANIM:null null null\n");
        assert!(!state.layout_bounds);
        assert!(!state.gpu_overdraw);
        assert_eq!(state.animation_scale, Some(1.0));

        assert_eq!(shared_scale("1.0 0.5 1.0"), None);
        assert_eq!(shared_scale("garbage"), None);
    }
}
