use std::time::Duration;

use crate::error::{AppError, Result};
use crate::models::{ShellResult, ToolStatus};
use crate::services::adb;
use crate::util::process;
use crate::util::validation::shell_quote;

const SHELL_TIMEOUT: Duration = Duration::from_secs(60);
const LOOKUP_TIMEOUT: Duration = Duration::from_secs(15);
const LOG_LEVELS: &str = "VDIWEF";
/// `logcat --uid` arrived in Android 12; older phones can only follow one process id.
const UID_FILTER_MIN_SDK: u32 = 31;

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

/// Which lines of one app logcat keeps. Uids survive app restarts; a pid does not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppScope {
    Uids(Vec<u32>),
    Pid(u32),
}

pub fn validate_level(level: char) -> Result<char> {
    let level = level.to_ascii_uppercase();
    if LOG_LEVELS.contains(level) {
        Ok(level)
    } else {
        Err(AppError::Invalid(format!("Unknown log level: {level}")))
    }
}

/// The arguments after `logcat` for a scope and minimum level.
pub fn logcat_args(scope: Option<&AppScope>, min_level: Option<char>) -> Vec<String> {
    let mut args = vec!["-v".to_string(), "threadtime".to_string()];
    match scope {
        Some(AppScope::Uids(uids)) => {
            let list: Vec<String> = uids.iter().map(u32::to_string).collect();
            args.push(format!("--uid={}", list.join(",")));
        }
        Some(AppScope::Pid(pid)) => args.push(format!("--pid={pid}")),
        None => {}
    }
    if let Some(level) = min_level {
        args.push(format!("*:{level}"));
    }
    args
}

/// The uids of `package` in `pm list packages -U` output (one per user profile).
pub fn parse_package_uids(text: &str, package: &str) -> Vec<u32> {
    text.lines()
        .filter_map(|line| line.trim().strip_prefix("package:"))
        .filter_map(|rest| rest.split_once(" uid:"))
        .find(|(name, _)| *name == package)
        .map(|(_, uids)| {
            uids.split(',')
                .filter_map(|uid| uid.trim().parse().ok())
                .collect()
        })
        .unwrap_or_default()
}

pub async fn app_scope(adb_path: &str, serial: &str, package: &str, sdk: u32) -> Result<AppScope> {
    let quoted = shell_quote(package);
    if sdk >= UID_FILTER_MIN_SDK {
        let command = format!("pm list packages -U {quoted}");
        let output = adb::shell(adb_path, serial, &command, LOOKUP_TIMEOUT).await?;
        let uids = parse_package_uids(&output.stdout, package);
        if uids.is_empty() {
            return Err(AppError::Invalid(format!("{package} is not installed")));
        }
        return Ok(AppScope::Uids(uids));
    }

    let command = format!("pidof -s {quoted}");
    let output = adb::shell(adb_path, serial, &command, LOOKUP_TIMEOUT).await?;
    output
        .stdout
        .trim()
        .parse()
        .map(AppScope::Pid)
        .map_err(|_| AppError::Invalid(format!("{package} is not running")))
}

/// Spawn `adb -s <serial> logcat <args>` with piped output for a caller that forwards the lines to
/// the UI. The child is killed when dropped.
pub fn spawn_logcat(
    adb_path: &str,
    serial: &str,
    args: &[String],
) -> Result<tokio::process::Child> {
    let mut full = vec!["-s".to_string(), serial.to_string(), "logcat".to_string()];
    full.extend_from_slice(args);

    process::base_command(adb_path, &full)
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| AppError::CommandFailed(format!("Could not start adb logcat: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_exact_package_uids() {
        let text = "package:com.android.systemui.clocks.metro uid:10242\n\
                    package:com.android.systemui uid:10235,1010235\n";
        assert_eq!(
            parse_package_uids(text, "com.android.systemui"),
            vec![10235, 1010235]
        );
        assert!(parse_package_uids(text, "com.example").is_empty());
    }

    #[test]
    fn builds_logcat_args() {
        assert_eq!(logcat_args(None, None), ["-v", "threadtime"]);
        assert_eq!(
            logcat_args(Some(&AppScope::Uids(vec![10235, 1010235])), Some('W')),
            ["-v", "threadtime", "--uid=10235,1010235", "*:W"]
        );
        assert_eq!(
            logcat_args(Some(&AppScope::Pid(42)), None),
            ["-v", "threadtime", "--pid=42"]
        );
    }

    #[test]
    fn validates_levels() {
        assert_eq!(validate_level('w').unwrap(), 'W');
        assert!(validate_level('X').is_err());
    }
}
