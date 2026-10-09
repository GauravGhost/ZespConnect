//! Download a tool from upstream and unpack it into the app-managed directory. Deliberately
//! Tauri-free: the caller passes a progress callback, and everything is written under
//! `<app_data_dir>/tools`, which the user owns (no administrator rights).

use std::path::{Path, PathBuf};

use futures_util::StreamExt;
use tokio::io::AsyncWriteExt;

use crate::error::{AppError, Result};
use crate::models::ToolKind;
use crate::services::{tools, updates};

const PROGRESS_STEP: u64 = 10;

/// Download the latest `tool`, extract it to `<app_data_dir>/tools/<slot>/<version>/`, and return the binary path.
pub async fn install<F: Fn(String) + Send>(
    managed_dir: &Path,
    tool: ToolKind,
    on_line: F,
) -> Result<PathBuf> {
    let name = tool.name();
    let (version, url) = match tool {
        ToolKind::Scrcpy => updates::scrcpy_download().await?,
        ToolKind::Adb => updates::platform_tools_download().await?,
    };

    on_line(format!("Downloading {name} {version}"));

    let root = tools::managed_root(managed_dir);
    let work = root.join(".download");
    std::fs::create_dir_all(&work)?;
    let archive = work.join(archive_file_name(&url, name));

    download(&url, &archive, &on_line).await?;

    on_line("Extracting…".to_string());
    let target = root.join(tools::managed_slot(tool)).join(&version);
    if target.exists() {
        let _ = std::fs::remove_dir_all(&target);
    }
    std::fs::create_dir_all(&target)?;

    {
        let archive = archive.clone();
        let target = target.clone();
        tokio::task::spawn_blocking(move || extract(&archive, &target))
            .await
            .map_err(|e| AppError::Other(format!("extraction task failed: {e}")))??;
    }

    let binary = tools::find_binary(&target, name).ok_or_else(|| {
        AppError::Other(format!(
            "the {name} archive did not contain a {name} binary"
        ))
    })?;

    verify_companions(tool, &binary, &on_line)?;
    make_executable(&binary);

    let _ = std::fs::remove_file(&archive);
    on_line(format!("Installed {name} {version}"));
    Ok(binary)
}

/// Stream `url` into `dest`, reporting progress every 10%.
async fn download<F: Fn(String) + Send>(url: &str, dest: &Path, on_line: &F) -> Result<()> {
    let client = updates::download_client()?;
    let response = client
        .get(url)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|e| AppError::Other(format!("download failed: {e}")))?;

    let total = response.content_length();
    let mut file = tokio::fs::File::create(dest).await?;
    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;
    let mut milestone: u64 = PROGRESS_STEP;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| AppError::Other(format!("download interrupted: {e}")))?;
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;

        if let Some(total) = total.filter(|total| *total > 0) {
            let percent = downloaded.saturating_mul(100) / total;
            if percent >= milestone {
                on_line(format!("Downloading… {percent}%"));
                milestone = (percent / PROGRESS_STEP + 1) * PROGRESS_STEP;
            }
        }
    }

    file.flush().await?;
    Ok(())
}

fn extract(archive: &Path, dest: &Path) -> Result<()> {
    let lower = archive.to_string_lossy().to_lowercase();
    if lower.ends_with(".zip") {
        extract_zip(archive, dest)
    } else if lower.ends_with(".tar.gz") || lower.ends_with(".tgz") {
        extract_tar_gz(archive, dest)
    } else {
        Err(AppError::Other(format!(
            "unsupported archive format: {}",
            archive.display()
        )))
    }
}

