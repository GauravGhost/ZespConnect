//! scrcpy control-message encoding (client → device).
//!
//! Wire formats follow scrcpy's `app/src/control_msg.h` and the server's
//! `ControlMessageReader`.

use crate::error::{AppError, Result};
use crate::models::{KeyAction, TouchAction};

/// `ControlMessage.TYPE_*` values in scrcpy-server.
const TYPE_INJECT_KEYCODE: u8 = 0;
const TYPE_INJECT_TEXT: u8 = 1;
const TYPE_INJECT_TOUCH_EVENT: u8 = 2;
const TYPE_INJECT_SCROLL_EVENT: u8 = 3;
const TYPE_SET_DISPLAY_POWER: u8 = 10;
const TYPE_START_APP: u8 = 16;
const TYPE_RESET_VIDEO: u8 = 17;

/// `AMOTION_EVENT_ACTION_*` values carried by a touch message.
const ACTION_DOWN: u8 = 0;
const ACTION_UP: u8 = 1;
const ACTION_MOVE: u8 = 2;
const ACTION_HOVER_MOVE: u8 = 7;

/// `AKEY_EVENT_ACTION_*` values carried by a keycode message.
const KEY_ACTION_DOWN: u8 = 0;
const KEY_ACTION_UP: u8 = 1;

/// The server rejects longer `INJECT_TEXT` payloads (`INJECT_TEXT_MAX_LENGTH`).
pub const INJECT_TEXT_MAX_BYTES: usize = 300;

/// Scroll amounts travel as a 16-bit fixed-point fraction of this range.
const SCROLL_RANGE: f32 = 16.0;

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

/// A pointer position in the video's coordinate space, plus the size of the
/// frame it was measured against. The server drops an event unless that size
/// exactly matches the current video size.
#[derive(Debug, Clone, Copy)]
pub struct Position {
    pub x: u32,
    pub y: u32,
    pub width: u16,
    pub height: u16,
}

impl Position {
    fn write(&self, message: &mut Vec<u8>) {
        message.extend_from_slice(&self.x.to_be_bytes());
        message.extend_from_slice(&self.y.to_be_bytes());
        message.extend_from_slice(&self.width.to_be_bytes());
        message.extend_from_slice(&self.height.to_be_bytes());
    }
}

/// `INJECT_TOUCH_EVENT` — `[type][u8 action][u64 pointerId][u32 x][u32 y]
/// [u16 width][u16 height][u16 pressure][u32 actionButton][u32 buttons]`.
///
/// `action_button` is the `AMOTION_EVENT_BUTTON_*` that changed and `buttons`
/// the ones held. Primary-button-only input is injected as a finger; a hover or
/// any other button makes the server inject a real mouse event (desktop mode's
/// hover effects and right-click context menus).
pub fn inject_touch(
    action: TouchAction,
    position: Position,
    action_button: u32,
    buttons: u32,
) -> Vec<u8> {
    let action = match action {
        TouchAction::Down => ACTION_DOWN,
        TouchAction::Move => ACTION_MOVE,
        TouchAction::Up => ACTION_UP,
        TouchAction::Hover => ACTION_HOVER_MOVE,
    };
    // A lifted or hovering pointer carries no pressure; a pressed one is fully pressed.
    let pressure: u16 = if action == ACTION_UP || action == ACTION_HOVER_MOVE {
        0
    } else {
        u16::MAX
    };

    let mut message = Vec::with_capacity(32);
    message.push(TYPE_INJECT_TOUCH_EVENT);
    message.push(action);
    message.extend_from_slice(&POINTER_ID_MOUSE.to_be_bytes());
    position.write(&mut message);
    message.extend_from_slice(&pressure.to_be_bytes());
    message.extend_from_slice(&action_button.to_be_bytes());
    message.extend_from_slice(&buttons.to_be_bytes());
    message
}

/// `INJECT_SCROLL_EVENT` — `[type][u32 x][u32 y][u16 width][u16 height]
/// [i16 hscroll][i16 vscroll][u32 buttons]`. Amounts are in wheel notches,
/// clamped to ±16 and sent as a fixed-point fraction of that range. Positive
/// `vscroll` scrolls up, positive `hscroll` scrolls right (Android's axes).
pub fn inject_scroll(position: Position, hscroll: f32, vscroll: f32) -> Vec<u8> {
    let mut message = Vec::with_capacity(21);
    message.push(TYPE_INJECT_SCROLL_EVENT);
    position.write(&mut message);
    message.extend_from_slice(&scroll_fixed_point(hscroll).to_be_bytes());
    message.extend_from_slice(&scroll_fixed_point(vscroll).to_be_bytes());
    message.extend_from_slice(&0u32.to_be_bytes()); // buttons
    message
}

fn scroll_fixed_point(value: f32) -> i16 {
    let fraction = if value.is_finite() {
        (value / SCROLL_RANGE).clamp(-1.0, 1.0)
    } else {
        0.0
    };
    // 1.0 maps to i16::MAX, matching scrcpy's `sc_float_to_i16fp`.
    (fraction * f32::from(i16::MAX)).round() as i16
}

