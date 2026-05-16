use std::sync::Arc;
use std::time::Instant;

use glam::{Quat, Vec3};
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::window::{WindowAttributes, WindowId};

use crate::camera::orbit::OrbitController;
use crate::camera::{Camera, CameraUniform};
use crate::gpu::context::GpuContext;
use crate::input::InputState;
use crate::renderer::pass::RenderState;
use crate::scene::light::{LightUniform, PointLight};
use crate::scene::mesh::{create_cube, create_sphere};
use crate::scene::transform::Transform;
use crate::scene::Scene;

/// Engine state that is initialized after the window is created.
struct EngineState {
    gpu: GpuContext,
    render_state: RenderState,
    camera: Camera,
    camera_uniform: CameraUniform,
    orbit: OrbitController,
    input: InputState,
    scene: Scene,
    start_time: Instant,
    last_frame_time: Instant,
}

/// Top-level application handler for winit 0.30+.
pub struct App {
    engine: Option<EngineState>,
}

impl App {
    pub fn new() -> Self {
        Self { engine: None }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.engine.is_some() {
            return;
        }

        log::info!("Creating window and initializing GPU...");

        let window_attrs = WindowAttributes::default()
            .with_title("🔥 Rust 3D Render Engine")
            .with_inner_size(winit::dpi::LogicalSize::new(1280, 720));

        let window = Arc::new(
            event_loop
                .create_window(window_attrs)
                .expect("Failed to create window"),
        );

        let gpu = pollster::block_on(GpuContext::new(Arc::clone(&window)));
        let render_state = RenderState::new(&gpu);

        let mut camera = Camera::new(gpu.aspect_ratio());
        camera.eye = Vec3::new(0.0, 3.0, 8.0);
        camera.target = Vec3::new(0.0, 0.5, 0.0);

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_from_camera(&camera);

        let mut orbit = OrbitController::new();
        orbit.distance = 8.0;
        orbit.pitch = 20.0_f32.to_radians();

        let input = InputState::new();

        // =============================================
        // Build a beautiful demo scene
        // =============================================
        let mut scene = Scene::new();

        // --- Central hero cube (will rotate) ---
        let cube = create_cube(&gpu.device);
        let mut t = Transform::new();
        t.position = Vec3::new(0.0, 0.5, 0.0);
        t.scale = Vec3::splat(1.2);
        scene.add_mesh(cube, t, 0);

        // --- Large polished sphere ---
        let sphere = create_sphere(&gpu.device, 48, 48);
        let mut t = Transform::new();
        t.position = Vec3::new(-2.5, 0.6, -1.0);
        t.scale = Vec3::splat(1.2);
        scene.add_mesh(sphere, t, 0);

        // --- Small floating orbs in a ring (8 orbs) ---
        for i in 0..8 {
            let angle = std::f32::consts::TAU * i as f32 / 8.0;
            let radius = 3.5;
            let orb = create_sphere(&gpu.device, 16, 16);
            let mut t = Transform::new();
            t.position = Vec3::new(angle.cos() * radius, 1.0, angle.sin() * radius);
            t.scale = Vec3::splat(0.2);
            scene.add_mesh(orb, t, 0);
        }

        // --- Medium spheres (satellites) ---
        let sat1 = create_sphere(&gpu.device, 24, 24);
        let mut t = Transform::new();
        t.position = Vec3::new(2.5, 0.3, 1.5);
        t.scale = Vec3::splat(0.6);
        scene.add_mesh(sat1, t, 0);

        let sat2 = create_sphere(&gpu.device, 24, 24);
        let mut t = Transform::new();
        t.position = Vec3::new(-1.0, 0.3, 2.5);
        t.scale = Vec3::splat(0.45);
        scene.add_mesh(sat2, t, 0);

        // --- Ground plane ---
        let ground = create_cube(&gpu.device);
        let mut t = Transform::new();
        t.position = Vec3::new(0.0, -0.5, 0.0);
        t.scale = Vec3::new(20.0, 0.1, 20.0);
        scene.add_mesh(ground, t, 0);

        log::info!(
            "Scene built: {} objects",
            scene.meshes.len()
        );
        log::info!("Engine initialized successfully!");

        self.engine = Some(EngineState {
            gpu,
            render_state,
            camera,
            camera_uniform,
            orbit,
            input,
            scene,
            start_time: Instant::now(),
            last_frame_time: Instant::now(),
        });
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(engine) = self.engine.as_mut() else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => {
                log::info!("Window closed. Shutting down.");
                event_loop.exit();
            }

            WindowEvent::Resized(new_size) => {
                engine.gpu.resize(new_size);
                engine
                    .render_state
                    .resize(&engine.gpu.device, new_size.width, new_size.height);
                engine.camera.aspect = engine.gpu.aspect_ratio();
            }

            WindowEvent::KeyboardInput { event, .. } => {
                engine.input.process_key(event.physical_key, event.state);
            }

            WindowEvent::MouseInput { button, state, .. } => {
                engine.input.process_mouse_button(button, state);
            }

            WindowEvent::CursorMoved { position, .. } => {
                engine
                    .input
                    .process_mouse_move((position.x, position.y));
            }

            WindowEvent::MouseWheel { delta, .. } => {
                engine.input.process_scroll(delta);
            }

            WindowEvent::RedrawRequested => {
                let time = engine.start_time.elapsed().as_secs_f32();

                // =========================================
                // Animate objects
                // =========================================

                // Hero cube: gentle tumble rotation
                if let Some(cube) = engine.scene.meshes.get_mut(0) {
                    cube.transform.rotation =
                        Quat::from_euler(glam::EulerRot::YXZ, time * 0.5, time * 0.3, time * 0.1);
                    // Gentle levitation
                    cube.transform.position.y = 0.5 + (time * 0.8).sin() * 0.15;
                }

                // Large sphere: slow spin + breathing scale
                if let Some(sphere) = engine.scene.meshes.get_mut(1) {
                    sphere.transform.rotation = Quat::from_rotation_y(time * 0.2);
                    let breath = 1.2 + (time * 1.2).sin() * 0.05;
                    sphere.transform.scale = Vec3::splat(breath);
                }

                // Floating orbs: orbit + individual bob (indices 2..10)
                for i in 0..8u32 {
                    let idx = 2 + i as usize;
                    if let Some(orb) = engine.scene.meshes.get_mut(idx) {
                        let base_angle = std::f32::consts::TAU * i as f32 / 8.0;
                        let orbit_angle = base_angle + time * 0.4; // slow orbit
                        let radius = 3.5;
                        let bob = (time * 2.0 + i as f32 * 0.7).sin() * 0.3;

                        orb.transform.position = Vec3::new(
                            orbit_angle.cos() * radius,
                            1.0 + bob,
                            orbit_angle.sin() * radius,
                        );

                        // Pulse scale
                        let pulse = 0.2 + (time * 3.0 + i as f32 * 0.5).sin() * 0.05;
                        orb.transform.scale = Vec3::splat(pulse);
                    }
                }

                // Satellite spheres: figure-8 and orbits
                if let Some(sat1) = engine.scene.meshes.get_mut(10) {
                    let angle = time * 0.6;
                    sat1.transform.position = Vec3::new(
                        angle.cos() * 2.5,
                        0.3 + (time * 1.5).sin() * 0.2,
                        angle.sin() * 2.5,
                    );
                }
                if let Some(sat2) = engine.scene.meshes.get_mut(11) {
                    let angle = time * 0.8 + 2.0;
                    sat2.transform.position = Vec3::new(
                        (angle * 0.5).sin() * 2.0,
                        0.3 + (time * 2.0).cos() * 0.15,
                        angle.cos() * 2.0,
                    );
                }

                // =========================================
                // Animate 3 orbiting colored point lights
                // =========================================
                let mut light_uniform = LightUniform::default();

                // Warm sunlight from above-left
                light_uniform.dir_direction = [-0.3, -0.8, -0.5, 0.7];
                light_uniform.dir_color = [1.0, 0.95, 0.85, 0.0];

                // Light 1: Cyan orbiting light
                let l1_angle = time * 0.7;
                let l1 = PointLight::new(
                    [l1_angle.cos() * 4.0, 2.5, l1_angle.sin() * 4.0],
                    [0.3, 0.8, 1.0],
                    2.5,
                );
                light_uniform.set_point_light(0, &l1);

                // Light 2: Magenta orbiting light (opposite direction)
                let l2_angle = -time * 0.5 + std::f32::consts::PI;
                let l2 = PointLight::new(
                    [l2_angle.cos() * 3.5, 1.5, l2_angle.sin() * 3.5],
                    [1.0, 0.3, 0.7],
                    2.0,
                );
                light_uniform.set_point_light(1, &l2);

                // Light 3: Gold pulsing light at center-above
                let pulse_intensity = 1.5 + (time * 2.0).sin() * 0.8;
                let l3 = PointLight::new(
                    [0.0, 3.0 + (time * 0.5).sin() * 0.5, 0.0],
                    [1.0, 0.85, 0.4],
                    pulse_intensity,
                );
                light_uniform.set_point_light(2, &l3);

                // Store time in the uniform for shader use
                light_uniform.num_point_lights[1] = time;

                engine.scene.light_uniform = light_uniform;

                // =========================================
                // Camera + render
                // =========================================
                let now = Instant::now();
                let dt = now.duration_since(engine.last_frame_time).as_secs_f32();
                engine.last_frame_time = now;

                engine.orbit.update(&engine.input, &mut engine.camera, dt);
                engine
                    .camera_uniform
                    .update_from_camera(&engine.camera);

                engine.input.begin_frame();

                engine.render_state.render(
                    &engine.gpu,
                    &engine.camera_uniform,
                    &engine.scene,
                );

                engine.gpu.window.request_redraw();
            }

            _ => {}
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        _event: DeviceEvent,
    ) {
    }
}
