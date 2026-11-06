use std::collections::HashMap;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Mutex;

use crate::error::{AppError, Result};
use crate::models::{GameOptions, GameOrientation};
use crate::runtime::controllers::ControllerBridge;
use crate::runtime::mirror_session::ControlChannel;
use crate::runtime::phone_audio::{self, AudioSettings};
use crate::services::{adb, control, scrcpy, toggles};
use crate::state::AppState;

pub const EVENT_UPDATED: &str = "game:updated";
pub const EVENT_WARNING: &str = "game:warning";
/// How long a new session may take to stream before its phone changes are given up.
const STREAM_WAIT: Duration = Duration::from_secs(15);
const STREAM_POLL: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, serde::Serialize)]
struct Updated<'a> {
    id: u64,
    options: &'a GameOptions,
}

#[derive(Debug, Clone, serde::Serialize)]
struct Warning {
    id: u64,
    message: String,
}

/// What Game Mode changed on the phone, so stopping can put it back.
#[derive(Default)]
struct Restore {
    /// Game Mode turned Do Not Disturb on (it was off).
    dnd: bool,
    /// The rotation state from before Game Mode locked it.
    rotation: Option<String>,
}

struct GameSession {
    serial: String,
    control: ControlChannel,
    /// The latest toggles asked for.
    requested: GameOptions,
    /// What is applied on the phone; starts empty and catches up once the session streams.
    applied: GameOptions,
    restore: Restore,
    controllers: Option<ControllerBridge>,
}

/// Every running Game Mode session. One lock covers all of them, so changes to a session
/// (start, live toggles, stop) apply in order.
#[derive(Default)]
pub struct GameSessions {
    sessions: Mutex<HashMap<u64, GameSession>>,
}

/// Track a new Game Mode session and apply `options` once its video is live.
pub async fn begin(
    app: &AppHandle,
    id: u64,
    serial: &str,
    control: ControlChannel,
    options: GameOptions,
) {
    app.state::<AppState>()
        .games()
        .sessions
        .lock()
        .await
        .insert(
            id,
            GameSession {
                serial: serial.to_string(),
                control,
                requested: options,
                applied: GameOptions::default(),
                restore: Restore::default(),
                controllers: None,
            },
        );

    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        // The server takes control messages only once capture is up.
        if wait_for_stream(&app, id).await {
            apply(&app, id).await;
        }
    });
}

/// Change a running session's toggles; before its video is live they wait for [`begin`].
pub async fn update(app: &AppHandle, id: u64, options: GameOptions) -> Result<()> {
    let state = app.state::<AppState>();
    state
        .games()
        .sessions
        .lock()
        .await
        .get_mut(&id)
        .ok_or_else(|| AppError::Invalid("This Game Mode session has ended".into()))?
        .requested = options;
    if state.is_streaming(id) {
        apply(app, id).await;
    }
    Ok(())
}

/// Put the phone back as Game Mode found it; the server's cleanup turns a switched-off screen back on.
pub async fn finish(app: &AppHandle, id: u64) {
    let Some(mut session) = app
        .state::<AppState>()
        .games()
        .sessions
        .lock()
        .await
        .remove(&id)
    else {
        return;
    };
    session.controllers = None;
    let tools = app.state::<AppState>().tools();
    if session.restore.dnd {
        let _ = toggles::set_toggle(&tools, &session.serial, "dnd", false).await;
    }
    if let (Some(state), Ok(adb_bin)) = (session.restore.rotation.take(), adb::adb_path(&tools)) {
        let _ = scrcpy::set_user_rotation(&adb_bin, &session.serial, &state).await;
    }
    if session.applied.sound_on_computer {
        phone_audio::stop(app, &session.serial);
    }
}

/// Wait until session `id` streams; false if it ends or takes too long.
pub(crate) async fn wait_for_stream(app: &AppHandle, id: u64) -> bool {
    let deadline = tokio::time::Instant::now() + STREAM_WAIT;
    loop {
        let state = app.state::<AppState>();
        if state.session_control(id).is_none() || tokio::time::Instant::now() >= deadline {
            return false;
        }
        if state.is_streaming(id) {
            return true;
        }
        tokio::time::sleep(STREAM_POLL).await;
    }
}

