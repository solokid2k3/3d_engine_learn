/// Interactive object transform controls — Blender-style G/R/S keyboard modes.
///
/// Blender flow:
///   Press G/R/S → mouse movement applies transform immediately (no click hold)
///   Left-click  → confirm transform
///   Right-click / Escape → revert to snapshot (cancel)

use glam::Vec3;

/// Which transform operation is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformMode {
    /// No active transform — camera controls are active.
    None,
    /// Grab (translate) — press G.
    Grab,
    /// Rotate — press R.
    Rotate,
    /// Scale — press S.
    Scale,
}

impl TransformMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::None => "",
            Self::Grab => "GRAB",
            Self::Rotate => "ROTATE",
            Self::Scale => "SCALE",
        }
    }
}

/// Axis constraint applied to the active transform mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisConstraint {
    /// Free — all axes.
    Free,
    X,
    Y,
    Z,
}

impl AxisConstraint {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Free => "Free",
            Self::X => "X",
            Self::Y => "Y",
            Self::Z => "Z",
        }
    }
}

/// Snapshot of a model's transform before an operation begins (for cancel/revert).
#[derive(Debug, Clone)]
pub struct TransformSnapshot {
    pub position: [f32; 3],
    pub rotation_deg: [f32; 3],
    pub scale: [f32; 3],
}

/// Persistent state for the object transform controller.
pub struct ObjectControls {
    pub mode: TransformMode,
    pub axis: AxisConstraint,
    /// Snapshot of the transform before the current operation started.
    /// Used to revert on cancel (right-click / Escape).
    pub snapshot: Option<TransformSnapshot>,
}

impl ObjectControls {
    pub fn new() -> Self {
        Self {
            mode: TransformMode::None,
            axis: AxisConstraint::Free,
            snapshot: None,
        }
    }

    /// Enter a transform mode (resets axis constraint).
    /// `current_transform` is the selected model's current state — saved as a snapshot
    /// so we can revert on cancel.
    pub fn enter_mode(&mut self, mode: TransformMode, current_transform: Option<TransformSnapshot>) {
        if self.mode == mode {
            // Toggle off if pressing the same key again — confirm the transform.
            self.confirm();
        } else {
            self.mode = mode;
            self.axis = AxisConstraint::Free;
            self.snapshot = current_transform;
        }
    }

    /// Set axis constraint (only meaningful when a mode is active).
    pub fn set_axis(&mut self, axis: AxisConstraint) {
        if self.mode != TransformMode::None {
            // Toggle off if pressing the same axis key again.
            if self.axis == axis {
                self.axis = AxisConstraint::Free;
            } else {
                self.axis = axis;
            }
        }
    }

    /// Confirm the current transform — clear mode and discard snapshot.
    pub fn confirm(&mut self) {
        self.mode = TransformMode::None;
        self.axis = AxisConstraint::Free;
        self.snapshot = None;
    }

    /// Cancel the current transform mode — caller should revert using the snapshot.
    /// Returns the snapshot to revert to, if any.
    pub fn cancel(&mut self) -> Option<TransformSnapshot> {
        self.mode = TransformMode::None;
        self.axis = AxisConstraint::Free;
        self.snapshot.take()
    }

    /// Returns true if a transform mode is active (blocks camera input).
    pub fn is_active(&self) -> bool {
        self.mode != TransformMode::None
    }

    /// Compute world-space transform deltas from screen-space mouse movement.
    ///
    /// **Camera-relative**: uses the camera's right/up vectors so that dragging
    /// right on screen always moves the object right from the user's perspective,
    /// regardless of camera orientation.
    ///
    /// Returns the (position_delta, rotation_deg_delta, scale_delta) to apply.
    pub fn compute_delta(
        &self,
        mouse_dx: f32,
        mouse_dy: f32,
        camera_distance: f32,
        camera_right: Vec3,
        camera_up: Vec3,
    ) -> ([f32; 3], [f32; 3], [f32; 3]) {
        let mut pos = [0.0_f32; 3];
        let mut rot = [0.0_f32; 3];
        let mut scl = [0.0_f32; 3];

        match self.mode {
            TransformMode::None => {}
            TransformMode::Grab => {
                // Scale move speed with camera distance for natural feel.
                let speed = 0.005 * camera_distance.max(0.5);

                // Project screen movement into world space using camera vectors.
                // mouse_dx → camera right direction
                // mouse_dy → camera up direction (negated because screen Y is down)
                let world_delta = camera_right * (mouse_dx * speed)
                    + camera_up * (-mouse_dy * speed);

                match self.axis {
                    AxisConstraint::Free => {
                        pos[0] = world_delta.x;
                        pos[1] = world_delta.y;
                        pos[2] = world_delta.z;
                    }
                    // Constrained: project the full world delta onto the constraint axis.
                    AxisConstraint::X => {
                        let combined = mouse_dx - mouse_dy;
                        pos[0] = combined * speed;
                    }
                    AxisConstraint::Y => {
                        let combined = mouse_dx - mouse_dy;
                        pos[1] = combined * speed;
                    }
                    AxisConstraint::Z => {
                        let combined = mouse_dx - mouse_dy;
                        pos[2] = combined * speed;
                    }
                }
            }
            TransformMode::Rotate => {
                let speed = 0.3; // degrees per pixel
                let delta = (mouse_dx - mouse_dy) * speed;
                match self.axis {
                    AxisConstraint::Free => {
                        // Horizontal mouse → rotate around Y (yaw)
                        // Vertical mouse → rotate around X (pitch)
                        rot[1] = mouse_dx * speed;
                        rot[0] = mouse_dy * speed;
                    }
                    AxisConstraint::X => rot[0] = delta,
                    AxisConstraint::Y => rot[1] = delta,
                    AxisConstraint::Z => rot[2] = delta,
                }
            }
            TransformMode::Scale => {
                let speed = 0.005;
                let delta = (mouse_dx - mouse_dy) * speed; // Right/up = bigger.
                match self.axis {
                    AxisConstraint::Free => {
                        scl[0] = delta;
                        scl[1] = delta;
                        scl[2] = delta;
                    }
                    AxisConstraint::X => scl[0] = delta,
                    AxisConstraint::Y => scl[1] = delta,
                    AxisConstraint::Z => scl[2] = delta,
                }
            }
        }

        (pos, rot, scl)
    }
}
