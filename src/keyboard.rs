use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VIRTUAL_KEY};

pub struct KeyboardManager {
    keys: [KeyState; 256],
}

impl KeyboardManager {
    pub fn new() -> Self {
        KeyboardManager {
            keys: [KeyState::Released; 256],
        }
    }

    pub fn get_key_state(&mut self, key: VIRTUAL_KEY) -> KeyState {
        if key.0 > 256 {
            panic!("Virtual keys can't have an index of more than 256!")
        }

        let new_state = get_key_state(key);
        let old_state = self.keys[key.0 as usize];

        if old_state != new_state {
            self.keys[key.0 as usize] = new_state;
            new_state
        } else {
            KeyState::Unchanged
        }
    }
}

#[derive(Debug, Copy, Clone, PartialOrd, PartialEq)]
pub enum KeyState {
    Pressed,
    Unchanged,
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
