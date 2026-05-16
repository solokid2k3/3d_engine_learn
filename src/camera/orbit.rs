use glam::Vec3;
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

use crate::camera::Camera;
use crate::input::InputState;

/// Full 3D viewport camera controller with orbit, pan, zoom, fly, and smooth damping.
///
/// Controls:
///   Left-click drag        → Orbit (rotate around target)
///   Right-click drag       → Pan (translate target in screen plane)
///   Middle-click drag      → Pan (alternate)
///   Shift + Left-click     → Pan (alternate)
///   Scroll wheel           → Zoom in/out (smooth)
///   WASD / Arrow keys      → Fly-move camera target
///   Q / E                  → Move target down / up
///   Home / Double-click    → Reset camera to home position
pub struct OrbitController {
    // Orbit state
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,

    // Sensitivity
    pub orbit_sensitivity: f32,
    pub pan_sensitivity: f32,
    pub zoom_speed: f32,
    pub fly_speed: f32,

    // Limits
    pub min_distance: f32,
    pub max_distance: f32,
    pub min_pitch: f32,
    pub max_pitch: f32,

    // Smooth damping state
    velocity_yaw: f32,
    velocity_pitch: f32,
    velocity_distance: f32,
    velocity_pan: Vec3,
    pub damping: f32, // 0.0 = instant, 0.95 = very smooth

    // Home position (for reset)
    home_yaw: f32,
    home_pitch: f32,
    home_distance: f32,
    home_target: Vec3,
}

impl OrbitController {
    pub fn new() -> Self {
        let yaw = -90.0_f32.to_radians();
        let pitch = 20.0_f32.to_radians();
        let distance = 5.0;

        Self {
            yaw,
            pitch,
            distance,

            orbit_sensitivity: 0.005,
            pan_sensitivity: 0.005,
            zoom_speed: 0.5,
            fly_speed: 3.0,

            min_distance: 0.5,
            max_distance: 50.0,
            min_pitch: -85.0_f32.to_radians(),
            max_pitch: 85.0_f32.to_radians(),

            velocity_yaw: 0.0,
            velocity_pitch: 0.0,
            velocity_distance: 0.0,
            velocity_pan: Vec3::ZERO,
            damping: 0.85,

            home_yaw: yaw,
            home_pitch: pitch,
            home_distance: distance,
            home_target: Vec3::ZERO,
        }
    }

