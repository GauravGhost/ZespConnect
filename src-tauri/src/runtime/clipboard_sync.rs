//! Two-way clipboard sync with a device through a control-only scrcpy server: the server reports
//! each phone copy, and the computer's clipboard is polled and pushed with `SET_CLIPBOARD`.
//! Needs no helper app on the phone.

use std::collections::VecDeque;
use std::sync::mpsc as std_mpsc;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncWriteExt, BufReader};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::process::Child;
use tokio::sync::mpsc;

use crate::error::{AppError, Result};
use crate::runtime::server_link::{self, ServerLink};
use crate::services::control::{self, DeviceMessage};
use crate::services::scrcpy_server;
use crate::state::AppState;

pub const EVENT_CHANGED: &str = "clipboard-sync:changed";
/// How often the computer's clipboard is checked for a new copy (no OS change event is used).
const HOST_POLL: Duration = Duration::from_millis(500);
const ECHO_MEMORY: usize = 4;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Changed<'a> {
    serial: &'a str,
    on: bool,
}

fn announce(app: &AppHandle, serial: &str, on: bool) {
    let _ = app.emit(EVENT_CHANGED, Changed { serial, on });
}

pub async fn start(app: &AppHandle, serial: &str) -> Result<()> {
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

pub fn stop(app: &AppHandle, serial: &str) {
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
