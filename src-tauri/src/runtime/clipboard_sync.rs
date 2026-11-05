use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::mpsc as std_mpsc;
use std::sync::RwLock;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncWriteExt, BufReader};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::process::Child;
use tokio::sync::mpsc;

use crate::error::{AppError, Result};
use crate::models::Device;
use crate::runtime::server_link::{self, ServerLink};
use crate::services::control::{self, DeviceMessage};
use crate::services::scrcpy_server;
use crate::state::AppState;
use crate::util;

pub const EVENT_CHANGED: &str = "clipboard-sync:changed";
/// How often the computer's clipboard is checked for a new copy (no OS change event is used).
const HOST_POLL: Duration = Duration::from_millis(500);
const ECHO_MEMORY: usize = 4;
const SAVED_FILE: &str = "clipboard-sync.json";

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Changed<'a> {
    serial: &'a str,
    on: bool,
}

fn announce(app: &AppHandle, serial: &str, on: bool) {
    let _ = app.emit(EVENT_CHANGED, Changed { serial, on });
    crate::tray::sync(app);
}

/// A phone the user switched sync on for. Wireless serials change between connections, so the
/// stable mDNS name also identifies it.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct SavedPhone {
    serial: String,
    mdns_name: Option<String>,
}

impl SavedPhone {
    fn of(device: &Device) -> Self {
        SavedPhone {
            serial: device.serial.clone(),
            mdns_name: device.mdns_name.clone(),
        }
    }

    fn is(&self, device: &Device) -> bool {
        device.serial == self.serial
            || (self.mdns_name.is_some() && device.mdns_name == self.mdns_name)
    }
}

/// The phones that keep syncing across app restarts and reconnects. Only the user's own switch
/// changes this; a link that ends on its own (cable out, phone rebooted) stays remembered.
pub struct SavedPhones {
    path: PathBuf,
    phones: RwLock<Vec<SavedPhone>>,
    /// Serials whose automatic start is in flight, so quick device-list changes start one link.
    starting: RwLock<HashSet<String>>,
}

impl SavedPhones {
    pub fn load(app_data_dir: &Path) -> Self {
        let path = app_data_dir.join(SAVED_FILE);
        let phones = util::read_json(&path).ok().flatten().unwrap_or_default();
        SavedPhones {
            path,
            phones: RwLock::new(phones),
            starting: RwLock::new(HashSet::new()),
        }
    }

    fn update(&self, change: impl FnOnce(&mut Vec<SavedPhone>)) -> Result<()> {
        let mut phones = self
            .phones
            .write()
            .map_err(|_| AppError::Other("state poisoned".into()))?;
        change(&mut phones);
        util::write_json(&self.path, &*phones)
    }

    fn remember(&self, device: &Device) -> Result<()> {
        self.update(|phones| {
            phones.retain(|phone| !phone.is(device));
            phones.push(SavedPhone::of(device));
        })
    }

    fn forget(&self, device: &Device) -> Result<()> {
        self.update(|phones| phones.retain(|phone| !phone.is(device)))
    }

    fn contains(&self, device: &Device) -> bool {
        self.phones
            .read()
            .map(|phones| phones.iter().any(|phone| phone.is(device)))
            .unwrap_or(false)
    }

    fn claim_start(&self, serial: &str) -> bool {
        self.starting
            .write()
            .map(|mut starting| starting.insert(serial.to_string()))
            .unwrap_or(false)
    }

    fn release_start(&self, serial: &str) {
        if let Ok(mut starting) = self.starting.write() {
            starting.remove(serial);
        }
    }
}

/// Switch sync on at the user's request and keep it on for this phone from now on.
pub async fn enable(app: &AppHandle, serial: &str) -> Result<()> {
    start(app, serial).await?;
    let state = app.state::<AppState>();
    match state.device(serial) {
        Some(device) => state.clipboard_phones().remember(&device),
        None => Ok(()),
    }
}

/// Switch sync off at the user's request so it stays off after a restart or reconnect.
pub fn disable(app: &AppHandle, serial: &str) -> Result<()> {
    stop(app, serial);
    let state = app.state::<AppState>();
    match state.device(serial) {
        Some(device) => state.clipboard_phones().forget(&device),
        None => Ok(()),
    }
}

