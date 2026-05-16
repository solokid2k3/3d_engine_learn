use glam::Vec2;
use winit::event::{ElementState, MouseButton, MouseScrollDelta};
use winit::keyboard::{KeyCode, PhysicalKey};

/// Tracks the current state of keyboard + mouse input each frame.
pub struct InputState {
    pub keys_pressed: std::collections::HashSet<KeyCode>,
    pub mouse_position: Vec2,
    pub mouse_delta: Vec2,
    pub mouse_buttons: std::collections::HashSet<MouseButton>,
    pub scroll_delta: f32,
    pub double_click_detected: bool,
    last_mouse_position: Option<Vec2>,
    last_click_time: Option<std::time::Instant>,
    last_click_button: Option<MouseButton>,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            keys_pressed: std::collections::HashSet::new(),
            mouse_position: Vec2::ZERO,
            mouse_delta: Vec2::ZERO,
            mouse_buttons: std::collections::HashSet::new(),
            scroll_delta: 0.0,
            double_click_detected: false,
            last_mouse_position: None,
            last_click_time: None,
            last_click_button: None,
        }
    }

    /// Call at the start of each frame to reset per-frame deltas.
    pub fn begin_frame(&mut self) {
        self.mouse_delta = Vec2::ZERO;
        self.scroll_delta = 0.0;
        self.double_click_detected = false;
    }

    pub fn process_key(&mut self, key: PhysicalKey, state: ElementState) {
        if let PhysicalKey::Code(code) = key {
            match state {
                ElementState::Pressed => {
                    self.keys_pressed.insert(code);
                }
                ElementState::Released => {
                    self.keys_pressed.remove(&code);
                }
            }
        }
    }

    pub fn process_mouse_button(&mut self, button: MouseButton, state: ElementState) {
        match state {
            ElementState::Pressed => {
                // Detect double-click (within 300ms on same button)
                let now = std::time::Instant::now();
                if let (Some(last_time), Some(last_btn)) =
                    (self.last_click_time, self.last_click_button)
                {
                    if last_btn == button && now.duration_since(last_time).as_millis() < 300 {
                        self.double_click_detected = true;
                    }
                }
                self.last_click_time = Some(now);
                self.last_click_button = Some(button);

                self.mouse_buttons.insert(button);
            }
            ElementState::Released => {
                self.mouse_buttons.remove(&button);
            }
        }
    }

    pub fn process_mouse_move(&mut self, position: (f64, f64)) {
        let new_pos = Vec2::new(position.0 as f32, position.1 as f32);
        if let Some(last) = self.last_mouse_position {
            self.mouse_delta = new_pos - last;
        }
        self.last_mouse_position = Some(new_pos);
        self.mouse_position = new_pos;
    }

    pub fn process_scroll(&mut self, delta: MouseScrollDelta) {
        self.scroll_delta = match delta {
            MouseScrollDelta::LineDelta(_, y) => y,
            MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 100.0,
        };
    }

    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.keys_pressed.contains(&key)
    }

    pub fn is_mouse_button_pressed(&self, button: MouseButton) -> bool {
        self.mouse_buttons.contains(&button)
    }

    /// Check if Shift is held (either side).
    pub fn is_shift_held(&self) -> bool {
        self.keys_pressed.contains(&KeyCode::ShiftLeft)
            || self.keys_pressed.contains(&KeyCode::ShiftRight)
    }
}
