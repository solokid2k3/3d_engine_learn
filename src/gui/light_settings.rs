use crate::scene::light::{LightUniform, PointLight};

/// Editable settings for the directional (sun) light.
#[derive(Debug, Clone)]
pub struct DirectionalLightSettings {
    pub enabled: bool,
    pub direction: [f32; 3],
    pub color: [f32; 3],
    pub intensity: f32,
}

impl Default for DirectionalLightSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            direction: [-0.3, -0.8, -0.5],
            color: [1.0, 0.95, 0.85],
            intensity: 0.7,
        }
    }
}

/// Editable settings for a single point light.
#[derive(Debug, Clone)]
pub struct PointLightSettings {
    pub label: String,
    pub enabled: bool,
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub intensity: f32,
    pub constant: f32,
    pub linear: f32,
    pub quadratic: f32,
    /// If true, this light animates (orbits/pulses) automatically.
    pub animated: bool,
}

impl PointLightSettings {
    pub fn new(label: &str, position: [f32; 3], color: [f32; 3], intensity: f32) -> Self {
        Self {
            label: label.to_string(),
            enabled: true,
            position,
            color,
            intensity,
            constant: 1.0,
            linear: 0.09,
            quadratic: 0.032,
            animated: true,
        }
    }
}

/// Environment / post-processing settings.
#[derive(Debug, Clone)]
pub struct EnvironmentSettings {
    pub fog_density: f32,
    pub fog_color: [f32; 3],
    pub rim_strength: f32,
    pub rim_color: [f32; 3],
    pub rim_power: f32,
}

impl Default for EnvironmentSettings {
    fn default() -> Self {
        Self {
            fog_density: 0.04,
            fog_color: [0.02, 0.02, 0.05],
            rim_strength: 0.6,
            rim_color: [0.4, 0.6, 1.0],
            rim_power: 3.0,
        }
    }
}

/// Top-level runtime-editable light configuration.
pub struct LightSettings {
    pub panel_visible: bool,
    pub directional: DirectionalLightSettings,
    pub point_lights: Vec<PointLightSettings>,
    pub environment: EnvironmentSettings,
}

impl LightSettings {
    /// Create default settings matching the original hardcoded scene.
    pub fn new() -> Self {
        Self {
            panel_visible: false,
            directional: DirectionalLightSettings::default(),
            point_lights: vec![
                PointLightSettings::new("Cyan Orbit", [4.0, 2.5, 0.0], [0.3, 0.8, 1.0], 2.5),
                PointLightSettings::new("Magenta Orbit", [-3.5, 1.5, 0.0], [1.0, 0.3, 0.7], 2.0),
                PointLightSettings::new("Gold Pulse", [0.0, 3.0, 0.0], [1.0, 0.85, 0.4], 1.5),
            ],
            environment: EnvironmentSettings::default(),
        }
    }

    /// Animate the lights that have `animated: true`. Call each frame with elapsed time.
    pub fn animate(&mut self, time: f32) {
        for (i, light) in self.point_lights.iter_mut().enumerate() {
            if !light.animated || !light.enabled {
                continue;
            }
            match i {
                0 => {
                    // Cyan orbiting light
                    let angle = time * 0.7;
                    light.position = [angle.cos() * 4.0, 2.5, angle.sin() * 4.0];
                }
                1 => {
                    // Magenta orbiting light (opposite direction)
                    let angle = -time * 0.5 + std::f32::consts::PI;
                    light.position = [angle.cos() * 3.5, 1.5, angle.sin() * 3.5];
                }
                2 => {
                    // Gold pulsing light at center-above
                    let pulse = 1.5 + (time * 2.0).sin() * 0.8;
                    light.position = [0.0, 3.0 + (time * 0.5).sin() * 0.5, 0.0];
                    light.intensity = pulse;
                }
                _ => {}
            }
        }
    }

    /// Convert current settings into the GPU-uploadable `LightUniform`.
    pub fn to_light_uniform(&self, time: f32) -> LightUniform {
        let mut uniform = LightUniform::default();

        // Directional light
        if self.directional.enabled {
            uniform.dir_direction = [
                self.directional.direction[0],
                self.directional.direction[1],
                self.directional.direction[2],
                self.directional.intensity,
            ];
            uniform.dir_color = [
                self.directional.color[0],
                self.directional.color[1],
                self.directional.color[2],
                0.0,
            ];
        } else {
            uniform.dir_direction = [0.0, -1.0, 0.0, 0.0];
            uniform.dir_color = [0.0, 0.0, 0.0, 0.0];
        }

        // Point lights
        let mut count = 0u32;
        for light in &self.point_lights {
            if !light.enabled || count >= 4 {
                continue;
            }
            let pl = PointLight::new(light.position, light.color, light.intensity);
            let idx = count as usize;
            uniform.point_positions[idx] = [
                pl.position[0],
                pl.position[1],
                pl.position[2],
                pl.intensity,
            ];
            uniform.point_colors[idx] = [pl.color[0], pl.color[1], pl.color[2], 0.0];
            uniform.point_attenuation[idx] = [light.constant, light.linear, light.quadratic, 0.0];
            count += 1;
        }
        uniform.num_point_lights[0] = count as f32;
        uniform.num_point_lights[1] = time;

        uniform
    }
}
