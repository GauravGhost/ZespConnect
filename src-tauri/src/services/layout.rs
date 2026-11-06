use crate::error::{AppError, Result};
use crate::services::adb;

/// `/data/local/tmp` is writable by the shell user without storage permission.
const DUMP_PATH: &str = "/data/local/tmp/zesp_layout.xml";

/// One adb round trip: dump, print and delete. Failures are silenced on the device and detected
/// from the missing hierarchy instead, since uiautomator reports some errors with exit code 0.
fn dump_command() -> String {
    format!("uiautomator dump {DUMP_PATH} >/dev/null 2>&1; cat {DUMP_PATH} 2>/dev/null; rm -f {DUMP_PATH}")
}

pub fn layout_from_output(bytes: &[u8]) -> Result<String> {
    let text = String::from_utf8_lossy(bytes);
    if text.contains("<hierarchy") {
        Ok(text.trim().to_string())
    } else {
        Err(AppError::CommandFailed(
            "Could not read the screen layout. Wait until the screen is still and try again."
                .into(),
        ))
    }
}

/// The current screen's view hierarchy as uiautomator XML.
pub async fn dump(adb_path: &str, serial: &str) -> Result<String> {
    let bytes = adb::exec_out(adb_path, serial, &dump_command(), adb::DEFAULT_TIMEOUT).await?;
    layout_from_output(&bytes)
}

/// A PNG of the current screen, in the same pixel space as the layout bounds.
pub async fn screenshot(adb_path: &str, serial: &str) -> Result<Vec<u8>> {
    adb::exec_out(adb_path, serial, "screencap -p", adb::DEFAULT_TIMEOUT).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_a_dumped_hierarchy() {
        let xml = "<?xml version='1.0' ?><hierarchy rotation=\"0\"><node /></hierarchy>\n";
        assert_eq!(layout_from_output(xml.as_bytes()).unwrap(), xml.trim());
    }

    #[test]
    fn rejects_output_without_a_hierarchy() {
        assert!(layout_from_output(b"").is_err());
        assert!(layout_from_output(b"ERROR: could not get idle state.").is_err());
    }
}
