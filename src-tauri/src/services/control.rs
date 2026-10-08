//! scrcpy control-message encoding (client → device).
//!
//! Wire formats follow scrcpy's `app/src/control_msg.h` and the server's
//! `ControlMessageReader`.

use crate::error::{AppError, Result};
use crate::models::TouchAction;

/// `ControlMessage.TYPE_START_APP` in scrcpy-server.
const TYPE_START_APP: u8 = 16;

/// `ControlMessage.TYPE_INJECT_TOUCH_EVENT` in scrcpy-server.
const TYPE_INJECT_TOUCH_EVENT: u8 = 2;

/// `AMOTION_EVENT_ACTION_*` values carried by a touch message.
const ACTION_DOWN: u8 = 0;
const ACTION_UP: u8 = 1;
const ACTION_MOVE: u8 = 2;

/// `SC_POINTER_ID_MOUSE`: scrcpy's pointer id for mouse-originated input. With no
/// buttons pressed the server injects it as a regular touchscreen finger.
const POINTER_ID_MOUSE: u64 = u64::MAX;

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

/// `INJECT_TOUCH_EVENT` — `[type][u8 action][u64 pointerId][u32 x][u32 y]
/// [u16 width][u16 height][u16 pressure][u32 actionButton][u32 buttons]`.
///
/// `x`/`y` and `width`/`height` are in the video's coordinate space. The server
/// drops the event unless `width`/`height` exactly match the current video size,
/// so the caller must pass the size of the frame it measured the point against.
pub fn inject_touch(action: TouchAction, x: u32, y: u32, width: u16, height: u16) -> Vec<u8> {
    let action = match action {
        TouchAction::Down => ACTION_DOWN,
        TouchAction::Move => ACTION_MOVE,
        TouchAction::Up => ACTION_UP,
    };
    // A lifted finger carries no pressure; a pressed one is fully pressed.
    let pressure: u16 = if action == ACTION_UP { 0 } else { u16::MAX };

    let mut message = Vec::with_capacity(32);
    message.push(TYPE_INJECT_TOUCH_EVENT);
    message.push(action);
    message.extend_from_slice(&POINTER_ID_MOUSE.to_be_bytes());
    message.extend_from_slice(&x.to_be_bytes());
    message.extend_from_slice(&y.to_be_bytes());
    message.extend_from_slice(&width.to_be_bytes());
    message.extend_from_slice(&height.to_be_bytes());
    message.extend_from_slice(&pressure.to_be_bytes());
    message.extend_from_slice(&0u32.to_be_bytes()); // action_button
    message.extend_from_slice(&0u32.to_be_bytes()); // buttons
    message
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inject_touch_is_a_32_byte_touch_event() {
        let message = inject_touch(TouchAction::Down, 100, 200, 1080, 2400);

        assert_eq!(message.len(), 32);
        assert_eq!(message[0], TYPE_INJECT_TOUCH_EVENT);
        assert_eq!(message[1], ACTION_DOWN);
        assert_eq!(&message[2..10], &POINTER_ID_MOUSE.to_be_bytes());
        assert_eq!(&message[10..14], &100u32.to_be_bytes());
        assert_eq!(&message[14..18], &200u32.to_be_bytes());
        assert_eq!(&message[18..20], &1080u16.to_be_bytes());
        assert_eq!(&message[20..22], &2400u16.to_be_bytes());
        assert_eq!(&message[22..24], &u16::MAX.to_be_bytes());
        assert_eq!(&message[24..32], &[0u8; 8]);
    }

    #[test]
    fn inject_touch_maps_actions_and_releases_pressure_on_up() {
        assert_eq!(inject_touch(TouchAction::Move, 0, 0, 1, 1)[1], ACTION_MOVE);

        let up = inject_touch(TouchAction::Up, 0, 0, 1, 1);
        assert_eq!(up[1], ACTION_UP);
        assert_eq!(&up[22..24], &0u16.to_be_bytes());
    }

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
