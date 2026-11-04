//! Bootstrap helpers for running scrcpy's *standalone server* ourselves: we push `scrcpy-server`
//! to the device and read the video socket directly, configured to emit a plain H.264 stream (the
//! metadata preamble off, the 12-byte frame header on) so no version-specific handshake is needed.
//! scrcpy 4.0 renamed the metadata option, so the name is chosen from the detected version.

use std::path::{Path, PathBuf};

use crate::error::{AppError, Result};
use crate::models::MirrorKind;
use crate::services::{scrcpy, updates};

/// scrcpy 4.0 renamed `send_codec_meta` to `send_stream_meta`.
const STREAM_META_MIN_VERSION: &str = "4.0";

/// The oldest scrcpy whose standalone-server options we target.
pub const MIN_VERSION: &str = "3.0";

/// How long the phone may idle before sleeping during a desktop session; scrcpy's cleanup restores the user's timeout.
const DESKTOP_SCREEN_OFF_TIMEOUT_MS: u32 = 24 * 60 * 60 * 1000;

/// The server artifact shipped next to the scrcpy binary.
const SERVER_FILE: &str = "scrcpy-server";

/// A per-session server path on the device.
pub fn remote_jar(scid: &str) -> String {
    format!("/data/local/tmp/zesp-scrcpy-server-{scid}.jar")
}

/// A random 31-bit socket id, formatted like scrcpy's own (`%08x`).
pub fn scid() -> String {
    let bytes = uuid::Uuid::new_v4().into_bytes();
    let value = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) & 0x7fff_ffff;
    format!("{value:08x}")
}

/// The abstract socket the server exposes on the device.
fn socket_name(scid: &str) -> String {
    format!("scrcpy_{scid}")
}

/// The device side of the `adb reverse` tunnel to the server's socket.
pub fn tunnel_name(scid: &str) -> String {
    format!("localabstract:{}", socket_name(scid))
}

/// Whether we know how to drive this scrcpy version's server.
pub fn supported(version: Option<&str>) -> bool {
    version
        .map(|value| updates::compare(value, MIN_VERSION).is_ge())
        .unwrap_or(false)
}

/// Locate `scrcpy-server`, which scrcpy ships beside its own binary.
pub fn server_path(scrcpy_bin: &str) -> Result<PathBuf> {
    let dir = Path::new(scrcpy_bin)
        .parent()
        .ok_or_else(|| AppError::Other("Could not locate the scrcpy folder".into()))?;

    let exact = dir.join(SERVER_FILE);
    if exact.is_file() {
        return Ok(exact);
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.file_name().to_string_lossy().starts_with(SERVER_FILE)
                && entry.path().is_file()
            {
                return Ok(entry.path());
            }
        }
    }
    Err(AppError::Other(format!(
        "scrcpy-server was not found next to {scrcpy_bin}"
    )))
}

/// Everything the server needs to know about one session.
#[derive(Debug, Clone)]
pub struct ServerOptions {
    pub kind: MirrorKind,
    /// `""` reuses the main display's size, `WxH/dpi` sizes it (desktop), `None` mirrors an existing display.
    pub new_display: Option<String>,
    /// Let the client resize the virtual display (`RESIZE_DISPLAY`); scrcpy 4.0+.
    pub flex_display: bool,
    pub max_size: u32,
    pub max_fps: Option<u32>,
    pub bit_rate: Option<u32>,
    pub camera_facing: Option<String>,
    pub camera_zoom: Option<f64>,
    pub camera_torch: bool,
    pub stay_awake: bool,
}

fn meta_option(version: &str) -> &'static str {
    if updates::compare(version, STREAM_META_MIN_VERSION).is_ge() {
        "send_stream_meta"
    } else {
        "send_codec_meta"
    }
}

