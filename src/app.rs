use std::sync::Arc;
use std::time::Instant;

use glam::Vec3;
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{WindowAttributes, WindowId};

use crate::camera::orbit::OrbitController;
use crate::camera::{Camera, CameraUniform};
use crate::gpu::context::GpuContext;
use crate::gui::integration::EguiIntegration;
use crate::gui::light_settings::LightSettings;
use crate::gui::model_panel::{draw_model_panel, ModelManager};
use crate::gui::panel::draw_light_panel;
use crate::input::InputState;
use crate::loader::gltf_loader::load_gltf;
use crate::renderer::pass::RenderState;
use crate::scene::ground::create_ground_plane;
use crate::scene::transform::Transform;
use crate::scene::{MeshInstance, MeshSource, Scene};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragMode {
    Translate,
    Rotate,
}

/// Engine state that is initialized after the window is created.
struct EngineState {
    gpu: GpuContext,
    render_state: RenderState,
    camera: Camera,
    camera_uniform: CameraUniform,
    orbit: OrbitController,
    input: InputState,
    scene: Scene,
    egui: EguiIntegration,
    light_settings: LightSettings,
    model_manager: ModelManager,
    start_time: Instant,
    last_frame_time: Instant,
    /// Tracks Tab key state to detect press (not hold).
    tab_was_pressed: bool,
    active_drag_mode: Option<DragMode>,
    particle_system: crate::scene::particle::ParticleSystem,
    emitter_selected: bool,
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

        // Initialize egui
        let egui = EguiIntegration::new(&gpu.device, gpu.config.format, &window);
        let light_settings = LightSettings::new();
        let model_manager = ModelManager::new();

        // =============================================
        // Build scene — ground plane with caro tiles
        // =============================================
        let mut scene = Scene::new();

        // Create the ground plane with caro tile texture
        let ground_mesh = create_ground_plane(
            &gpu.device,
            &gpu.queue,
            &render_state.material_bgl,
        );
        scene.meshes.push(MeshInstance {
            mesh: ground_mesh,
            transform: Transform::new(),
            material_index: 0,
            source: MeshSource::EngineDefault,
        });

        log::info!("Engine initialized — ground plane ready, waiting for model upload.");

