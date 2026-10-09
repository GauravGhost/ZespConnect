//! scrcpy argv building and Android API detection for app windows; pure or thin adb calls, unit tested directly.

use std::time::Duration;

use once_cell::sync::Lazy;

use crate::error::{AppError, Result};
use crate::models::{MirrorOptions, ToolStatus};
use crate::services::{adb, updates};

const API_TIMEOUT: Duration = Duration::from_secs(15);

/// Bounds for the UI's numeric options; values are clamped, not rejected, to keep the UI forgiving.
const MIN_RESOLUTION: u32 = 240;
const MAX_RESOLUTION: u32 = 4096;
const MAX_FPS_LIMIT: u32 = 240;
const MAX_BITRATE: u32 = 200;

/// `--render-fit` values scrcpy understands.
const FIT_VALUES: &[&str] = &["contain", "cover", "stretch"];

/// Default `-m` cap when an app is mirrored on the main display.
pub const DEFAULT_MAX_SIZE: u32 = 1024;

/// scrcpy's `--new-display` crashes SystemUI on Android 12-13, so those (and unknown) versions mirror the main display.
const UNSAFE_VIRTUAL_DISPLAY_APIS: &[u32] = &[31, 32, 33];

/// scrcpy refuses a virtual display before Android 10, so older devices mirror the main display.
const MIN_VIRTUAL_DISPLAY_API: u32 = 29;

/// scrcpy added `--flex-display` in 4.0; older releases reject it as an unrecognized option.
const FLEX_DISPLAY_MIN_VERSION: &str = "4.0";

/// scrcpy added `--no-vd-system-decorations` (server: `vd_system_decorations`) in 3.1. Decorated
/// virtual displays get the launcher/wallpaper/taskbar, on which a cold app never draws while the
/// phone sleeps, so app windows turn decorations off when supported.
const VD_DECORATIONS_MIN_VERSION: &str = "3.1";

pub fn scrcpy_path(tools: &ToolStatus) -> Result<String> {
    tools.scrcpy.path.clone().ok_or(AppError::ScrcpyMissing)
}

/// Whether it is safe to give an app its own virtual display on this device.
pub fn virtual_display_safe(api: Option<u32>) -> bool {
    match api {
        Some(value) => {
            value >= MIN_VIRTUAL_DISPLAY_API && !UNSAFE_VIRTUAL_DISPLAY_APIS.contains(&value)
        }
        None => false,
    }
}

/// The device's Android API level (`ro.build.version.sdk`), if readable.
pub async fn android_api(adb_path: &str, serial: &str) -> Option<u32> {
    let output = adb::shell(
        adb_path,
        serial,
        "getprop ro.build.version.sdk",
        API_TIMEOUT,
    )
    .await
    .ok()?;
    output.stdout.trim().parse().ok()
}

/// Whether the installed scrcpy understands `--flex-display`; unknown versions are treated as unsupported.
pub fn flex_display_supported(version: Option<&str>) -> bool {
    version
        .map(|version| updates::compare(version, FLEX_DISPLAY_MIN_VERSION).is_ge())
        .unwrap_or(false)
}

/// Whether the installed scrcpy can turn off virtual display decorations; unknown versions are unsupported.
pub fn vd_decorations_option_supported(version: Option<&str>) -> bool {
    version
        .map(|version| updates::compare(version, VD_DECORATIONS_MIN_VERSION).is_ge())
        .unwrap_or(false)
}

/// How [`app_window_args`] shows the app; the last two only apply on a virtual display.
#[derive(Debug, Clone, Copy, Default)]
pub struct AppWindowMode {
    /// Give the app its own virtual display rather than the main one.
    pub virtual_display: bool,
    /// The installed scrcpy supports `--flex-display`.
    pub flex_display: bool,
    /// Turn off the virtual display's system decorations.
    pub no_decorations: bool,
}

/// Build the argv for an app window (virtual display, else main display with the app in front).
/// The adb binary is not an argument: scrcpy 4.x takes it from the `ADB` env var (set by the caller).
pub fn app_window_args(
    serial: &str,
    package: &str,
    window_title: &str,
    mode: AppWindowMode,
    max_size: u32,
) -> Vec<String> {
    let mut args = vec![
        "--serial".to_string(),
        serial.to_string(),
        "--window-title".to_string(),
        window_title.to_string(),
        format!("--start-app={package}"),
    ];

    if mode.virtual_display {
        args.push("--new-display".to_string());
        if mode.no_decorations {
            args.push("--no-vd-system-decorations".to_string());
        }
        if mode.flex_display {
            args.push("--flex-display".to_string());
        } else {
            // Older scrcpy has no `--flex-display`; cap the virtual display instead.
            args.push("-m".to_string());
            args.push(max_size.to_string());
        }
    } else {
        args.push("-m".to_string());
        args.push(max_size.to_string());
    }

    args.push("--stay-awake".to_string());
    args
}

