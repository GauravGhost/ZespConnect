use std::collections::HashMap;
use std::process::Stdio;

use crate::error::{AppError, Result};
use crate::models::ProcessInfo;
use crate::services::apps;
use crate::util::process;
use crate::util::validation::validate_package;

/// toybox top's own sampling interval; it keeps one process running instead of one per refresh.
const REFRESH_SECS: u32 = 3;
const MAX_ROWS: u32 = 500;
const COLUMNS: &str = "PID,USER,%CPU,%MEM,RES,S,ARGS";

/// Stream `top` in batch mode; dropping the child ends it on the phone too.
pub fn spawn_top(adb_path: &str, serial: &str) -> Result<tokio::process::Child> {
    let args = vec![
        "-s".to_string(),
        serial.to_string(),
        "shell".to_string(),
        format!("top -b -d {REFRESH_SECS} -m {MAX_ROWS} -o {COLUMNS}"),
    ];
    process::base_command(adb_path, &args)
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| AppError::CommandFailed(format!("Could not start adb shell: {e}")))
}

/// Gathers `top` lines into whole refreshes. Each refresh starts with a `Tasks:` summary and
/// ends with blank lines, so either one closes the refresh being collected.
pub struct TopFrames {
    labels: HashMap<String, String>,
    rows: Vec<ProcessInfo>,
}

impl TopFrames {
    pub fn new(labels: HashMap<String, String>) -> Self {
        TopFrames {
            labels,
            rows: Vec::new(),
        }
    }

    /// Feed one line; returns a finished refresh when this line closes one.
    pub fn push(&mut self, line: &str) -> Option<Vec<ProcessInfo>> {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("Tasks:") {
            return (!self.rows.is_empty()).then(|| std::mem::take(&mut self.rows));
        }
        if let Some(row) = parse_row(trimmed, &self.labels) {
            self.rows.push(row);
        }
        None
    }
}

fn parse_row(line: &str, labels: &HashMap<String, String>) -> Option<ProcessInfo> {
    let mut fields = line.split_whitespace();
    let pid = fields.next()?.parse().ok()?;
    let user = fields.next()?.to_string();
    let cpu = fields.next()?.parse().ok()?;
    let mem = fields.next()?.parse().ok()?;
    let res_kb = parse_size_kb(fields.next()?)?;
    let state = fields.next()?.to_string();
    let name = fields.collect::<Vec<_>>().join(" ");
    if name.is_empty() {
        return None;
    }
    let package = app_package(&user, &name);
    let label = package
        .as_deref()
        .map(|package| apps::display_label(labels, package));
    Some(ProcessInfo {
        pid,
        user,
        cpu,
        mem,
        res_kb,
        state,
        name,
        package,
        label,
    })
}

/// toybox prints sizes as `0`, `980K`, `5.4M` or `1.2G`; a bare number is already kilobytes.
fn parse_size_kb(text: &str) -> Option<u64> {
    let (number, multiplier) = match text.chars().last()? {
        'K' => (&text[..text.len() - 1], 1.0),
        'M' => (&text[..text.len() - 1], 1024.0),
        'G' => (&text[..text.len() - 1], 1024.0 * 1024.0),
        'T' => (&text[..text.len() - 1], 1024.0 * 1024.0 * 1024.0),
        _ => (text, 1.0),
    };
    number
        .parse::<f64>()
        .ok()
        .map(|value| (value * multiplier) as u64)
}

/// App processes run as `u<user>_a<id>` (or `u0_i<id>` for isolated ones) and are named after
/// their package, optionally with a `:process` suffix.
fn app_package(user: &str, name: &str) -> Option<String> {
    let is_app_user = user.starts_with('u') && (user.contains("_a") || user.contains("_i"));
    if !is_app_user {
        return None;
    }
    let package = name.split(':').next()?;
    validate_package(package).ok()?;
    Some(package.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME: &str = "Tasks: 785 total,   1 running, 784 sleeping,   0 stopped,   0 zombie
  Mem:  7754700K total,  6967384K used,   787316K free,     1100K buffers
 Swap:  3877344K total,  3107840K used,   769504K free,  1730464K cached
900%cpu  54%user   5%nice  68%sys 754%idle   0%iow  14%irq   5%sirq   0%host
  PID USER        [%CPU] %MEM  RES S ARGS
31875 u0_a313      18.9   1.3 105M S com.huami.watch.hmwatchmanager
 1721 system       10.8   7.1 544M S system_server
 4386 root          5.4   0.0    0 I [kworker/u18:0-async_vote_wq]
 6001 u0_a292       1.0   0.4 9.5M S com.resilio.sync:remote
 8257 shell        32.4   0.0 5.4M R top -b -d 3

";

    fn frames(text: &str) -> Vec<Vec<ProcessInfo>> {
        let mut labels = HashMap::new();
        labels.insert("com.resilio.sync".to_string(), "Resilio Sync".to_string());
        let mut parser = TopFrames::new(labels);
        text.lines().filter_map(|line| parser.push(line)).collect()
    }

    #[test]
    fn blank_line_closes_a_refresh() {
        let frames = frames(FRAME);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].len(), 5);
    }

    #[test]
    fn next_summary_also_closes_a_refresh() {
        let text = FRAME.trim_end().to_string() + "\n" + FRAME.lines().next().unwrap();
        assert_eq!(frames(&text).len(), 1);
    }

    #[test]
    fn parses_rows_and_app_packages() {
        let rows = &frames(FRAME)[0];
        let watch = &rows[0];
        assert_eq!(watch.pid, 31875);
        assert_eq!(watch.cpu, 18.9);
        assert_eq!(watch.res_kb, 105 * 1024);
        assert_eq!(
            watch.package.as_deref(),
            Some("com.huami.watch.hmwatchmanager")
        );

        assert_eq!(rows[1].package, None);
        assert_eq!(rows[2].name, "[kworker/u18:0-async_vote_wq]");
        assert_eq!(rows[2].res_kb, 0);
        assert_eq!(rows[3].package.as_deref(), Some("com.resilio.sync"));
        assert_eq!(rows[3].label.as_deref(), Some("Resilio Sync"));
        assert_eq!(rows[4].name, "top -b -d 3");
        assert_eq!(rows[4].package, None);
    }

    #[test]
    fn sizes_become_kilobytes() {
        assert_eq!(parse_size_kb("980K"), Some(980));
        assert_eq!(parse_size_kb("5.4M"), Some(5529));
        assert_eq!(parse_size_kb("1.5G"), Some(1_572_864));
        assert_eq!(parse_size_kb("12"), Some(12));
        assert_eq!(parse_size_kb("x"), None);
    }
}
