use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Manager};

use crate::error::{AppError, Result};
use crate::models::{MirrorKind, MirrorSession};
use crate::runtime::controllers::ControllerBridge;
use crate::state::AppState;
use crate::util;

pub const EVENT_CHANGED: &str = "controller-passthrough:changed";
const FILE: &str = "controller_phones.json";

#[derive(Debug, Clone, serde::Serialize)]
struct Changed<'a> {
    serial: &'a str,
    on: bool,
}

/// Phones whose mirror windows take this computer's controllers (the Features page switch).
/// Each phone gets one bridge, on one of its open windows, so input is never sent twice.
pub struct Passthrough {
    path: PathBuf,
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    phones: HashSet<String>,
    /// The session each phone's bridge drives.
    bridges: HashMap<String, (u64, ControllerBridge)>,
}

impl Passthrough {
    pub fn load(app_data_dir: &Path) -> Self {
        let path = app_data_dir.join(FILE);
        let phones = util::read_json(&path).ok().flatten().unwrap_or_default();
        Passthrough {
            path,
            inner: Mutex::new(Inner {
                phones,
                bridges: HashMap::new(),
            }),
        }
    }

    pub fn serials(&self) -> Vec<String> {
        self.inner
            .lock()
            .map(|inner| inner.phones.iter().cloned().collect())
            .unwrap_or_default()
    }
}

pub fn set(app: &AppHandle, serial: &str, on: bool) -> Result<()> {
    let state = app.state::<AppState>();
    let passthrough = state.controller_passthrough();
    {
        let mut inner = passthrough
            .inner
            .lock()
            .map_err(|_| AppError::Other("Controller settings are unavailable".into()))?;
        let changed = if on {
            inner.phones.insert(serial.to_string())
        } else {
            inner.bridges.remove(serial);
            inner.phones.remove(serial)
        };
        if changed {
            util::write_json(&passthrough.path, &inner.phones)?;
        }
    }
    if on {
        attach(app, serial);
    }
    let _ = app.emit(EVENT_CHANGED, Changed { serial, on });
    Ok(())
}

/// A window opened; give it the phone's controllers if they are not already on another window.
pub fn session_started(app: &AppHandle, serial: &str) {
    attach(app, serial);
}

/// A window closed; move its controllers to another open window of the same phone, if any.
pub fn session_ended(app: &AppHandle, id: u64, serial: &str) {
    let state = app.state::<AppState>();
    if let Ok(mut inner) = state.controller_passthrough().inner.lock() {
        if inner
            .bridges
            .get(serial)
            .is_some_and(|(owner, _)| *owner == id)
        {
            inner.bridges.remove(serial);
        }
    }
    attach(app, serial);
}

fn attach(app: &AppHandle, serial: &str) {
    let state = app.state::<AppState>();
    let Ok(mut inner) = state.controller_passthrough().inner.lock() else {
        return;
    };
    if !inner.phones.contains(serial) || inner.bridges.contains_key(serial) {
        return;
    }
    let target = state
        .list_sessions()
        .into_iter()
        .find(|session| takes_controllers(session, serial) && state.is_streaming(session.id))
        .and_then(|session| Some((session.id, state.session_control(session.id)?)));
    if let Some((id, control)) = target {
        let bridge = ControllerBridge::start(app.clone(), control);
        inner.bridges.insert(serial.to_string(), (id, bridge));
    }
}

/// Game Mode has its own controller switch, and a camera window shows no phone screen.
fn takes_controllers(session: &MirrorSession, serial: &str) -> bool {
    session.serial == serial && session.game.is_none() && session.kind != MirrorKind::Camera
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(kind: MirrorKind, game: bool) -> MirrorSession {
        MirrorSession {
            id: 1,
            serial: "phone".into(),
            kind,
            package: None,
            label: None,
            started: 0,
            phone_screen: false,
            flex_display: false,
            display_scale: 1.0,
            camera_controls: false,
            torch: false,
            game: game.then(Default::default),
        }
    }

    #[test]
    fn only_regular_windows_of_the_phone_take_controllers() {
        assert!(takes_controllers(
            &session(MirrorKind::Mirror, false),
            "phone"
        ));
        assert!(takes_controllers(&session(MirrorKind::App, false), "phone"));
        assert!(!takes_controllers(
            &session(MirrorKind::Mirror, false),
            "other"
        ));
        assert!(!takes_controllers(
            &session(MirrorKind::Mirror, true),
            "phone"
        ));
        assert!(!takes_controllers(
            &session(MirrorKind::Camera, false),
            "phone"
        ));
    }
}
