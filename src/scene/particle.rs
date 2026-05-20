use glam::{Vec3, Vec4};

#[derive(Debug, Clone)]
pub struct Particle {
    pub position: Vec3,
    pub velocity: Vec3,
    pub color: Vec4,
    pub size: f32,
    pub age: f32,
    pub lifetime: f32,
    pub angle: f32,
    pub angular_velocity: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParticleEmitterShape {
    Point,
    Sphere { radius: f32 },
    Box { extents: Vec3 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParticleBlendMode {
    Additive,
    Alpha,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParticleRenderType {
    GlowCircle = 0,
    Spark = 1,
    Flame = 2,
    Smoke = 3,
    Star = 4,
}

#[derive(Debug, Clone)]
pub struct ParticleEmitterSettings {
    pub max_particles: usize,
    pub spawn_rate: f32, // particles per second
    pub shape: ParticleEmitterShape,
    
    pub min_lifetime: f32,
    pub max_lifetime: f32,
    
    pub min_speed: f32,
    pub max_speed: f32,
    
    pub start_color: Vec4,
    pub end_color: Vec4,
    
    pub start_size: f32,
    pub end_size: f32,
    
    pub gravity: Vec3,
    
    pub blend_mode: ParticleBlendMode,
    pub render_type: ParticleRenderType,
}

pub struct ParticleSystem {
    pub particles: Vec<Particle>,
    pub settings: ParticleEmitterSettings,
    pub position: Vec3,
    pub spawn_accumulator: f32,
    pub is_playing: bool,
    pub is_looping: bool,
    pub elapsed_time: f32,
    rng: Lcg,
}

struct Lcg {
    state: u64,
}

impl Lcg {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.state >> 32) as u32
    }

    fn next_f32(&mut self) -> f32 {
        (self.next_u32() as f32) / (u32::MAX as f32)
    }

    fn range(&mut self, min: f32, max: f32) -> f32 {
        min + self.next_f32() * (max - min)
    }
}

impl ParticleSystem {
    pub fn new(settings: ParticleEmitterSettings) -> Self {
        Self {
            particles: Vec::new(),
            settings,
            position: Vec3::new(0.0, 1.0, 0.0),
            spawn_accumulator: 0.0,
            is_playing: true,
            is_looping: true,
            elapsed_time: 0.0,
            rng: Lcg::new(1337),
        }
    }

    pub fn update(&mut self, dt: f32) {
        if !self.is_playing {
            return;
        }

        self.elapsed_time += dt;

        // 1. Update existing particles
        for p in &mut self.particles {
            p.age += dt;
            // Apply gravity
            p.velocity += self.settings.gravity * dt;
            p.position += p.velocity * dt;
            p.angle += p.angular_velocity * dt;

            // Interpolate color and size
            let t = (p.age / p.lifetime).clamp(0.0, 1.0);
            p.color = self.settings.start_color.lerp(self.settings.end_color, t);
            p.size = self.settings.start_size + (self.settings.end_size - self.settings.start_size) * t;
        }

        // Keep only alive particles
        self.particles.retain(|p| p.age < p.lifetime);

        // 2. Spawn new particles
        if self.particles.len() < self.settings.max_particles {
            self.spawn_accumulator += dt;
            let spawn_interval = 1.0 / self.settings.spawn_rate.max(0.1);
            while self.spawn_accumulator >= spawn_interval {
                self.spawn_accumulator -= spawn_interval;
                if self.particles.len() < self.settings.max_particles {
                    let new_particle = self.create_particle();
                    self.particles.push(new_particle);
                } else {
                    break;
                }
            }
        }
    }

    fn create_particle(&mut self) -> Particle {
        // Spawn offset based on shape
        let offset = match self.settings.shape {
            ParticleEmitterShape::Point => Vec3::ZERO,
            ParticleEmitterShape::Sphere { radius } => {
                // Spherical random coordinate
                let theta = self.rng.range(0.0, std::f32::consts::TAU);
                let phi = self.rng.range(0.0, std::f32::consts::PI);
                let r = self.rng.range(0.0, radius);
                Vec3::new(
                    r * phi.sin() * theta.cos(),
                    r * phi.sin() * theta.sin(),
                    r * phi.cos(),
                )
            }
            ParticleEmitterShape::Box { extents } => Vec3::new(
                self.rng.range(-extents.x, extents.x),
                self.rng.range(-extents.y, extents.y),
                self.rng.range(-extents.z, extents.z),
            ),
        };

        let start_pos = self.position + offset;

        // Velocity along a random direction (cone/sphere)
        let theta = self.rng.range(0.0, std::f32::consts::TAU);
        let phi = self.rng.range(0.0, std::f32::consts::PI);
        let speed = self.rng.range(self.settings.min_speed, self.settings.max_speed);
        
        // Emitter shape-specific velocity tweaks
        let velocity = match self.settings.shape {
            // Emitters like fire push upwards mostly
            _ if self.settings.render_type == ParticleRenderType::Flame => {
                // Cone pointing mostly up
                let spread = 0.3; // spread angle
                let vx = self.rng.range(-spread, spread);
                let vz = self.rng.range(-spread, spread);
                Vec3::new(vx, 1.0, vz).normalize() * speed
            }
            _ => {
                // Sphere random direction
                Vec3::new(
                    phi.sin() * theta.cos(),
                    phi.sin() * theta.sin(),
                    phi.cos(),
                ).normalize() * speed
            }
        };

        let lifetime = self.rng.range(self.settings.min_lifetime, self.settings.max_lifetime);
        let angular_velocity = self.rng.range(-2.0, 2.0);

        Particle {
            position: start_pos,
            velocity,
            color: self.settings.start_color,
            size: self.settings.start_size,
            age: 0.0,
            lifetime,
            angle: self.rng.range(0.0, std::f32::consts::TAU),
            angular_velocity,
        }
    }

    // Presets
    pub fn fire() -> Self {
        Self::new(ParticleEmitterSettings {
            max_particles: 400,
            spawn_rate: 120.0,
            shape: ParticleEmitterShape::Sphere { radius: 0.2 },
            min_lifetime: 0.6,
            max_lifetime: 1.4,
            min_speed: 1.5,
            max_speed: 3.0,
            start_color: Vec4::new(1.0, 0.6, 0.1, 1.0), // fire orange
            end_color: Vec4::new(0.3, 0.0, 0.0, 0.0), // deep fading red
            start_size: 0.45,
            end_size: 0.05,
            gravity: Vec3::new(0.0, 2.0, 0.0), // rising up
            blend_mode: ParticleBlendMode::Additive,
            render_type: ParticleRenderType::Flame,
        })
    }

    pub fn magic() -> Self {
        Self::new(ParticleEmitterSettings {
            max_particles: 250,
            spawn_rate: 60.0,
            shape: ParticleEmitterShape::Point,
            min_lifetime: 1.2,
            max_lifetime: 2.2,
            min_speed: 2.0,
            max_speed: 5.0,
            start_color: Vec4::new(0.2, 0.7, 1.0, 1.0), // cyan sparkle
            end_color: Vec4::new(0.8, 0.1, 1.0, 0.0), // fading magenta/purple
            start_size: 0.2,
            end_size: 0.0,
            gravity: Vec3::new(0.0, -1.5, 0.0), // falling slightly
            blend_mode: ParticleBlendMode::Additive,
            render_type: ParticleRenderType::Spark,
        })
    }

    pub fn smoke() -> Self {
        Self::new(ParticleEmitterSettings {
            max_particles: 150,
            spawn_rate: 20.0,
            shape: ParticleEmitterShape::Sphere { radius: 0.3 },
            min_lifetime: 2.0,
            max_lifetime: 4.0,
            min_speed: 0.3,
            max_speed: 0.8,
            start_color: Vec4::new(0.5, 0.5, 0.5, 0.4), // grey smoke
            end_color: Vec4::new(0.1, 0.1, 0.1, 0.0), // fading dark smoke
            start_size: 0.3,
            end_size: 1.2, // expanding puff
            gravity: Vec3::new(0.0, 0.6, 0.0), // slow rise
            blend_mode: ParticleBlendMode::Alpha,
            render_type: ParticleRenderType::Smoke,
        })
    }

    pub fn explosion() -> Self {
        Self::new(ParticleEmitterSettings {
            max_particles: 500,
            spawn_rate: 250.0,
            shape: ParticleEmitterShape::Sphere { radius: 0.05 },
            min_lifetime: 0.5,
            max_lifetime: 0.9,
            min_speed: 3.5,
            max_speed: 8.0,
            start_color: Vec4::new(1.0, 0.85, 0.4, 1.0), // bright white-orange burst
            end_color: Vec4::new(0.2, 0.0, 0.0, 0.0),
            start_size: 0.4,
            end_size: 0.1,
            gravity: Vec3::new(0.0, -2.0, 0.0), // gravity drops them
            blend_mode: ParticleBlendMode::Additive,
            render_type: ParticleRenderType::GlowCircle,
        })
    }

    pub fn rain() -> Self {
        let mut sys = Self::new(ParticleEmitterSettings {
            max_particles: 600,
            spawn_rate: 200.0,
            shape: ParticleEmitterShape::Box { extents: Vec3::new(4.0, 0.1, 4.0) },
            min_lifetime: 1.0,
            max_lifetime: 1.5,
            min_speed: 0.0,
            max_speed: 0.1,
            start_color: Vec4::new(0.6, 0.8, 1.0, 0.6),
            end_color: Vec4::new(0.6, 0.8, 1.0, 0.2),
            start_size: 0.06,
            end_size: 0.06,
            gravity: Vec3::new(0.0, -12.0, 0.0), // heavy downward gravity
            blend_mode: ParticleBlendMode::Alpha,
            render_type: ParticleRenderType::Spark,
        });
        sys.position = Vec3::new(0.0, 10.0, 0.0); // start rain from 10 units high
        sys
    }
}
