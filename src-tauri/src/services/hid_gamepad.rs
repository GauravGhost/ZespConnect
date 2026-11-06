//! The virtual gamepad a computer controller becomes on the phone. The descriptor and report
//! layout follow scrcpy's `hid_gamepad.c` (Apache-2.0, Copyright (C) Genymobile), so Android maps
//! it like a scrcpy gamepad.

/// Two 16-bit sticks (X, Y, Z, Rz), two 15-bit triggers (Brake, Accelerator), 16 buttons and a hat.
pub const REPORT_DESCRIPTOR: &[u8] = &[
    0x05, 0x01, // Usage Page (Generic Desktop)
    0x09, 0x05, // Usage (Gamepad)
    0xA1, 0x01, // Collection (Application)
    0xA1, 0x00, //   Collection (Physical)
    0x05, 0x01, //     Usage Page (Generic Desktop)
    0x09, 0x30, //     Usage (X): left stick x
    0x09, 0x31, //     Usage (Y): left stick y
    0x09, 0x32, //     Usage (Z): right stick x
    0x09, 0x35, //     Usage (Rz): right stick y
    0x15, 0x00, //     Logical Minimum (0)
    0x27, 0xFF, 0xFF, 0x00, 0x00, // Logical Maximum (65535)
    0x75, 0x10, //     Report Size (16)
    0x95, 0x04, //     Report Count (4)
    0x81, 0x02, //     Input (Data, Variable, Absolute)
    0x05, 0x02, //     Usage Page (Simulation Controls)
    0x09, 0xC5, //     Usage (Brake): left trigger
    0x09, 0xC4, //     Usage (Accelerator): right trigger
    0x15, 0x00, //     Logical Minimum (0)
    0x26, 0xFF, 0x7F, // Logical Maximum (32767)
    0x75, 0x10, //     Report Size (16)
    0x95, 0x02, //     Report Count (2)
    0x81, 0x02, //     Input (Data, Variable, Absolute)
    0xC0, //         End Collection
    0x05, 0x09, //   Usage Page (Button)
    0x19, 0x01, //   Usage Minimum (1)
    0x29, 0x10, //   Usage Maximum (16)
    0x15, 0x00, //   Logical Minimum (0)
    0x25, 0x01, //   Logical Maximum (1)
    0x95, 0x10, //   Report Count (16)
    0x75, 0x01, //   Report Size (1)
    0x81, 0x02, //   Input (Data, Variable, Absolute)
    0x05, 0x01, //   Usage Page (Generic Desktop)
    0x09, 0x39, //   Usage (Hat switch)
    0x15, 0x01, //   Logical Minimum (1)
    0x25, 0x08, //   Logical Maximum (8)
    0x75, 0x04, //   Report Size (4)
    0x95, 0x01, //   Report Count (1)
    0x81, 0x42, //   Input (Data, Variable, Null State)
    0x75, 0x04, //   Report Size (4): padding
    0x95, 0x01, //   Report Count (1)
    0x81, 0x01, //   Input (Constant)
    0xC0, //       End Collection
];

pub const REPORT_SIZE: usize = 15;

/// Button bits; each is the HID button number minus one, which Linux maps to `BTN_A`.. `BTN_THUMBR`.
pub mod button {
    pub const SOUTH: u16 = 1 << 0;
    pub const EAST: u16 = 1 << 1;
    pub const WEST: u16 = 1 << 3;
    pub const NORTH: u16 = 1 << 4;
    pub const LEFT_SHOULDER: u16 = 1 << 6;
    pub const RIGHT_SHOULDER: u16 = 1 << 7;
    pub const BACK: u16 = 1 << 10;
    pub const START: u16 = 1 << 11;
    pub const GUIDE: u16 = 1 << 12;
    pub const LEFT_STICK: u16 = 1 << 13;
    pub const RIGHT_STICK: u16 = 1 << 14;
}

/// One controller's state. Sticks run -1 (left/down) to 1 (right/up), triggers 0 to 1.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct GamepadState {
    pub left_x: f32,
    pub left_y: f32,
    pub right_x: f32,
    pub right_y: f32,
    pub left_trigger: f32,
    pub right_trigger: f32,
    pub buttons: u16,
    pub dpad_up: bool,
    pub dpad_down: bool,
    pub dpad_left: bool,
    pub dpad_right: bool,
}

