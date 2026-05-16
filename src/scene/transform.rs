use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Quat, Vec3};

/// Transform component: position, rotation, scale → model matrix.
#[derive(Debug, Clone)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
    pub fn new() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }

    pub fn from_position(position: Vec3) -> Self {
        Self {
            position,
            ..Self::new()
        }
    }

    pub fn to_model_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position)
    }

    /// The normal matrix is the transpose of the inverse of the upper-left 3x3
    /// of the model matrix, padded to 3x Vec4 for GPU alignment.
    pub fn to_normal_matrix(&self) -> Mat4 {
        self.to_model_matrix().inverse().transpose()
    }
}

/// GPU-uploadable model transform uniform.
#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct TransformUniform {
    pub model: [[f32; 4]; 4],
    pub normal: [[f32; 4]; 4],
}

impl TransformUniform {
    pub fn from_transform(transform: &Transform) -> Self {
        Self {
            model: transform.to_model_matrix().to_cols_array_2d(),
            normal: transform.to_normal_matrix().to_cols_array_2d(),
        }
    }
}
