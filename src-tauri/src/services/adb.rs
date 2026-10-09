//! Thin wrappers around the `adb` executable. No Tauri types here.

use std::time::Duration;

use crate::error::{AppError, Result};
use crate::models::ToolStatus;
use crate::util::process::{self, ProcessOutput};

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
pub const LONG_TIMEOUT: Duration = Duration::from_secs(300);
/// Local adb server round trips (`start-server`, `devices`) that should answer quickly.
const SERVER_TIMEOUT: Duration = Duration::from_secs(15);
const KILL_SERVER_TIMEOUT: Duration = Duration::from_secs(10);

/// adb subcommands that must NOT be scoped with `-s <serial>`.
const UNSCOPED: &[&str] = &[
    "connect",
    "pair",
    "disconnect",
    "devices",
    "start-server",
    "kill-server",
    "mdns",
    "version",
    "help",
    "track-devices",
];

pub fn adb_path(tools: &ToolStatus) -> Result<String> {
    tools.adb.path.clone().ok_or(AppError::AdbMissing)
}

pub async fn run(adb: &str, args: &[String], timeout: Duration) -> Result<ProcessOutput> {
    process::run(adb, args, timeout).await
}

/// Run adb, scoping the command with `-s <serial>` unless the subcommand is global.
pub async fn run_scoped(
    adb: &str,
    serial: &str,
    args: &[String],
    timeout: Duration,
) -> Result<ProcessOutput> {
    let unscoped = args
        .first()
        .map(|a| UNSCOPED.contains(&a.as_str()))
        .unwrap_or(false);

    let mut full: Vec<String> = Vec::with_capacity(args.len() + 2);
    if !unscoped && !serial.is_empty() {
        full.push("-s".into());
        full.push(serial.into());
    }
    full.extend_from_slice(args);

    process::run(adb, &full, timeout).await
}

/// `adb -s <serial> shell <command>` - `command` is a single remote shell line.
pub async fn shell(
    adb: &str,
    serial: &str,
    command: &str,
    timeout: Duration,
) -> Result<ProcessOutput> {
    let args = vec!["shell".to_string(), command.to_string()];
    run_scoped(adb, serial, &args, timeout).await
}

pub async fn push(
    adb: &str,
    serial: &str,
    local: &str,
    remote: &str,
    timeout: Duration,
) -> Result<ProcessOutput> {
    let args = vec!["push".to_string(), local.to_string(), remote.to_string()];
    run_scoped(adb, serial, &args, timeout).await
}

/// `adb -s <serial> reverse <remote> tcp:<local_port>` - the device connects back
/// to a host port we are listening on (scrcpy's default tunnel direction).
pub async fn reverse(
    adb: &str,
    serial: &str,
    remote: &str,
    local_port: u16,
    timeout: Duration,
) -> Result<ProcessOutput> {
    let args = vec![
        "reverse".to_string(),
        remote.to_string(),
        format!("tcp:{local_port}"),
    ];
    run_scoped(adb, serial, &args, timeout).await
}

pub async fn reverse_remove(
    adb: &str,
    serial: &str,
    remote: &str,
    timeout: Duration,
) -> Result<ProcessOutput> {
    let args = vec![
        "reverse".to_string(),
        "--remove".to_string(),
        remote.to_string(),
    ];
    run_scoped(adb, serial, &args, timeout).await
}

pub async fn start_server(adb: &str) -> Result<ProcessOutput> {
    run(adb, &["start-server".to_string()], SERVER_TIMEOUT).await
}

pub async fn kill_server(adb: &str) -> Result<ProcessOutput> {
    run(adb, &["kill-server".to_string()], KILL_SERVER_TIMEOUT).await
}

pub async fn list_devices_raw(adb: &str) -> Result<ProcessOutput> {
    run(
        adb,
        &["devices".to_string(), "-l".to_string()],
        SERVER_TIMEOUT,
    )
    .await
}

/// `adb connect <address>` - unscoped (there is no device serial yet).
pub async fn connect(adb: &str, address: &str) -> Result<ProcessOutput> {
    run(
        adb,
        &["connect".to_string(), address.to_string()],
        DEFAULT_TIMEOUT,
    )
    .await
}

pub async fn disconnect(adb: &str, target: &str) -> Result<ProcessOutput> {
    run(
        adb,
        &["disconnect".to_string(), target.to_string()],
        DEFAULT_TIMEOUT,
    )
    .await
}

/// `adb -s <serial> tcpip <port>` - restart adbd on the device in TCP mode.
pub async fn tcpip(adb: &str, serial: &str, port: u16) -> Result<ProcessOutput> {
    let args = vec!["tcpip".to_string(), port.to_string()];
    run_scoped(adb, serial, &args, DEFAULT_TIMEOUT).await
}
