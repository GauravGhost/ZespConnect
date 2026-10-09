//! Device file browsing and transfer over adb. Listing uses `ls -la <path>/` (the trailing slash
//! follows `/sdcard`, a symlink) and parses it into [`FileEntry`]s; parsing is pure and unit-tested.

use std::time::Duration;

use crate::error::{AppError, Result};
use crate::models::{FileEntry, ToolStatus};
use crate::services::adb;
use crate::util::process::ProcessOutput;
use crate::util::validation::{shell_quote, validate_remote_path};

const LIST_TIMEOUT: Duration = Duration::from_secs(20);
const MUTATE_TIMEOUT: Duration = Duration::from_secs(20);
const TRANSFER_TIMEOUT: Duration = Duration::from_secs(600);

// ---------------------------------------------------------------------------
// Parsers
// ---------------------------------------------------------------------------

/// True for an `ls -la` permission column (`drwxr-xr-x`, `lrwxrwxrwx`, …).
fn looks_like_permissions(value: &str) -> bool {
    let mut chars = value.chars();
    let kind = matches!(chars.next(), Some('-' | 'd' | 'l' | 'c' | 'b' | 's' | 'p'));
    kind && value.len() >= 10
        && value[1..].chars().all(|c| {
            matches!(
                c,
                'r' | 'w' | 'x' | 's' | 'S' | 't' | 'T' | '-' | '+' | '.' | '@'
            )
        })
}

/// Human-readable byte size (`1.0 MB`, `512 B`).
pub fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }

    let mut value = bytes as f64;
    let mut index = 0;
    while value >= 1024.0 && index < UNITS.len() - 1 {
        value /= 1024.0;
        index += 1;
    }
    format!("{value:.1} {}", UNITS[index])
}

/// Join a directory path and an entry name with exactly one separator.
pub fn join_path(base: &str, name: &str) -> String {
    if base.ends_with('/') {
        format!("{base}{name}")
    } else {
        format!("{base}/{name}")
    }
}