    /// Process all mouse + keyboard input and update the camera.
    pub fn update(&mut self, input: &InputState, camera: &mut Camera, dt: f32) {
        let left_held = input.is_mouse_button_pressed(MouseButton::Left);
        let right_held = input.is_mouse_button_pressed(MouseButton::Right);
        let middle_held = input.is_mouse_button_pressed(MouseButton::Middle);
        let shift_held = input.is_shift_held();

        // --- Double-click or Home key: reset camera ---
        if input.double_click_detected || input.is_key_pressed(KeyCode::Home) {
            self.reset(camera);
            return;
        }

        // --- Determine action from mouse buttons + modifiers ---
        let is_orbiting = left_held && !shift_held;
        let is_panning = right_held || middle_held || (left_held && shift_held);

        // --- Orbit ---
        if is_orbiting && (input.mouse_delta.x != 0.0 || input.mouse_delta.y != 0.0) {
            self.velocity_yaw = input.mouse_delta.x * self.orbit_sensitivity;
            self.velocity_pitch = input.mouse_delta.y * self.orbit_sensitivity;
        }

        // --- Pan ---
        if is_panning && (input.mouse_delta.x != 0.0 || input.mouse_delta.y != 0.0) {
            // Compute the camera's local right and up vectors in world space
            let forward = (camera.target - camera.eye).normalize();
            let right = forward.cross(camera.up).normalize();
            let up = right.cross(forward).normalize();

            let pan_amount = self.pan_sensitivity * self.distance; // Scale pan with distance
            let pan = -right * input.mouse_delta.x * pan_amount
                + up * input.mouse_delta.y * pan_amount;

            self.velocity_pan = pan;
        }

        // --- Zoom (scroll) ---
        if input.scroll_delta != 0.0 {
            self.velocity_distance = -input.scroll_delta * self.zoom_speed;
        }

        // --- Keyboard fly movement (WASD + QE + Arrows) ---
        let mut fly_delta = Vec3::ZERO;
        {
            let forward = (camera.target - camera.eye).normalize();
            let right = forward.cross(camera.up).normalize();

            if input.is_key_pressed(KeyCode::KeyW) || input.is_key_pressed(KeyCode::ArrowUp) {
                fly_delta += forward;
            }
            if input.is_key_pressed(KeyCode::KeyS) || input.is_key_pressed(KeyCode::ArrowDown) {
                fly_delta -= forward;
            }
            if input.is_key_pressed(KeyCode::KeyA) || input.is_key_pressed(KeyCode::ArrowLeft) {
                fly_delta -= right;
            }
            if input.is_key_pressed(KeyCode::KeyD) || input.is_key_pressed(KeyCode::ArrowRight) {
                fly_delta += right;
            }
            if input.is_key_pressed(KeyCode::KeyE) || input.is_key_pressed(KeyCode::Space) {
                fly_delta += Vec3::Y;
            }
            if input.is_key_pressed(KeyCode::KeyQ) || input.is_key_pressed(KeyCode::ControlLeft) {
                fly_delta -= Vec3::Y;
            }
        }

        if fly_delta.length_squared() > 0.0 {
            fly_delta = fly_delta.normalize() * self.fly_speed * dt;
            camera.target += fly_delta;
        }

        // --- Apply velocities with damping ---
        self.yaw += self.velocity_yaw;
        self.pitch += self.velocity_pitch;
        self.pitch = self.pitch.clamp(self.min_pitch, self.max_pitch);

        self.distance += self.velocity_distance;
        self.distance = self.distance.clamp(self.min_distance, self.max_distance);

        camera.target += self.velocity_pan;

        // Damp velocities
        self.velocity_yaw *= self.damping;
        self.velocity_pitch *= self.damping;
        self.velocity_distance *= self.damping;
        self.velocity_pan *= self.damping;

        // Kill tiny velocities to avoid infinite drift
        if self.velocity_yaw.abs() < 0.00001 {
            self.velocity_yaw = 0.0;
        }
        if self.velocity_pitch.abs() < 0.00001 {
            self.velocity_pitch = 0.0;
        }
        if self.velocity_distance.abs() < 0.00001 {
            self.velocity_distance = 0.0;
        }
        if self.velocity_pan.length_squared() < 0.0000001 {
            self.velocity_pan = Vec3::ZERO;
        }

        // --- Compute final eye position from spherical coordinates ---
        let x = self.distance * self.pitch.cos() * self.yaw.cos();
        let y = self.distance * self.pitch.sin();
        let z = self.distance * self.pitch.cos() * self.yaw.sin();

        camera.eye = camera.target + Vec3::new(x, y, z);
    }

    /// Reset camera to home position with smooth transition.
    fn reset(&mut self, camera: &mut Camera) {
        self.yaw = self.home_yaw;
        self.pitch = self.home_pitch;
        self.distance = self.home_distance;
        camera.target = self.home_target;

        // Kill all velocity
        self.velocity_yaw = 0.0;
        self.velocity_pitch = 0.0;
        self.velocity_distance = 0.0;
        self.velocity_pan = Vec3::ZERO;

        // Recompute eye
        let x = self.distance * self.pitch.cos() * self.yaw.cos();
        let y = self.distance * self.pitch.sin();
        let z = self.distance * self.pitch.cos() * self.yaw.sin();
        camera.eye = camera.target + Vec3::new(x, y, z);

        log::info!("Camera reset to home position");
    }
}
