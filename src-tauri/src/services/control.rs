use crate::error::{AppError, Result};
use crate::models::{KeyAction, TouchAction};

/// `ControlMessage.TYPE_*` values in scrcpy-server.
const TYPE_INJECT_KEYCODE: u8 = 0;
const TYPE_INJECT_TEXT: u8 = 1;
const TYPE_INJECT_TOUCH_EVENT: u8 = 2;
const TYPE_INJECT_SCROLL_EVENT: u8 = 3;
const TYPE_BACK_OR_SCREEN_ON: u8 = 4;
const TYPE_EXPAND_NOTIFICATION_PANEL: u8 = 5;
const TYPE_EXPAND_SETTINGS_PANEL: u8 = 6;
const TYPE_COLLAPSE_PANELS: u8 = 7;
const TYPE_SET_CLIPBOARD: u8 = 9;
const TYPE_SET_DISPLAY_POWER: u8 = 10;
const TYPE_ROTATE_DEVICE: u8 = 11;
const TYPE_UHID_CREATE: u8 = 12;
const TYPE_UHID_INPUT: u8 = 13;
const TYPE_UHID_DESTROY: u8 = 14;
const TYPE_START_APP: u8 = 16;
const TYPE_RESET_VIDEO: u8 = 17;
// Types 18+ exist only from scrcpy 4.0; an older server drops the session on an unknown type.
const TYPE_CAMERA_SET_TORCH: u8 = 18;
const TYPE_CAMERA_ZOOM_IN: u8 = 19;
const TYPE_CAMERA_ZOOM_OUT: u8 = 20;
const TYPE_RESIZE_DISPLAY: u8 = 21;

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

/// The server rejects longer `SET_CLIPBOARD` texts (256 KiB minus the 14-byte header).
pub const CLIPBOARD_MAX_BYTES: usize = (1 << 18) - 14;

/// `DeviceMessage.TYPE_*` values in scrcpy-server (device -> client).
const DEVICE_CLIPBOARD: u8 = 0;
const DEVICE_ACK_CLIPBOARD: u8 = 1;
const DEVICE_UHID_OUTPUT: u8 = 2;
const DEVICE_MESSAGE_MAX_BYTES: usize = 1 << 18;

/// Scroll amounts travel as a 16-bit fixed-point fraction of this range.
const SCROLL_RANGE: f32 = 16.0;

/// `SC_POINTER_ID_MOUSE`: scrcpy's pointer id; with no buttons pressed the server injects it as a finger.
const POINTER_ID_MOUSE: u64 = u64::MAX;

/// `START_APP` - `[type][u8 length][utf8 package]`; launches an app on the `new_display` virtual display.
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

/// A pointer position in the video's space plus the frame size; the server drops it unless that size matches.
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

/// `INJECT_TOUCH_EVENT` - `[type][u8 action][u64 pointerId][u32 x][u32 y][u16 width][u16 height]
/// [u16 pressure][u32 actionButton][u32 buttons]`. Primary-button-only input is injected as a finger;
/// a hover or other button becomes a real mouse event (hover effects, right-click menus).
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

/// `INJECT_SCROLL_EVENT` - `[type][u32 x][u32 y][u16 width][u16 height][i16 hscroll][i16 vscroll]
/// [u32 buttons]`. Amounts are wheel notches clamped to ±16; positive vscroll goes up, hscroll right.
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

/// `INJECT_KEYCODE` - `[type][u8 action][u32 keycode][u32 repeat][u32 metaState]` (Android keycodes/meta).
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

/// `INJECT_TEXT` - `[type][u32 length][utf8 text]`, at most `INJECT_TEXT_MAX_BYTES` bytes.
pub fn inject_text(text: &str) -> Result<Vec<u8>> {
    let bytes = text.as_bytes();
    if bytes.is_empty() || bytes.len() > INJECT_TEXT_MAX_BYTES {
        return Err(AppError::Invalid("Text must be 1-300 bytes".into()));
    }
    let mut message = Vec::with_capacity(5 + bytes.len());
    message.push(TYPE_INJECT_TEXT);
    message.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    message.extend_from_slice(bytes);
    Ok(message)
}

/// `SET_CLIPBOARD` - `[type][u64 sequence][u8 paste][u32 length][utf8 text]`. Sequence 0 asks for
/// no acknowledgement; text over [`CLIPBOARD_MAX_BYTES`] is cut at a character boundary.
pub fn set_clipboard(text: &str) -> Vec<u8> {
    let mut end = text.len().min(CLIPBOARD_MAX_BYTES);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let bytes = &text.as_bytes()[..end];
    let mut message = Vec::with_capacity(14 + bytes.len());
    message.push(TYPE_SET_CLIPBOARD);
    message.extend_from_slice(&0u64.to_be_bytes());
    message.push(0);
    message.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    message.extend_from_slice(bytes);
    message
}

/// A message the server sends back on the control socket.
#[derive(Debug, PartialEq, Eq)]
pub enum DeviceMessage {
    Clipboard(String),
    Other,
}

