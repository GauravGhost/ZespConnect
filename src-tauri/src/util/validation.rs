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
    fn quotes_only_when_needed() {
        assert_eq!(shell_quote("com.android.chrome"), "com.android.chrome");
        assert_eq!(shell_quote("/sdcard/My Files"), "'/sdcard/My Files'");
        assert_eq!(shell_quote("it's"), "'it'\\''s'");
    }
}