/// The `key=value` arguments passed to `com.genymobile.scrcpy.Server`.
pub fn options_args(version: &str, scid: &str, opts: &ServerOptions) -> Vec<String> {
    let mut args = vec![
        format!("scid={scid}"),
        "audio=false".to_string(),
        "control=true".to_string(),
        "video_codec=h264".to_string(),
        "send_device_meta=false".to_string(),
        "send_dummy_byte=false".to_string(),
        "send_frame_meta=true".to_string(),
        format!("{}=false", meta_option(version)),
        format!("max_size={}", opts.max_size),
        "clipboard_autosync=false".to_string(),
    ];

    // scrcpy applies `stay_awake` only from its CleanUp thread (needs `cleanup=true`); our teardown drops the server.
    args.push(format!("cleanup={}", opts.stay_awake));

    if let Some(fps) = opts.max_fps {
        args.push(format!("max_fps={fps}"));
    }
    if let Some(mbps) = opts.bit_rate {
        args.push(format!("video_bit_rate={}", mbps.saturating_mul(1_000_000)));
    }

    if opts.kind == MirrorKind::Camera {
        let facing = scrcpy::camera_facing(opts.camera_facing.as_deref());
        args.push("video_source=camera".to_string());
        args.push(format!("camera_facing={facing}"));
        if let Some(zoom) = scrcpy::camera_zoom(opts.camera_zoom) {
            args.push(format!("camera_zoom={zoom}"));
        }
        if opts.camera_torch {
            args.push("camera_torch=true".to_string());
        }
    }

    if let Some(size) = opts.new_display.as_deref() {
        // `new_display` takes a size spec (or empty for the main display's size), not a boolean.
        args.push(format!("new_display={size}"));
        if opts.flex_display {
            args.push("flex_display=true".to_string());
        }
        if opts.kind == MirrorKind::Desktop {
            // Desktop mode is the decorations; typing comes from the computer, so hide the phone IME.
            // Sleep-on-close is decided by the session's phone-screen toggle (see `finish_desktop`).
            args.push("display_ime_policy=hide".to_string());
            args.push(format!(
                "screen_off_timeout={DESKTOP_SCREEN_OFF_TIMEOUT_MS}"
            ));
        } else if scrcpy::vd_decorations_option_supported(Some(version)) {
            // Without decorations only the app is shown, so a cold start draws even while the phone sleeps.
            args.push("vd_system_decorations=false".to_string());
        }
    }

    if opts.stay_awake {
        args.push("stay_awake=true".to_string());
    }

    args
}

/// Audio-only server: the phone's whole output as raw PCM ([`PCM_RATE`] Hz, [`PCM_CHANNELS`]
/// channels, s16le). The `output` source silences the phone's own speaker while it runs.
/// The stream meta stays on so the first 4 bytes tell whether capture started (see [`AudioStart`]).
pub fn audio_args(version: &str, scid: &str) -> Vec<String> {
    vec![
        format!("scid={scid}"),
        "video=false".to_string(),
        "control=false".to_string(),
        "audio_codec=raw".to_string(),
        "audio_source=output".to_string(),
        "send_device_meta=false".to_string(),
        "send_dummy_byte=false".to_string(),
        "send_frame_meta=true".to_string(),
        format!("{}=true", meta_option(version)),
        "cleanup=false".to_string(),
    ]
}

/// Audio-only server for a recording: AAC straight from the phone's encoder, captured from app
/// playback and duplicated, so the phone keeps playing its sound (Android 13+).
pub fn recording_audio_args(version: &str, scid: &str) -> Vec<String> {
    vec![
        format!("scid={scid}"),
        "video=false".to_string(),
        "control=false".to_string(),
        "audio_codec=aac".to_string(),
        "audio_source=playback".to_string(),
        "audio_dup=true".to_string(),
        "send_device_meta=false".to_string(),
        "send_dummy_byte=false".to_string(),
        "send_frame_meta=true".to_string(),
        format!("{}=true", meta_option(version)),
        "cleanup=false".to_string(),
    ]
}

/// Control-only server that reports every phone clipboard change as a device message.
pub fn clipboard_args(scid: &str) -> Vec<String> {
    vec![
        format!("scid={scid}"),
        "video=false".to_string(),
        "audio=false".to_string(),
        "clipboard_autosync=true".to_string(),
        "send_device_meta=false".to_string(),
        "send_dummy_byte=false".to_string(),
        "cleanup=false".to_string(),
    ]
}

pub const PCM_RATE: u32 = 48_000;
pub const PCM_CHANNELS: u16 = 2;
pub const RAW_CODEC_ID: u32 = 0x0072_6177;
pub const AAC_CODEC_ID: u32 = 0x0061_6163;

/// What the 4-byte codec id that opens an audio stream says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioStart {
    Streaming,
    /// The phone could not capture (before Android 11, or Android 11 while locked).
    Unavailable,
    Failed,
}

/// Read the codec id against the codec the server was asked for.
pub fn audio_start(codec_id: u32, expected: u32) -> AudioStart {
    match codec_id {
        0 => AudioStart::Unavailable,
        id if id == expected => AudioStart::Streaming,
        _ => AudioStart::Failed,
    }
}

