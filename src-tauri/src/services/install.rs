//! Install missing tools through the operating system's package manager.
//!
//! Kept free of Tauri types: the caller passes a callback that receives each
//! output line (the command layer forwards those to the UI as events).

use std::process::Stdio;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;

use crate::error::{AppError, Result};
use crate::models::ToolKind;
use crate::platform;

/// Installing can take a while (downloads, extraction).
const INSTALL_TIMEOUT: Duration = Duration::from_secs(900);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagerPlan {
    pub manager: &'static str,
    pub program: &'static str,
    pub args: Vec<String>,
}

fn argv(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| item.to_string()).collect()
}

/// Package-manager install commands for a tool, most preferred first.
#[cfg(windows)]
pub fn plans(tool: ToolKind) -> Vec<ManagerPlan> {
    match tool {
        ToolKind::Scrcpy => vec![
            ManagerPlan {
                manager: "winget",
                program: "winget",
                args: argv(&[
                    "install",
                    "--id",
                    "Genymobile.scrcpy",
                    "-e",
                    "--accept-source-agreements",
                    "--accept-package-agreements",
                ]),
            },
            ManagerPlan {
                manager: "scoop",
                program: "scoop",
                args: argv(&["install", "scrcpy"]),
            },
            ManagerPlan {
                manager: "chocolatey",
                program: "choco",
                args: argv(&["install", "scrcpy", "-y"]),
            },
        ],
        ToolKind::Adb => vec![
            ManagerPlan {
                manager: "winget",
                program: "winget",
                args: argv(&[
                    "install",
                    "--id",
                    "Google.PlatformTools",
                    "-e",
                    "--accept-source-agreements",
                    "--accept-package-agreements",
                ]),
            },
            ManagerPlan {
                manager: "scoop",
                program: "scoop",
                args: argv(&["install", "adb"]),
            },
            ManagerPlan {
                manager: "chocolatey",
                program: "choco",
                args: argv(&["install", "adb", "-y"]),
            },
        ],
    }
}

#[cfg(target_os = "macos")]
pub fn plans(tool: ToolKind) -> Vec<ManagerPlan> {
    match tool {
        ToolKind::Scrcpy => vec![ManagerPlan {
            manager: "homebrew",
            program: "brew",
            args: argv(&["install", "scrcpy"]),
        }],
        ToolKind::Adb => vec![ManagerPlan {
            manager: "homebrew",
            program: "brew",
            args: argv(&["install", "--cask", "android-platform-tools"]),
        }],
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
pub fn plans(tool: ToolKind) -> Vec<ManagerPlan> {
    let scrcpy_pkg = "scrcpy";
    let adb_pkg = "android-tools";
    let adb_pkg_deb = "android-tools-adb";
    match tool {
        ToolKind::Scrcpy => vec![
            ManagerPlan {
                manager: "apt",
                program: "apt-get",
                args: argv(&["install", "-y", scrcpy_pkg]),
            },
            ManagerPlan {
                manager: "dnf",
                program: "dnf",
                args: argv(&["install", "-y", scrcpy_pkg]),
            },
            ManagerPlan {
                manager: "pacman",
                program: "pacman",
                args: argv(&["-S", "--noconfirm", scrcpy_pkg]),
            },
        ],
        ToolKind::Adb => vec![
            ManagerPlan {
                manager: "apt",
                program: "apt-get",
                args: argv(&["install", "-y", adb_pkg_deb]),
            },
            ManagerPlan {
                manager: "dnf",
                program: "dnf",
                args: argv(&["install", "-y", adb_pkg]),
            },
            ManagerPlan {
                manager: "pacman",
                program: "pacman",
                args: argv(&["-S", "--noconfirm", adb_pkg]),
            },
        ],
    }
}

/// Pick the first plan whose manager exists, using an injected predicate so the
/// choice is unit-testable.
pub fn select_with<F: Fn(&str) -> bool>(tool: ToolKind, exists: F) -> Option<ManagerPlan> {
    plans(tool).into_iter().find(|plan| exists(plan.program))
}

pub fn select(tool: ToolKind) -> Option<ManagerPlan> {
    select_with(tool, program_on_path)
}

fn program_on_path(program: &str) -> bool {
    let exe = format!("{program}{}", platform::EXE_SUFFIX);
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|dir| dir.join(&exe).is_file()))
        .unwrap_or(false)
}

/// Run an install plan, forwarding each output line to `on_line`.
pub async fn run<F: Fn(String) + Send>(plan: &ManagerPlan, on_line: F) -> Result<()> {
    let (program, args) = platform::prepare(plan.program, &plan.args);

    let mut std_cmd = std::process::Command::new(&program);
    std_cmd
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    platform::configure_command(&mut std_cmd);

    let mut child = tokio::process::Command::from(std_cmd)
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| AppError::CommandFailed(format!("Could not start {}: {e}", plan.manager)))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| AppError::Other("installer produced no output stream".into()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| AppError::Other("installer produced no error stream".into()))?;

    let (tx, mut rx) = mpsc::channel::<String>(256);
    let tx_err = tx.clone();

    let stdout_task = tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = tx.send(line).await;
        }
    });
    let stderr_task = tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = tx_err.send(line).await;
        }
    });

    let status = tokio::time::timeout(INSTALL_TIMEOUT, async {
        while let Some(line) = rx.recv().await {
            on_line(line);
        }
        child.wait().await
    })
    .await
    .map_err(|_| AppError::CommandFailed(format!("{} timed out", plan.manager)))?
    .map_err(|e| AppError::CommandFailed(e.to_string()))?;

    let _ = stdout_task.await;
    let _ = stderr_task.await;

    if status.success() {
        Ok(())
    } else {
        Err(AppError::CommandFailed(format!(
            "{} exited with code {:?}",
            plan.manager,
            status.code().unwrap_or(-1)
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plans_exist_for_both_tools() {
        assert!(!plans(ToolKind::Adb).is_empty());
        assert!(!plans(ToolKind::Scrcpy).is_empty());
    }

    #[test]
    fn selection_prefers_the_first_available_manager() {
        let available = plans(ToolKind::Scrcpy);
        let preferred = available[0].program;
        let chosen = select_with(ToolKind::Scrcpy, |program| program == preferred);
        assert_eq!(chosen.map(|plan| plan.program), Some(preferred));
    }

    #[test]
    fn selection_falls_back_to_a_later_manager() {
        let available = plans(ToolKind::Scrcpy);
        let last = available.last().map(|plan| plan.program);
        let chosen = select_with(ToolKind::Scrcpy, |program| Some(program) == last);
        assert_eq!(chosen.map(|plan| plan.program), last);
    }

    #[test]
    fn selection_is_none_without_a_manager() {
        assert!(select_with(ToolKind::Scrcpy, |_| false).is_none());
    }
}
