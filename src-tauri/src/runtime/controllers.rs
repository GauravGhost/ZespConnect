use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use gilrs::{Axis, Button, EventType, Gamepad, GamepadId, Gilrs};
use tauri::{AppHandle, Emitter};

use crate::models::Controller;
use crate::runtime::mirror_session::ControlChannel;
use crate::services::control;
use crate::services::hid_gamepad::{self, button, GamepadState};

pub const EVENT_CHANGED: &str = "game:controllers";
/// How often the bridge thread looks up from waiting for input to check it should stop.
const STOP_CHECK: Duration = Duration::from_millis(250);
/// Long enough for the OS to report the controllers that are already plugged in.
const ENUMERATE_WAIT: Duration = Duration::from_millis(300);
/// scrcpy's own keyboard and mouse take UHID ids 1 and 2; gamepads start after them.
const FIRST_UHID_ID: u16 = 3;

/// Forwards every controller on this computer to one session's phone until dropped.
pub struct ControllerBridge {
    stop: Arc<AtomicBool>,
}

impl ControllerBridge {
    pub fn start(app: AppHandle, control: ControlChannel) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        std::thread::spawn(move || run(&app, &control, &flag));
        ControllerBridge { stop }
    }
}

impl Drop for ControllerBridge {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// The controllers this computer sees right now.
pub fn list() -> Vec<Controller> {
    let Some(mut gilrs) = open() else {
        return Vec::new();
    };
    let deadline = std::time::Instant::now() + ENUMERATE_WAIT;
    while std::time::Instant::now() < deadline {
        gilrs.next_event_blocking(Some(ENUMERATE_WAIT));
    }
    connected(&gilrs)
}

fn open() -> Option<Gilrs> {
    match Gilrs::new() {
        Ok(gilrs) => Some(gilrs),
        // A platform without controller support still gives a usable (empty) instance.
        Err(gilrs::Error::NotImplemented(gilrs)) => Some(gilrs),
        Err(_) => None,
    }
}

fn connected(gilrs: &Gilrs) -> Vec<Controller> {
    gilrs
        .gamepads()
        .map(|(id, pad)| Controller {
            id: usize::from(id),
            name: pad.name().to_string(),
        })
        .collect()
}

fn run(app: &AppHandle, control: &ControlChannel, stop: &AtomicBool) {
    let Some(mut gilrs) = open() else {
        return;
    };
    let mut pads = Pads::default();
    for (id, pad) in gilrs.gamepads() {
        pads.add(control, id, &pad);
    }
    let _ = app.emit(EVENT_CHANGED, connected(&gilrs));

    while !stop.load(Ordering::Relaxed) {
        let Some(event) = gilrs.next_event_blocking(Some(STOP_CHECK)) else {
            continue;
        };
        let sent = match event.event {
            EventType::Connected => {
                let sent = pads.add(control, event.id, &gilrs.gamepad(event.id));
                let _ = app.emit(EVENT_CHANGED, connected(&gilrs));
                sent
            }
            EventType::Disconnected => {
                let sent = pads.remove(control, event.id);
                let _ = app.emit(EVENT_CHANGED, connected(&gilrs));
                sent
            }
            _ => pads.update(control, event.id, &gilrs.gamepad(event.id)),
        };
        // The session's control socket closed, so there is no phone to drive.
        if !sent {
            break;
        }
    }
    pads.remove_all(control);
}

/// The phone-side gamepad of each connected controller.
#[derive(Default)]
struct Pads {
    uhid_ids: HashMap<GamepadId, u16>,
    next_id: u16,
}

impl Pads {
    fn add(&mut self, control: &ControlChannel, id: GamepadId, pad: &Gamepad<'_>) -> bool {
        if self.uhid_ids.contains_key(&id) {
            return true;
        }
        let uhid = FIRST_UHID_ID + self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        self.uhid_ids.insert(id, uhid);
        let created = control::uhid_create(uhid, pad.name(), hid_gamepad::REPORT_DESCRIPTOR);
        control.send(created).is_ok() && self.update(control, id, pad)
    }

    fn update(&self, control: &ControlChannel, id: GamepadId, pad: &Gamepad<'_>) -> bool {
        match self.uhid_ids.get(&id) {
            Some(uhid) => control
                .send(control::uhid_input(*uhid, &state_of(pad).report()))
                .is_ok(),
            None => true,
        }
    }

    fn remove(&mut self, control: &ControlChannel, id: GamepadId) -> bool {
        match self.uhid_ids.remove(&id) {
            Some(uhid) => control.send(control::uhid_destroy(uhid)).is_ok(),
            None => true,
        }
    }

    fn remove_all(&mut self, control: &ControlChannel) {
        for (_, uhid) in self.uhid_ids.drain() {
            let _ = control.send(control::uhid_destroy(uhid));
        }
    }
}

fn state_of(pad: &Gamepad<'_>) -> GamepadState {
    let trigger = |which| {
        pad.button_data(which)
            .map(|data| data.value())
            .unwrap_or(0.0)
    };
    let bits = [
        (Button::South, button::SOUTH),
        (Button::East, button::EAST),
        (Button::West, button::WEST),
        (Button::North, button::NORTH),
        (Button::LeftTrigger, button::LEFT_SHOULDER),
        (Button::RightTrigger, button::RIGHT_SHOULDER),
        (Button::Select, button::BACK),
        (Button::Start, button::START),
        (Button::Mode, button::GUIDE),
        (Button::LeftThumb, button::LEFT_STICK),
        (Button::RightThumb, button::RIGHT_STICK),
    ];
    GamepadState {
        left_x: pad.value(Axis::LeftStickX),
        left_y: pad.value(Axis::LeftStickY),
        right_x: pad.value(Axis::RightStickX),
        right_y: pad.value(Axis::RightStickY),
        left_trigger: trigger(Button::LeftTrigger2),
        right_trigger: trigger(Button::RightTrigger2),
        buttons: bits
            .iter()
            .filter(|(gilrs_button, _)| pad.is_pressed(*gilrs_button))
            .fold(0, |mask, (_, bit)| mask | bit),
        dpad_up: pad.is_pressed(Button::DPadUp),
        dpad_down: pad.is_pressed(Button::DPadDown),
        dpad_left: pad.is_pressed(Button::DPadLeft),
        dpad_right: pad.is_pressed(Button::DPadRight),
    }
}
