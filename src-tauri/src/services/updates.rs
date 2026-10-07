//! Look up the latest upstream versions of adb (Android platform-tools) and
//! scrcpy, so the UI can say "update available" instead of guessing.
//!
//! Pure helpers (version comparison, asset naming) are unit-tested; the network
//! calls are thin wrappers over the upstream APIs.

use std::cmp::Ordering;
use std::time::Duration;

use once_cell::sync::Lazy;
use regex::Regex;
use serde::Deserialize;

use crate::error::{AppError, Result};

const HTTP_TIMEOUT: Duration = Duration::from_secs(20);
const USER_AGENT: &str = concat!("ZespConnect/", env!("CARGO_PKG_VERSION"));

/// GitHub marks the latest non-prerelease scrcpy release here.
const SCRCPY_RELEASE_URL: &str = "https://api.github.com/repos/Genymobile/scrcpy/releases/latest";
/// Google's SDK repository manifest; carries the platform-tools revision.
const PLATFORM_TOOLS_REPO_URL: &str = "https://dl.google.com/android/repository/repository2-3.xml";

/// Pulls the `platform-tools` revision out of the (large) repository manifest.
static PLATFORM_TOOLS_REVISION_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"(?s)path="platform-tools".*?<revision>\s*<major>(\d+)</major>\s*(?:<minor>(\d+)</minor>)?\s*(?:<micro>(\d+)</micro>)?"#,
    )
    .expect("valid regex")
});

/// The latest versions published upstream. A source that failed has no version
/// but carries an `*_error` explaining why.
#[derive(Debug, Clone, Default)]
pub struct Latest {
    pub scrcpy: Option<String>,
    pub adb: Option<String>,
    pub scrcpy_error: Option<String>,
    pub adb_error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    #[serde(default)]
    assets: Vec<GithubAsset>,
}

#[derive(Debug, Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

/// Fetch both latest versions concurrently. A failure of one source does not
/// hide the other; each carries its own error.
pub async fn fetch_latest() -> Latest {
    let client = match http_client() {
        Ok(client) => client,
        Err(error) => {
            let error = error.to_string();
            return Latest {
                scrcpy_error: Some(error.clone()),
                adb_error: Some(error),
                ..Latest::default()
            };
        }
    };

    let (scrcpy, adb) = tokio::join!(latest_scrcpy(&client), latest_platform_tools(&client));

    let (scrcpy, scrcpy_error) = match scrcpy {
        Ok(version) => (Some(version), None),
        Err(error) => (None, Some(error.to_string())),
    };
    let (adb, adb_error) = match adb {
        Ok(version) => (Some(version), None),
        Err(error) => (None, Some(error.to_string())),
    };

    Latest {
        scrcpy,
        adb,
        scrcpy_error,
        adb_error,
    }
}

/// Resolve everything needed to download the latest scrcpy for this OS/arch:
/// the version string and the archive URL.
pub async fn scrcpy_download() -> Result<(String, String)> {
    let client = http_client()?;
    let release = scrcpy_release(&client).await?;
    let version = release.tag_name.trim().trim_start_matches('v').to_string();
    if version.is_empty() {
        return Err(AppError::Parse("empty scrcpy tag".into()));
    }
    let name = asset_name(std::env::consts::OS, std::env::consts::ARCH, &version).ok_or_else(
        || {
            AppError::Other(format!(
                "scrcpy publishes no build for {}/{} — install it with your package manager instead",
                std::env::consts::OS,
                std::env::consts::ARCH
            ))
        },
    )?;
    let url = pick_asset(&release.assets, &name)
        .ok_or_else(|| {
            AppError::Other(format!(
                "scrcpy release {version} has no asset named {name}"
            ))
        })?
        .to_string();
    Ok((version, url))
}

/// Resolve the latest platform-tools (adb) version and its download URL. The
/// URL is version-agnostic; the version comes from Google's repository manifest.
pub async fn platform_tools_download() -> Result<(String, String)> {
    let client = http_client()?;
    let version = latest_platform_tools(&client).await?;
    let os = match std::env::consts::OS {
        "windows" => "windows",
        "macos" => "darwin",
        "linux" => "linux",
        other => {
            return Err(AppError::Other(format!(
                "no platform-tools build for {other}"
            )))
        }
    };
    let url = format!("https://dl.google.com/android/repository/platform-tools-latest-{os}.zip");
    Ok((version, url))
}

/// True when `latest` is strictly newer than `installed`. Unknown installed
/// version (`None`) is treated as "cannot tell", i.e. not outdated.
pub fn is_outdated(installed: Option<&str>, latest: &str) -> bool {
    installed
        .map(|current| compare(current, latest) == Ordering::Less)
        .unwrap_or(false)
}

/// Numeric, dot-separated version comparison. Non-numeric tails are ignored
/// (`35.0.2-android-tools` compares as `35.0.2`).
pub fn compare(a: &str, b: &str) -> Ordering {
    let (left, right) = (parse_parts(a), parse_parts(b));
    let len = left.len().max(right.len());
    for index in 0..len {
        let x = left.get(index).copied().unwrap_or(0);
        let y = right.get(index).copied().unwrap_or(0);
        match x.cmp(&y) {
            Ordering::Equal => continue,
            other => return other,
        }
    }
    Ordering::Equal
}