/// `INJECT_KEYCODE` — `[type][u8 action][u32 keycode][u32 repeat][u32 metaState]`,
/// with an Android `KEYCODE_*` and `META_*` mask.
pub fn inject_keycode(action: KeyAction, keycode: u32, repeat: u32, meta_state: u32) -> Vec<u8> {
    let action = match action {
        KeyAction::Down => KEY_ACTION_DOWN,
        KeyAction::Up => KEY_ACTION_UP,
    };
    let mut message = Vec::with_capacity(14);
    message.push(TYPE_INJECT_KEYCODE);
    message.push(action);
    message.extend_from_slice(&keycode.to_be_bytes());
    message.extend_from_slice(&repeat.to_be_bytes());
    message.extend_from_slice(&meta_state.to_be_bytes());
    message
}

/// `INJECT_TEXT` — `[type][u32 length][utf8 text]`, at most
/// [`INJECT_TEXT_MAX_BYTES`] bytes.
pub fn inject_text(text: &str) -> Result<Vec<u8>> {
    let bytes = text.as_bytes();
    if bytes.is_empty() || bytes.len() > INJECT_TEXT_MAX_BYTES {
        return Err(AppError::Invalid("Text must be 1–300 bytes".into()));
    }
    let mut message = Vec::with_capacity(5 + bytes.len());
    message.push(TYPE_INJECT_TEXT);
    message.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    message.extend_from_slice(bytes);
    Ok(message)
}

/// `RESET_VIDEO` — `[type]`. Restarts the capture/encoder, so the next packets
/// are a fresh codec config and key frame. A decoder that dropped frames can
/// only resync on a key frame, and the encoder sends none while the screen is
/// still, so the client asks for one.
pub fn reset_video() -> Vec<u8> {
    vec![TYPE_RESET_VIDEO]
}

/// `SET_DISPLAY_POWER` — `[type][u8 on]`. While a virtual display is mirrored
/// the server applies it to the phone's own (main) display, so the phone screen
/// can go dark while the virtual display keeps rendering.
pub fn set_display_power(on: bool) -> Vec<u8> {
    vec![TYPE_SET_DISPLAY_POWER, u8::from(on)]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(x: u32, y: u32, width: u16, height: u16) -> Position {
        Position {
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn inject_touch_is_a_32_byte_touch_event() {
        let message = inject_touch(TouchAction::Down, at(100, 200, 1080, 2400), 0, 0);

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
        assert_eq!(
            inject_touch(TouchAction::Move, at(0, 0, 1, 1), 0, 0)[1],
            ACTION_MOVE
        );

        let up = inject_touch(TouchAction::Up, at(0, 0, 1, 1), 0, 0);
        assert_eq!(up[1], ACTION_UP);
        assert_eq!(&up[22..24], &0u16.to_be_bytes());
    }

    #[test]
    fn hover_carries_no_pressure_and_right_click_carries_its_buttons() {
        let hover = inject_touch(TouchAction::Hover, at(5, 6, 10, 10), 0, 0);
        assert_eq!(hover[1], ACTION_HOVER_MOVE);
        assert_eq!(&hover[22..24], &0u16.to_be_bytes());

        let right = inject_touch(TouchAction::Down, at(5, 6, 10, 10), 2, 2);
        assert_eq!(&right[24..28], &2u32.to_be_bytes());
        assert_eq!(&right[28..32], &2u32.to_be_bytes());
    }

    #[test]
    fn scroll_is_a_21_byte_fixed_point_event() {
        let message = inject_scroll(at(1, 2, 3, 4), 0.0, 16.0);
        assert_eq!(message.len(), 21);
        assert_eq!(message[0], TYPE_INJECT_SCROLL_EVENT);
        assert_eq!(&message[13..15], &0i16.to_be_bytes());
        assert_eq!(&message[15..17], &i16::MAX.to_be_bytes());

        // Out-of-range and non-finite amounts are clamped, not wrapped.
        let wild = inject_scroll(at(1, 2, 3, 4), -100.0, f32::NAN);
        assert_eq!(&wild[13..15], &(-i16::MAX).to_be_bytes());
        assert_eq!(&wild[15..17], &0i16.to_be_bytes());
    }

    #[test]
    fn keycode_is_a_14_byte_event() {
        let message = inject_keycode(KeyAction::Up, 66, 2, 0x1000);
        assert_eq!(message.len(), 14);
        assert_eq!(message[0], TYPE_INJECT_KEYCODE);
        assert_eq!(message[1], KEY_ACTION_UP);
        assert_eq!(&message[2..6], &66u32.to_be_bytes());
        assert_eq!(&message[6..10], &2u32.to_be_bytes());
        assert_eq!(&message[10..14], &0x1000u32.to_be_bytes());
    }

    #[test]
    fn text_is_length_prefixed_and_bounded() {
        let message = inject_text("hé").unwrap();
        assert_eq!(message[0], TYPE_INJECT_TEXT);
        assert_eq!(&message[1..5], &3u32.to_be_bytes());
        assert_eq!(&message[5..], "hé".as_bytes());

        assert!(inject_text("").is_err());
        assert!(inject_text(&"a".repeat(INJECT_TEXT_MAX_BYTES)).is_ok());
        assert!(inject_text(&"a".repeat(INJECT_TEXT_MAX_BYTES + 1)).is_err());
    }

    #[test]
    fn reset_video_is_a_bare_type() {
        assert_eq!(reset_video(), vec![TYPE_RESET_VIDEO]);
    }

    #[test]
    fn display_power_is_a_type_then_a_flag() {
        assert_eq!(set_display_power(false), vec![TYPE_SET_DISPLAY_POWER, 0]);
        assert_eq!(set_display_power(true), vec![TYPE_SET_DISPLAY_POWER, 1]);
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
