//! `.apks` / `.xapk` bundles: one zip holding a split app's base and split APKs at its top level,
//! so the app can be shared and installed as a single file.

use std::fs::File;
use std::path::{Path, PathBuf};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::error::{AppError, Result};

const EXTENSIONS: &[&str] = &["apks", "xapk"];

pub fn is_bundle(path: &Path) -> bool {
    has_extension(path, EXTENSIONS)
}

pub fn is_apk(path: &Path) -> bool {
    has_extension(path, &["apk"])
}

fn has_extension(path: &Path, extensions: &[&str]) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            extensions
                .iter()
                .any(|wanted| ext.eq_ignore_ascii_case(wanted))
        })
}

fn zip_error(error: zip::result::ZipError) -> AppError {
    AppError::Other(format!("Could not read APK bundle: {error}"))
}

/// Zip `apks` into `bundle` under their file names. Entries are stored uncompressed because APKs
/// are already compressed.
pub fn write(bundle: &Path, apks: &[PathBuf]) -> Result<()> {
    let mut zip = ZipWriter::new(File::create(bundle)?);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    for apk in apks {
        let name = apk
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| AppError::Invalid(format!("Bad APK name: {}", apk.display())))?;
        zip.start_file(name, options).map_err(zip_error)?;
        std::io::copy(&mut File::open(apk)?, &mut zip)?;
    }
    zip.finish().map_err(zip_error)?;
    Ok(())
}

/// Extract the top-level APKs of `bundle` into `dest`. Anything else (OBB data, manifests,
/// icons) is skipped.
pub fn extract(bundle: &Path, dest: &Path) -> Result<Vec<PathBuf>> {
    let mut zip = ZipArchive::new(File::open(bundle)?).map_err(zip_error)?;
    let mut apks = Vec::new();
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index).map_err(zip_error)?;
        // `enclosed_name` rejects entries that escape `dest` (zip-slip).
        let Some(name) = entry.enclosed_name() else {
            continue;
        };
        if entry.is_dir() || name.components().count() != 1 || !is_apk(&name) {
            continue;
        }
        let out = dest.join(name);
        std::io::copy(&mut entry, &mut File::create(&out)?)?;
        apks.push(out);
    }
    if apks.is_empty() {
        return Err(AppError::Invalid(format!(
            "No APK found inside {}",
            bundle.display()
        )));
    }
    Ok(apks)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("zesp-bundle-test-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn recognises_bundle_and_apk_extensions() {
        assert!(is_bundle(Path::new(r"C:\Downloads\App.APKS")));
        assert!(is_bundle(Path::new("/home/me/app.xapk")));
        assert!(!is_bundle(Path::new("/home/me/app.apk")));
        assert!(is_apk(Path::new("/home/me/App.APK")));
        assert!(!is_apk(Path::new("/home/me/apk")));
    }

    #[test]
    fn written_bundle_extracts_to_the_same_apks() {
        let dir = temp_dir("roundtrip");
        let apks: Vec<PathBuf> = ["base.apk", "split_config.en.apk"]
            .iter()
            .map(|name| {
                let path = dir.join(name);
                std::fs::write(&path, name.as_bytes()).unwrap();
                path
            })
            .collect();
        let bundle = dir.join("com.a.apks");
        write(&bundle, &apks).unwrap();

        let out = dir.join("out");
        std::fs::create_dir_all(&out).unwrap();
        let extracted = extract(&bundle, &out).unwrap();

        assert_eq!(
            extracted,
            vec![out.join("base.apk"), out.join("split_config.en.apk")]
        );
        assert_eq!(
            std::fs::read(&extracted[1]).unwrap(),
            b"split_config.en.apk"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn extract_skips_nested_and_non_apk_entries() {
        let dir = temp_dir("skip");
        let bundle = dir.join("game.xapk");
        let mut zip = ZipWriter::new(File::create(&bundle).unwrap());
        for name in [
            "manifest.json",
            "Android/obb/game/main.obb",
            "nested/x.apk",
            "game.apk",
        ] {
            zip.start_file(name, SimpleFileOptions::default()).unwrap();
            zip.write_all(b"data").unwrap();
        }
        zip.finish().unwrap();

        let extracted = extract(&bundle, &dir).unwrap();
        assert_eq!(extracted, vec![dir.join("game.apk")]);

        let empty = dir.join("empty.apks");
        ZipWriter::new(File::create(&empty).unwrap())
            .finish()
            .unwrap();
        assert!(extract(&empty, &dir).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
