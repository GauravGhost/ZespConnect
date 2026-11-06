use tauri::ipc::Channel;
use tauri::{AppHandle, State};

use crate::error::Result;
use crate::models::{PerfSample, ProcessInfo};
use crate::runtime::device_stream;
use crate::services::performance::{self, Sampler};
use crate::services::processes::{self, TopFrames};
use crate::services::{adb, apps};
use crate::state::AppState;
use crate::util::validation::validate_serial;

/// Stream `serial`'s vitals once a second until [`close_performance`] or the window goes away.
#[tauri::command]
pub async fn open_performance(
    serial: String,
    channel: Channel<PerfSample>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<()> {
    validate_serial(&serial)?;
    let adb_path = adb::adb_path(&state.tools())?;
    let child = performance::spawn_watch(&adb_path, &serial)?;
    let labels = apps::cached_labels(&state.app_data_dir(), &serial);
    let mut sampler = Sampler::default();
    let mut pass = String::new();
    device_stream::open(
        &app,
        AppState::performance,
        &serial,
        child,
        channel,
        move |line| {
            if line.trim() != performance::END_MARKER {
                pass.push_str(line);
                pass.push('\n');
                return None;
            }
            let snapshot = performance::parse_snapshot(&pass);
            pass.clear();
            Some(sampler.sample(snapshot, |package| apps::display_label(&labels, package)))
        },
    )
}

#[tauri::command]
pub fn close_performance(serial: String, state: State<'_, AppState>) -> Result<()> {
    validate_serial(&serial)?;
    state.performance().stop(&serial);
    Ok(())
}

/// Stream `serial`'s process table every few seconds until [`close_processes`].
#[tauri::command]
pub async fn open_processes(
    serial: String,
    channel: Channel<Vec<ProcessInfo>>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<()> {
    validate_serial(&serial)?;
    let adb_path = adb::adb_path(&state.tools())?;
    let child = processes::spawn_top(&adb_path, &serial)?;
    let mut frames = TopFrames::new(apps::cached_labels(&state.app_data_dir(), &serial));
    device_stream::open(
        &app,
        AppState::processes,
        &serial,
        child,
        channel,
        move |line| frames.push(line),
    )
}

#[tauri::command]
pub fn close_processes(serial: String, state: State<'_, AppState>) -> Result<()> {
    validate_serial(&serial)?;
    state.processes().stop(&serial);
    Ok(())
}
