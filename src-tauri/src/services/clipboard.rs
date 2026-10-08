//! Device clipboard access through the AdbClipboard helper app, plus the host
//! clipboard via `arboard`.
//!
//! Android only lets a foreground app read its clipboard, so device → desktop
//! reads go through [AdbClipboard](https://github.com/PRosenb/AdbClipboard):
//! writing broadcasts an intent, and the last text it read is left in a plain
//! file on the device. The desktop side uses `arboard` directly (not a webview
//! plugin) so copying still works when the window is hidden.

use std::time::Duration;

use crate::error::{AppError, Result};
use crate::models::ToolStatus;
use crate::services::adb;
use crate::util::validation::shell_quote;

const ACTION_TIMEOUT: Duration = Duration::from_secs(15);

/// The device app that exposes the clipboard over adb.
const HELPER_PACKAGE: &str = "ch.pete.adbclipboard";

/// Where AdbClipboard leaves the text it last read from the device clipboard.
const CLIPBOARD_FILE: &str = "/sdcard/Android/data/ch.pete.adbclipboard/files/clipboard.txt";

/// The `am broadcast …` command that writes `text` to the device clipboard.
pub fn write_command(text: &str) -> String {
    format!(
        "am broadcast -a {HELPER_PACKAGE}.WRITE -n {HELPER_PACKAGE}/.WriteReceiver -e text {}",
        shell_quote(text)
    )
}

fn helper_missing(output: &str) -> bool {
    let text = output.to_lowercase();
    text.contains("unable to resolve")
        || text.contains("does not exist")
        || text.contains("not found")
}

/// Send `text` to the device clipboard.
pub async fn send(tools: &ToolStatus, serial: &str, text: &str) -> Result<()> {
    let adb_path = adb::adb_path(tools)?;
    let output = adb::shell(&adb_path, serial, &write_command(text), ACTION_TIMEOUT).await?;
    let text_out = output.text();

    if helper_missing(&text_out) {
        return Err(AppError::Invalid(format!(
            "AdbClipboard ({HELPER_PACKAGE}) is not installed on the device"
        )));
    }
    if !output.success {
        return Err(AppError::CommandFailed(text_out));
    }
    Ok(())
}

/// Read the text AdbClipboard last exposed from the device clipboard.
pub async fn read(tools: &ToolStatus, serial: &str) -> Result<String> {
    let adb_path = adb::adb_path(tools)?;
    let command = format!("cat {CLIPBOARD_FILE}");
    let output = adb::shell(&adb_path, serial, &command, ACTION_TIMEOUT).await?;
    if !output.success {
        return Err(AppError::Invalid(format!(
            "No device clipboard yet. Install AdbClipboard ({HELPER_PACKAGE}) and tap its floating window to publish the clipboard."
        )));
    }
    Ok(output.stdout.trim_end_matches(['\n', '\r']).to_string())
}

/// Put `text` on the desktop clipboard.
pub fn copy_to_host(text: &str) -> Result<()> {
    let mut clipboard = arboard::Clipboard::new()
        .map_err(|e| AppError::Other(format!("Desktop clipboard unavailable: {e}")))?;
    clipboard
        .set_text(text.to_string())
        .map_err(|e| AppError::Other(format!("Could not write to the desktop clipboard: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_command_targets_the_write_receiver_and_quotes_text() {
        let command = write_command("hello world");
        assert!(command.contains("-a ch.pete.adbclipboard.WRITE"));
        assert!(command.contains("-n ch.pete.adbclipboard/.WriteReceiver"));
        assert!(command.ends_with("-e text 'hello world'"));
    }

    #[test]
    fn detects_a_missing_helper_app() {
        assert!(helper_missing("Error: Unable to resolve intent"));
        assert!(!helper_missing("Broadcast completed: result=0"));
    }
}
