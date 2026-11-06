use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use tauri::async_runtime::JoinHandle;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::process::ChildStdin;
use tokio::sync::mpsc;

use crate::error::{AppError, Result};
use crate::models::ShellEvent;
use crate::services::{adb, console};
use crate::state::AppState;

const READ_CHUNK: usize = 8192;
/// adb prints its own errors (device offline, unauthorized) on stderr just before exiting.
const STDERR_DRAIN: Duration = Duration::from_secs(1);

struct Session {
    input: mpsc::UnboundedSender<Vec<u8>>,
    task: JoinHandle<()>,
}

/// Open `adb shell` sessions by id. Dropping a session's task kills its adb process.
#[derive(Default)]
pub struct ShellSessions {
    next_id: AtomicU64,
    sessions: Mutex<HashMap<u64, Session>>,
}

impl ShellSessions {
    /// Register a session; the lock is held while its task spawns so a task that ends at once
    /// still finds itself registered.
    fn start(
        &self,
        input: mpsc::UnboundedSender<Vec<u8>>,
        spawn: impl FnOnce(u64) -> JoinHandle<()>,
    ) -> Result<u64> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let mut guard = self
            .sessions
            .lock()
            .map_err(|_| AppError::Other("shell sessions lock poisoned".into()))?;
        guard.insert(
            id,
            Session {
                input,
                task: spawn(id),
            },
        );
        Ok(id)
    }

    pub fn write(&self, id: u64, input: String) -> Result<()> {
        let ended = || AppError::Invalid("This shell session has ended".into());
        let guard = self.sessions.lock().map_err(|_| ended())?;
        let session = guard.get(&id).ok_or_else(ended)?;
        session.input.send(input.into_bytes()).map_err(|_| ended())
    }

    pub fn close(&self, id: u64) {
        if let Ok(mut guard) = self.sessions.lock() {
            if let Some(session) = guard.remove(&id) {
                session.task.abort();
            }
        }
    }

    fn finished(&self, id: u64) {
        if let Ok(mut guard) = self.sessions.lock() {
            guard.remove(&id);
        }
    }
}

/// Start an interactive shell on `serial` that streams into `channel` until [`ShellSessions::close`]
/// runs, the shell exits, or the window goes away. Returns the session id.
pub fn open(
    app: &AppHandle,
    state: &AppState,
    serial: &str,
    channel: Channel<ShellEvent>,
) -> Result<u64> {
    let adb_path = adb::adb_path(&state.tools())?;
    let mut child = console::spawn_shell(&adb_path, serial)?;
    let (Some(stdin), Some(stdout), Some(stderr)) =
        (child.stdin.take(), child.stdout.take(), child.stderr.take())
    else {
        return Err(AppError::Other("adb shell started without pipes".into()));
    };

    let (input, inputs) = mpsc::unbounded_channel();
    tauri::async_runtime::spawn(write_input(stdin, inputs));

    let app = app.clone();
    state.shells().start(input, move |id| {
        tauri::async_runtime::spawn(async move {
            let err_task = tauri::async_runtime::spawn(forward_output(stderr, channel.clone()));
            if forward_output(stdout, channel.clone()).await {
                let _ = tokio::time::timeout(STDERR_DRAIN, err_task).await;
                let code = child.wait().await.ok().and_then(|status| status.code());
                let _ = channel.send(ShellEvent::Exit { code });
            } else {
                err_task.abort();
            }
            // Dropping the child kills adb if it is still running (`kill_on_drop`).
            drop(child);
            app.state::<AppState>().shells().finished(id);
        })
    })
}

async fn write_input(mut stdin: ChildStdin, mut inputs: mpsc::UnboundedReceiver<Vec<u8>>) {
    while let Some(bytes) = inputs.recv().await {
        if stdin.write_all(&bytes).await.is_err() || stdin.flush().await.is_err() {
            break;
        }
    }
}

/// Forward output until the stream ends (`true`) or the window stops listening (`false`).
async fn forward_output<R: AsyncRead + Unpin>(mut reader: R, channel: Channel<ShellEvent>) -> bool {
    let mut buffer = vec![0; READ_CHUNK];
    let mut pending = Vec::new();
    loop {
        let read = match reader.read(&mut buffer).await {
            Ok(0) | Err(_) => return true,
            Ok(read) => read,
        };
        let text = console::take_utf8(&mut pending, &buffer[..read]);
        if !text.is_empty() && channel.send(ShellEvent::Output { text }).is_err() {
            return false;
        }
    }
}
