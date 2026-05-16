use bytemuck::{Pod, Zeroable};

/// Material properties for Blinn-Phong lighting.
#[derive(Debug, Clone)]
pub struct Material {
    pub ambient: [f32; 3],
    pub diffuse: [f32; 3],
    pub specular: [f32; 3],
    pub shininess: f32,
}

impl Material {
    /// A neutral gray material.
    pub fn default_material() -> Self {
        Self {
            ambient: [0.1, 0.1, 0.1],
            diffuse: [0.8, 0.8, 0.8],
            specular: [1.0, 1.0, 1.0],
            shininess: 32.0,
        }
    }

    /// A vibrant colored material for demo purposes.
    pub fn colored(r: f32, g: f32, b: f32) -> Self {
        Self {
            ambient: [r * 0.15, g * 0.15, b * 0.15],
            diffuse: [r, g, b],
            specular: [1.0, 1.0, 1.0],
            shininess: 64.0,
        }
    }
}

/// GPU-uploadable material data.
#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct MaterialUniform {
    pub ambient: [f32; 4],   // xyz = ambient, w = shininess
    pub diffuse: [f32; 4],   // xyz = diffuse, w = unused
    pub specular: [f32; 4],  // xyz = specular, w = unused
}

impl MaterialUniform {
    pub fn from_material(mat: &Material) -> Self {
        Self {
            ambient: [mat.ambient[0], mat.ambient[1], mat.ambient[2], mat.shininess],
            diffuse: [mat.diffuse[0], mat.diffuse[1], mat.diffuse[2], 0.0],
            specular: [mat.specular[0], mat.specular[1], mat.specular[2], 0.0],
        }
    }
}
