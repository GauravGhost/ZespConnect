use std::collections::HashMap;
use std::time::{Duration, Instant};

use once_cell::sync::Lazy;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Notify;

use crate::models::{ReconnectEvent, ReconnectTarget};
use crate::runtime::tracker;
use crate::services::mdns;
use crate::services::wireless::{self, Reconnect};
use crate::state::AppState;

// Event-driven on purpose: wake-ups only read in-memory state. Asking adb's mDNS spawns a
// process, so it only runs on this backing-off fallback timer.
const FALLBACK_MIN: Duration = Duration::from_secs(30);
const FALLBACK_MAX: Duration = Duration::from_secs(300);
/// A failed address is left alone this long so an unreachable phone is not hammered.
const RETRY_AFTER: Duration = Duration::from_secs(30);
const EVENT_RECONNECTED: &str = "wireless:reconnected";
const EVENT_UNPAIRED: &str = "wireless:unpaired";

static DEVICES_CHANGED: Lazy<Notify> = Lazy::new(Notify::new);

/// Wake the watcher because the device list changed (a saved phone may have dropped).
pub fn devices_changed() {
    DEVICES_CHANGED.notify_one();
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Lookup {
    /// Our own browser's cache only.
    Cached,
    /// Also ask adb's mDNS, which spawns a process.
    WithAdb,
}

/// What a tick found, so the fallback timer can back off.
#[derive(PartialEq, Eq)]
enum Outcome {
    Idle,
    StillMissing,
    Reconnected,
}

pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut failures = HashMap::new();
        let mut fallback = FALLBACK_MIN;
        loop {
            let lookup = tokio::select! {
                _ = mdns::changed() => Lookup::Cached,
                _ = DEVICES_CHANGED.notified() => Lookup::Cached,
                _ = tokio::time::sleep(fallback) => Lookup::WithAdb,
            };
            let outcome = tick(&app, &mut failures, lookup).await;
            fallback = if outcome == Outcome::StillMissing && lookup == Lookup::WithAdb {
                (fallback * 2).min(FALLBACK_MAX)
            } else if outcome == Outcome::StillMissing {
                fallback
            } else {
                FALLBACK_MIN
            };
        }
    });
}

async fn tick(app: &AppHandle, failures: &mut HashMap<String, Instant>, lookup: Lookup) -> Outcome {
    let state = app.state::<AppState>();
    let devices = state.devices();
    if lookup == Lookup::Cached {
        drop_stale_transports(app, &devices).await;
    }
    let offline: Vec<ReconnectTarget> = state
        .reconnect_targets()
        .into_iter()
        .filter(|target| !wireless::is_online(&devices, target, None))
        .collect();
    if offline.is_empty() {
        return Outcome::Idle;
    }
    // A pairing session drives the adb server itself; stay out of its way.
    let Some(adb_path) = state.adb_path().filter(|_| !state.is_pairing()) else {
        return Outcome::StillMissing;
    };

    let services = match lookup {
        Lookup::Cached => wireless::cached_connect_services(),
        Lookup::WithAdb => wireless::connect_services(&adb_path).await,
    };
    let mut reconnected = false;
    for target in &offline {
        let Some(service) = wireless::locate(&services, target) else {
            continue;
        };
        if wireless::is_online(&devices, target, Some(service)) {
            continue;
        }
        let address = format!("{}:{}", service.host, service.port);
        if failures
            .get(&address)
            .is_some_and(|failed| failed.elapsed() < RETRY_AFTER)
        {
            continue;
        }

        match wireless::reconnect(&adb_path, &address).await {
            Reconnect::Connected(address) => {
                failures.remove(&address);
                reconnected = true;
                emit(app, EVENT_RECONNECTED, target, Some(address));
            }
            Reconnect::Unpaired => {
                failures.insert(address, Instant::now());
                emit_unpaired(app, target);
            }
            Reconnect::Unavailable(_) => {
                failures.insert(address, Instant::now());
            }
        }
    }

    if reconnected {
        tracker::refresh(app).await;
        Outcome::Reconnected
    } else {
        Outcome::StillMissing
    }
}

/// adb leaves a phone's old port listed as offline after it moves; drop it so the UI shows the
/// live address. Free unless a stale port is suspected, which then costs one `adb devices`.
async fn drop_stale_transports(app: &AppHandle, devices: &[crate::models::Device]) {
    let Some(adb_path) = app.state::<AppState>().adb_path() else {
        return;
    };
    let moved: Vec<String> = wireless::cached_connect_services()
        .iter()
        .filter(|service| wireless::has_stale_port(devices, service))
        .map(|service| format!("{}:{}", service.host, service.port))
        .collect();
    let mut dropped = false;
    for live in &moved {
        dropped |= wireless::drop_stale(&adb_path, live).await;
    }
    // Refreshing wakes this watcher again, so only refresh when something actually changed.
    if dropped {
        tracker::refresh(app).await;
    }
}

/// Tell the UI a saved phone forgot this computer, so it stops watching it and asks to pair.
pub fn emit_unpaired(app: &AppHandle, target: &ReconnectTarget) {
    emit(app, EVENT_UNPAIRED, target, None);
}

fn emit(app: &AppHandle, event: &str, target: &ReconnectTarget, address: Option<String>) {
    let payload = ReconnectEvent {
        serial: target.serial.clone(),
        address,
    };
    let _ = app.emit(event, payload);
}
