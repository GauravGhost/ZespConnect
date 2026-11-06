use std::time::Duration;

use crate::error::{AppError, Result};
use crate::models::{CrashKind, CrashReport};
use crate::services::adb;

/// A bug report usually takes one to five minutes; slow phones over Wi-Fi take longer.
const BUG_REPORT_TIMEOUT: Duration = Duration::from_secs(15 * 60);

/// Dropbox tags for app and system crashes, native crashes and ANRs. Missing tags print nothing.
const TAGS: &[&str] = &[
    "data_app_crash",
    "data_app_native_crash",
    "data_app_anr",
    "system_app_crash",
    "system_app_native_crash",
    "system_app_anr",
    "system_server_crash",
    "system_server_native_crash",
    "system_server_anr",
];

/// `dumpsys dropbox --print` starts every entry with this line.
const ENTRY_SEPARATOR: &str = "========================================";
/// First line of the preamble each dumpsys run prints before its entries.
const PREAMBLE_START: &str = "Drop box contents:";

/// ANR entries carry every thread's stack; the first part holds the reason and the main thread.
const MAX_DETAILS_BYTES: usize = 64 * 1024;

/// One adb round trip; `--print` takes a single tag, so each tag gets its own dumpsys.
fn dropbox_command() -> String {
    TAGS.iter()
        .map(|tag| format!("dumpsys dropbox --print {tag}"))
        .collect::<Vec<_>>()
        .join("; ")
}

fn kind_of(tag: &str) -> Option<CrashKind> {
    if tag.ends_with("_native_crash") {
        Some(CrashKind::NativeCrash)
    } else if tag.ends_with("_anr") {
        Some(CrashKind::Anr)
    } else if tag.ends_with("_crash") {
        Some(CrashKind::Crash)
    } else {
        None
    }
}

fn header<'a>(headers: &[&'a str], key: &str) -> Option<&'a str> {
    headers.iter().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        (name == key)
            .then(|| value.trim())
            .filter(|value| !value.is_empty())
    })
}

fn summary(kind: CrashKind, headers: &[&str], trace: &[&str]) -> String {
    let found = match kind {
        CrashKind::Crash => trace
            .iter()
            .map(|line| line.trim())
            .find(|line| !line.is_empty()),
        CrashKind::Anr => header(headers, "Subject"),
        CrashKind::NativeCrash => trace
            .iter()
            .find_map(|line| line.strip_prefix("Abort message:"))
            .map(|message| message.trim().trim_matches('\''))
            .or_else(|| {
                trace
                    .iter()
                    .copied()
                    .find(|line| line.starts_with("signal "))
            }),
    };
    let fallback = match kind {
        CrashKind::Crash => "Crash",
        CrashKind::NativeCrash => "Native crash",
        CrashKind::Anr => "App not responding",
    };
    found.unwrap_or(fallback).to_string()
}

fn truncate(text: &mut String) -> bool {
    if text.len() <= MAX_DETAILS_BYTES {
        return false;
    }
    let mut end = MAX_DETAILS_BYTES;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text.truncate(end);
    true
}

/// An entry is its title line (`<date> <time> <tag> (text, N bytes)`), a block of `Key: value`
/// headers, a blank line and the trace.
fn parse_entry(lines: &[&str]) -> Option<CrashReport> {
    let (title, body) = lines.split_first()?;
    let mut title = title.split_whitespace();
    let (date, time, tag) = (title.next()?, title.next()?, title.next()?);
    let kind = kind_of(tag)?;

    let header_end = body
        .iter()
        .position(|line| line.trim().is_empty())
        .unwrap_or(body.len());
    let (headers, trace) = body.split_at(header_end);
    let app = header(headers, "Package")
        .and_then(|package| package.split_whitespace().next())
        .or_else(|| header(headers, "Process"))
        .unwrap_or("unknown");

    let mut details = body.join("\n").trim_end().to_string();
    let truncated = truncate(&mut details);
    Some(CrashReport {
        kind,
        source: tag.to_string(),
        time: format!("{date} {time}"),
        app: app.to_string(),
        summary: summary(kind, headers, trace),
        details,
        truncated,
    })
}

/// Reports from `dumpsys dropbox --print` output, newest first. Each dumpsys run's preamble is
/// skipped so it does not end up in the trace of the previous run's last entry.
pub fn parse_dropbox(text: &str) -> Vec<CrashReport> {
    let mut entries: Vec<Vec<&str>> = Vec::new();
    let mut current: Option<Vec<&str>> = None;
    for line in text.lines() {
        if line == ENTRY_SEPARATOR {
            entries.extend(current.replace(Vec::new()));
        } else if line.starts_with(PREAMBLE_START) {
            entries.extend(current.take());
        } else if let Some(entry) = current.as_mut() {
            entry.push(line);
        }
    }
    entries.extend(current);
    let mut reports: Vec<CrashReport> = entries.iter().filter_map(|e| parse_entry(e)).collect();
    reports.sort_by(|a, b| b.time.cmp(&a.time));
    reports
}

pub async fn list(adb_path: &str, serial: &str) -> Result<Vec<CrashReport>> {
    let bytes = adb::exec_out(adb_path, serial, &dropbox_command(), adb::DEFAULT_TIMEOUT).await?;
    Ok(parse_dropbox(&String::from_utf8_lossy(&bytes)))
}

