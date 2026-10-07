//! Process helpers: run a child to completion, or spawn a detached one.
//!
//! Always spawns with an argv array — never a shell string — so nothing the
//! UI passes through can be interpreted by a shell.

use std::process::Stdio;
use std::time::Duration;

use tokio::process::Command;

use crate::error::{AppError, Result};
use crate::platform;

#[derive(Debug, Clone)]
pub struct ProcessOutput {
    pub success: bool,
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl ProcessOutput {
    /// stdout, falling back to stderr, trimmed.
    pub fn text(&self) -> String {
        let value = if self.stdout.trim().is_empty() {
            &self.stderr
        } else {
            &self.stdout
        };
        value.trim().to_string()
    }
}

fn base_command(program: &str, args: &[String]) -> Command {
    let (program, args) = platform::prepare(program, args);
    let mut std_cmd = std::process::Command::new(program);
    std_cmd
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    platform::configure_command(&mut std_cmd);
    Command::from(std_cmd)
}

/// Run `program args` to completion, capturing output, with a timeout.
pub async fn run(program: &str, args: &[String], timeout: Duration) -> Result<ProcessOutput> {
    let mut command = base_command(program, args);
    let child = command
        .spawn()
        .map_err(|e| AppError::CommandFailed(format!("Could not start {program}: {e}")))?;

    let output = tokio::time::timeout(timeout, child.wait_with_output())
        .await
        .map_err(|_| AppError::CommandFailed(format!("{program} timed out after {timeout:?}")))?
        .map_err(|e| AppError::CommandFailed(e.to_string()))?;

    Ok(ProcessOutput {
        success: output.status.success(),
        code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

/// Spawn a detached process whose stdout/stderr are captured into `log_path`.
/// Returns the child's pid. The caller is responsible for tracking it.
pub fn spawn_detached(program: &str, args: &[String], log_path: &std::path::Path) -> Result<u32> {
    let (program, args) = platform::prepare(program, args);
    let display = program.clone();
    let log = std::fs::File::create(log_path)?;
    let log_err = log.try_clone()?;

    let mut std_cmd = std::process::Command::new(&program);
    std_cmd
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(log_err));
    platform::configure_command(&mut std_cmd);

    let child = std_cmd
        .spawn()
        .map_err(|e| AppError::CommandFailed(format!("Could not start {display}: {e}")))?;
    Ok(child.id())
}

/// Spawn a process whose stdout/stderr are captured into `log_path`, returning
/// the child so the caller can await its exit (and optionally kill it). Used for
/// long-lived scrcpy windows. `kill_on_drop(false)` keeps the window alive if the
/// task that owns the child is dropped.
pub fn spawn_captured(
    program: &str,
    args: &[String],
    log_path: &std::path::Path,
    env: &[(&str, &str)],
) -> Result<tokio::process::Child> {
    let (program, args) = platform::prepare(program, args);
    let display = program.clone();
    let log = std::fs::File::create(log_path)?;
    let log_err = log.try_clone()?;

    let mut std_cmd = std::process::Command::new(&program);
    std_cmd
        .args(args)
        .envs(env.iter().copied())
        .stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(log_err));
    platform::configure_command(&mut std_cmd);

    tokio::process::Command::from(std_cmd)
        .kill_on_drop(false)
        .spawn()
        .map_err(|e| AppError::CommandFailed(format!("Could not start {display}: {e}")))
}

/// Read up to `max` lines from a file, for surfacing a process's log tail.
pub fn read_log_tail(path: &std::path::Path, max_lines: usize) -> String {
    match std::fs::read_to_string(path) {
        Ok(contents) => {
            let lines: Vec<&str> = contents.lines().filter(|l| !l.trim().is_empty()).collect();
            let start = lines.len().saturating_sub(max_lines);
            lines[start..].join(" ")
        }
        Err(_) => String::new(),
    }
}