/// Resume sync for remembered phones that are connected but not syncing.
pub fn devices_changed(app: &AppHandle, devices: &[Device]) {
    let state = app.state::<AppState>();
    let phones = state.clipboard_phones();
    for device in devices {
        if !device.connected
            || !phones.contains(device)
            || state.clipboard_sync().contains(&device.serial)
            || !phones.claim_start(&device.serial)
        {
            continue;
        }
        let app = app.clone();
        let serial = device.serial.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(error) = start(&app, &serial).await {
                eprintln!("clipboard sync for {serial} did not resume: {error}");
            }
            app.state::<AppState>()
                .clipboard_phones()
                .release_start(&serial);
        });
    }
}

async fn start(app: &AppHandle, serial: &str) -> Result<()> {
    if app.state::<AppState>().clipboard_sync().contains(serial) {
        return Ok(());
    }

    let server = server_link::resolve(app)?;
    let scid = scrcpy_server::scid();
    let ServerLink {
        mut child,
        streams,
        teardown,
    } = server_link::open(
        app,
        &server,
        serial,
        &scid,
        scrcpy_server::clipboard_args(&scid),
        1,
    )
    .await?;

    let opened = match streams.into_iter().next() {
        Some(stream) => HostClipboard::spawn().map(|host| (stream, host)),
        None => Err(AppError::Other("The scrcpy server did not connect".into())),
    };
    let (stream, host) = match opened {
        Ok(opened) => opened,
        Err(error) => {
            let _ = child.kill().await;
            teardown.run().await;
            return Err(error);
        }
    };

    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
    let Some(link_id) = app
        .state::<AppState>()
        .clipboard_sync()
        .insert(serial, stop_tx)
    else {
        let _ = child.kill().await;
        teardown.run().await;
        return Ok(());
    };
    announce(app, serial, true);

    let handle = app.clone();
    let serial = serial.to_string();
    tauri::async_runtime::spawn(async move {
        let (reader, writer) = stream.into_split();
        sync(reader, writer, host, &mut child, stop_rx).await;
        let _ = child.kill().await;
        teardown.run().await;
        handle
            .state::<AppState>()
            .clipboard_sync()
            .finished(&serial, link_id);
        announce(&handle, &serial, false);
    });
    Ok(())
}

fn stop(app: &AppHandle, serial: &str) {
    app.state::<AppState>().clipboard_sync().stop(serial);
}

async fn sync(
    reader: OwnedReadHalf,
    mut writer: OwnedWriteHalf,
    host: HostClipboard,
    child: &mut Child,
    mut stop: tokio::sync::oneshot::Receiver<()>,
) {
    let HostClipboard {
        to_host,
        mut from_host,
    } = host;

    let phone_to_host = async {
        let mut reader = BufReader::new(reader);
        while let Ok(message) = control::read_device_message(&mut reader).await {
            if let DeviceMessage::Clipboard(text) = message {
                if to_host.send(text).is_err() {
                    break;
                }
            }
        }
    };
    let host_to_phone = async {
        while let Some(text) = from_host.recv().await {
            if writer
                .write_all(&control::set_clipboard(&text))
                .await
                .is_err()
            {
                break;
            }
        }
    };

    tokio::select! {
        _ = phone_to_host => {}
        _ = host_to_phone => {}
        _ = &mut stop => {}
        _ = child.wait() => {}
    }
}

/// The phone reports text set on it too, sometimes late and twice; remembering the last few texts
/// sent keeps a late echo of an older copy from overwriting a newer one on the computer.
fn is_echo(sent: &VecDeque<String>, text: &str) -> bool {
    sent.iter().any(|previous| previous == text)
}

fn remember(sent: &mut VecDeque<String>, text: String) {
    if sent.len() == ECHO_MEMORY {
        sent.pop_front();
    }
    sent.push_back(text);
}

/// The computer's clipboard, owned by one thread for the link's lifetime. Text sent to it is
/// written to the clipboard; new copies made on the computer come out of `from_host`. The thread
/// remembers the last text either way, so a synced copy is never echoed back.
struct HostClipboard {
    to_host: std_mpsc::Sender<String>,
    from_host: mpsc::UnboundedReceiver<String>,
}