/// The upstream asset name for a platform, or `None` when upstream publishes no
/// build for it (notably Linux/aarch64).
pub fn asset_name(os: &str, arch: &str, version: &str) -> Option<String> {
    let name = match (os, arch) {
        ("macos", "aarch64") => format!("scrcpy-macos-aarch64-v{version}.tar.gz"),
        ("macos", "x86_64") => format!("scrcpy-macos-x86_64-v{version}.tar.gz"),
        ("linux", "x86_64") => format!("scrcpy-linux-x86_64-v{version}.tar.gz"),
        ("windows", "x86_64") => format!("scrcpy-win64-v{version}.zip"),
        ("windows", "x86") => format!("scrcpy-win32-v{version}.zip"),
        ("windows", "aarch64") => format!("scrcpy-winarm64-v{version}.zip"),
        _ => return None,
    };
    Some(name)
}

fn parse_parts(version: &str) -> Vec<u64> {
    version
        .trim()
        .trim_start_matches('v')
        .split('.')
        .map(|part| {
            let digits: String = part.chars().take_while(char::is_ascii_digit).collect();
            digits.parse().unwrap_or(0)
        })
        .collect()
}

fn platform_tools_revision(xml: &str) -> Option<String> {
    let caps = PLATFORM_TOOLS_REVISION_RE.captures(xml)?;
    let major = caps.get(1)?.as_str();
    let minor = caps.get(2).map(|m| m.as_str()).unwrap_or("0");
    let micro = caps.get(3).map(|m| m.as_str()).unwrap_or("0");
    Some(format!("{major}.{minor}.{micro}"))
}

/// Build the shared HTTP client (timeout + User-Agent, both required by GitHub).
pub fn http_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(HTTP_TIMEOUT)
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| AppError::Other(format!("could not start HTTP client: {e}")))
}

async fn scrcpy_release(client: &reqwest::Client) -> Result<GithubRelease> {
    client
        .get(SCRCPY_RELEASE_URL)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|e| AppError::Other(format!("GitHub request failed: {e}")))?
        .json()
        .await
        .map_err(|e| AppError::Other(format!("could not read the scrcpy release: {e}")))
}

async fn latest_scrcpy(client: &reqwest::Client) -> Result<String> {
    let release = scrcpy_release(client).await?;
    let version = release.tag_name.trim().trim_start_matches('v');
    if version.is_empty() {
        return Err(AppError::Parse("empty scrcpy tag".into()));
    }
    Ok(version.to_string())
}

async fn latest_platform_tools(client: &reqwest::Client) -> Result<String> {
    let xml = client
        .get(PLATFORM_TOOLS_REPO_URL)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|e| AppError::Other(format!("Google request failed: {e}")))?
        .text()
        .await
        .map_err(|e| AppError::Other(format!("could not read the platform-tools manifest: {e}")))?;

    platform_tools_revision(&xml)
        .ok_or_else(|| AppError::Parse("platform-tools revision not found".into()))
}

fn pick_asset<'a>(assets: &'a [GithubAsset], name: &str) -> Option<&'a str> {
    assets
        .iter()
        .find(|asset| asset.name == name)
        .map(|asset| asset.browser_download_url.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_numeric_versions() {
        assert_eq!(compare("3.3.3", "5.0"), Ordering::Less);
        assert_eq!(compare("5.0", "5.0"), Ordering::Equal);
        assert_eq!(compare("v5.0.1", "5.0"), Ordering::Greater);
        assert_eq!(compare("1.0.41", "35.0.1"), Ordering::Less);
    }

    #[test]
    fn ignores_non_numeric_suffixes() {
        assert_eq!(compare("35.0.2-android-tools", "35.0.2"), Ordering::Equal);
        assert_eq!(compare("1.0.41", "1.0.41"), Ordering::Equal);
    }

    #[test]
    fn outdated_needs_a_known_installed_version() {
        assert!(is_outdated(Some("3.3.3"), "5.0"));
        assert!(!is_outdated(Some("5.0"), "5.0"));
        assert!(!is_outdated(None, "5.0"));
    }

    #[test]
    fn names_assets_per_platform() {
        assert_eq!(
            asset_name("macos", "aarch64", "5.0").unwrap(),
            "scrcpy-macos-aarch64-v5.0.tar.gz"
        );
        assert_eq!(
            asset_name("windows", "x86_64", "5.0").unwrap(),
            "scrcpy-win64-v5.0.zip"
        );
        assert_eq!(
            asset_name("linux", "x86_64", "5.0").unwrap(),
            "scrcpy-linux-x86_64-v5.0.tar.gz"
        );
        assert!(asset_name("linux", "aarch64", "5.0").is_none());
    }

    #[test]
    fn parses_the_platform_tools_revision() {
        let xml = r#"
            <remotePackage path="ndk;30.0.0">
              <revision><major>30</major><minor>0</minor><micro>0</micro></revision>
            </remotePackage>
            <remotePackage path="platform-tools">
              <type-details xsi:type="generic:genericDetailsType"/>
              <revision>
                <major>37</major>
                <minor>0</minor>
                <micro>1</micro>
              </revision>
            </remotePackage>
        "#;
        assert_eq!(platform_tools_revision(xml).as_deref(), Some("37.0.1"));
    }

    #[test]
    fn picks_an_asset_url_by_name() {
        let assets = vec![
            GithubAsset {
                name: "scrcpy-win64-v5.0.zip".into(),
                browser_download_url: "https://example.test/win64".into(),
            },
            GithubAsset {
                name: "scrcpy-macos-aarch64-v5.0.tar.gz".into(),
                browser_download_url: "https://example.test/mac".into(),
            },
        ];
        assert_eq!(
            pick_asset(&assets, "scrcpy-macos-aarch64-v5.0.tar.gz"),
            Some("https://example.test/mac")
        );
        assert_eq!(pick_asset(&assets, "missing"), None);
    }
}