impl GamepadState {
    pub fn report(&self) -> [u8; REPORT_SIZE] {
        let mut report = [0u8; REPORT_SIZE];
        // HID sticks grow rightward and downward, so up (positive) becomes a low value.
        let fields = [
            stick(self.left_x),
            stick(-self.left_y),
            stick(self.right_x),
            stick(-self.right_y),
            trigger(self.left_trigger),
            trigger(self.right_trigger),
            self.buttons,
        ];
        for (index, value) in fields.iter().enumerate() {
            report[index * 2..index * 2 + 2].copy_from_slice(&value.to_le_bytes());
        }
        report[14] = self.hat();
        report
    }

    /// 1 is up, counting clockwise to 8 (up-left); 0 is centered.
    fn hat(&self) -> u8 {
        let vertical = i8::from(self.dpad_down) - i8::from(self.dpad_up);
        let horizontal = i8::from(self.dpad_right) - i8::from(self.dpad_left);
        match (vertical, horizontal) {
            (-1, 0) => 1,
            (-1, 1) => 2,
            (0, 1) => 3,
            (1, 1) => 4,
            (1, 0) => 5,
            (1, -1) => 6,
            (0, -1) => 7,
            (-1, -1) => 8,
            _ => 0,
        }
    }
}

fn stick(value: f32) -> u16 {
    let unit = (value.clamp(-1.0, 1.0) + 1.0) / 2.0;
    (unit * f32::from(u16::MAX)).round() as u16
}

fn trigger(value: f32) -> u16 {
    (value.clamp(0.0, 1.0) * f32::from(i16::MAX as u16)).round() as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(report: &[u8; REPORT_SIZE], index: usize) -> u16 {
        u16::from_le_bytes([report[index * 2], report[index * 2 + 1]])
    }

    #[test]
    fn a_resting_pad_centers_its_sticks() {
        let report = GamepadState::default().report();
        for index in 0..4 {
            assert!(field(&report, index).abs_diff(32768) <= 1);
        }
        assert_eq!(field(&report, 4), 0);
        assert_eq!(field(&report, 6), 0);
        assert_eq!(report[14], 0);
    }

    #[test]
    fn up_on_a_stick_is_the_low_end() {
        let report = GamepadState {
            left_y: 1.0,
            right_x: 1.0,
            ..Default::default()
        }
        .report();
        assert_eq!(field(&report, 1), 0);
        assert_eq!(field(&report, 2), u16::MAX);
    }

    #[test]
    fn triggers_and_buttons_fill_their_fields() {
        let report = GamepadState {
            left_trigger: 1.0,
            right_trigger: 0.5,
            buttons: button::SOUTH | button::START,
            ..Default::default()
        }
        .report();
        assert_eq!(field(&report, 4), 32767);
        assert!(field(&report, 5).abs_diff(16384) <= 1);
        assert_eq!(field(&report, 6), 0x0801);
    }

    #[test]
    fn the_dpad_becomes_a_clockwise_hat() {
        let pad = |up, right, down, left| GamepadState {
            dpad_up: up,
            dpad_right: right,
            dpad_down: down,
            dpad_left: left,
            ..Default::default()
        };
        assert_eq!(pad(true, false, false, false).report()[14], 1);
        assert_eq!(pad(true, true, false, false).report()[14], 2);
        assert_eq!(pad(false, false, true, false).report()[14], 5);
        assert_eq!(pad(true, false, false, true).report()[14], 8);
        assert_eq!(pad(true, false, true, false).report()[14], 0);
    }

    #[test]
    fn the_descriptor_describes_a_fifteen_byte_report() {
        // 4 sticks + 2 triggers at 16 bits, 16 one-bit buttons, a 4-bit hat and 4 bits of padding.
        assert_eq!((4 * 16 + 2 * 16 + 16 + 4 + 4) / 8, REPORT_SIZE);
        assert_eq!(REPORT_DESCRIPTOR.first(), Some(&0x05));
        assert_eq!(REPORT_DESCRIPTOR.last(), Some(&0xC0));
    }
}
