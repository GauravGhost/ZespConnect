//! Process helpers: run a child to completion, or spawn a detached one. Always spawns with an
//! argv array (never a shell string), so nothing from the UI can be shell-interpreted.

use std::path::Path;
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

/// `program args` with stdin closed and the platform's spawn flags applied.
fn std_command(program: &str, args: &[String]) -> std::process::Command {
    let (program, args) = platform::prepare(program, args);
    let mut command = std::process::Command::new(program);
    command.args(args).stdin(Stdio::null());
    platform::configure_command(&mut command);
    command
}

/// An async command with stdout and stderr piped back to the caller.
pub(crate) fn base_command(program: &str, args: &[String]) -> Command {
    let mut command = std_command(program, args);
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    Command::from(command)
}

/// A command whose stdout and stderr both go to a fresh `log_path`.
fn logged_command(
    program: &str,
    args: &[String],
    log_path: &Path,
) -> Result<std::process::Command> {
    let log = std::fs::File::create(log_path)?;
    let log_err = log.try_clone()?;
    let mut command = std_command(program, args);
    command
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(log_err));
    Ok(command)
}

/// Maps a spawn failure to an error naming the program that would not start.
fn start_error(command: &std::process::Command) -> impl FnOnce(std::io::Error) -> AppError {
    let program = command.get_program().to_string_lossy().to_string();
    move |error| AppError::CommandFailed(format!("Could not start {program}: {error}"))
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

/// Run `program args` to completion and return its raw stdout; a failed exit is an error carrying stderr.
pub async fn read_stdout(program: &str, args: &[String], timeout: Duration) -> Result<Vec<u8>> {
    let child = base_command(program, args)
        .spawn()
        .map_err(|e| AppError::CommandFailed(format!("Could not start {program}: {e}")))?;

    let output = tokio::time::timeout(timeout, child.wait_with_output())
        .await
        .map_err(|_| AppError::CommandFailed(format!("{program} timed out after {timeout:?}")))?
        .map_err(|e| AppError::CommandFailed(e.to_string()))?;

    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(AppError::CommandFailed(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ))
    }
}

/// Spawn a detached process logging to `log_path`; returns the pid for the caller to track.
pub fn spawn_detached(program: &str, args: &[String], log_path: &Path) -> Result<u32> {
    let mut command = logged_command(program, args, log_path)?;
    let on_error = start_error(&command);
    let child = command.spawn().map_err(on_error)?;
    Ok(child.id())
}

/// Spawn a process logging to `log_path`, returning the child so the caller can await or kill it.
/// `kill_on_drop(false)` keeps a long-lived scrcpy window alive if its owning task is dropped.
pub fn spawn_captured(
    program: &str,
    args: &[String],
    log_path: &Path,
    env: &[(&str, &str)],
) -> Result<tokio::process::Child> {
    let mut command = logged_command(program, args, log_path)?;
    command.envs(env.iter().copied());
    let on_error = start_error(&command);
    Command::from(command)
        .kill_on_drop(false)
        .spawn()
        .map_err(on_error)
}

/// Read up to `max` lines from a file, for surfacing a process's log tail.
pub fn read_log_tail(path: &Path, max_lines: usize) -> String {
    match std::fs::read_to_string(path) {
        Ok(contents) => {
            let lines: Vec<&str> = contents.lines().filter(|l| !l.trim().is_empty()).collect();
            let start = lines.len().saturating_sub(max_lines);
            lines[start..].join(" ")
        }
        Err(_) => String::new(),
    }
}

/// The most informative line from a failed process's log: the first that looks like an error,
/// otherwise the first non-empty. A Java stack trace puts its message before the frames, so a tail
/// is useless for the scrcpy server.
pub fn read_log_error(path: &Path) -> String {
    let Ok(contents) = std::fs::read_to_string(path) else {
        return String::new();
    };

    let mut first = "";
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if first.is_empty() {
            first = line;
        }
        let looks_like_error = line.contains("Exception")
            || line.contains("Error")
            || line.starts_with("Invalid")
            || line.contains("not supported")
            || line.contains("Unknown");
        if looks_like_error {
            return line.to_string();
        }
    }
    first.to_string()
}