/// Parse `ls -la` into entries rooted at `base`, directories first then case-insensitive by name; skips `.`/`..`.
pub fn parse_ls(text: &str, base: &str) -> Vec<FileEntry> {
    let mut entries = Vec::new();

    for raw in text.lines() {
        let line = raw.trim_start();
        if line.is_empty() || line.starts_with("total ") {
            continue;
        }

        let mut fields = line.split_whitespace();
        let Some(permissions) = fields.next() else {
            continue;
        };
        if !looks_like_permissions(permissions) {
            continue;
        }

        let (Some(_links), Some(_owner), Some(_group), Some(size), Some(date), Some(time)) = (
            fields.next(),
            fields.next(),
            fields.next(),
            fields.next(),
            fields.next(),
            fields.next(),
        ) else {
            continue;
        };

        let name = fields.collect::<Vec<_>>().join(" ");
        if name.is_empty() || name == "." || name == ".." {
            continue;
        }
        let name = name.split(" -> ").next().unwrap_or(&name).to_string();

        let is_directory = permissions.starts_with('d');
        let size = size.parse::<u64>().unwrap_or(0);
        entries.push(FileEntry {
            path: join_path(base, &name),
            name,
            is_directory,
            size,
            formatted_size: format_size(size),
            date: format!("{date} {time}"),
            permissions: permissions.to_string(),
        });
    }

    entries.sort_by(|a, b| {
        b.is_directory
            .cmp(&a.is_directory)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    entries
}

// ---------------------------------------------------------------------------
// I/O
// ---------------------------------------------------------------------------

fn ensure_success(output: &ProcessOutput) -> Result<()> {
    if output.success {
        Ok(())
    } else {
        Err(AppError::CommandFailed(output.text()))
    }
}

pub async fn list(tools: &ToolStatus, serial: &str, path: &str) -> Result<Vec<FileEntry>> {
    validate_remote_path(path)?;
    let adb_path = adb::adb_path(tools)?;

    let target = if path.ends_with('/') {
        path.to_string()
    } else {
        format!("{path}/")
    };
    let command = format!("ls -la {}", shell_quote(&target));
    let output = adb::shell(&adb_path, serial, &command, LIST_TIMEOUT).await?;
    if !output.success && output.stdout.trim().is_empty() {
        return Err(AppError::CommandFailed(output.text()));
    }
    Ok(parse_ls(&output.stdout, path))
}

pub async fn pull(tools: &ToolStatus, serial: &str, remote: &str, local: &str) -> Result<()> {
    validate_remote_path(remote)?;
    if local.trim().is_empty() {
        return Err(AppError::Invalid("Missing local destination".into()));
    }
    let adb_path = adb::adb_path(tools)?;
    let args = vec!["pull".to_string(), remote.to_string(), local.to_string()];
    let output = adb::run_scoped(&adb_path, serial, &args, TRANSFER_TIMEOUT).await?;
    ensure_success(&output)
}

pub async fn push(tools: &ToolStatus, serial: &str, local: &str, remote: &str) -> Result<()> {
    validate_remote_path(remote)?;
    if local.trim().is_empty() {
        return Err(AppError::Invalid("Missing local source".into()));
    }
    let adb_path = adb::adb_path(tools)?;
    let args = vec!["push".to_string(), local.to_string(), remote.to_string()];
    let output = adb::run_scoped(&adb_path, serial, &args, TRANSFER_TIMEOUT).await?;
    ensure_success(&output)
}

pub async fn create_dir(tools: &ToolStatus, serial: &str, path: &str) -> Result<()> {
    validate_remote_path(path)?;
    let adb_path = adb::adb_path(tools)?;
    let command = format!("mkdir -p {}", shell_quote(path));
    let output = adb::shell(&adb_path, serial, &command, MUTATE_TIMEOUT).await?;
    ensure_success(&output)
}

pub async fn delete(tools: &ToolStatus, serial: &str, path: &str) -> Result<()> {
    validate_remote_path(path)?;
    let adb_path = adb::adb_path(tools)?;
    let command = format!("rm -rf {}", shell_quote(path));
    let output = adb::shell(&adb_path, serial, &command, MUTATE_TIMEOUT).await?;
    ensure_success(&output)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LS: &str = include_str!("../../tests/fixtures/adb_ls_la.txt");

    #[test]
    fn parses_directories_before_files() {
        let entries = parse_ls(LS, "/sdcard");

        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "Android",
                "DCIM",
                "Downloads",
                "My Notes.txt",
                "photo.png",
                "sdcard",
            ]
        );
    }

    #[test]
    fn classifies_entries_and_builds_paths() {
        let entries = parse_ls(LS, "/sdcard");

        let dir = entries.iter().find(|e| e.name == "DCIM").expect("DCIM");
        assert!(dir.is_directory);
        assert_eq!(dir.path, "/sdcard/DCIM");

        let file = entries
            .iter()
            .find(|e| e.name == "My Notes.txt")
            .expect("notes file");
        assert!(!file.is_directory);
        assert_eq!(file.size, 1_048_576);
        assert_eq!(file.formatted_size, "1.0 MB");
        assert_eq!(file.date, "2024-05-02 14:22");
        assert_eq!(file.permissions, "-rw-rw----");
    }

    #[test]
    fn strips_symlink_targets_and_keeps_them_out_of_dirs() {
        let entries = parse_ls(LS, "/sdcard");
        let link = entries
            .iter()
            .find(|e| e.name == "sdcard")
            .expect("symlink");
        assert!(!link.is_directory);
        assert_eq!(link.path, "/sdcard/sdcard");
    }

    #[test]
    fn skips_dot_entries_and_non_listing_lines() {
        let text = "\
total 4
drwxr-xr-x 2 root root 4096 2024-05-01 09:00 .
drwxr-xr-x 4 root root 4096 2024-05-01 09:00 ..
-rw-r--r-- 1 root root  100 2024-05-01 09:00 keep.txt
not a listing line
";
        let entries = parse_ls(text, "/data");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, "/data/keep.txt");
    }

    #[test]
    fn formats_sizes_with_a_sensible_unit() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(5_368_709_120), "5.0 GB");
    }

    #[test]
    fn joins_paths_without_doubling_the_separator() {
        assert_eq!(join_path("/sdcard", "DCIM"), "/sdcard/DCIM");
        assert_eq!(join_path("/sdcard/", "DCIM"), "/sdcard/DCIM");
    }
}
