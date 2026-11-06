use serde::{Deserialize, Serialize};

/// What a shell session streams to its tab: terminal output or the end of the session.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ShellEvent {
    Output {
        text: String,
    },
    /// `code` is `None` when adb was killed rather than exiting on its own.
    Exit {
        code: Option<i32>,
    },
}

/// Which OS pipe a streamed logcat line arrived on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogStream {
    Stdout,
    Stderr,
}

/// One line streamed from `adb logcat` into the in-app viewer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogLine {
    pub stream: LogStream,
    pub text: String,
}

/// What `adb logcat` should stream: one app's lines and/or a minimum severity.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogcatFilter {
    pub package: Option<String>,
    /// One of `VDIWEF`; lower levels are dropped on the device.
    pub min_level: Option<char>,
}
