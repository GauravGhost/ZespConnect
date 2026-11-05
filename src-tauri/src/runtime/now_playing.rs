use std::collections::HashMap;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::ChildStdout;

use crate::error::{AppError, Result};
use crate::models::{MediaKey, NowPlaying};
use crate::services::{adb, apps, media_session};
use crate::state::AppState;

pub const EVENT_CHANGED: &str = "now-playing:changed";
/// Time for an app to update its session after a media button, before reading it again.
const KEY_SETTLE: Duration = Duration::from_millis(400);

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Changed<'a> {
    serial: &'a str,
    now_playing: Option<&'a NowPlaying>,
}

fn announce(app: &AppHandle, serial: &str, now_playing: Option<&NowPlaying>) {
    let _ = app.emit(
        EVENT_CHANGED,
        Changed {
            serial,
            now_playing,
        },
    );
}

/// Watch `serial` (or nothing) from now on. Always starts a fresh watch, so the window gets a
/// first snapshot even when it reloaded while an older watch was running.
pub fn watch(app: &AppHandle, serial: Option<&str>) -> Result<()> {
    let state = app.state::<AppState>();
    let watches = state.now_playing();
    for watched in watches.serials() {
        watches.stop(&watched);
    }
    let Some(serial) = serial else {
        return Ok(());
    };

    let adb_path = adb::adb_path(&state.tools())?;
    let mut child = media_session::spawn_watch(&adb_path, serial)?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| AppError::Other("no stdout from adb shell".into()))?;
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
    let Some(watch_id) = watches.insert(serial, stop_tx) else {
        return Ok(());
    };

    let labels = apps::cached_labels(&state.app_data_dir(), serial);
    let handle = app.clone();
    let serial = serial.to_string();
    tauri::async_runtime::spawn(async move {
        tokio::select! {
            _ = forward(&handle, &serial, stdout, &labels) => {}
            _ = stop_rx => {}
        }
        // Dropping the child kills adb, which ends the loop on the phone.
        drop(child);
        handle
            .state::<AppState>()
            .now_playing()
            .finished(&serial, watch_id);
    });
    Ok(())
}

/// Announce each snapshot the phone sends that differs from the last one.
async fn forward(
    app: &AppHandle,
    serial: &str,
    stdout: ChildStdout,
    labels: &HashMap<String, String>,
) {
    let mut lines = BufReader::new(stdout).lines();
    let mut snapshot = String::new();
    let mut last: Option<Option<NowPlaying>> = None;
    while let Ok(Some(line)) = lines.next_line().await {
        if line.trim() != media_session::END_MARKER {
            snapshot.push_str(&line);
            snapshot.push('\n');
            continue;
        }
        let now_playing =
            media_session::parse(&snapshot, |package| apps::display_label(labels, package));
        snapshot.clear();
        if last.as_ref() != Some(&now_playing) {
            announce(app, serial, now_playing.as_ref());
            last = Some(now_playing);
        }
    }
}

/// Press a media button, then report the result without waiting for the watch loop.
pub async fn press(app: &AppHandle, serial: &str, key: MediaKey) -> Result<()> {
    let state = app.state::<AppState>();
    let adb_path = adb::adb_path(&state.tools())?;
    media_session::send_key(&adb_path, serial, key).await?;
    if !state.now_playing().contains(serial) {
        return Ok(());
    }

    tokio::time::sleep(KEY_SETTLE).await;
    // The press already worked; a failed re-read just leaves the update to the watch loop.
    if let Ok(text) = media_session::read_sessions(&adb_path, serial).await {
        let labels = apps::cached_labels(&state.app_data_dir(), serial);
        let now_playing =
            media_session::parse(&text, |package| apps::display_label(&labels, package));
        announce(app, serial, now_playing.as_ref());
    }
    Ok(())
}
