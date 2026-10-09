//! `adb shell` execution, logcat process spawning, and host-clipboard helpers
//! for the console feature.

use std::process::Stdio;
use std::time::Duration;

use crate::error::{AppError, Result};
use crate::models::{ShellResult, ToolStatus};
use crate::platform;
use crate::services::adb;

const SHELL_TIMEOUT: Duration = Duration::from_secs(60);

pub async fn exec(tools: &ToolStatus, serial: &str, command: &str) -> Result<ShellResult> {
    if command.trim().is_empty() {
        return Err(AppError::Invalid("Empty command".into()));
    }

    let adb_path = adb::adb_path(tools)?;
    let output = adb::shell(&adb_path, serial, command, SHELL_TIMEOUT).await?;
    let code = output.code.unwrap_or(if output.success { 0 } else { 1 });
    Ok(ShellResult {
        stdout: output.stdout,
        stderr: output.stderr,
        code,
    })
}

/// Spawn `adb -s <serial> logcat -v threadtime` with piped output for a caller
/// that forwards the lines to the UI. The child is killed when dropped.
pub fn spawn_logcat(adb_path: &str, serial: &str) -> Result<tokio::process::Child> {
    let args: Vec<String> = vec![
        "-s".to_string(),
        serial.to_string(),
        "logcat".to_string(),
        "-v".to_string(),
        "threadtime".to_string(),
    ];

    let (program, args) = platform::prepare(adb_path, &args);
    let mut std_cmd = std::process::Command::new(program);
    std_cmd
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    platform::configure_command(&mut std_cmd);

    tokio::process::Command::from(std_cmd)
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| AppError::CommandFailed(format!("Could not start adb logcat: {e}")))
}