/// Read one device message (`DeviceMessageWriter.java`); errors on a closed or corrupt stream.
pub async fn read_device_message<R>(reader: &mut R) -> std::io::Result<DeviceMessage>
where
    R: tokio::io::AsyncRead + Unpin,
{
    use std::io::{Error, ErrorKind};
    use tokio::io::AsyncReadExt;

    match reader.read_u8().await? {
        DEVICE_CLIPBOARD => {
            let len = reader.read_u32().await? as usize;
            if len > DEVICE_MESSAGE_MAX_BYTES {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "clipboard message too large",
                ));
            }
            let mut text = vec![0u8; len];
            reader.read_exact(&mut text).await?;
            Ok(DeviceMessage::Clipboard(
                String::from_utf8_lossy(&text).into_owned(),
            ))
        }
        DEVICE_ACK_CLIPBOARD => {
            reader.read_u64().await?;
            Ok(DeviceMessage::Other)
        }
        DEVICE_UHID_OUTPUT => {
            reader.read_u16().await?;
            let len = reader.read_u16().await? as usize;
            let mut data = vec![0u8; len];
            reader.read_exact(&mut data).await?;
            Ok(DeviceMessage::Other)
        }
        kind => Err(Error::new(
            ErrorKind::InvalidData,
            format!("unknown device message {kind}"),
        )),
    }
}

/// `RESET_VIDEO` - `[type]`. Restarts the capture/encoder so the next packets are a fresh config and key frame.
pub fn reset_video() -> Vec<u8> {
    vec![TYPE_RESET_VIDEO]
}

/// `SET_DISPLAY_POWER` - `[type][u8 on]`. With a virtual display the server applies it to the main display.
pub fn set_display_power(on: bool) -> Vec<u8> {
    vec![TYPE_SET_DISPLAY_POWER, u8::from(on)]
}

/// `UHID_CREATE` (scrcpy 3.0+) - `[type][u16 id][u16 vendor][u16 product][u8 name length][name]
/// [u16 descriptor length][descriptor]`; adds a virtual HID device to the phone.
pub fn uhid_create(id: u16, name: &str, report_descriptor: &[u8]) -> Vec<u8> {
    // The server reads the name with a one-byte length (`readString` tiny).
    let name = &name.as_bytes()[..name.len().min(127)];
    let mut message = Vec::with_capacity(10 + name.len() + report_descriptor.len());
    message.push(TYPE_UHID_CREATE);
    message.extend_from_slice(&id.to_be_bytes());
    // No vendor or product, so Android maps the device with its generic gamepad layout.
    message.extend_from_slice(&0u16.to_be_bytes());
    message.extend_from_slice(&0u16.to_be_bytes());
    message.push(name.len() as u8);
    message.extend_from_slice(name);
    message.extend_from_slice(&(report_descriptor.len() as u16).to_be_bytes());
    message.extend_from_slice(report_descriptor);
    message
}

/// `UHID_INPUT` - `[type][u16 id][u16 size][report]`.
pub fn uhid_input(id: u16, report: &[u8]) -> Vec<u8> {
    let mut message = Vec::with_capacity(5 + report.len());
    message.push(TYPE_UHID_INPUT);
    message.extend_from_slice(&id.to_be_bytes());
    message.extend_from_slice(&(report.len() as u16).to_be_bytes());
    message.extend_from_slice(report);
    message
}

/// `UHID_DESTROY` - `[type][u16 id]`.
pub fn uhid_destroy(id: u16) -> Vec<u8> {
    let mut message = vec![TYPE_UHID_DESTROY];
    message.extend_from_slice(&id.to_be_bytes());
    message
}

/// `BACK_OR_SCREEN_ON` - `[type][u8 action]`. Presses BACK, or wakes the screen if it is off.
pub fn back_or_screen_on(action: KeyAction) -> Vec<u8> {
    let action = match action {
        KeyAction::Down => KEY_ACTION_DOWN,
        KeyAction::Up => KEY_ACTION_UP,
    };
    vec![TYPE_BACK_OR_SCREEN_ON, action]
}

/// `EXPAND_NOTIFICATION_PANEL` - `[type]`.
pub fn expand_notification_panel() -> Vec<u8> {
    vec![TYPE_EXPAND_NOTIFICATION_PANEL]
}

/// `EXPAND_SETTINGS_PANEL` - `[type]`. Opens quick settings.
pub fn expand_settings_panel() -> Vec<u8> {
    vec![TYPE_EXPAND_SETTINGS_PANEL]
}

/// `COLLAPSE_PANELS` - `[type]`.
pub fn collapse_panels() -> Vec<u8> {
    vec![TYPE_COLLAPSE_PANELS]
}

/// `ROTATE_DEVICE` - `[type]`. Turns the mirrored display a quarter turn.
pub fn rotate_device() -> Vec<u8> {
    vec![TYPE_ROTATE_DEVICE]
}

/// `CAMERA_SET_TORCH` - `[type][u8 on]`. scrcpy 4.0+, camera sessions only.
pub fn camera_set_torch(on: bool) -> Vec<u8> {
    vec![TYPE_CAMERA_SET_TORCH, u8::from(on)]
}

