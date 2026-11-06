use std::path::{Path, PathBuf};
use std::time::Duration;

use tauri::{AppHandle, Manager};
use tokio::net::{TcpListener, TcpStream};
use tokio::process::Child;

use crate::error::{AppError, Result};
use crate::runtime::mirror;
use crate::services::{adb, scrcpy, scrcpy_server};
use crate::state::AppState;
use crate::util::{process, validation};

// How long to wait for the server to connect back before giving up.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(12);
// How often to look for a connection while also watching for a dead server.
const ACCEPT_POLL: Duration = Duration::from_millis(200);

/// The binaries and version needed to run the embedded server.
pub struct ServerTools {
    pub adb_bin: String,
    pub version: String,
    pub jar: PathBuf,
}

pub fn resolve(app: &AppHandle) -> Result<ServerTools> {
    let tools = app.state::<AppState>().tools();
    let adb_bin = adb::adb_path(&tools)?;
    let scrcpy_bin = scrcpy::scrcpy_path(&tools)?;
    let version = tools
        .scrcpy
        .version
        .clone()
        .filter(|value| scrcpy_server::supported(Some(value)))
        .ok_or_else(|| {
            AppError::Other("This scrcpy version cannot be embedded; using a window instead".into())
        })?;
    let jar = scrcpy_server::server_path(&scrcpy_bin)?;
    Ok(ServerTools {
        adb_bin,
        version,
        jar,
    })
}

/// What has to be undone on the device once a server is gone.
pub struct Teardown {
    adb_bin: String,
    serial: String,
    tunnel: String,
    remote: String,
    pub log_path: PathBuf,
}

impl Teardown {
    pub fn adb_bin(&self) -> &str {
        &self.adb_bin
    }

    /// Remove the reverse tunnel, the pushed server and the local log.
    pub async fn run(&self) {
        self.clean_device().await;
        let _ = std::fs::remove_file(&self.log_path);
    }

    async fn clean_device(&self) {
        let _ = adb::reverse_remove(
            &self.adb_bin,
            &self.serial,
            &self.tunnel,
            adb::DEFAULT_TIMEOUT,
        )
        .await;
        let command = format!("rm -f {}", validation::shell_quote(&self.remote));
        let _ = adb::shell(&self.adb_bin, &self.serial, &command, adb::DEFAULT_TIMEOUT).await;
    }
}

/// A running server and its sockets, in the order the server opened them.
pub struct ServerLink {
    pub child: Child,
    pub streams: Vec<TcpStream>,
    pub teardown: Teardown,
}

/// Push the server, start it with `options` and accept `stream_count` connections.
/// On failure the device is cleaned up and the error carries the server's own message.
pub async fn open(
    app: &AppHandle,
    tools: &ServerTools,
    serial: &str,
    scid: &str,
    options: Vec<String>,
    stream_count: usize,
) -> Result<ServerLink> {
    let tunnel = scrcpy_server::tunnel_name(scid);
    let remote = scrcpy_server::remote_jar(scid);

    let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
    let port = listener.local_addr()?.port();

    let teardown = Teardown {
        adb_bin: tools.adb_bin.clone(),
        serial: serial.to_string(),
        tunnel: tunnel.clone(),
        remote: remote.clone(),
        log_path: mirror::log_dir(app)?.join(format!("{scid}.log")),
    };
    let args = scrcpy_server::launch_args(serial, &remote, &tools.version, options);

    adb::push(
        &tools.adb_bin,
        serial,
        &tools.jar.to_string_lossy(),
        &remote,
        adb::LONG_TIMEOUT,
    )
    .await?;
    adb::reverse(&tools.adb_bin, serial, &tunnel, port, adb::DEFAULT_TIMEOUT).await?;

    let mut child = process::spawn_captured(&tools.adb_bin, &args, &teardown.log_path, &[])?;

    let mut streams = Vec::with_capacity(stream_count);
    for _ in 0..stream_count {
        match accept(&listener, &mut child, &teardown.log_path).await {
            Ok(stream) => streams.push(stream),
            Err(error) => {
                let _ = child.kill().await;
                // Keep the log: startup failures are exactly when it matters.
                teardown.clean_device().await;
                return Err(error);
            }
        }
    }

    Ok(ServerLink {
        child,
        streams,
        teardown,
    })
}

async fn accept(listener: &TcpListener, child: &mut Child, log_path: &Path) -> Result<TcpStream> {
    let deadline = tokio::time::Instant::now() + CONNECT_TIMEOUT;
    loop {
        if let Some(status) = child.try_wait()? {
            return Err(AppError::CommandFailed(failure_message(
                log_path,
                &format!("scrcpy server exited ({status})"),
            )));
        }

        match tokio::time::timeout(ACCEPT_POLL, listener.accept()).await {
            Ok(Ok((stream, _))) => {
                let _ = stream.set_nodelay(true);
                return Ok(stream);
            }
            Ok(Err(error)) => return Err(AppError::Io(error.to_string())),
            Err(_) if tokio::time::Instant::now() >= deadline => {
                return Err(AppError::CommandFailed(failure_message(
                    log_path,
                    "Timed out waiting for the scrcpy server",
                )));
            }
            Err(_) => {}
        }
    }
}

pub fn failure_message(log_path: &Path, fallback: &str) -> String {
    let message = process::read_log_error(log_path);
    if message.is_empty() {
        fallback.to_string()
    } else {
        message
    }
}
