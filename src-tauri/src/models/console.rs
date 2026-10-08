use serde::{Deserialize, Serialize};

/// The result of one `adb shell` command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellResult {
    pub stdout: String,
    pub stderr: String,
    /// The remote command's exit code (`adb shell` propagates it).
    pub code: i32,
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