/// Android 16 (API 36) brought desktop windowing (taskbar, launcher, freeform windows) to connected displays.
const MIN_DESKTOP_API: u32 = 36;

/// The longest side a desktop display may take; larger screens are scaled down with the density.
const MAX_DESKTOP_SIDE: f64 = 2560.0;

/// Android's baseline density: one dp is one pixel at 160 dpi.
const BASE_DENSITY: f64 = 160.0;

/// Desktop streams a large, text-heavy display, so it defaults to a higher bitrate (Mbps) than a phone mirror.
pub const DESKTOP_BIT_RATE: u32 = 16;

/// The density range a desktop display is kept within.
const MIN_DESKTOP_DPI: f64 = 120.0;
const MAX_DESKTOP_DPI: f64 = 640.0;

/// Wake the phone (no-op when awake) if it supports desktop mode, in the same round trip as the
/// check. Prints `<sdk> <freeform> <wakefulness>` first; a virtual display started while the phone
/// sleeps never draws cold-launched apps.
static DESKTOP_PREPARE_COMMAND: Lazy<String> = Lazy::new(|| {
    format!(
        "sdk=$(getprop ro.build.version.sdk); \
ff=$(pm has-feature android.software.freeform_window_management); \
w=$(dumpsys power 2>/dev/null | grep -m1 -o 'mWakefulness=[A-Za-z]*'); \
echo \"$sdk $ff ${{w#mWakefulness=}}\"; \
[ \"$sdk\" -ge {MIN_DESKTOP_API} ] && [ \"$ff\" = true ] && input keyevent KEYCODE_WAKEUP"
    )
});

/// The `WxH/dpi` virtual display matching a computer screen of `width`×`height` pixels at `scale`.
pub fn desktop_display(width: u32, height: u32, scale: f64) -> String {
    let (width, height) = (f64::from(width.max(1)), f64::from(height.max(1)));
    let scale = if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    };
    let shrink = (MAX_DESKTOP_SIDE / width.max(height)).min(1.0);
    // Video encoders want dimensions aligned to 8.
    let align = |value: f64| ((value * shrink / 8.0).floor() * 8.0).max(8.0) as u32;
    let dpi = (BASE_DENSITY * scale * shrink)
        .round()
        .clamp(MIN_DESKTOP_DPI, MAX_DESKTOP_DPI) as u32;
    format!("{}x{}/{dpi}", align(width), align(height))
}

/// Check that `serial` can run desktop mode and wake it; returns whether its screen was already on.
pub async fn prepare_desktop(adb_path: &str, serial: &str) -> Result<bool> {
    let output = adb::shell(adb_path, serial, &DESKTOP_PREPARE_COMMAND, API_TIMEOUT).await?;
    parse_desktop_prepare(&output.stdout)
}

/// Parse [`DESKTOP_PREPARE_COMMAND`]'s first line.
fn parse_desktop_prepare(output: &str) -> Result<bool> {
    let first = output.lines().next().unwrap_or_default();
    let mut parts = first.split_whitespace();
    let api = parts.next().and_then(|value| value.parse::<u32>().ok());
    let freeform = parts.next() == Some("true");
    let screen_on = parts.next() == Some("Awake");

    match api {
        Some(api) if api < MIN_DESKTOP_API => Err(AppError::Other(
            "Desktop mode needs Android 16 or newer on the phone".into(),
        )),
        Some(_) if !freeform => Err(AppError::Other(
            "This phone does not support desktop windows".into(),
        )),
        Some(_) => Ok(screen_on),
        None => Err(AppError::Other(
            "Could not read the phone's Android version".into(),
        )),
    }
}

