//! A phone's media sessions, read from `dumpsys media_session` and controlled with media key
//! events. Parsing is pure and unit-tested.
//!
//! `cmd media_session monitor <tag>` would be event-driven, but tags are not unique (YouTube runs
//! many sessions under one) and it misses updates, so a phone-side loop prints the filtered dump
//! only when it changes instead.

use std::process::Stdio;
use std::time::Duration;

use crate::error::{AppError, Result};
use crate::models::{MediaKey, NowPlaying};
use crate::services::adb;
use crate::util::process;

/// Printed after each changed dump so the reader knows a whole snapshot has arrived.
pub const END_MARKER: &str = "ZC-NOW-PLAYING-END";
/// The session stack, reduced to the lines [`parse`] reads.
const SESSIONS_COMMAND: &str = "dumpsys media_session \
     | sed -n '/Sessions Stack/,/^Audio playback/p' \
     | grep -E '^ +(package=|state=|metadata:)'";
/// How often the phone-side loop re-reads the sessions.
const WATCH_INTERVAL_SECS: u32 = 2;
const SHELL_TIMEOUT: Duration = Duration::from_secs(10);

/// `PlaybackState.STATE_*` values that mean sound is (about to be) playing.
const PLAYING_STATES: &[u32] = &[3, 4, 5, 6, 8, 9, 10, 11];
/// `PlaybackState.STATE_ERROR`.
const ERROR_STATE: u32 = 7;
/// `PlaybackState.ACTION_SKIP_TO_PREVIOUS` and `ACTION_SKIP_TO_NEXT`.
const ACTION_SKIP_TO_PREVIOUS: u64 = 1 << 4;
const ACTION_SKIP_TO_NEXT: u64 = 1 << 5;

/// One media session as listed in the dump.
#[derive(Debug, Default)]
struct Session {
    package: String,
    state: Option<u32>,
    actions: u64,
    error: Option<String>,
    title: Option<String>,
    artist: Option<String>,
}

impl Session {
    fn is_playing(&self) -> bool {
        self.state
            .is_some_and(|state| PLAYING_STATES.contains(&state))
    }
}

/// The session worth showing: the first playing one with a title, else the first with a title.
/// `app_label` names the app from its package.
pub fn parse(text: &str, app_label: impl Fn(&str) -> String) -> Option<NowPlaying> {
    let sessions: Vec<Session> = parse_sessions(text)
        .into_iter()
        .filter(|session| session.title.is_some())
        .collect();
    let session = sessions
        .iter()
        .find(|session| session.is_playing())
        .or_else(|| sessions.first())?;

    Some(NowPlaying {
        app: app_label(&session.package),
        package: session.package.clone(),
        title: session.title.clone().unwrap_or_default(),
        artist: session.artist.clone(),
        playing: session.is_playing(),
        can_previous: session.actions & ACTION_SKIP_TO_PREVIOUS != 0,
        can_next: session.actions & ACTION_SKIP_TO_NEXT != 0,
        error: session
            .error
            .clone()
            .filter(|_| session.state == Some(ERROR_STATE)),
    })
}

fn parse_sessions(text: &str) -> Vec<Session> {
    let mut sessions: Vec<Session> = Vec::new();
    for line in text.lines().map(str::trim) {
        if let Some(package) = line.strip_prefix("package=") {
            sessions.push(Session {
                package: package.to_string(),
                ..Session::default()
            });
            continue;
        }
        let Some(session) = sessions.last_mut() else {
            continue;
        };
        if let Some(state) = line.strip_prefix("state=") {
            read_state(state, session);
        } else if let Some(metadata) = line.strip_prefix("metadata:") {
            read_metadata(metadata, session);
        }
    }
    sessions
}

/// `PlaybackState {state=PLAYING(3), ..., actions=8615, custom actions=[], ..., error=null}`.
fn read_state(text: &str, session: &mut Session) {
    session.state = text
        .split_once('(')
        .and_then(|(_, rest)| rest.split_once(')'))
        .and_then(|(code, _)| code.parse().ok());
    session.actions = text
        .split_once(", actions=")
        .and_then(|(_, rest)| rest.split(',').next())
        .and_then(|actions| actions.trim().parse().ok())
        .unwrap_or(0);
    session.error = text
        .split_once(", error=")
        .map(|(_, rest)| rest.trim_end_matches('}').trim())
        .and_then(present);
}

/// ` size=6, description=<title>, <subtitle>, <description>`. Split from the right, since the
/// title is free text that may hold commas itself.
fn read_metadata(text: &str, session: &mut Session) {
    let Some((_, description)) = text.split_once("description=") else {
        return;
    };
    let mut parts = description.rsplitn(3, ", ");
    let _album = parts.next();
    let artist = parts.next();
    let title = parts.next();
    session.title = title.and_then(present);
    session.artist = artist.and_then(present);
}

/// A dump field, or `None` when the phone printed `null` or nothing.
fn present(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty() && value != "null").then(|| value.to_string())
}

/// The phone-side loop: print the sessions and [`END_MARKER`] now and whenever they change.
fn watch_script() -> String {
    format!(
        "p=x; while :; do s=$({SESSIONS_COMMAND}); \
         if [ \"$s\" != \"$p\" ]; then echo \"$s\"; echo {END_MARKER}; p=$s; fi; \
         sleep {WATCH_INTERVAL_SECS}; done"
    )
}

