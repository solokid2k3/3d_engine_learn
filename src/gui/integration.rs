use std::sync::Arc;

use winit::event::WindowEvent;
use winit::window::Window;

/// Manages the full egui lifecycle: context, winit state, and wgpu renderer.
pub struct EguiIntegration {
    pub ctx: egui::Context,
    state: egui_winit::State,
    renderer: egui_wgpu::Renderer,
}

impl EguiIntegration {
    /// Initialize egui with the given wgpu device and surface format.
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        window: &Arc<Window>,
    ) -> Self {
        let ctx = egui::Context::default();

        // Apply a dark, semi-transparent visual style
        let mut visuals = egui::Visuals::dark();
        visuals.window_shadow = egui::epaint::Shadow::NONE;
        visuals.panel_fill = egui::Color32::from_rgba_premultiplied(20, 20, 28, 230);
        visuals.widgets.noninteractive.bg_fill = egui::Color32::from_gray(30);
        visuals.widgets.inactive.bg_fill = egui::Color32::from_gray(40);
        visuals.widgets.hovered.bg_fill = egui::Color32::from_gray(55);
        visuals.widgets.active.bg_fill = egui::Color32::from_gray(70);
        ctx.set_visuals(visuals);

        // Set default font size a bit larger for readability
        let mut style = (*ctx.global_style()).clone();
        style.text_styles.insert(
            egui::TextStyle::Body,
            egui::FontId::new(14.0, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Small,
            egui::FontId::new(11.0, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Heading,
            egui::FontId::new(18.0, egui::FontFamily::Proportional),
        );
        ctx.set_global_style(style);

        let state = egui_winit::State::new(
            ctx.clone(),
            egui::ViewportId::ROOT,
            window.as_ref(),
            None, // native pixels per point
            None, // max texture side
            None,
        );

        let renderer = egui_wgpu::Renderer::new(
            device,
            surface_format,
            egui_wgpu::RendererOptions::default(),
        );

        Self {
            ctx,
            state,
            renderer,
        }
    }

    /// Forward a winit `WindowEvent` to egui. Returns `true` if egui consumed it
    /// (i.e. the event was over the GUI and should NOT be forwarded to the 3D viewport).
    pub fn handle_window_event(
        &mut self,
        window: &Window,
        event: &WindowEvent,
    ) -> bool {
        let response = self.state.on_window_event(window, event);
        response.consumed
    }

    /// Returns true if the mouse is currently over any egui area (panel, popup, etc.).
    pub fn wants_pointer_input(&self) -> bool {
        self.ctx.is_pointer_over_egui()
    }

    /// Returns true if egui wants keyboard input (e.g., a text field is focused).
    pub fn wants_keyboard_input(&self) -> bool {
        self.ctx.egui_wants_keyboard_input()
    }

    /// Start a new egui frame. Call this before any `egui::SidePanel`, `egui::Window`, etc.
    pub fn begin_frame(&mut self, window: &Window) {
        let raw_input = self.state.take_egui_input(window);
        self.ctx.begin_pass(raw_input);
    }

    /// End the egui frame, tessellate, and render as an overlay on top of the 3D scene.
    /// Returns a `CommandBuffer` that should be submitted to the queue after the 3D scene.
    pub fn end_frame_and_render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_view: &wgpu::TextureView,
        window: &Window,
    ) -> wgpu::CommandBuffer {
        let full_output = self.ctx.end_pass();

        // Handle platform output (clipboard, cursor, etc.)
        self.state
            .handle_platform_output(window, full_output.platform_output);

        let tris = self
            .ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        // Update textures
        for (id, delta) in &full_output.textures_delta.set {
            self.renderer.update_texture(device, queue, *id, delta);
        }

        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [
                window.inner_size().width,
                window.inner_size().height,
            ],
            pixels_per_point: full_output.pixels_per_point,
        };

        // Create a dedicated encoder for egui — avoids wgpu 29 lifetime issues
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("egui Encoder"),
        });

        // Upload mesh data
        self.renderer
            .update_buffers(device, queue, &mut encoder, &tris, &screen_descriptor);

        // Render egui overlay
        {
            let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load, // Preserve the 3D scene underneath
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });

            // forget_lifetime() drops the borrow on encoder, allowing wgpu 29 compatibility
            let mut render_pass = render_pass.forget_lifetime();

            self.renderer
                .render(&mut render_pass, &tris, &screen_descriptor);
        }

        // Free textures after render_pass is dropped
        for id in &full_output.textures_delta.free {
            self.renderer.free_texture(id);
        }

        encoder.finish()
    }
}
