use serde::{Deserialize, Serialize};

/// One entry parsed from `adb shell ls -la`, shaped for the file browser.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    /// Absolute device path (`<dir>/<name>`), used for navigation and actions.
    pub path: String,
    pub is_directory: bool,
    /// Raw size in bytes; directories report their own inode size.
    pub size: u64,
    /// Human-readable size for the list.
    pub formatted_size: String,
    /// `ls` date and time joined (`2024-05-01 09:12`).
    pub date: String,
    /// The `ls -la` permission string (`drwxrwx--x`).
    pub permissions: String,
}
