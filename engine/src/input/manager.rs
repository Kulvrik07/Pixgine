use std::collections::HashSet;
use winit::keyboard::KeyCode;

/// Tracks input state across frames
#[derive(Debug, Clone)]
pub struct InputManager {
    keys_pressed: HashSet<KeyCode>,
    keys_just_pressed: HashSet<KeyCode>,
    keys_just_released: HashSet<KeyCode>,
    mouse_x: f32,
    mouse_y: f32,
    mouse_buttons_pressed: HashSet<u32>,
    mouse_buttons_just_pressed: HashSet<u32>,
}

impl InputManager {
    pub fn new() -> Self {
        Self {
            keys_pressed: HashSet::new(),
            keys_just_pressed: HashSet::new(),
            keys_just_released: HashSet::new(),
            mouse_x: 0.0,
            mouse_y: 0.0,
            mouse_buttons_pressed: HashSet::new(),
            mouse_buttons_just_pressed: HashSet::new(),
        }
    }

    /// Call at the end of each frame to reset frame-scoped state
    pub fn end_frame(&mut self) {
        self.keys_just_pressed.clear();
        self.keys_just_released.clear();
        self.mouse_buttons_just_pressed.clear();
    }

    /// Handle a key press event
    pub fn handle_key_down(&mut self, key: KeyCode) {
        if !self.keys_pressed.contains(&key) {
            self.keys_just_pressed.insert(key);
        }
        self.keys_pressed.insert(key);
    }

    /// Handle a key release event
    pub fn handle_key_up(&mut self, key: KeyCode) {
        self.keys_pressed.remove(&key);
        self.keys_just_released.insert(key);
    }

    /// Check if a key is currently held down
    pub fn is_key_down(&self, key: KeyCode) -> bool {
        self.keys_pressed.contains(&key)
    }

    /// Check if a key was just pressed this frame
    pub fn is_key_just_pressed(&self, key: KeyCode) -> bool {
        self.keys_just_pressed.contains(&key)
    }

    /// Check if a key was just released this frame
    pub fn is_key_just_released(&self, key: KeyCode) -> bool {
        self.keys_just_released.contains(&key)
    }

    /// Update mouse position
    pub fn set_mouse_position(&mut self, x: f32, y: f32) {
        self.mouse_x = x;
        self.mouse_y = y;
    }

    pub fn mouse_position(&self) -> (f32, f32) {
        (self.mouse_x, self.mouse_y)
    }

    pub fn handle_mouse_down(&mut self, button: u32) {
        if !self.mouse_buttons_pressed.contains(&button) {
            self.mouse_buttons_just_pressed.insert(button);
        }
        self.mouse_buttons_pressed.insert(button);
    }

    pub fn handle_mouse_up(&mut self, button: u32) {
        self.mouse_buttons_pressed.remove(&button);
    }

    pub fn is_mouse_button_down(&self, button: u32) -> bool {
        self.mouse_buttons_pressed.contains(&button)
    }

    pub fn is_mouse_just_pressed(&self, button: u32) -> bool {
        self.mouse_buttons_just_pressed.contains(&button)
    }
}