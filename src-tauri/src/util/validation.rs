//! Input validation for values that reach `adb shell`.

use once_cell::sync::Lazy;
use regex::Regex;

use crate::error::{AppError, Result};

/// A valid Android package id, e.g. `com.google.android.apps.maps`.
static PACKAGE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[A-Za-z][A-Za-z0-9_]*(\.[A-Za-z0-9_]+)+$").expect("valid regex"));

/// Safe remote path tokens we allow unquoted handling for; anything else is quoted.
pub fn validate_package(package: &str) -> Result<()> {
    if PACKAGE_RE.is_match(package) {
        Ok(())
    } else {
        Err(AppError::Invalid(format!(
            "Invalid package name: {package}"
        )))
    }
}

/// Validate a device serial / host address (adb serials are fairly permissive).
pub fn validate_serial(serial: &str) -> Result<()> {
    if serial.is_empty() || serial.chars().any(|c| c.is_whitespace() || c == '\0') {
        return Err(AppError::Invalid("Invalid device serial".into()));
    }
    Ok(())
}

/// A wireless-debugging pairing service name (an mDNS instance name, ≤63 bytes).
static SERVICE_NAME_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[A-Za-z0-9][A-Za-z0-9-]{0,62}$").expect("valid regex"));

/// The numeric pairing secret embedded in a QR payload.
static PAIRING_PASSWORD_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^\d{6,16}$").expect("valid regex"));

/// Validate a pairing service name before it is used for mDNS matching.
pub fn validate_service_name(name: &str) -> Result<()> {
    if SERVICE_NAME_RE.is_match(name) {
        Ok(())
    } else {
        Err(AppError::Invalid(format!(
            "Invalid pairing service name: {name}"
        )))
    }
}

/// Validate the QR pairing secret. Digits only; the value is never echoed back.
pub fn validate_pairing_password(password: &str) -> Result<()> {
    if PAIRING_PASSWORD_RE.is_match(password) {
        Ok(())
    } else {
        Err(AppError::Invalid("Invalid pairing password".into()))
    }
}

/// Split an `host:port` value from `adb mdns services`, validating both parts.
pub fn parse_host_port(value: &str) -> Result<(String, u16)> {
    let (host, port) = value
        .rsplit_once(':')
        .ok_or_else(|| AppError::Invalid(format!("Invalid address: {value}")))?;
    let port: u16 = port
        .parse()
        .map_err(|_| AppError::Invalid(format!("Invalid port: {value}")))?;
    if port == 0 {
        return Err(AppError::Invalid(format!("Invalid port: {value}")));
    }
    let host = host.trim_matches(|c| c == '[' || c == ']');
    let valid = !host.is_empty()
        && host.len() <= 253
        && (host.parse::<std::net::IpAddr>().is_ok()
            || host
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':')));
    if !valid {
        return Err(AppError::Invalid(format!("Invalid host: {value}")));
    }
    Ok((host.to_string(), port))
}

/// Validate a device path before it reaches `adb shell`. Requires an absolute
/// path, forbids traversal, and rejects the filesystem root so a `rm -rf` can
/// never be aimed at `/`.
pub fn validate_remote_path(path: &str) -> Result<()> {
    let invalid = !path.starts_with('/')
        || path.len() < 2
        || path.contains('\0')
        || path.contains('\n')
        || path.contains('\r');
    if invalid {
        return Err(AppError::Invalid(format!("Invalid path: {path}")));
    }
    if path.split('/').any(|segment| segment == "..") {
        return Err(AppError::Invalid("Path traversal is not allowed".into()));
    }
    Ok(())
}

/// POSIX single-quote escaping, mirroring Python's `shlex.quote`.
///
/// adb joins its arguments into a single remote shell line, so any value that
/// becomes part of an `adb shell` command must be quoted for that shell.
pub fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }
    if value.chars().all(|c| {
        c.is_ascii_alphanumeric()
            || matches!(c, '_' | '-' | '.' | '/' | ':' | '@' | ',' | '+' | '=')
    }) {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_packages() {
        assert!(validate_package("com.android.chrome").is_ok());
        assert!(validate_package("org.mozilla.firefox").is_ok());
        assert!(validate_package("a.b").is_ok());
    }

    #[test]
    fn rejects_invalid_packages() {
        assert!(validate_package("com").is_err());
        assert!(validate_package("com.android.chrome; rm -rf /").is_err());
        assert!(validate_package("").is_err());
        assert!(validate_package("1com.foo").is_err());
    }

    #[test]
    fn validates_remote_paths() {
        assert!(validate_remote_path("/sdcard/Download").is_ok());
        assert!(validate_remote_path("/sdcard").is_ok());
        assert!(validate_remote_path("/").is_err());
        assert!(validate_remote_path("sdcard/Download").is_err());
        assert!(validate_remote_path("/sdcard/../etc").is_err());
        assert!(validate_remote_path("").is_err());
    }

    #[test]
    fn quotes_only_when_needed() {
        assert_eq!(shell_quote("com.android.chrome"), "com.android.chrome");
        assert_eq!(shell_quote("/sdcard/My Files"), "'/sdcard/My Files'");
        assert_eq!(shell_quote("it's"), "'it'\\''s'");
    }

    #[test]
    fn validates_pairing_service_names() {
        assert!(validate_service_name("zesp-0123456789").is_ok());
        assert!(validate_service_name("a").is_ok());
        assert!(validate_service_name("").is_err());
        assert!(validate_service_name("-leading").is_err());
        assert!(validate_service_name("has space").is_err());
        assert!(validate_service_name("bad;name").is_err());
        assert!(validate_service_name(&"x".repeat(64)).is_err());
    }

    #[test]
    fn validates_pairing_passwords() {
        assert!(validate_pairing_password("123456").is_ok());
        assert!(validate_pairing_password("4829137465").is_ok());
        assert!(validate_pairing_password("12345").is_err());
        assert!(validate_pairing_password("12345678901234567").is_err());
        assert!(validate_pairing_password("12ab56").is_err());
    }

    #[test]
    fn parses_host_port() {
        assert_eq!(
            parse_host_port("192.168.1.5:37039").unwrap(),
            ("192.168.1.5".to_string(), 37039)
        );
        assert_eq!(
            parse_host_port("[fe80::1]:5555").unwrap(),
            ("fe80::1".to_string(), 5555)
        );
        assert!(parse_host_port("192.168.1.5").is_err());
        assert!(parse_host_port("192.168.1.5:0").is_err());
        assert!(parse_host_port("192.168.1.5:99999").is_err());
        assert!(parse_host_port(":5555").is_err());
    }
}