        self.engine = Some(EngineState {
            gpu,
            render_state,
            camera,
            camera_uniform,
            orbit,
            input,
            scene,
            egui,
            light_settings,
            model_manager,
            start_time: Instant::now(),
            last_frame_time: Instant::now(),
            tab_was_pressed: false,
            active_drag_mode: None,
            particle_system: crate::scene::particle::ParticleSystem::fire(),
            emitter_selected: false,
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

        // Forward event to egui first
        let egui_consumed = engine
            .egui
            .handle_window_event(&engine.gpu.window, &event);

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
                // Tab toggle — detect press edge (not hold)
                if let PhysicalKey::Code(KeyCode::Tab) = event.physical_key {
                    if event.state.is_pressed() && !engine.tab_was_pressed {
                        engine.light_settings.panel_visible =
                            !engine.light_settings.panel_visible;
                        engine.tab_was_pressed = true;
                    }
                    if !event.state.is_pressed() {
                        engine.tab_was_pressed = false;
                    }
                }

                // Only forward to 3D input if egui doesn't want keyboard
                if !engine.egui.wants_keyboard_input() {
                    engine.input.process_key(event.physical_key, event.state);
                }
            }

            WindowEvent::MouseInput { button, state, .. } => {
                if !egui_consumed && !engine.egui.wants_pointer_input() {
                    engine.input.process_mouse_button(button, state);

                    if state.is_pressed() {
                        if button == winit::event::MouseButton::Left || button == winit::event::MouseButton::Right {
                            if engine.emitter_selected {
                                if button == winit::event::MouseButton::Left {
                                    engine.active_drag_mode = Some(DragMode::Translate);
                                } else {
                                    engine.active_drag_mode = Some(DragMode::Rotate);
                                }
                            } else {
                                let (width, height) = (engine.gpu.size.width as f32, engine.gpu.size.height as f32);
                                let view_proj = engine.camera.build_view_projection_matrix();
                                let inv_view_proj = view_proj.inverse();

                                let hit_gid = crate::scene::picking::pick_object(
                                    engine.input.mouse_position.x,
                                    engine.input.mouse_position.y,
                                    width,
                                    height,
                                    inv_view_proj,
                                    &engine.scene,
                                );

                                if let Some(gid) = hit_gid {
                                    engine.model_manager.selected_group_id = Some(gid);
                                    if button == winit::event::MouseButton::Left {
                                        engine.active_drag_mode = Some(DragMode::Translate);
                                    } else {
                                        engine.active_drag_mode = Some(DragMode::Rotate);
                                    }
                                } else {
                                    if button == winit::event::MouseButton::Left {
                                        engine.model_manager.selected_group_id = None;
                                    }
                                    engine.active_drag_mode = None;
                                }
                            }
                        }
                    } else {
                        if button == winit::event::MouseButton::Left && engine.active_drag_mode == Some(DragMode::Translate) {
                            engine.active_drag_mode = None;
                        }
                        if button == winit::event::MouseButton::Right && engine.active_drag_mode == Some(DragMode::Rotate) {
                            engine.active_drag_mode = None;
                        }
                    }
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                if !engine.egui.wants_pointer_input() {
                    engine
                        .input
                        .process_mouse_move((position.x, position.y));

                    // If dragging the emitter, handle the movement
                    if engine.emitter_selected {
                        if let Some(drag_mode) = engine.active_drag_mode {
                            let forward = (engine.camera.target - engine.camera.eye).normalize();
                            let right = forward.cross(engine.camera.up).normalize();
                            let up = right.cross(forward).normalize();
                            let distance = (engine.camera.target - engine.camera.eye).length();

                            match drag_mode {
                                DragMode::Translate => {
                                    let speed = 0.005 * distance.max(0.5);
                                    let dx = engine.input.mouse_delta.x;
                                    let dy = engine.input.mouse_delta.y;

                                    let world_delta = right * (dx * speed) + up * (-dy * speed);
                                    engine.particle_system.position.x += world_delta.x;
                                    engine.particle_system.position.y += world_delta.y;
                                    engine.particle_system.position.z += world_delta.z;
                                }
                                DragMode::Rotate => {}
                            }
                        }
                    } else if let Some(selected_gid) = engine.model_manager.selected_group_id {
                        if let Some(drag_mode) = engine.active_drag_mode {
                            // Calculate movement delta using camera vectors
                            let forward = (engine.camera.target - engine.camera.eye).normalize();
                            let right = forward.cross(engine.camera.up).normalize();
                            let up = right.cross(forward).normalize();
                            let distance = (engine.camera.target - engine.camera.eye).length();

                            if let Some(model) = engine.model_manager.models.iter_mut().find(|m| m.group_id == selected_gid) {
                                match drag_mode {
                                    DragMode::Translate => {
                                        // Speed scales with distance for natural feel
                                        let speed = 0.005 * distance.max(0.5);
                                        let dx = engine.input.mouse_delta.x;
                                        let dy = engine.input.mouse_delta.y;

                                        let world_delta = right * (dx * speed) + up * (-dy * speed);
                                        model.position[0] += world_delta.x;
                                        model.position[1] += world_delta.y;
                                        model.position[2] += world_delta.z;
                                    }
                                    DragMode::Rotate => {
                                        let speed = 0.3; // degrees per pixel
                                        let dx = engine.input.mouse_delta.x;
                                        let dy = engine.input.mouse_delta.y;

                                        // Horizontal drag -> rotate around Y (yaw)
                                        model.rotation_deg[1] = (model.rotation_deg[1] + dx * speed) % 360.0;
                                        // Vertical drag -> rotate around X (pitch)
                                        model.rotation_deg[0] = (model.rotation_deg[0] + dy * speed) % 360.0;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                if !engine.egui.wants_pointer_input() {
                    engine.input.process_scroll(delta);
                }
            }

            WindowEvent::RedrawRequested => {
                let time = engine.start_time.elapsed().as_secs_f32();

                // =========================================
                // Sync user-model transforms from the panel
                // =========================================
                for model_entry in &engine.model_manager.models {
                    let gid = model_entry.group_id;
                    let pos = Vec3::from(model_entry.position);
                    let rot = glam::Quat::from_euler(
                        glam::EulerRot::YXZ,
                        model_entry.rotation_deg[1].to_radians(),
                        model_entry.rotation_deg[0].to_radians(),
                        model_entry.rotation_deg[2].to_radians(),
                    );
                    let scl = Vec3::from(model_entry.scale);

                    for inst in engine.scene.meshes.iter_mut() {
                        if matches!(&inst.source, MeshSource::UserModel { group_id, .. } if *group_id == gid)
                        {
                            inst.transform.position = pos;
                            inst.transform.rotation = rot;
                            inst.transform.scale = scl;
                        }
                    }
                }

                // =========================================
                // Animate lights + convert to GPU uniform
                // =========================================
                engine.light_settings.animate(time);
                engine.scene.light_uniform = engine.light_settings.to_light_uniform(time);

                // =========================================
                // Camera + render
                // =========================================
                let now = Instant::now();
                let dt = now.duration_since(engine.last_frame_time).as_secs_f32();
                engine.last_frame_time = now;

                engine.particle_system.update(dt);

                if engine.active_drag_mode.is_none() {
                    engine.orbit.update(&engine.input, &mut engine.camera, dt);
                }
                engine
                    .camera_uniform
                    .update_from_camera(&engine.camera);

                engine.input.begin_frame();

                // --- Acquire surface ---
                let surface_texture = match engine.gpu.surface.get_current_texture() {
                    wgpu::CurrentSurfaceTexture::Success(st) => st,
                    wgpu::CurrentSurfaceTexture::Suboptimal(st) => st,
                    wgpu::CurrentSurfaceTexture::Timeout
                    | wgpu::CurrentSurfaceTexture::Occluded => {
                        engine.gpu.window.request_redraw();
                        return;
                    }
                    wgpu::CurrentSurfaceTexture::Outdated
                    | wgpu::CurrentSurfaceTexture::Lost => {
                        engine
                            .gpu
                            .surface
                            .configure(&engine.gpu.device, &engine.gpu.config);
                        engine.gpu.window.request_redraw();
                        return;
                    }
                    _ => {
                        engine.gpu.window.request_redraw();
                        return;
                    }
                };
                let view = surface_texture
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                let mut encoder = engine
                    .gpu
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("Render Encoder"),
                    });

                // --- Sky pass (renders first, clears framebuffer) ---
                engine.render_state.render_sky(
                    &engine.gpu,
                    &engine.camera_uniform,
                    &mut encoder,
                    &view,
                    engine.light_settings.directional.direction,
                    engine.light_settings.directional.color,
                    engine.light_settings.directional.intensity,
                );

                // --- 3D scene pass ---
                engine.render_state.render_scene(
                    &engine.gpu,
                    &engine.camera_uniform,
                    &engine.scene,
                    &mut encoder,
                    &view,
                    engine.model_manager.selected_group_id,
                    &engine.particle_system.particles,
                    engine.particle_system.settings.blend_mode,
                    engine.particle_system.settings.render_type,
                );

                // --- egui overlay pass ---
                engine.egui.begin_frame(&engine.gpu.window);
                draw_light_panel(&engine.egui.ctx, &mut engine.light_settings);
                draw_model_panel(
                    &engine.egui.ctx,
                    &mut engine.model_manager,
                    engine.light_settings.panel_visible,
                );
                crate::gui::vfx_panel::draw_vfx_panel(
                    &engine.egui.ctx,
                    &mut engine.particle_system,
                    &mut engine.emitter_selected,
                    engine.light_settings.panel_visible,
                );
                let egui_cmd = engine.egui.end_frame_and_render(
                    &engine.gpu.device,
                    &engine.gpu.queue,
                    &view,
                    &engine.gpu.window,
                );

                // --- Submit & present ---
                engine.gpu.queue.submit([encoder.finish(), egui_cmd]);
                surface_texture.present();

                // =========================================
                // Process pending model loads / removals
                // =========================================
                if let Some(path) = engine.model_manager.pending_load.take() {
                    log::info!("Loading model from {:?}", path);
                    match load_gltf(
                        &path,
                        &engine.gpu.device,
                        &engine.gpu.queue,
                        &engine.render_state.material_bgl,
                    ) {
                        Ok(meshes) => {
                            let mesh_count = meshes.len();
                            let filename = path
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_else(|| "unknown".to_string());

                            let transform = Transform::new();
                            let group_id = engine.scene.add_user_model(
                                meshes,
                                transform,
                                &filename,
                            );
                            engine
                                .model_manager
                                .register_model(group_id, &filename, mesh_count);

                            log::info!(
                                "Loaded '{}' — {} mesh(es), group_id={}",
                                filename,
                                mesh_count,
                                group_id
                            );
                        }
                        Err(e) => {
                            log::error!("Failed to load model {:?}: {}", path, e);
                        }
                    }
                    engine.model_manager.is_loading = false;
                }

                // Process pending removals
                for gid in engine.model_manager.pending_remove.drain(..) {
                    engine.scene.remove_user_model(gid);
                    log::info!("Removed user model group_id={}", gid);
                }

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