/// Probes a desktop session's phone, one line per finding: the lock-screen state, and
/// `launcherUnfocused` when a secondary-display launcher is focused with no focused window (or only
/// an ANR dialog), which can time the next key out into a "not responding" dialog.
const DESKTOP_PROBE_COMMAND: &str =
    "dumpsys activity activities | grep -m1 -o 'mKeyguardShowing=[a-z]*'; \
dumpsys input | awk '\
/^Input Dispatcher State at time of last ANR/{exit} \
/^  FocusedApplications:/{s=1;next} \
/^  FocusedWindows:/{s=2;next} \
/^  [A-Za-z]/{s=0} \
s==1&&/SecondaryDisplayLauncher/{split($1,p,\"=\");sub(\",\",\"\",p[2]);app[p[2]]=1} \
s==2&&!/Application Not Responding/{split($1,p,\"=\");sub(\",\",\"\",p[2]);win[p[2]]=1} \
END{for(d in app)if(!(d in win)){print \"launcherUnfocused\";exit}}'";

/// What [`desktop_probe`] found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DesktopProbe {
    /// Whether the lock screen is showing; while it is, Android hides the desktop and only the wallpaper draws.
    pub locked: Option<bool>,
    /// The desktop launcher has lost input focus (see [`DESKTOP_PROBE_COMMAND`]).
    pub launcher_unfocused: bool,
}

/// Parse [`DESKTOP_PROBE_COMMAND`]'s output.
fn parse_desktop_probe(output: &str) -> DesktopProbe {
    let mut probe = DesktopProbe {
        locked: None,
        launcher_unfocused: false,
    };
    for line in output.lines().map(str::trim) {
        match line.strip_prefix("mKeyguardShowing=") {
            Some("true") => probe.locked = Some(true),
            Some("false") => probe.locked = Some(false),
            Some(_) => {}
            None => probe.launcher_unfocused |= line == "launcherUnfocused",
        }
    }
    probe
}

/// Check a desktop session's phone; `None` when adb could not be reached.
pub async fn desktop_probe(adb_path: &str, serial: &str) -> Option<DesktopProbe> {
    let output = adb::shell(adb_path, serial, DESKTOP_PROBE_COMMAND, API_TIMEOUT)
        .await
        .ok()?;
    Some(parse_desktop_probe(&output.stdout))
}

/// Put the phone to sleep after a desktop session; the short delay lets scrcpy's cleanup restore display power first.
const SLEEP_COMMAND: &str = "sleep 0.5; input keyevent KEYCODE_SLEEP";

pub async fn sleep_phone(adb_path: &str, serial: &str) {
    let _ = adb::shell(adb_path, serial, SLEEP_COMMAND, API_TIMEOUT).await;
}

/// Build the argv for a desktop-mode window (decorated virtual display; phone screen off, asleep on close).
pub fn desktop_args(serial: &str, window_title: &str, display: &str) -> Vec<String> {
    vec![
        "--serial".to_string(),
        serial.to_string(),
        "--window-title".to_string(),
        window_title.to_string(),
        format!("--new-display={display}"),
        "--display-ime-policy=hide".to_string(),
        "--turn-screen-off".to_string(),
        "--power-off-on-close".to_string(),
        "--stay-awake".to_string(),
        // Forward every mouse button (right-click opens context menus).
        "--mouse-bind=++++".to_string(),
        format!("--video-bit-rate={DESKTOP_BIT_RATE}M"),
        "--no-audio".to_string(),
    ]
}

/// Whether `value` is a `#rrggbb` colour.
fn is_hex_color(value: &str) -> bool {
    value.len() == 7 && value.starts_with('#') && value[1..].chars().all(|c| c.is_ascii_hexdigit())
}

/// The requested `-m` size, clamped to a sane range.
pub fn max_size(opts: &MirrorOptions) -> Option<u32> {
    opts.resolution
        .map(|value| value.clamp(MIN_RESOLUTION, MAX_RESOLUTION))
}

/// The requested frame rate, clamped to `1..=240`.
pub fn max_fps(opts: &MirrorOptions) -> Option<u32> {
    opts.fps.map(|value| value.clamp(1, MAX_FPS_LIMIT))
}

/// The requested bitrate in Mbps, clamped to `1..=200`.
pub fn bit_rate(opts: &MirrorOptions) -> Option<u32> {
    opts.bitrate.map(|value| value.clamp(1, MAX_BITRATE))
}

/// The camera to open: anything other than an explicit "front" is treated as the back camera.
pub fn camera_facing(requested: Option<&str>) -> &'static str {
    match requested {
        Some("front") => "front",
        _ => "back",
    }
}

/// The requested camera zoom, if it is one scrcpy accepts.
pub fn camera_zoom(requested: Option<f64>) -> Option<f64> {
    requested.filter(|zoom| zoom.is_finite() && *zoom > 0.0)
}