/// `CAMERA_ZOOM_IN` - `[type]`. scrcpy 4.0+, camera sessions only.
pub fn camera_zoom_in() -> Vec<u8> {
    vec![TYPE_CAMERA_ZOOM_IN]
}

/// `CAMERA_ZOOM_OUT` - `[type]`. scrcpy 4.0+, camera sessions only.
pub fn camera_zoom_out() -> Vec<u8> {
    vec![TYPE_CAMERA_ZOOM_OUT]
}

/// `RESIZE_DISPLAY` - `[type][u16 width][u16 height]`. scrcpy 4.0+, and only for a `flex_display`
/// session: the server throws on any other, which ends the session.
pub fn resize_display(width: u16, height: u16) -> Vec<u8> {
    let mut message = Vec::with_capacity(5);
    message.push(TYPE_RESIZE_DISPLAY);
    message.extend_from_slice(&width.to_be_bytes());
    message.extend_from_slice(&height.to_be_bytes());
    message
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
    fn set_clipboard_has_no_sequence_and_no_paste() {
        let message = set_clipboard("hé");
        assert_eq!(message[0], TYPE_SET_CLIPBOARD);
        assert_eq!(&message[1..9], &0u64.to_be_bytes());
        assert_eq!(message[9], 0);
        assert_eq!(&message[10..14], &3u32.to_be_bytes());
        assert_eq!(&message[14..], "hé".as_bytes());
    }

    #[test]
    fn set_clipboard_cuts_long_text_on_a_character_boundary() {
        let text = format!("{}é", "a".repeat(CLIPBOARD_MAX_BYTES - 1));
        let message = set_clipboard(&text);
        assert_eq!(message.len() - 14, CLIPBOARD_MAX_BYTES - 1);
    }

    #[tokio::test]
    async fn reads_clipboard_and_skips_other_device_messages() {
        let mut bytes = vec![DEVICE_ACK_CLIPBOARD];
        bytes.extend_from_slice(&7u64.to_be_bytes());
        bytes.push(DEVICE_UHID_OUTPUT);
        bytes.extend_from_slice(&1u16.to_be_bytes());
        bytes.extend_from_slice(&2u16.to_be_bytes());
        bytes.extend_from_slice(&[9, 9]);
        bytes.push(DEVICE_CLIPBOARD);
        bytes.extend_from_slice(&5u32.to_be_bytes());
        bytes.extend_from_slice(b"hello");

        let mut reader = bytes.as_slice();
        assert_eq!(
            read_device_message(&mut reader).await.unwrap(),
            DeviceMessage::Other
        );
        assert_eq!(
            read_device_message(&mut reader).await.unwrap(),
            DeviceMessage::Other
        );
        assert_eq!(
            read_device_message(&mut reader).await.unwrap(),
            DeviceMessage::Clipboard("hello".into())
        );
        assert!(read_device_message(&mut reader).await.is_err());
    }

    #[test]
    fn uhid_create_carries_ids_name_and_descriptor() {
        let message = uhid_create(3, "Pad", &[0xA1, 0xC0]);
        assert_eq!(
            message,
            vec![12, 0, 3, 0, 0, 0, 0, 3, b'P', b'a', b'd', 0, 2, 0xA1, 0xC0]
        );
    }

    #[test]
    fn uhid_create_truncates_a_long_name() {
        let message = uhid_create(1, &"n".repeat(300), &[]);
        assert_eq!(message[7], 127);
        assert_eq!(message.len(), 8 + 127 + 2);
    }

    #[test]
    fn uhid_input_and_destroy_frame_the_id() {
        assert_eq!(uhid_input(4, &[9, 8]), vec![13, 0, 4, 0, 2, 9, 8]);
        assert_eq!(uhid_destroy(4), vec![14, 0, 4]);
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

    #[test]
    fn back_or_screen_on_carries_the_key_action() {
        assert_eq!(back_or_screen_on(KeyAction::Down), vec![4, 0]);
        assert_eq!(back_or_screen_on(KeyAction::Up), vec![4, 1]);
    }

    #[test]
    fn panel_and_rotate_messages_are_bare_types() {
        assert_eq!(expand_notification_panel(), vec![5]);
        assert_eq!(expand_settings_panel(), vec![6]);
        assert_eq!(collapse_panels(), vec![7]);
        assert_eq!(rotate_device(), vec![11]);
    }

    #[test]
    fn camera_messages_match_scrcpy_4() {
        assert_eq!(camera_set_torch(true), vec![18, 1]);
        assert_eq!(camera_set_torch(false), vec![18, 0]);
        assert_eq!(camera_zoom_in(), vec![19]);
        assert_eq!(camera_zoom_out(), vec![20]);
    }

    #[test]
    fn resize_display_is_a_type_then_two_u16() {
        assert_eq!(
            resize_display(1080, 0x0960),
            vec![21, 0x04, 0x38, 0x09, 0x60]
        );
    }
}