/// `adb bugreport` writes a zip of logs, dumpsys and traces; checked by the file it leaves behind
/// since adb exits 0 on some failures.
pub async fn bug_report(adb_path: &str, serial: &str, path: &str) -> Result<()> {
    let args = vec!["bugreport".to_string(), path.to_string()];
    let output = adb::run_scoped(adb_path, serial, &args, BUG_REPORT_TIMEOUT).await?;
    let written = tokio::fs::metadata(path)
        .await
        .is_ok_and(|meta| meta.len() > 0);
    if output.success && written {
        Ok(())
    } else {
        Err(AppError::CommandFailed(format!(
            "Could not capture a bug report: {}",
            output.text()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PREAMBLE: &str = "Drop box contents: 12 entries\n\
        Max entries: 1000\n\
        Low priority tags: {data_app_wtf, keymaster}\n\
        Searching for: TAG\n\n";

    fn run(tag: &str, entries: &[&str]) -> String {
        let mut text = PREAMBLE.replace("TAG", tag);
        if entries.is_empty() {
            text.push_str("(No entries found.)\n");
        }
        for entry in entries {
            text.push_str(ENTRY_SEPARATOR);
            text.push('\n');
            text.push_str(entry);
        }
        text
    }

    const APP_CRASH: &str = "2026-11-06 12:56:57 system_app_crash (text, 1247 bytes)\n\
        Process: com.google.android.calculator\n\
        PID: 17810\n\
        Package: com.google.android.calculator v85022643 (9.3 (967717812))\n\
        Dropped-Count: 0\n\
        \n\
        android.app.RemoteServiceException$CrashedByAdbException: shell-induced crash\n\
        \tat android.app.ActivityThread.main(ActivityThread.java:9901)\n\n";

    const SERVER_CRASH: &str = "2026-11-04 14:05:35 system_server_crash (text, 790 bytes)\n\
        Process: system_server\n\
        Dropped-Count: 0\n\
        \n\
        java.lang.AssertionError: Unexpected message type: 2\n\
        \tat java.lang.Thread.run(Thread.java:1571)\n\n";

    const ANR: &str = "2026-11-05 09:00:00 data_app_anr (text, 4000 bytes)\n\
        Process: com.example.app\n\
        Package: com.example.app v1 (1.0)\n\
        Subject: Input dispatching timed out (Waited 5001ms for KeyEvent)\n\
        \n\
        CPU usage from 0ms to 5000ms later\n\n";

    const NATIVE: &str = "2026-11-05 10:00:00 data_app_native_crash (text, 6000 bytes)\n\
        Process: com.example.app\n\
        \n\
        *** *** *** *** *** *** *** *** *** *** *** *** *** *** *** ***\n\
        signal 6 (SIGABRT), code -1 (SI_QUEUE), fault addr --------\n\
        Abort message: 'No pending exception expected'\n";

    #[test]
    fn reads_entries_across_dumpsys_runs_newest_first() {
        let text = [
            run("data_app_crash", &[]),
            run("data_app_anr", &[ANR]),
            run("system_app_crash", &[APP_CRASH]),
            run("system_server_crash", &[SERVER_CRASH]),
        ]
        .concat();
        let reports = parse_dropbox(&text);
        let apps: Vec<_> = reports.iter().map(|r| r.app.as_str()).collect();
        assert_eq!(
            apps,
            [
                "com.google.android.calculator",
                "com.example.app",
                "system_server"
            ]
        );
        // The next run's preamble must not leak into the previous entry.
        assert!(!reports[1].details.contains("Drop box contents"));
    }

    #[test]
    fn summarizes_each_kind() {
        let crash = &parse_dropbox(&run("system_app_crash", &[APP_CRASH]))[0];
        assert_eq!(crash.kind, CrashKind::Crash);
        assert_eq!(crash.source, "system_app_crash");
        assert_eq!(crash.time, "2026-11-06 12:56:57");
        assert_eq!(
            crash.summary,
            "android.app.RemoteServiceException$CrashedByAdbException: shell-induced crash"
        );
        assert!(crash
            .details
            .starts_with("Process: com.google.android.calculator"));
        assert!(crash.details.ends_with("(ActivityThread.java:9901)"));

        let anr = &parse_dropbox(&run("data_app_anr", &[ANR]))[0];
        assert_eq!(anr.kind, CrashKind::Anr);
        assert_eq!(
            anr.summary,
            "Input dispatching timed out (Waited 5001ms for KeyEvent)"
        );

        let native = &parse_dropbox(&run("data_app_native_crash", &[NATIVE]))[0];
        assert_eq!(native.kind, CrashKind::NativeCrash);
        assert_eq!(native.summary, "No pending exception expected");
    }

    #[test]
    fn caps_huge_entries() {
        let entry = format!("{ANR}{}\n", "x".repeat(MAX_DETAILS_BYTES * 2));
        let anr = &parse_dropbox(&run("data_app_anr", &[&entry]))[0];
        assert!(anr.truncated);
        assert_eq!(anr.details.len(), MAX_DETAILS_BYTES);
    }

    #[test]
    fn skips_other_tags() {
        let tombstone = "2026-11-05 01:08:50 SYSTEM_TOMBSTONE (compressed text, 6397 bytes)\nisPrevious: true\n";
        assert!(parse_dropbox(&run("SYSTEM_TOMBSTONE", &[tombstone])).is_empty());
    }
}
