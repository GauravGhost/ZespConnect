use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use mdns_sd::{ServiceDaemon, ServiceEvent};
use once_cell::sync::Lazy;
use tokio::sync::Notify;

pub const PAIRING_SERVICE: &str = "_adb-tls-pairing._tcp";
pub const CONNECT_SERVICE: &str = "_adb-tls-connect._tcp";

/// A discovered service instance, e.g. `adb-<serialno>-<id>` / `_adb-tls-connect._tcp` / ip / port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MdnsService {
    pub name: String,
    pub service_type: String,
    pub host: String,
    pub port: u16,
}

type Cache = Arc<RwLock<HashMap<String, MdnsService>>>;

/// Started on first use and kept for the app's life, so its cache is warm on later lookups.
static BROWSER: Lazy<Option<Cache>> = Lazy::new(start);
static CHANGED: Lazy<Notify> = Lazy::new(Notify::new);

/// The services currently announced on the network (empty when the browser failed to start).
pub fn services() -> Vec<MdnsService> {
    BROWSER
        .as_ref()
        .and_then(|cache| cache.read().ok().map(|map| map.values().cloned().collect()))
        .unwrap_or_default()
}

/// Resolves when a service appears, moves or disappears. Never resolves if the browser
/// failed to start, so callers should also wake on a timer.
pub async fn changed() {
    Lazy::force(&BROWSER);
    CHANGED.notified().await;
}

fn start() -> Option<Cache> {
    let daemon = ServiceDaemon::new().ok()?;
    let cache: Cache = Arc::default();
    for service_type in [CONNECT_SERVICE, PAIRING_SERVICE] {
        let receiver = daemon.browse(&format!("{service_type}.local.")).ok()?;
        let cache = cache.clone();
        std::thread::spawn(move || {
            while let Ok(event) = receiver.recv() {
                apply(&cache, service_type, event);
            }
        });
    }
    // The daemon stops when dropped; it lives as long as the app.
    std::mem::forget(daemon);
    Some(cache)
}

fn apply(cache: &Cache, service_type: &str, event: ServiceEvent) {
    let Ok(mut map) = cache.write() else { return };
    let changed = match event {
        ServiceEvent::ServiceResolved(info) => {
            let fullname = info.get_fullname().to_string();
            let Some(host) = info.get_addresses_v4().into_iter().next() else {
                return;
            };
            let service = MdnsService {
                name: instance_name(&fullname, service_type),
                service_type: service_type.to_string(),
                host: host.to_string(),
                port: info.get_port(),
            };
            // Phones re-announce periodically; only a new or moved service is news.
            map.insert(fullname, service.clone()) != Some(service)
        }
        ServiceEvent::ServiceRemoved(_, fullname) => map.remove(&fullname).is_some(),
        _ => false,
    };
    drop(map);
    if changed {
        CHANGED.notify_one();
    }
}

/// `adb-X-y._adb-tls-connect._tcp.local.` -> `adb-X-y`.
fn instance_name(fullname: &str, service_type: &str) -> String {
    fullname
        .strip_suffix(&format!(".{service_type}.local."))
        .unwrap_or(fullname)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_the_service_type_from_the_instance_name() {
        assert_eq!(
            instance_name("adb-0123-xyz._adb-tls-connect._tcp.local.", CONNECT_SERVICE),
            "adb-0123-xyz"
        );
        assert_eq!(instance_name("odd", CONNECT_SERVICE), "odd");
    }
}
