//! scrcpy argv building and Android API detection for app windows.
//!
//! No Tauri types here - everything is pure or a thin adb call, so it is unit
//! tested directly. The lifecycle lives in `runtime::mirror`.

use std::time::Duration;

use crate::error::{AppError, Result};
use crate::models::{MirrorOptions, ToolStatus};
use crate::services::{adb, updates};

const API_TIMEOUT: Duration = Duration::from_secs(15);

/// Bounds for the numeric options the UI can send, so a bad value can never
/// reach scrcpy (they are clamped, not rejected, to keep the UI forgiving).
pub const MIN_RESOLUTION: u32 = 240;
pub const MAX_RESOLUTION: u32 = 4096;
pub const MAX_FPS_LIMIT: u32 = 240;
pub const MAX_BITRATE: u32 = 200;

/// `--render-fit` values scrcpy understands.
const FIT_VALUES: &[&str] = &["contain", "cover", "stretch"];

/// Default `-m` cap when an app is mirrored on the main display.
pub const DEFAULT_MAX_SIZE: u32 = 1024;

/// scrcpy's `--new-display` creates a virtual display. On Android 12, 12L and 13
/// the platform SystemUI crashes on it and the phone ends up locked, so those
/// versions (and any unknown version) must mirror the main display instead.
const UNSAFE_VIRTUAL_DISPLAY_APIS: &[u32] = &[31, 32, 33];

/// scrcpy refuses a virtual display before Android 10, so older devices mirror
/// the main display with the app brought to the foreground.
const MIN_VIRTUAL_DISPLAY_API: u32 = 29;

/// scrcpy added `--flex-display` in 4.0; older releases (e.g. 3.3.3) reject it
/// with "unrecognized option", so it must only be sent when supported.
const FLEX_DISPLAY_MIN_VERSION: &str = "4.0";

/// scrcpy added `--no-vd-system-decorations` (server: `vd_system_decorations`)
/// in 3.1. A decorated virtual display gets the launcher, wallpaper, status bar
/// and - in Android 16+ desktop mode - a taskbar. On Android 17 an app
/// cold-started on it while the phone is asleep never draws, leaving only the
/// wallpaper, so app windows always turn the decorations off when supported.
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

/// Whether the installed scrcpy understands `--flex-display` (added in 4.0).
/// An unknown version is treated as unsupported, so we err toward omitting it.
pub fn flex_display_supported(version: Option<&str>) -> bool {
    version
        .map(|version| updates::compare(version, FLEX_DISPLAY_MIN_VERSION).is_ge())
        .unwrap_or(false)
}

/// Whether the installed scrcpy can turn off virtual display decorations.
/// An unknown version is treated as unsupported, so we err toward omitting it.
pub fn vd_decorations_option_supported(version: Option<&str>) -> bool {
    version
        .map(|version| updates::compare(version, VD_DECORATIONS_MIN_VERSION).is_ge())
        .unwrap_or(false)
}

