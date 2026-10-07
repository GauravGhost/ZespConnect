//! Thin wrappers around the `adb` executable. No Tauri types here.

use std::time::Duration;

use crate::error::{AppError, Result};
use crate::models::ToolStatus;
use crate::util::process::{self, ProcessOutput};

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
pub const LONG_TIMEOUT: Duration = Duration::from_secs(300);

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

/// Run adb with raw arguments.
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

/// `adb -s <serial> shell <command>` — `command` is a single remote shell line.
pub async fn shell(
    adb: &str,
    serial: &str,
    command: &str,
    timeout: Duration,
) -> Result<ProcessOutput> {
    let args = vec!["shell".to_string(), command.to_string()];
    run_scoped(adb, serial, &args, timeout).await
}

pub async fn start_server(adb: &str) -> Result<ProcessOutput> {
    run(adb, &["start-server".to_string()], Duration::from_secs(15)).await
}

pub async fn list_devices_raw(adb: &str) -> Result<ProcessOutput> {
    run(
        adb,
        &["devices".to_string(), "-l".to_string()],
        Duration::from_secs(15),
    )
    .await
}
