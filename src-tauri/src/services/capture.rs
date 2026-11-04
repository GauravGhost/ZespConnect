//! Where screenshots and recordings are saved on the computer.

use std::path::PathBuf;

use crate::error::{AppError, Result};

const APP_FOLDER: &str = "Zesp Connect";

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureDirs {
    pub pictures: String,
    pub videos: String,
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
}
