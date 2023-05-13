use std::collections::HashSet;
use winit::event::{ElementState, VirtualKeyCode};

pub struct Keyboard {
    pressed_keys: HashSet<VirtualKeyCode>,
    held_keys: HashSet<VirtualKeyCode>,
    released_keys: HashSet<VirtualKeyCode>,
}

impl Keyboard {
    pub(crate) fn new() -> Self {
        Self {
            pressed_keys: HashSet::new(),
            held_keys: HashSet::new(),
            released_keys: HashSet::new(),
        }
    }
    pub(crate) fn update_with_input(&mut self, input: &winit::event::KeyboardInput) {
        let keycode = input.virtual_keycode.unwrap();
        self.pressed_keys.clear();
        self.released_keys.clear();

        if input.state == ElementState::Pressed {
            // Check if the key is already held before inserting to prevent key repeats from registering
            if !self.held_keys.contains(&keycode) {
                self.pressed_keys.insert(keycode);
                self.held_keys.insert(keycode);
            }
        } else {
            self.held_keys.remove(&input.virtual_keycode.unwrap());
            self.released_keys.insert(keycode);
        }
    }

    pub fn key_pressed(&self, keycode: VirtualKeyCode) -> bool {
        self.pressed_keys.contains(&keycode)
    }
    pub fn key_down(&self, keycode: VirtualKeyCode) -> bool {
        self.held_keys.contains(&keycode)
    }
    pub fn key_released(&self, keycode: VirtualKeyCode) -> bool {
        self.released_keys.contains(&keycode)
    }
}
