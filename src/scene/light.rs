use bytemuck::{Pod, Zeroable};

/// Directional light (sun-like).
#[derive(Debug, Clone)]
pub struct DirectionalLight {
    pub direction: [f32; 3],
    pub color: [f32; 3],
    pub intensity: f32,
}

impl DirectionalLight {
    pub fn new(direction: [f32; 3], color: [f32; 3], intensity: f32) -> Self {
        Self {
            direction,
            color,
            intensity,
        }
    }
}

/// Point light with attenuation.
#[derive(Debug, Clone)]
pub struct PointLight {
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub intensity: f32,
    pub constant: f32,
    pub linear: f32,
    pub quadratic: f32,
}

impl PointLight {
    pub fn new(position: [f32; 3], color: [f32; 3], intensity: f32) -> Self {
        Self {
            position,
            color,
            intensity,
            constant: 1.0,
            linear: 0.09,
            quadratic: 0.032,
        }
    }
}

/// GPU-uploadable light data. Supports 1 directional + up to 4 point lights.
#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct LightUniform {
    // Directional light
    pub dir_direction: [f32; 4], // xyz = direction, w = intensity
    pub dir_color: [f32; 4],     // xyz = color, w = unused

    // Point lights (up to 4)
    pub point_positions: [[f32; 4]; 4],   // xyz = position, w = intensity
    pub point_colors: [[f32; 4]; 4],      // xyz = color, w = unused
    pub point_attenuation: [[f32; 4]; 4], // x = constant, y = linear, z = quadratic, w = unused

    pub num_point_lights: [f32; 4], // x = count, yzw = padding
}

impl Default for LightUniform {
    fn default() -> Self {
        Self {
            dir_direction: [0.0, -1.0, -0.5, 1.0],
            dir_color: [1.0, 0.95, 0.9, 0.0],
            point_positions: [[0.0; 4]; 4],
            point_colors: [[0.0; 4]; 4],
            point_attenuation: [[1.0, 0.09, 0.032, 0.0]; 4],
            num_point_lights: [0.0; 4],
        }
    }
}

impl LightUniform {
    pub fn set_directional(&mut self, light: &DirectionalLight) {
        self.dir_direction = [
            light.direction[0],
            light.direction[1],
            light.direction[2],
            light.intensity,
        ];
        self.dir_color = [light.color[0], light.color[1], light.color[2], 0.0];
    }

    pub fn set_point_light(&mut self, index: usize, light: &PointLight) {
        if index < 4 {
            self.point_positions[index] = [
                light.position[0],
                light.position[1],
                light.position[2],
                light.intensity,
            ];
            self.point_colors[index] = [light.color[0], light.color[1], light.color[2], 0.0];
            self.point_attenuation[index] =
                [light.constant, light.linear, light.quadratic, 0.0];
            self.num_point_lights[0] = (index + 1) as f32;
        }
    }
}