/// `-m <size>`.
fn push_max_size(args: &mut Vec<String>, opts: &MirrorOptions) {
    if let Some(value) = max_size(opts) {
        args.push("-m".to_string());
        args.push(value.to_string());
    }
}

/// `--max-fps <fps>`.
fn push_max_fps(args: &mut Vec<String>, opts: &MirrorOptions) {
    if let Some(value) = max_fps(opts) {
        args.push("--max-fps".to_string());
        args.push(value.to_string());
    }
}

/// `-b <bitrate>M`.
fn push_bitrate(args: &mut Vec<String>, opts: &MirrorOptions) {
    if let Some(value) = bit_rate(opts) {
        args.push("-b".to_string());
        args.push(format!("{value}M"));
    }
}

/// Window flags shared by mirror and camera sessions; values scrcpy would reject are dropped.
fn push_window_flags(args: &mut Vec<String>, opts: &MirrorOptions) {
    if opts.stay_awake {
        args.push("--stay-awake".to_string());
    }
    if opts.borderless {
        args.push("--window-borderless".to_string());
    }
    if opts.always_on_top {
        args.push("--always-on-top".to_string());
    }
    if !opts.aspect_lock {
        args.push("--no-window-aspect-ratio-lock".to_string());
    }
    if let Some(background) = opts
        .background
        .as_deref()
        .filter(|value| is_hex_color(value))
    {
        args.push(format!("--background-color={background}"));
    }
    if let Some(fit) = opts
        .fit
        .as_deref()
        .filter(|value| FIT_VALUES.contains(value))
    {
        args.push(format!("--render-fit={fit}"));
    }
}

/// Build the argv for a full-screen mirror of `serial`.
pub fn mirror_args(serial: &str, window_title: &str, opts: &MirrorOptions) -> Vec<String> {
    let mut args = vec![
        "--serial".to_string(),
        serial.to_string(),
        "--window-title".to_string(),
        window_title.to_string(),
    ];

    push_max_size(&mut args, opts);
    push_max_fps(&mut args, opts);
    push_bitrate(&mut args, opts);
    if !opts.audio {
        args.push("--no-audio".to_string());
    }
    push_window_flags(&mut args, opts);
    args
}