impl HostClipboard {
    fn spawn() -> Result<Self> {
        let (to_host, incoming) = std_mpsc::channel::<String>();
        let (outgoing, from_host) = mpsc::unbounded_channel::<String>();
        let (ready_tx, ready_rx) = std_mpsc::channel::<std::result::Result<(), String>>();

        std::thread::Builder::new()
            .name("clipboard-sync".into())
            .spawn(move || {
                let mut clipboard = match arboard::Clipboard::new() {
                    Ok(clipboard) => {
                        let _ = ready_tx.send(Ok(()));
                        clipboard
                    }
                    Err(error) => {
                        let _ = ready_tx.send(Err(error.to_string()));
                        return;
                    }
                };
                // Only copies made after sync starts are sent to the phone.
                let mut last = clipboard.get_text().ok();
                let mut sent = VecDeque::with_capacity(ECHO_MEMORY);
                loop {
                    match incoming.recv_timeout(HOST_POLL) {
                        Ok(text) => {
                            if !is_echo(&sent, &text) {
                                // A real phone copy: earlier echoes have passed, so the same
                                // text copied again on the phone later must still come through.
                                sent.clear();
                                let _ = clipboard.set_text(text.clone());
                                last = Some(text);
                            }
                        }
                        Err(std_mpsc::RecvTimeoutError::Timeout) => {
                            let Ok(text) = clipboard.get_text() else {
                                continue;
                            };
                            if text.is_empty() || last.as_deref() == Some(text.as_str()) {
                                continue;
                            }
                            last = Some(text.clone());
                            remember(&mut sent, text.clone());
                            if outgoing.send(text).is_err() {
                                break;
                            }
                        }
                        Err(std_mpsc::RecvTimeoutError::Disconnected) => break,
                    }
                }
            })
            .map_err(|error| AppError::Other(error.to_string()))?;

        match ready_rx.recv() {
            Ok(Ok(())) => Ok(HostClipboard { to_host, from_host }),
            Ok(Err(error)) => Err(AppError::Other(format!(
                "Desktop clipboard unavailable: {error}"
            ))),
            Err(_) => Err(AppError::Other("Desktop clipboard unavailable".into())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{DeviceState, DeviceTransport};

    fn device(serial: &str, mdns_name: Option<&str>) -> Device {
        Device {
            serial: serial.to_string(),
            display_name: String::new(),
            model: String::new(),
            android_version: None,
            sdk: None,
            transport: DeviceTransport::Wifi,
            state: DeviceState::Device,
            connected: true,
            battery: None,
            charging: None,
            address: None,
            mdns_name: mdns_name.map(str::to_string),
        }
    }

    #[test]
    fn a_saved_phone_is_known_by_serial_or_its_wireless_name() {
        let saved = SavedPhone::of(&device("192.168.1.5:40001", Some("adb-ABC-x")));

        assert!(saved.is(&device("192.168.1.5:40001", None)));
        assert!(saved.is(&device("192.168.1.5:38211", Some("adb-ABC-x"))));
        assert!(!saved.is(&device("192.168.1.5:38211", None)));
        assert!(!saved.is(&device("192.168.1.9:5555", Some("adb-XYZ-y"))));
    }

    #[test]
    fn saved_phones_survive_a_reload_until_forgotten() {
        let dir = std::env::temp_dir().join(format!("zesp-clipboard-{}", uuid::Uuid::new_v4()));
        let phone = device("adb-ABC-x._adb-tls-connect._tcp", Some("adb-ABC-x"));

        SavedPhones::load(&dir).remember(&phone).unwrap();
        let reloaded = SavedPhones::load(&dir);
        assert!(reloaded.contains(&phone));

        reloaded.forget(&phone).unwrap();
        assert!(!SavedPhones::load(&dir).contains(&phone));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn one_automatic_start_per_serial_at_a_time() {
        let phones = SavedPhones::load(&std::env::temp_dir().join("zesp-clipboard-unused"));
        assert!(phones.claim_start("s"));
        assert!(!phones.claim_start("s"));
        phones.release_start("s");
        assert!(phones.claim_start("s"));
    }

    #[test]
    fn remembers_the_last_few_sent_texts_as_echoes() {
        let mut sent = VecDeque::new();
        for text in ["a", "b", "c", "d", "e"] {
            remember(&mut sent, text.to_string());
        }
        assert!(!is_echo(&sent, "a"), "the oldest is forgotten");
        assert!(is_echo(&sent, "b"));
        assert!(is_echo(&sent, "e"));
        assert!(!is_echo(&sent, "phone copy"));
    }
}
