//! Where screenshots and recordings are saved on the computer.

use std::path::{Path, PathBuf};

use crate::error::{AppError, Result};

const APP_FOLDER: &str = "Zesp Connect";

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureDirs {
    pub pictures: String,
    pub videos: String,
}

/// `Zesp_<model>_<stamp>.<ext>`, falling back to the serial when the model is unknown.
pub fn file_name(model: &str, serial: &str, stamp: &str, ext: &str) -> String {
    let source = if model.trim().is_empty() {
        serial
    } else {
        model.trim()
    };
    let safe: String = source
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!("Zesp_{safe}_{stamp}.{ext}")
}

/// Write `bytes` to `dir/name`, creating the folder; returns the full path.
pub fn save_to_folder(dir: &Path, name: &str, bytes: &[u8]) -> Result<PathBuf> {
    let failed = |error: std::io::Error| {
        AppError::Io(format!("Could not save to {}: {error}", dir.display()))
    };
    std::fs::create_dir_all(dir).map_err(failed)?;
    let path = dir.join(name);
    std::fs::write(&path, bytes).map_err(failed)?;
    Ok(path)
}

/// Undo `encodeURIComponent`, which carries non-ASCII paths through ASCII-only IPC headers.
pub fn decode_component(value: &str) -> Result<String> {
    let invalid = || AppError::Invalid(format!("Invalid folder: {value}"));
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let hex = value.get(index + 1..index + 3).ok_or_else(invalid)?;
            decoded.push(u8::from_str_radix(hex, 16).map_err(|_| invalid())?);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).map_err(|_| invalid())
}

/// `<Pictures>/Zesp Connect` and `<Videos>/Zesp Connect`; created only when something is saved.
pub fn default_dirs() -> Result<CaptureDirs> {
    let in_folder = |base: Option<PathBuf>, kind: &str| {
        // Headless Linux may define no XDG media folders.
        base.or_else(|| dirs::home_dir().map(|home| home.join(kind)))
            .map(|dir| dir.join(APP_FOLDER).to_string_lossy().into_owned())
            .ok_or_else(|| AppError::Other(format!("This computer has no {kind} folder")))
    };
    Ok(CaptureDirs {
        pictures: in_folder(dirs::picture_dir(), "Pictures")?,
        videos: in_folder(dirs::video_dir(), "Videos")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_dirs_end_in_zesp_connect() {
        let dirs = default_dirs().unwrap();
        assert!(dirs.pictures.ends_with("Zesp Connect"));
        assert!(dirs.videos.ends_with("Zesp Connect"));
    }

    #[test]
    fn decodes_uri_components() {
        assert_eq!(
            decode_component("C%3A%5CUsers%5C%E0%A4%85%20b").unwrap(),
            "C:\\Users\\\u{905} b"
        );
        assert!(decode_component("%E0%A4").is_err());
        assert!(decode_component("%zz").is_err());
    }

    const STAMP: &str = "2026-11-04_10-00-00";

    #[test]
    fn file_name_sanitizes_model() {
        assert_eq!(
            file_name("Pixel 8/Pro", "abc", STAMP, "png"),
            "Zesp_Pixel_8_Pro_2026-11-04_10-00-00.png"
        );
    }

    #[test]
    fn file_name_falls_back_to_serial() {
        assert_eq!(
            file_name("", "192.168.1.5:5555", STAMP, "mp4"),
            "Zesp_192_168_1_5_5555_2026-11-04_10-00-00.mp4"
        );
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("zesp-capture-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn save_creates_missing_folder() {
        let root = scratch("missing");
        let path = save_to_folder(&root.join("a").join("b"), "shot.png", b"png").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"png");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn save_reports_unwritable_folder() {
        let root = scratch("file");
        std::fs::create_dir_all(&root).unwrap();
        let blocker = root.join("not-a-dir");
        std::fs::write(&blocker, b"x").unwrap();
        assert!(save_to_folder(&blocker, "shot.png", b"png").is_err());
        let _ = std::fs::remove_dir_all(root);
    }
}
