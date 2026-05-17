use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::camera::CameraUniform;

/// GPU-uploadable sky uniforms.
#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct SkyUniforms {
    pub inv_view_proj: [[f32; 4]; 4],
    pub sun_direction: [f32; 4], // xyz = direction TO sun (normalized), w = unused
    pub sun_color: [f32; 4],     // xyz = color, w = intensity
}

/// Manages the sky background render pass.
pub struct SkyPass {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl SkyPass {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        // --- Shader ---
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Sky Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/sky.wgsl").into()),
        });

        // --- Bind group layout ---
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Sky BGL"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        // --- Uniform buffer ---
        let uniforms = SkyUniforms {
            inv_view_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
            sun_direction: [0.3, 0.8, 0.5, 0.0],
            sun_color: [1.0, 0.95, 0.85, 1.0],
        };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sky Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // --- Bind group ---
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Sky Bind Group"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // --- Pipeline layout ---
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Sky Pipeline Layout"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });

        // --- Pipeline (no depth write, no vertex buffers) ---
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Sky Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[], // fullscreen triangle generated from vertex_index
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None, // Sky renders behind everything — no depth
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });

        Self {
            pipeline,
            uniform_buffer,
            bind_group,
        }
    }

    /// Render the sky as the first pass (clears the framebuffer with sky colors).
    pub fn render(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        camera_uniform: &CameraUniform,
        sun_direction: [f32; 3],
        sun_color: [f32; 3],
        sun_intensity: f32,
    ) {
        // Normalize the direction from the light (which points FROM sky) to get
        // direction TO the sun
        let len = (sun_direction[0].powi(2)
            + sun_direction[1].powi(2)
            + sun_direction[2].powi(2))
        .sqrt()
        .max(0.001);
        let to_sun = [
            -sun_direction[0] / len,
            -sun_direction[1] / len,
            -sun_direction[2] / len,
        ];

        let uniforms = SkyUniforms {
            inv_view_proj: camera_uniform.inv_view_proj,
            sun_direction: [to_sun[0], to_sun[1], to_sun[2], 0.0],
            sun_color: [sun_color[0], sun_color[1], sun_color[2], sun_intensity],
        };

        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Sky Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.05,
                            g: 0.05,
                            b: 0.08,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });

            rpass.set_pipeline(&self.pipeline);
            rpass.set_bind_group(0, &self.bind_group, &[]);
            rpass.draw(0..3, 0..1); // Fullscreen triangle (3 vertices)
        }
    }
}