/// Build the argv for a camera feed of `serial`.
pub fn camera_args(serial: &str, window_title: &str, opts: &MirrorOptions) -> Vec<String> {
    let facing = camera_facing(opts.camera_facing.as_deref());
    let mut args = vec![
        "--serial".to_string(),
        serial.to_string(),
        "--window-title".to_string(),
        window_title.to_string(),
        "--video-source=camera".to_string(),
        format!("--camera-facing={facing}"),
    ];

    if opts.camera_torch {
        args.push("--camera-torch".to_string());
    }
    if let Some(zoom) = camera_zoom(opts.camera_zoom) {
        args.push(format!("--camera-zoom={zoom}"));
    }

    // A camera feed is not a display, so `-m` is omitted; fps/bitrate still apply.
    push_max_fps(&mut args, opts);
    push_bitrate(&mut args, opts);
    args.push(if opts.audio {
        "--audio-source=playback".to_string()
    } else {
        "--no-audio".to_string()
    });
    push_window_flags(&mut args, opts);
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn virtual_display_is_unsafe_on_android_12_to_13_and_unknown() {
        assert!(virtual_display_safe(Some(30)));
        assert!(virtual_display_safe(Some(34)));
        assert!(!virtual_display_safe(Some(31)));
        assert!(!virtual_display_safe(Some(32)));
        assert!(!virtual_display_safe(Some(33)));
        assert!(!virtual_display_safe(None));
    }

    #[test]
    fn virtual_display_needs_android_10() {
        assert!(virtual_display_safe(Some(29)));
        assert!(!virtual_display_safe(Some(28)));
    }

    #[test]
    fn flex_display_needs_scrcpy_4() {
        assert!(flex_display_supported(Some("4.0")));
        assert!(flex_display_supported(Some("5.0")));
        assert!(!flex_display_supported(Some("3.3.3")));
        assert!(!flex_display_supported(None));
    }

    #[test]
    fn desktop_prepare_reports_whether_the_screen_was_on() {
        assert!(parse_desktop_prepare("36 true Awake\n").unwrap());
        assert!(!parse_desktop_prepare("36 true Asleep").unwrap());
        assert!(!parse_desktop_prepare("36 true").unwrap());
        assert!(parse_desktop_prepare("35 true Awake").is_err());
        assert!(parse_desktop_prepare("36 false Awake").is_err());
        assert!(parse_desktop_prepare("").is_err());
    }

    #[test]
    fn parses_the_desktop_probe() {
        let probe = parse_desktop_probe("mKeyguardShowing=true\n");
        assert_eq!(probe.locked, Some(true));
        assert!(!probe.launcher_unfocused);

        let probe = parse_desktop_probe("mKeyguardShowing=false\nlauncherUnfocused\n");
        assert_eq!(probe.locked, Some(false));
        assert!(probe.launcher_unfocused);

        assert_eq!(parse_desktop_probe("").locked, None);
        assert_eq!(parse_desktop_probe("mKeyguardShowing=maybe").locked, None);
    }

    #[test]
    fn desktop_display_matches_a_retina_laptop() {
        // A 2560x1664 panel at 2x is a 1280x832 dp desktop.
        assert_eq!(desktop_display(2560, 1664, 2.0), "2560x1664/320");
    }

    #[test]
    fn desktop_display_scales_big_screens_down_with_their_density() {
        // 4K at 2x keeps a 1920x1080 dp layout in 2560x1440 pixels.
        assert_eq!(desktop_display(3840, 2160, 2.0), "2560x1440/213");
    }

    #[test]
    fn desktop_display_aligns_to_8_and_tolerates_bad_input() {
        assert_eq!(desktop_display(1366, 768, 1.0), "1360x768/160");
        assert_eq!(desktop_display(1920, 1080, f64::NAN), "1920x1080/160");
    }

    #[test]
    fn desktop_window_keeps_decorations_and_turns_the_phone_off() {
        let args = desktop_args("SERIAL", "title", "1920x1080/160");
        assert!(args.contains(&"--new-display=1920x1080/160".to_string()));
        assert!(args.contains(&"--turn-screen-off".to_string()));
        assert!(!args.contains(&"--no-vd-system-decorations".to_string()));
        assert!(!args.iter().any(|a| a.starts_with("--start-app")));
    }

    #[test]
    fn vd_decorations_option_needs_scrcpy_3_1() {
        assert!(vd_decorations_option_supported(Some("3.1")));
        assert!(vd_decorations_option_supported(Some("5.0")));
        assert!(!vd_decorations_option_supported(Some("3.0")));
        assert!(!vd_decorations_option_supported(None));
    }

    #[test]
    fn builds_virtual_display_app_args() {
        let mode = AppWindowMode {
            virtual_display: true,
            flex_display: true,
            no_decorations: true,
        };
        let args = app_window_args(
            "SERIAL",
            "com.example.app",
            "Zesp Connect - Example",
            mode,
            1024,
        );

        assert!(args.contains(&"--start-app=com.example.app".to_string()));
        assert!(args.contains(&"--new-display".to_string()));
        assert!(args.contains(&"--flex-display".to_string()));
        assert!(args.contains(&"--no-vd-system-decorations".to_string()));
        assert!(!args.contains(&"-m".to_string()));
        // scrcpy 4.x rejects `--adb`; the path travels in the ADB env var instead.
        assert!(!args.contains(&"--adb".to_string()));
    }

    #[test]
    fn omits_flex_display_on_older_scrcpy() {
        let mode = AppWindowMode {
            virtual_display: true,
            ..Default::default()
        };
        let args = app_window_args("SERIAL", "com.example.app", "title", mode, 1024);

        assert!(args.contains(&"--new-display".to_string()));
        assert!(!args.contains(&"--flex-display".to_string()));
        assert!(!args.contains(&"--no-vd-system-decorations".to_string()));

        let size_index = args.iter().position(|a| a == "-m").expect("-m");
        assert_eq!(args[size_index + 1], "1024");
    }

    #[test]
    fn builds_main_display_app_args() {
        let mode = AppWindowMode {
            no_decorations: true,
            ..Default::default()
        };
        let args = app_window_args("SERIAL", "com.example.app", "title", mode, 800);

        assert!(args.contains(&"--start-app=com.example.app".to_string()));
        assert!(!args.contains(&"--new-display".to_string()));
        // Decorations only apply to a virtual display.
        assert!(!args.contains(&"--no-vd-system-decorations".to_string()));

        let size_index = args.iter().position(|a| a == "-m").expect("-m");
        assert_eq!(args[size_index + 1], "800");
    }

    fn value_after<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
        args.iter()
            .position(|arg| arg == flag)
            .map(|index| args[index + 1].as_str())
    }

    #[test]
    fn builds_screen_mirror_args_with_every_option() {
        let opts = MirrorOptions {
            resolution: Some(1080),
            fps: Some(60),
            bitrate: Some(8),
            audio: true,
            borderless: true,
            stay_awake: true,
            always_on_top: true,
            aspect_lock: true,
            background: Some("#112233".to_string()),
            fit: Some("contain".to_string()),
            ..Default::default()
        };

        let args = mirror_args("SERIAL", "Zesp Connect", &opts);

        assert_eq!(value_after(&args, "--serial"), Some("SERIAL"));
        assert_eq!(value_after(&args, "-m"), Some("1080"));
        assert_eq!(value_after(&args, "--max-fps"), Some("60"));
        assert_eq!(value_after(&args, "-b"), Some("8M"));
        assert!(!args.contains(&"--no-audio".to_string()));
        assert!(args.contains(&"--window-borderless".to_string()));
        assert!(args.contains(&"--stay-awake".to_string()));
        assert!(args.contains(&"--always-on-top".to_string()));
        assert!(!args.contains(&"--no-window-aspect-ratio-lock".to_string()));
        assert!(args.contains(&"--background-color=#112233".to_string()));
        assert!(args.contains(&"--render-fit=contain".to_string()));
    }

    #[test]
    fn mirror_defaults_omit_audio_and_unlock_aspect() {
        let args = mirror_args("SERIAL", "t", &MirrorOptions::default());

        assert!(args.contains(&"--no-audio".to_string()));
        // aspect_lock defaults to false, so the lock is disabled.
        assert!(args.contains(&"--no-window-aspect-ratio-lock".to_string()));
        assert!(!args.contains(&"-m".to_string()));
        assert!(!args.contains(&"--window-borderless".to_string()));
    }

    #[test]
    fn mirror_clamps_out_of_range_numbers() {
        let opts = MirrorOptions {
            resolution: Some(99_999),
            fps: Some(0),
            bitrate: Some(9_999),
            ..Default::default()
        };

        let args = mirror_args("s", "t", &opts);
        assert_eq!(value_after(&args, "-m"), Some("4096"));
        assert_eq!(value_after(&args, "--max-fps"), Some("1"));
        assert_eq!(value_after(&args, "-b"), Some("200M"));
    }

    #[test]
    fn mirror_drops_invalid_background_and_fit() {
        let opts = MirrorOptions {
            background: Some("red".to_string()),
            fit: Some("nonsense".to_string()),
            ..Default::default()
        };

        let args = mirror_args("s", "t", &opts);
        assert!(!args.iter().any(|a| a.starts_with("--background-color")));
        assert!(!args.iter().any(|a| a.starts_with("--render-fit")));
    }

    #[test]
    fn camera_defaults_to_back_with_no_audio() {
        let args = camera_args("SERIAL", "t", &MirrorOptions::default());

        assert!(args.contains(&"--video-source=camera".to_string()));
        assert!(args.contains(&"--camera-facing=back".to_string()));
        assert!(args.contains(&"--no-audio".to_string()));
        assert!(!args.contains(&"-m".to_string()));
        assert!(!args.contains(&"--camera-torch".to_string()));
    }

    #[test]
    fn camera_supports_front_torch_zoom_and_audio() {
        let opts = MirrorOptions {
            camera_facing: Some("front".to_string()),
            camera_torch: true,
            camera_zoom: Some(2.5),
            audio: true,
            fps: Some(30),
            bitrate: Some(12),
            ..Default::default()
        };

        let args = camera_args("SERIAL", "t", &opts);

        assert!(args.contains(&"--camera-facing=front".to_string()));
        assert!(args.contains(&"--camera-torch".to_string()));
        assert!(args.contains(&"--camera-zoom=2.5".to_string()));
        assert!(args.contains(&"--audio-source=playback".to_string()));
        assert!(!args.contains(&"--no-audio".to_string()));
        assert_eq!(value_after(&args, "--max-fps"), Some("30"));
        assert_eq!(value_after(&args, "-b"), Some("12M"));
    }

    #[test]
    fn camera_ignores_non_positive_zoom() {
        let opts = MirrorOptions {
            camera_zoom: Some(-1.0),
            ..Default::default()
        };

        let args = camera_args("s", "t", &opts);
        assert!(!args.iter().any(|a| a.starts_with("--camera-zoom")));
    }
}