/// Build the argv for an app window. With a virtual display the app gets its own
/// resizable window; otherwise the main display is mirrored with the app in front.
///
/// The adb binary is NOT an argument: scrcpy 4.x removed `--adb` and expects the
/// path in the `ADB` environment variable instead (set by the caller).
pub fn app_window_args(
    serial: &str,
    package: &str,
    window_title: &str,
    use_virtual_display: bool,
    flex_display: bool,
    no_decorations: bool,
    max_size: u32,
) -> Vec<String> {
    let mut args = vec![
        "--serial".to_string(),
        serial.to_string(),
        "--window-title".to_string(),
        window_title.to_string(),
        format!("--start-app={package}"),
    ];

    if use_virtual_display {
        args.push("--new-display".to_string());
        if no_decorations {
            args.push("--no-vd-system-decorations".to_string());
        }
        if flex_display {
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

/// Android 16 (API 36) brought desktop windowing to connected displays: the
/// taskbar, desktop launcher and freeform app windows.
pub const MIN_DESKTOP_API: u32 = 36;

/// The longest side a desktop display may take. Larger (4K/5K) screens are
/// scaled down with the density, so the desktop keeps its layout while the
/// phone's encoder and the link stay within comfortable limits.
const MAX_DESKTOP_SIDE: f64 = 2560.0;

/// Android's baseline density: one dp is one pixel at 160 dpi.
const BASE_DENSITY: f64 = 160.0;

/// Desktop sessions stream a large, text-heavy display; default to a higher
/// bitrate (Mbps) than a phone-sized mirror so text stays crisp.
pub const DESKTOP_BIT_RATE: u32 = 16;

/// Wake the phone (a no-op when it is awake) if it supports desktop mode, in
/// the same round trip as the check. Prints `<sdk> <freeform> <wakefulness>`
/// first, the wakefulness read before waking (`Awake` when the screen was on).
///
/// A virtual display started while the phone sleeps never draws apps that are
/// launched cold, which on a desktop leaves only the wallpaper.
const DESKTOP_PREPARE_COMMAND: &str = "sdk=$(getprop ro.build.version.sdk); \
ff=$(pm has-feature android.software.freeform_window_management); \
w=$(dumpsys power 2>/dev/null | grep -m1 -o 'mWakefulness=[A-Za-z]*'); \
echo \"$sdk $ff ${w#mWakefulness=}\"; \
[ \"$sdk\" -ge 36 ] && [ \"$ff\" = true ] && input keyevent KEYCODE_WAKEUP";

/// The `WxH/dpi` virtual display matching a computer screen of `width`×`height`
/// physical pixels at `scale` (its device-pixel ratio), so the desktop fills the
/// screen in full screen and is laid out like the computer's own UI.
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
    let dpi = (BASE_DENSITY * scale * shrink).round().clamp(120.0, 640.0) as u32;
    format!("{}x{}/{dpi}", align(width), align(height))
}

/// Check that `serial` can run Android's desktop mode and wake it for the
/// session. Returns whether the phone's screen was already on before the wake.
/// Fails with a readable reason when desktop mode cannot run.
pub async fn prepare_desktop(adb_path: &str, serial: &str) -> Result<bool> {
    let output = adb::shell(adb_path, serial, DESKTOP_PREPARE_COMMAND, API_TIMEOUT).await?;
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

/// Probes a desktop session's phone, one line per finding:
/// - whether the lock screen is up (`KeyguardController.mKeyguardShowing`);
/// - `launcherUnfocused` when a secondary-display launcher is the focused app but
///   has no focused window (or only an ANR dialog over it). Launcher3 can drop its
///   focus when the taskbar's all-apps/recents panel closes, and the next key then
///   times out into an "Application Not Responding" dialog that blocks the desktop.
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
    /// Whether the lock screen is showing, when it could be read. While it is,
    /// Android keeps the desktop (launcher, taskbar, app windows) hidden and only
    /// the wallpaper draws.
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

/// Put the phone to sleep (locking it) after a desktop session. The short delay
/// lets scrcpy's cleanup restore the display power first, so it cannot switch the
/// panel back on behind the sleep. A no-op when the phone is already asleep.
const SLEEP_COMMAND: &str = "sleep 0.5; input keyevent KEYCODE_SLEEP";

pub async fn sleep_phone(adb_path: &str, serial: &str) {
    let _ = adb::shell(adb_path, serial, SLEEP_COMMAND, API_TIMEOUT).await;
}

/// Build the argv for a desktop-mode window when it cannot be rendered in-app:
/// a decorated virtual display of `display` (`WxH/dpi`), with the phone's own
/// screen turned off and put back to sleep on close.
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

/// `-m <size>`, clamped to a sane range.
fn push_max_size(args: &mut Vec<String>, opts: &MirrorOptions) {
    if let Some(value) = opts
        .resolution
        .map(|v| v.clamp(MIN_RESOLUTION, MAX_RESOLUTION))
    {
        args.push("-m".to_string());
        args.push(value.to_string());
    }
}

/// `--max-fps <fps>`, clamped to `1..=240`.
fn push_max_fps(args: &mut Vec<String>, opts: &MirrorOptions) {
    if let Some(value) = opts.fps.map(|v| v.clamp(1, MAX_FPS_LIMIT)) {
        args.push("--max-fps".to_string());
        args.push(value.to_string());
    }
}

/// `-b <bitrate>M`, clamped to `1..=200`.
fn push_bitrate(args: &mut Vec<String>, opts: &MirrorOptions) {
    if let Some(value) = opts.bitrate.map(|v| v.clamp(1, MAX_BITRATE)) {
        args.push("-b".to_string());
        args.push(format!("{value}M"));
    }
}

/// Window/window-decoration flags shared by mirror and camera sessions. Values
/// that scrcpy would reject (bad colour, unknown fit mode) are simply dropped.
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
    // Anything other than an explicit "front" is treated as the back camera.
    let facing = match opts.camera_facing.as_deref() {
        Some("front") => "front",
        _ => "back",
    };

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
    if let Some(zoom) = opts
        .camera_zoom
        .filter(|zoom| zoom.is_finite() && *zoom > 0.0)
    {
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
        let args = app_window_args(
            "SERIAL",
            "com.example.app",
            "Zesp Connect - Example",
            true,
            true,
            true,
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
        let args = app_window_args(
            "SERIAL",
            "com.example.app",
            "title",
            true,
            false,
            false,
            1024,
        );

        assert!(args.contains(&"--new-display".to_string()));
        assert!(!args.contains(&"--flex-display".to_string()));
        assert!(!args.contains(&"--no-vd-system-decorations".to_string()));

        let size_index = args.iter().position(|a| a == "-m").expect("-m");
        assert_eq!(args[size_index + 1], "1024");
    }

    #[test]
    fn builds_main_display_app_args() {
        let args = app_window_args(
            "SERIAL",
            "com.example.app",
            "title",
            false,
            false,
            true,
            800,
        );

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
