use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CrashKind {
    Crash,
    NativeCrash,
    Anr,
}

/// One crash or ANR the phone recorded in its dropbox.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrashReport {
    pub kind: CrashKind,
    /// The dropbox tag, e.g. `data_app_crash`.
    pub source: String,
    /// Device local time as the phone printed it, `YYYY-MM-DD HH:MM:SS`.
    pub time: String,
    /// Package name, or the process name when the entry names no package (e.g. `system_server`).
    pub app: String,
    /// One line: the exception, the ANR reason or the fatal signal.
    pub summary: String,
    /// The full entry: headers, then the stack trace.
    pub details: String,
    pub truncated: bool,
}