/// Start the phone-side loop; dropping the child ends it on the phone too.
pub fn spawn_watch(adb_path: &str, serial: &str) -> Result<tokio::process::Child> {
    let args = vec![
        "-s".to_string(),
        serial.to_string(),
        "shell".to_string(),
        watch_script(),
    ];
    process::base_command(adb_path, &args)
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| AppError::CommandFailed(format!("Could not start adb shell: {e}")))
}

/// The sessions dump once, for [`parse`].
pub async fn read_sessions(adb_path: &str, serial: &str) -> Result<String> {
    let output = adb::shell(adb_path, serial, SESSIONS_COMMAND, SHELL_TIMEOUT).await?;
    // grep exits 1 when there are no sessions, which is not a failure here.
    if !output.success && !output.stderr.trim().is_empty() {
        return Err(AppError::CommandFailed(output.text()));
    }
    Ok(output.stdout)
}

/// Press a media button; Android routes it to the session that owns media buttons.
pub async fn send_key(adb_path: &str, serial: &str, key: MediaKey) -> Result<()> {
    let keycode = match key {
        MediaKey::PlayPause => "KEYCODE_MEDIA_PLAY_PAUSE",
        MediaKey::Next => "KEYCODE_MEDIA_NEXT",
        MediaKey::Previous => "KEYCODE_MEDIA_PREVIOUS",
    };
    let command = format!("input keyevent {keycode}");
    let output = adb::shell(adb_path, serial, &command, SHELL_TIMEOUT).await?;
    if !output.success {
        return Err(AppError::CommandFailed(output.text()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const YOUTUBE_PLAYING: &str = "\
      package=com.google.android.youtube
      state=PlaybackState {state=PLAYING(3), position=161, buffered position=0, speed=1.0, updated=462177104, actions=8615, custom actions=[], active item id=-1, error=null}
      metadata: size=6, description=Aayee Pardes Se Song | Falguni Pathak, Songs 4u, Songs 4u
";
    const GAME_SESSION: &str = "\
      package=com.example.game
      state=PlaybackState {state=PAUSED(2), position=0, buffered position=85, speed=0.0, updated=463296797, actions=7339997, custom actions=[], active item id=0, error=null}
      metadata: size=2, description=null, null, null
";
    const MUSIC_PAUSED: &str = "\
      package=com.example.music
      state=PlaybackState {state=PAUSED(2), position=0, buffered position=0, speed=0.0, updated=1, actions=48, custom actions=[], active item id=0, error=null}
      metadata: size=4, description=Song, With, Commas, The Artist, The Album
";

    fn label(package: &str) -> String {
        format!("label:{package}")
    }

    #[test]
    fn reads_a_playing_session() {
        let playing = parse(YOUTUBE_PLAYING, label).unwrap();
        assert_eq!(playing.package, "com.google.android.youtube");
        assert_eq!(playing.app, "label:com.google.android.youtube");
        assert_eq!(playing.title, "Aayee Pardes Se Song | Falguni Pathak");
        assert_eq!(playing.artist.as_deref(), Some("Songs 4u"));
        assert!(playing.playing);
        assert!(playing.can_next);
        assert!(!playing.can_previous);
        assert_eq!(playing.error, None);
    }

    #[test]
    fn keeps_commas_in_the_title() {
        let paused = parse(MUSIC_PAUSED, label).unwrap();
        assert_eq!(paused.title, "Song, With, Commas");
        assert_eq!(paused.artist.as_deref(), Some("The Artist"));
        assert!(!paused.playing);
        assert!(paused.can_previous && paused.can_next);
    }

    #[test]
    fn skips_sessions_without_a_title() {
        let text = format!("{GAME_SESSION}{MUSIC_PAUSED}");
        assert_eq!(parse(&text, label).unwrap().package, "com.example.music");
        assert_eq!(parse(GAME_SESSION, label), None);
    }

    #[test]
    fn prefers_a_playing_session_over_one_higher_in_the_stack() {
        let text = format!("{MUSIC_PAUSED}{YOUTUBE_PLAYING}");
        assert_eq!(
            parse(&text, label).unwrap().package,
            "com.google.android.youtube"
        );
    }

    #[test]
    fn reports_the_apps_error() {
        let text = "\
      package=com.google.android.youtube
      state=PlaybackState {state=ERROR(7), position=0, buffered position=0, speed=1.0, updated=1, actions=0, custom actions=[], active item id=-1, error=Required premium account to perform action in background}
      metadata: size=6, description=Some video, Some channel, null
";
        let failed = parse(text, label).unwrap();
        assert_eq!(
            failed.error.as_deref(),
            Some("Required premium account to perform action in background")
        );
        assert!(!failed.playing);
        assert_eq!(failed.artist.as_deref(), Some("Some channel"));
    }

    #[test]
    fn nothing_plays_without_sessions() {
        assert_eq!(parse("", label), None);
        let no_state = "      package=com.example\n      state=null\n      metadata: null\n";
        assert_eq!(parse(no_state, label), None);
    }
}
