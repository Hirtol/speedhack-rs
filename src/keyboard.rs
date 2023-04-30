use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VIRTUAL_KEY};

pub struct KeyboardManager {
    keys: [KeyState; 256],
    next_frame: [KeyState; 256],
}

impl KeyboardManager {
    pub fn new() -> Self {
        KeyboardManager {
            keys: [KeyState::Up; 256],
            next_frame: [KeyState::Up; 256],
        }
    }

    pub fn get_key_state(&mut self, key: VIRTUAL_KEY) -> KeyState {
        if key.0 > 256 {
            panic!("Virtual keys can't have an index of more than 256!")
        }

        let new_state = get_key_state(key);
        let old_state = self.keys[key.0 as usize];

        if old_state != new_state {
            self.next_frame[key.0 as usize] = new_state;
            new_state
        } else if new_state == KeyState::Pressed {
            KeyState::Down
        } else {
            KeyState::Up
        }
    }

    pub fn end_frame(&mut self) {
        self.keys = self.next_frame;
    }

    /// Returns `true` if all given `keys` are either [KeyState::Down] or [KeyState::Pressed], with at least *one* [
    pub fn all_pressed(&mut self, keys: impl Iterator<Item = VIRTUAL_KEY>) -> bool {
        let states = keys.map(|key| self.get_key_state(key)).collect::<Vec<_>>();

        states.iter().any(|key| *key == KeyState::Pressed)
            && states
                .iter()
                .all(|key| *key == KeyState::Pressed || *key == KeyState::Down)
    }

    /// Returns `true` if any of the keys are [KeyState::Released]
    pub fn any_released(&mut self, keys: impl Iterator<Item = VIRTUAL_KEY>) -> bool {
        let states = keys.map(|key| self.get_key_state(key)).collect::<Vec<_>>();

        states.iter().any(|key| *key == KeyState::Released)
    }
}

#[derive(Debug, Copy, Clone, PartialOrd, PartialEq)]
pub enum KeyState {
    Pressed,
    Down,
    Up,
    Released,
}

pub fn get_key_state(key: VIRTUAL_KEY) -> KeyState {
    unsafe {
        let value = GetAsyncKeyState(key.0 as i32) as u16;

        if value & 0x8000 != 0 {
            KeyState::Pressed
        } else {
            KeyState::Released
        }
    }
}