/// The full `adb` argv that starts the server with the given `key=value` options.
pub fn launch_args(
    serial: &str,
    remote_jar: &str,
    version: &str,
    options: Vec<String>,
) -> Vec<String> {
    let mut args = vec![
        "-s".to_string(),
        serial.to_string(),
        "shell".to_string(),
        format!("CLASSPATH={remote_jar}"),
        "app_process".to_string(),
        "/".to_string(),
        "com.genymobile.scrcpy.Server".to_string(),
        version.to_string(),
    ];
    args.extend(options);
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(kind: MirrorKind) -> ServerOptions {
        ServerOptions {
            kind,
            new_display: None,
            flex_display: false,
            max_size: 1024,
            max_fps: None,
            bit_rate: None,
            camera_facing: None,
            camera_zoom: None,
            camera_torch: false,
            stay_awake: false,
        }
    }

    #[test]
    fn chooses_the_meta_option_by_version() {
        let modern = options_args("4.0", "abcd1234", &options(MirrorKind::Mirror));
        assert!(modern.contains(&"send_stream_meta=false".to_string()));
        assert!(!modern.iter().any(|a| a.starts_with("send_codec_meta")));

        let legacy = options_args("3.3.3", "abcd1234", &options(MirrorKind::Mirror));
        assert!(legacy.contains(&"send_codec_meta=false".to_string()));
        assert!(!legacy.iter().any(|a| a.starts_with("send_stream_meta")));
    }

    #[test]
    fn disables_the_preamble_but_keeps_frame_headers() {
        let args = options_args("4.0", "abcd1234", &options(MirrorKind::Mirror));
        assert!(args.contains(&"send_device_meta=false".to_string()));
        assert!(args.contains(&"send_frame_meta=true".to_string()));
        assert!(args.contains(&"scid=abcd1234".to_string()));
        assert!(args.contains(&"video_codec=h264".to_string()));
        assert!(args.contains(&"audio=false".to_string()));
    }

    #[test]
    fn virtual_display_uses_an_empty_size_spec() {
        let mut opts = options(MirrorKind::App);
        opts.new_display = Some(String::new());

        let args = options_args("4.0", "s", &opts);
        // A boolean here makes scrcpy's Options.parse throw.
        assert!(args.contains(&"new_display=".to_string()));
        assert!(!args.contains(&"new_display=true".to_string()));
        assert!(args.contains(&"vd_system_decorations=false".to_string()));

        // scrcpy 3.0 rejects the option as unknown.
        let legacy = options_args("3.0", "s", &opts);
        assert!(!legacy
            .iter()
            .any(|a| a.starts_with("vd_system_decorations")));
    }

    #[test]
    fn desktop_keeps_decorations_and_manages_phone_power() {
        let mut opts = options(MirrorKind::Desktop);
        opts.new_display = Some("2560x1664/320".to_string());

        let args = options_args("5.0", "s", &opts);
        assert!(args.contains(&"new_display=2560x1664/320".to_string()));
        assert!(!args.iter().any(|a| a.starts_with("vd_system_decorations")));
        assert!(args.contains(&"display_ime_policy=hide".to_string()));
        assert!(!args.iter().any(|a| a.starts_with("power_off_on_close")));
        assert!(args.iter().any(|a| a.starts_with("screen_off_timeout=")));
    }

    #[test]
    fn flex_display_rides_on_the_virtual_display() {
        let mut opts = options(MirrorKind::App);
        opts.new_display = Some("820x1552/320".to_string());
        assert!(!options_args("4.0", "s", &opts)
            .iter()
            .any(|a| a.starts_with("flex_display")));

        opts.flex_display = true;
        assert!(options_args("4.0", "s", &opts).contains(&"flex_display=true".to_string()));
    }

    #[test]
    fn main_display_app_omits_the_virtual_display() {
        let args = options_args("4.0", "s", &options(MirrorKind::ScreenApp));

        assert!(!args.iter().any(|a| a.starts_with("new_display")));
        assert!(!args.iter().any(|a| a.starts_with("vd_system_decorations")));
    }

    #[test]
    fn always_enables_the_control_socket() {
        let args = options_args("4.0", "s", &options(MirrorKind::Mirror));
        assert!(args.contains(&"control=true".to_string()));
    }

    #[test]
    fn enables_cleanup_so_stay_awake_takes_effect() {
        let mut awake = options(MirrorKind::Mirror);
        awake.stay_awake = true;
        let args = options_args("4.0", "s", &awake);
        assert!(args.contains(&"cleanup=true".to_string()));
        assert!(args.contains(&"stay_awake=true".to_string()));

        let asleep = options_args("4.0", "s", &options(MirrorKind::Mirror));
        assert!(asleep.contains(&"cleanup=false".to_string()));
        assert!(!asleep.iter().any(|a| a.starts_with("stay_awake")));
    }

    #[test]
    fn camera_sets_source_facing_torch_and_zoom() {
        let mut opts = options(MirrorKind::Camera);
        opts.camera_facing = Some("front".to_string());
        opts.camera_torch = true;
        opts.camera_zoom = Some(2.0);

        let args = options_args("4.0", "s", &opts);
        assert!(args.contains(&"video_source=camera".to_string()));
        assert!(args.contains(&"camera_facing=front".to_string()));
        assert!(args.contains(&"camera_torch=true".to_string()));
        assert!(args.contains(&"camera_zoom=2".to_string()));
    }

    #[test]
    fn camera_ignores_non_positive_zoom() {
        let mut opts = options(MirrorKind::Camera);
        opts.camera_zoom = Some(0.0);

        let args = options_args("4.0", "s", &opts);
        assert!(!args.iter().any(|a| a.starts_with("camera_zoom")));
    }

    #[test]
    fn bit_rate_is_sent_in_bits_per_second() {
        let mut opts = options(MirrorKind::Mirror);
        opts.bit_rate = Some(8);

        let args = options_args("4.0", "s", &opts);
        assert!(args.contains(&"video_bit_rate=8000000".to_string()));
    }

    #[test]
    fn launch_argv_targets_the_server_class() {
        let args = launch_args(
            "SERIAL",
            "/data/local/tmp/x.jar",
            "4.0",
            options_args("4.0", "abcd1234", &options(MirrorKind::Mirror)),
        );

        assert_eq!(args[0], "-s");
        assert_eq!(args[1], "SERIAL");
        assert_eq!(args[2], "shell");
        assert_eq!(args[3], "CLASSPATH=/data/local/tmp/x.jar");
        assert_eq!(args[4], "app_process");
        assert_eq!(args[5], "/");
        assert_eq!(args[6], "com.genymobile.scrcpy.Server");
        assert_eq!(args[7], "4.0");
        assert!(args.contains(&"scid=abcd1234".to_string()));
    }

    #[test]
    fn audio_server_streams_raw_output_with_its_codec_header() {
        let args = audio_args("4.1", "abcd1234");
        for expected in [
            "video=false",
            "control=false",
            "audio_codec=raw",
            "audio_source=output",
            "send_stream_meta=true",
            "send_frame_meta=true",
        ] {
            assert!(args.contains(&expected.to_string()), "missing {expected}");
        }
        assert!(audio_args("3.3", "s").contains(&"send_codec_meta=true".to_string()));
    }

    #[test]
    fn clipboard_server_is_control_only_with_autosync() {
        let args = clipboard_args("abcd1234");
        assert!(args.contains(&"video=false".to_string()));
        assert!(args.contains(&"audio=false".to_string()));
        assert!(args.contains(&"clipboard_autosync=true".to_string()));
        assert!(!args.iter().any(|a| a.starts_with("control=")));
    }

    #[test]
    fn reads_the_audio_codec_header() {
        assert_eq!(
            audio_start(0x0072_6177, RAW_CODEC_ID),
            AudioStart::Streaming
        );
        assert_eq!(audio_start(0, RAW_CODEC_ID), AudioStart::Unavailable);
        assert_eq!(audio_start(1, RAW_CODEC_ID), AudioStart::Failed);
    }

    #[test]
    fn recording_audio_start_rejects_raw_and_unavailable() {
        assert_eq!(audio_start(RAW_CODEC_ID, AAC_CODEC_ID), AudioStart::Failed);
        assert_eq!(audio_start(0, AAC_CODEC_ID), AudioStart::Unavailable);
        assert_eq!(
            audio_start(AAC_CODEC_ID, AAC_CODEC_ID),
            AudioStart::Streaming
        );
    }

    #[test]
    fn recording_audio_args_use_aac_playback_dup() {
        let args = recording_audio_args("4.1", "abcd1234");
        assert!(args.contains(&"audio_codec=aac".to_string()));
        assert!(args.contains(&"audio_source=playback".to_string()));
        assert!(args.contains(&"audio_dup=true".to_string()));
        assert!(!args.contains(&"audio_source=output".to_string()));
    }

    #[test]
    fn supports_scrcpy_three_and_newer() {
        assert!(supported(Some("3.0")));
        assert!(supported(Some("4.0")));
        assert!(!supported(Some("2.7")));
        assert!(!supported(None));
    }

    #[test]
    fn scid_is_eight_hex_digits() {
        let value = scid();
        assert_eq!(value.len(), 8);
        assert!(value.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn socket_name_is_prefixed() {
        assert_eq!(socket_name("abcd1234"), "scrcpy_abcd1234");
        assert_eq!(tunnel_name("abcd1234"), "localabstract:scrcpy_abcd1234");
    }
}
