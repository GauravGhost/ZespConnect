//! scrcpy control-message encoding (client → device).
//!
//! Wire formats follow scrcpy's `app/src/control_msg.h` and the server's
//! `ControlMessageReader`. Only the messages this app currently sends are
//! implemented; input events will be added alongside them.

use crate::error::{AppError, Result};

/// `ControlMessage.TYPE_START_APP` in scrcpy-server.
const TYPE_START_APP: u8 = 16;

/// `START_APP` — `[type][u8 length][utf8 package]`. Launches an app on the
/// virtual display created by the `new_display` server option.
pub fn start_app(package: &str) -> Result<Vec<u8>> {
    let name = package.as_bytes();
    let length = u8::try_from(name.len())
        .map_err(|_| AppError::Invalid("Package name is too long".into()))?;

    let mut message = Vec::with_capacity(2 + name.len());
    message.push(TYPE_START_APP);
    message.push(length);
    message.extend_from_slice(name);
    Ok(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_app_is_a_type_then_a_length_prefixed_name() {
        let message = start_app("com.example.app").unwrap();

        assert_eq!(message[0], 16);
        assert_eq!(message[1], "com.example.app".len() as u8);
        assert_eq!(&message[2..], b"com.example.app");
    }

    #[test]
    fn start_app_rejects_names_that_do_not_fit_the_length_byte() {
        assert!(start_app(&"a".repeat(255)).is_ok());
        assert!(start_app(&"a".repeat(256)).is_err());
    }
}