fn extract_zip(archive: &Path, dest: &Path) -> Result<()> {
    let file = std::fs::File::open(archive)?;
    let mut zip = zip::ZipArchive::new(file)
        .map_err(|e| AppError::Other(format!("could not read zip: {e}")))?;

    for index in 0..zip.len() {
        let mut entry = zip
            .by_index(index)
            .map_err(|e| AppError::Other(format!("could not read zip entry: {e}")))?;

        // `enclosed_name` rejects entries that escape the destination (zip-slip).
        let Some(relative) = entry.enclosed_name() else {
            continue;
        };
        let out = dest.join(relative);

        if entry.is_dir() {
            std::fs::create_dir_all(&out)?;
            continue;
        }
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut writer = std::fs::File::create(&out)?;
        std::io::copy(&mut entry, &mut writer)?;

        #[cfg(unix)]
        if let Some(mode) = entry.unix_mode() {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&out, std::fs::Permissions::from_mode(mode));
        }
    }
    Ok(())
}

fn extract_tar_gz(archive: &Path, dest: &Path) -> Result<()> {
    let file = std::fs::File::open(archive)?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    // `unpack` refuses to write outside `dest`.
    archive
        .unpack(dest)
        .map_err(|e| AppError::Other(format!("could not extract tar.gz: {e}")))
}

/// Ensure the files scrcpy/adb need at runtime sit next to the binary.
fn verify_companions<F: Fn(String) + Send>(
    tool: ToolKind,
    binary: &Path,
    on_line: &F,
) -> Result<()> {
    let dir = binary.parent().unwrap_or_else(|| Path::new("."));
    if matches!(tool, ToolKind::Scrcpy) && !dir.join("scrcpy-server").is_file() {
        return Err(AppError::Other(
            "the scrcpy archive did not contain scrcpy-server".into(),
        ));
    }
    warn_missing_adb_dlls(tool, dir, on_line);
    Ok(())
}

/// Windows adb needs its two DLLs (shipped in the platform-tools zip); a no-op elsewhere.
#[cfg(windows)]
fn warn_missing_adb_dlls<F: Fn(String) + Send>(tool: ToolKind, dir: &Path, on_line: &F) {
    if !matches!(tool, ToolKind::Adb) {
        return;
    }
    for dll in ["AdbWinApi.dll", "AdbWinUsbApi.dll"] {
        if !dir.join(dll).is_file() {
            on_line(format!("warning: {dll} is missing next to adb.exe"));
        }
    }
}

#[cfg(not(windows))]
fn warn_missing_adb_dlls<F: Fn(String) + Send>(_tool: ToolKind, _dir: &Path, _on_line: &F) {}

#[cfg(unix)]
fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(metadata) = std::fs::metadata(path) {
        let mut permissions = metadata.permissions();
        permissions.set_mode(permissions.mode() | 0o755);
        let _ = std::fs::set_permissions(path, permissions);
    }
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) {}

fn archive_file_name(url: &str, fallback: &str) -> String {
    url.rsplit('/')
        .next()
        .filter(|segment| !segment.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("{fallback}.archive"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_a_file_name_from_the_url() {
        assert_eq!(
            archive_file_name("https://example.test/scrcpy-win64-v5.0.zip", "scrcpy"),
            "scrcpy-win64-v5.0.zip"
        );
        assert_eq!(
            archive_file_name(
                "https://example.test/platform-tools-latest-darwin.zip",
                "adb"
            ),
            "platform-tools-latest-darwin.zip"
        );
    }

    #[test]
    fn rejects_unknown_archive_formats() {
        let err = extract(Path::new("/tmp/whatever.rar"), Path::new("/tmp"));
        assert!(err.is_err());
    }

    #[test]
    fn scrcpy_requires_its_server_beside_the_binary() {
        let dir = std::env::temp_dir().join("zesp-companion-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let binary = dir.join("scrcpy");
        std::fs::write(&binary, b"stub").unwrap();

        let lines = std::sync::Mutex::new(Vec::new());
        let collect = |line: String| lines.lock().unwrap().push(line);
        assert!(verify_companions(ToolKind::Scrcpy, &binary, &collect).is_err());

        std::fs::write(dir.join("scrcpy-server"), b"stub").unwrap();
        assert!(verify_companions(ToolKind::Scrcpy, &binary, &collect).is_ok());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
