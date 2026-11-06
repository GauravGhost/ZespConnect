use serde::Serialize;

/// One row of the phone's process table.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessInfo {
    pub pid: u32,
    pub user: String,
    /// Share of one core, so a busy process on an 8-core phone can read up to 800.
    pub cpu: f32,
    pub mem: f32,
    pub res_kb: u64,
    /// Kernel state letter: R running, S sleeping, D waiting on I/O, Z zombie...
    pub state: String,
    pub name: String,
    /// The app package a process belongs to (`com.app:service` -> `com.app`), when it is an app.
    pub package: Option<String>,
    pub label: Option<String>,
}
