//! App icons, read on the phone by a small helper (`android/icon-helper`) run through `app_process`.
//! scrcpy and adb have no way to get icons, so this is the one source.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

use base64::Engine;

use crate::error::{AppError, Result};
use crate::models::{AppInfo, ToolStatus};
use crate::services::adb;
use crate::util;
use crate::util::validation::validate_package;

/// Rebuilt with `scripts/build-icon-helper.sh`.
const HELPER_JAR: &[u8] = include_bytes!("../../resources/icon-helper.jar");
const HELPER_CLASS: &str = "com.zesp.IconDump";
const LOCAL_JAR: &str = "icon-helper.jar";
const REMOTE_JAR: &str = "/data/local/tmp/zesp-icon-helper.jar";
const ICONS_DIR: &str = "app-icons";
const ICON_EXT: &str = "png";

const PUSH_TIMEOUT: Duration = Duration::from_secs(15);
const DUMP_TIMEOUT: Duration = Duration::from_secs(60);

fn device_dir(app_data_dir: &Path, serial: &str) -> PathBuf {
    app_data_dir
        .join(ICONS_DIR)
        .join(util::safe_file_name(serial))
}

/// Parse the helper's `<package> <base64 png>` lines, skipping anything malformed.
pub fn parse_dump(text: &str) -> Vec<(String, Vec<u8>)> {
    let engine = base64::engine::general_purpose::STANDARD;
    text.lines()
        .filter_map(|line| {
            let (package, data) = line.trim().split_once(' ')?;
            validate_package(package).ok()?;
            let png = engine.decode(data).ok()?;
            Some((package.to_string(), png))
        })
        .collect()
}

/// Read every launchable app's icon from the phone and replace the saved set for `serial`.
pub async fn refresh(tools: &ToolStatus, serial: &str, app_data_dir: &Path) -> Result<()> {
    let adb_path = adb::adb_path(tools)?;

    let local_jar = app_data_dir.join(LOCAL_JAR);
    std::fs::write(&local_jar, HELPER_JAR)?;
    let pushed = adb::push(
        &adb_path,
        serial,
        &local_jar.to_string_lossy(),
        REMOTE_JAR,
        PUSH_TIMEOUT,
    )
    .await?;
    if !pushed.success {
        return Err(AppError::CommandFailed(pushed.text()));
    }

    let command = format!("CLASSPATH={REMOTE_JAR} app_process / {HELPER_CLASS}");
    let output = adb::exec_out(&adb_path, serial, &command, DUMP_TIMEOUT).await?;
    let icons = parse_dump(&String::from_utf8_lossy(&output));
    if icons.is_empty() {
        return Err(AppError::CommandFailed(format!(
            "No app icons reported for {serial}"
        )));
    }

    let dir = device_dir(app_data_dir, serial);
    // Start clean so icons of uninstalled apps do not linger.
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir)?;
    for (package, png) in icons {
        std::fs::write(dir.join(format!("{package}.{ICON_EXT}")), png)?;
    }
    Ok(())
}

/// Fill in `icon_path` for the apps that have a saved icon.
pub fn attach(apps: &mut [AppInfo], app_data_dir: &Path, serial: &str) {
    let dir = device_dir(app_data_dir, serial);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return;
    };
    let saved: HashSet<String> = entries
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();

    for app in apps {
        let file = format!("{}.{ICON_EXT}", app.package);
        app.icon_path = saved
            .contains(&file)
            .then(|| dir.join(&file).to_string_lossy().into_owned());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::LabelSource;

    fn app(package: &str) -> AppInfo {
        AppInfo {
            package: package.to_string(),
            label: package.to_string(),
            is_system: false,
            version_name: None,
            label_source: LabelSource::Derived,
            icon_path: None,
        }
    }

    #[test]
    fn parses_dump_and_skips_bad_lines() {
        let text = "com.example.a aGVsbG8=\r\nnot-a-line\ncom.example.b !!!\n../evil aGVsbG8=\n";
        let icons = parse_dump(text);
        assert_eq!(
            icons,
            vec![("com.example.a".to_string(), b"hello".to_vec())]
        );
    }

    #[test]
    fn attaches_only_saved_icons() {
        let root = std::env::temp_dir().join("zesp-app-icons-test");
        let _ = std::fs::remove_dir_all(&root);
        let dir = device_dir(&root, "192.168.1.5:5555");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("com.example.a.png"), b"png").unwrap();

        let mut apps = vec![app("com.example.a"), app("com.example.b")];
        attach(&mut apps, &root, "192.168.1.5:5555");

        assert!(apps[0]
            .icon_path
            .as_deref()
            .unwrap()
            .ends_with("com.example.a.png"));
        assert!(apps[1].icon_path.is_none());
        let _ = std::fs::remove_dir_all(&root);
    }
}