/// Bring the phone and window from what is applied to what was requested, announcing the result.
/// A step that fails is reported (`EVENT_WARNING`) and left as it was, so the next change retries it.
async fn apply(app: &AppHandle, id: u64) {
    let state = app.state::<AppState>();
    let mut sessions = state.games().sessions.lock().await;
    let Some(session) = sessions.get_mut(&id) else {
        return;
    };
    let next = session.requested.clone();

    let mut failures = Vec::new();
    let mut applied = next.clone();
    let current = session.applied.clone();

    if next.do_not_disturb != current.do_not_disturb {
        if let Err(error) = set_dnd(app, session, next.do_not_disturb).await {
            applied.do_not_disturb = current.do_not_disturb;
            failures.push(format!("Do Not Disturb: {error}"));
        }
    }
    if next.orientation != current.orientation {
        if let Err(error) = set_orientation(app, session, next.orientation).await {
            applied.orientation = current.orientation;
            failures.push(format!("Orientation: {error}"));
        }
    }
    if next.screen_off != current.screen_off {
        if let Err(error) = session
            .control
            .send(control::set_display_power(!next.screen_off))
        {
            applied.screen_off = current.screen_off;
            failures.push(format!("Phone screen: {error}"));
        }
    }
    if audio_changed(&current, &next) {
        if let Err(error) = set_audio(app, &session.serial, &next).await {
            applied.sound_on_computer = false;
            failures.push(format!("Sound: {error}"));
        }
    }
    if next.controllers != session.controllers.is_some() {
        session.controllers = next
            .controllers
            .then(|| ControllerBridge::start(app.clone(), session.control.clone()));
    }

    session.applied = applied.clone();
    drop(sessions);

    state.set_session_game(id, applied.clone());
    let _ = app.emit(
        EVENT_UPDATED,
        Updated {
            id,
            options: &applied,
        },
    );
    if !failures.is_empty() {
        let _ = app.emit(
            EVENT_WARNING,
            Warning {
                id,
                message: failures.join(". "),
            },
        );
    }
}

/// Turn DND on only when it was off, so stopping never clears a DND the user had set.
async fn set_dnd(app: &AppHandle, session: &mut GameSession, on: bool) -> Result<()> {
    let tools = app.state::<AppState>().tools();
    if on {
        if !toggles::dnd_on(&tools, &session.serial).await? {
            toggles::set_toggle(&tools, &session.serial, "dnd", true).await?;
            session.restore.dnd = true;
        }
    } else if session.restore.dnd {
        toggles::set_toggle(&tools, &session.serial, "dnd", false).await?;
        session.restore.dnd = false;
    }
    Ok(())
}

async fn set_orientation(
    app: &AppHandle,
    session: &mut GameSession,
    orientation: GameOrientation,
) -> Result<()> {
    let adb_bin = adb::adb_path(&app.state::<AppState>().tools())?;
    let target = match orientation {
        GameOrientation::Portrait => "lock 0",
        GameOrientation::Landscape => "lock 1",
        GameOrientation::Auto => {
            if let Some(previous) = session.restore.rotation.take() {
                scrcpy::set_user_rotation(&adb_bin, &session.serial, &previous).await?;
            }
            return Ok(());
        }
    };
    if session.restore.rotation.is_none() {
        session.restore.rotation = Some(
            scrcpy::user_rotation(&adb_bin, &session.serial)
                .await
                .ok_or_else(|| {
                    AppError::CommandFailed("This phone cannot lock its rotation".into())
                })?,
        );
    }
    scrcpy::set_user_rotation(&adb_bin, &session.serial, target).await
}

fn audio_settings(options: &GameOptions) -> AudioSettings {
    AudioSettings {
        keep_on_phone: options.sound_on_phone,
        low_latency: options.low_latency_audio,
        device: options.audio_device.clone(),
    }
}

fn audio_changed(current: &GameOptions, next: &GameOptions) -> bool {
    current.sound_on_computer != next.sound_on_computer
        || (next.sound_on_computer && audio_settings(current) != audio_settings(next))
}

async fn set_audio(app: &AppHandle, serial: &str, options: &GameOptions) -> Result<()> {
    if options.sound_on_computer {
        phone_audio::restart_with(app, serial, &audio_settings(options)).await
    } else {
        phone_audio::stop(app, serial);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sound(on: bool) -> GameOptions {
        GameOptions {
            sound_on_computer: on,
            ..GameOptions::default()
        }
    }

    #[test]
    fn audio_restarts_only_for_changes_that_matter() {
        assert!(audio_changed(&sound(false), &sound(true)));
        assert!(audio_changed(&sound(true), &sound(false)));
        assert!(!audio_changed(&sound(true), &sound(true)));

        let low = GameOptions {
            low_latency_audio: true,
            ..sound(true)
        };
        assert!(audio_changed(&sound(true), &low));

        // Settings only matter while the sound plays on the computer.
        let off_low = GameOptions {
            low_latency_audio: true,
            ..sound(false)
        };
        assert!(!audio_changed(&sound(false), &off_low));
    }
}
