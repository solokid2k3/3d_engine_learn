use wgpu::util::DeviceExt;

use crate::camera::CameraUniform;
use crate::gpu::context::GpuContext;
use crate::gpu::texture::Texture;
use crate::renderer::pipeline::create_phong_pipeline;
use crate::renderer::sky::SkyPass;
use crate::scene::light::LightUniform;
use crate::scene::material::MaterialUniform;
use crate::scene::transform::TransformUniform;
use crate::scene::Scene;

const GRID_SHADER_SOURCE: &str = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    view_pos: vec4<f32>,
    camera_right: vec4<f32>,
    camera_up: vec4<f32>,
};
@group(0) @binding(0) var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(model.position, 1.0);
    out.color = model.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
"#;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LineVertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

impl LineVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<LineVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ParticleVertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
}

const PARTICLE_VERTICES: &[ParticleVertex] = &[
    ParticleVertex { position: [-0.5, -0.5], uv: [0.0, 1.0] },
    ParticleVertex { position: [0.5, -0.5], uv: [1.0, 1.0] },
    ParticleVertex { position: [0.5, 0.5], uv: [1.0, 0.0] },
    ParticleVertex { position: [-0.5, 0.5], uv: [0.0, 0.0] },
];

const PARTICLE_INDICES: &[u16] = &[
    0, 1, 2,
    0, 2, 3,
];

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ParticleInstanceRaw {
    pub center_pos: [f32; 3],
    pub size: f32,
    pub color: [f32; 4],
    pub angle: f32,
    pub render_type: f32,
}

impl ParticleInstanceRaw {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ParticleInstanceRaw>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 12,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32,
                },
                wgpu::VertexAttribute {
                    offset: 16,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 32,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32,
                },
                wgpu::VertexAttribute {
                    offset: 36,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32,
                },
            ],
        }
    }
}

fn generate_grid_vertices() -> Vec<LineVertex> {
    let mut vertices = Vec::new();
    let grid_size = 20;
    let grid_color = [0.35, 0.35, 0.35];
    let major_color = [0.55, 0.55, 0.55];

    for i in -grid_size..=grid_size {
        let pos = i as f32;
        let is_major = i % 5 == 0;
        let color = if is_major { major_color } else { grid_color };
        
        if i != 0 {
            vertices.push(LineVertex { position: [pos, 0.0, -grid_size as f32], color });
            vertices.push(LineVertex { position: [pos, 0.0, grid_size as f32], color });
            
            vertices.push(LineVertex { position: [-grid_size as f32, 0.0, pos], color });
            vertices.push(LineVertex { position: [grid_size as f32, 0.0, pos], color });
        }
    }

    // X Axis - Red
    vertices.push(LineVertex { position: [-grid_size as f32, 0.01, 0.0], color: [0.9, 0.1, 0.1] });
    vertices.push(LineVertex { position: [grid_size as f32, 0.01, 0.0], color: [0.9, 0.1, 0.1] });

    // Z Axis - Blue
    vertices.push(LineVertex { position: [0.0, 0.01, -grid_size as f32], color: [0.1, 0.1, 0.9] });
    vertices.push(LineVertex { position: [0.0, 0.01, grid_size as f32], color: [0.1, 0.1, 0.9] });

    // Y Axis - Green
    vertices.push(LineVertex { position: [0.0, 0.0, 0.0], color: [0.1, 0.9, 0.1] });
    vertices.push(LineVertex { position: [0.0, 5.0, 0.0], color: [0.1, 0.9, 0.1] });

    vertices
}

/// Manages all GPU resources needed for a frame: pipelines, bind groups, uniforms.
pub struct RenderState {
    pub pipeline: wgpu::RenderPipeline,
    pub outline_pipeline: wgpu::RenderPipeline,
    pub depth_texture: Texture,

    pub grid_pipeline: wgpu::RenderPipeline,
    pub grid_buffer: wgpu::Buffer,
    pub grid_vertex_count: u32,

    pub particle_additive_pipeline: wgpu::RenderPipeline,
    pub particle_alpha_pipeline: wgpu::RenderPipeline,
    pub particle_quad_vertex_buffer: wgpu::Buffer,
    pub particle_quad_index_buffer: wgpu::Buffer,

    // Bind group layouts
    pub camera_bgl: wgpu::BindGroupLayout,
    pub transform_bgl: wgpu::BindGroupLayout,
    pub light_bgl: wgpu::BindGroupLayout,
    pub material_bgl: wgpu::BindGroupLayout,

    // Uniform buffers
    pub camera_buffer: wgpu::Buffer,
    pub camera_bind_group: wgpu::BindGroup,

    pub transform_buffer: wgpu::Buffer,
    pub transform_bind_group: wgpu::BindGroup,
    pub transform_buffers: std::cell::RefCell<Vec<wgpu::Buffer>>,
    pub transform_bind_groups: std::cell::RefCell<Vec<wgpu::BindGroup>>,

    pub light_buffer: wgpu::Buffer,
    pub light_bind_group: wgpu::BindGroup,

    pub material_buffer: wgpu::Buffer,
    pub material_bind_group: wgpu::BindGroup,

    pub default_texture: Texture,

    pub sky_pass: SkyPass,
}

impl RenderState {
    pub fn new(gpu: &GpuContext) -> Self {
        let device = &gpu.device;
        let queue = &gpu.queue;

        // --- Bind group layouts ---
        let camera_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Camera BGL"),
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

        let transform_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Transform BGL"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let light_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Light BGL"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let material_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Material BGL"),
            entries: &[
                // Material uniform
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Diffuse texture
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                // Sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        // --- Uniform buffers ---
        let camera_uniform = CameraUniform::new();
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let transform_uniform = TransformUniform {
            model: glam::Mat4::IDENTITY.to_cols_array_2d(),
            normal: glam::Mat4::IDENTITY.to_cols_array_2d(),
        };
        let transform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Transform Buffer"),
            contents: bytemuck::cast_slice(&[transform_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let light_uniform = LightUniform::default();
        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light Buffer"),
            contents: bytemuck::cast_slice(&[light_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let material_uniform = MaterialUniform::from_material(
            &crate::scene::material::Material::default_material(),
        );
        let material_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Material Buffer"),
            contents: bytemuck::cast_slice(&[material_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Default white texture
        let default_texture = Texture::create_default_white(device, queue);

        // --- Bind groups ---
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let transform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Transform Bind Group"),
            layout: &transform_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: transform_buffer.as_entire_binding(),
            }],
        });

        let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Light Bind Group"),
            layout: &light_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: light_buffer.as_entire_binding(),
            }],
        });

        let material_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Material Bind Group"),
            layout: &material_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: material_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&default_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&default_texture.sampler),
                },
            ],
        });

        // --- Pipeline ---
        let pipeline = create_phong_pipeline(
            device,
            gpu.config.format,
            &camera_bgl,
            &transform_bgl,
            &light_bgl,
            &material_bgl,
        );

        // --- Outline Pipeline ---
        let outline_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Outline Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/outline.wgsl").into()),
        });

        let outline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Outline Pipeline Layout"),
            bind_group_layouts: &[
                Some(&camera_bgl),
                Some(&transform_bgl),
            ],
            immediate_size: 0,
        });

        let outline_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Outline Pipeline"),
            layout: Some(&outline_layout),
            vertex: wgpu::VertexState {
                module: &outline_shader,
                entry_point: Some("vs_main"),
                buffers: &[crate::renderer::vertex::Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &outline_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: gpu.config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Front), // Cull front faces for inverted hull outline
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: Texture::DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });

        // --- Grid Pipeline & Buffer ---
        let grid_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Grid Shader"),
            source: wgpu::ShaderSource::Wgsl(GRID_SHADER_SOURCE.into()),
        });

        let grid_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Grid Pipeline Layout"),
            bind_group_layouts: &[Some(&camera_bgl)],
            immediate_size: 0,
        });

        let grid_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Grid Pipeline"),
            layout: Some(&grid_layout),
            vertex: wgpu::VertexState {
                module: &grid_shader,
                entry_point: Some("vs_main"),
                buffers: &[LineVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &grid_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: gpu.config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: Texture::DEPTH_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let grid_verts = generate_grid_vertices();
        let grid_vertex_count = grid_verts.len() as u32;
        let grid_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Grid Vertex Buffer"),
            contents: bytemuck::cast_slice(&grid_verts),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // --- Particle Pipelines & Buffers ---
        let particle_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Particle Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/particle.wgsl").into()),
        });

        let particle_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Particle Pipeline Layout"),
            bind_group_layouts: &[Some(&camera_bgl)],
            immediate_size: 0,
        });

        let particle_additive_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Particle Additive Pipeline"),
            layout: Some(&particle_layout),
            vertex: wgpu::VertexState {
                module: &particle_shader,
                entry_point: Some("vs_main"),
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<ParticleVertex>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[
                            wgpu::VertexAttribute {
                                offset: 0,
                                shader_location: 0,
                                format: wgpu::VertexFormat::Float32x2,
                            },
                            wgpu::VertexAttribute {
                                offset: 8,
                                shader_location: 1,
                                format: wgpu::VertexFormat::Float32x2,
                            },
                        ],
                    },
                    ParticleInstanceRaw::desc(),
                ],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &particle_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: gpu.config.format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: Texture::DEPTH_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let particle_alpha_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Particle Alpha Pipeline"),
            layout: Some(&particle_layout),
            vertex: wgpu::VertexState {
                module: &particle_shader,
                entry_point: Some("vs_main"),
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<ParticleVertex>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[
                            wgpu::VertexAttribute {
                                offset: 0,
                                shader_location: 0,
                                format: wgpu::VertexFormat::Float32x2,
                            },
                            wgpu::VertexAttribute {
                                offset: 8,
                                shader_location: 1,
                                format: wgpu::VertexFormat::Float32x2,
                            },
                        ],
                    },
                    ParticleInstanceRaw::desc(),
                ],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &particle_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: gpu.config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: Texture::DEPTH_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let particle_quad_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Particle Quad Vertex Buffer"),
            contents: bytemuck::cast_slice(PARTICLE_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let particle_quad_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Particle Quad Index Buffer"),
            contents: bytemuck::cast_slice(PARTICLE_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        // --- Depth texture ---
        let depth_texture =
            Texture::create_depth_texture(device, gpu.size.width, gpu.size.height, "Depth");

        // --- Sky pass ---
        let sky_pass = SkyPass::new(device, gpu.config.format);

        Self {
            pipeline,
            outline_pipeline,
            depth_texture,
            grid_pipeline,
            grid_buffer,
            grid_vertex_count,
            particle_additive_pipeline,
            particle_alpha_pipeline,
            particle_quad_vertex_buffer,
            particle_quad_index_buffer,
            camera_bgl,
            transform_bgl,
            light_bgl,
            material_bgl,
            camera_buffer,
            camera_bind_group,
            transform_buffer,
            transform_bind_group,
            transform_buffers: std::cell::RefCell::new(Vec::new()),
            transform_bind_groups: std::cell::RefCell::new(Vec::new()),
            light_buffer,
            light_bind_group,
            material_buffer,
            material_bind_group,
            default_texture,
            sky_pass,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.depth_texture = Texture::create_depth_texture(device, width, height, "Depth");
    }

    /// Render the sky background pass. Must be called before `render_scene`.
    pub fn render_sky(
        &self,
        gpu: &GpuContext,
        camera_uniform: &CameraUniform,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        sun_direction: [f32; 3],
        sun_color: [f32; 3],
        sun_intensity: f32,
    ) {
        self.sky_pass.render(
            &gpu.queue,
            encoder,
            view,
            camera_uniform,
            sun_direction,
            sun_color,
            sun_intensity,
        );
    }

    /// Render the 3D scene into the given encoder. Returns the surface texture and view
    /// so the caller can add additional overlay passes (e.g. egui) before submitting.
    pub fn render_scene(
        &self,
        gpu: &GpuContext,
        camera_uniform: &CameraUniform,
        scene: &Scene,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        selected_group_id: Option<u32>,
        particles: &[crate::scene::particle::Particle],
        particle_blend_mode: crate::scene::particle::ParticleBlendMode,
        particle_render_type: crate::scene::particle::ParticleRenderType,
    ) {
        // Ensure we have enough transform buffers and bind groups
        {
            let mut buffers = self.transform_buffers.borrow_mut();
            let mut bind_groups = self.transform_bind_groups.borrow_mut();
            while buffers.len() < scene.meshes.len() {
                let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(&format!("Transform Buffer {}", buffers.len())),
                    size: std::mem::size_of::<TransformUniform>() as u64,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some(&format!("Transform Bind Group {}", bind_groups.len())),
                    layout: &self.transform_bgl,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buffer.as_entire_binding(),
                    }],
                });
                buffers.push(buffer);
                bind_groups.push(bind_group);
            }
        }

        let buffers = self.transform_buffers.borrow();
        let bind_groups = self.transform_bind_groups.borrow();

        // Update camera uniform
        gpu.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[*camera_uniform]),
        );

        // Update light uniform
        gpu.queue.write_buffer(
            &self.light_buffer,
            0,
            bytemuck::cast_slice(&[scene.light_uniform]),
        );

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Main Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            // Draw grid
            render_pass.set_pipeline(&self.grid_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.grid_buffer.slice(..));
            render_pass.draw(0..self.grid_vertex_count, 0..1);

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(2, &self.light_bind_group, &[]);
            render_pass.set_bind_group(3, &self.material_bind_group, &[]);

            // Draw each mesh
            for (idx, instance) in scene.meshes.iter().enumerate() {
                let transform_uniform = TransformUniform::from_transform(&instance.transform);
                gpu.queue.write_buffer(
                    &buffers[idx],
                    0,
                    bytemuck::cast_slice(&[transform_uniform]),
                );

                render_pass.set_bind_group(1, &bind_groups[idx], &[]);

                // Use per-mesh material bind group if present, else shared default
                if let Some(ref mbg) = instance.mesh.material_bind_group {
                    render_pass.set_bind_group(3, mbg, &[]);
                } else {
                    render_pass.set_bind_group(3, &self.material_bind_group, &[]);
                }

                render_pass.set_vertex_buffer(0, instance.mesh.vertex_buffer.slice(..));
                render_pass.set_index_buffer(
                    instance.mesh.index_buffer.slice(..),
                    wgpu::IndexFormat::Uint32,
                );
                render_pass.draw_indexed(0..instance.mesh.num_indices, 0, 0..1);
            }

            // Draw outline for selected model
            if let Some(selected_gid) = selected_group_id {
                render_pass.set_pipeline(&self.outline_pipeline);
                render_pass.set_bind_group(0, &self.camera_bind_group, &[]);

                for (idx, instance) in scene.meshes.iter().enumerate() {
                    if let crate::scene::MeshSource::UserModel { group_id, .. } = &instance.source {
                        if *group_id == selected_gid {
                            let transform_uniform = TransformUniform::from_transform(&instance.transform);
                            gpu.queue.write_buffer(
                                &buffers[idx],
                                0,
                                bytemuck::cast_slice(&[transform_uniform]),
                            );

                            render_pass.set_bind_group(1, &bind_groups[idx], &[]);
                            render_pass.set_vertex_buffer(0, instance.mesh.vertex_buffer.slice(..));
                            render_pass.set_index_buffer(
                                instance.mesh.index_buffer.slice(..),
                                wgpu::IndexFormat::Uint32,
                            );
                            render_pass.draw_indexed(0..instance.mesh.num_indices, 0, 0..1);
                        }
                    }
                }
            }

            // Draw particles
            if !particles.is_empty() {
                let render_type_val = particle_render_type as u32 as f32;
                let raw_instances: Vec<ParticleInstanceRaw> = particles
                    .iter()
                    .map(|p| ParticleInstanceRaw {
                        center_pos: [p.position.x, p.position.y, p.position.z],
                        size: p.size,
                        color: [p.color.x, p.color.y, p.color.z, p.color.w],
                        angle: p.angle,
                        render_type: render_type_val,
                    })
                    .collect();

                let instance_buffer = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Particle Instance Buffer"),
                    contents: bytemuck::cast_slice(&raw_instances),
                    usage: wgpu::BufferUsages::VERTEX,
                });

                match particle_blend_mode {
                    crate::scene::particle::ParticleBlendMode::Additive => {
                        render_pass.set_pipeline(&self.particle_additive_pipeline);
                    }
                    crate::scene::particle::ParticleBlendMode::Alpha => {
                        render_pass.set_pipeline(&self.particle_alpha_pipeline);
                    }
                }

                render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.particle_quad_vertex_buffer.slice(..));
                render_pass.set_vertex_buffer(1, instance_buffer.slice(..));
                render_pass.set_index_buffer(
                    self.particle_quad_index_buffer.slice(..),
                    wgpu::IndexFormat::Uint16,
                );
                render_pass.draw_indexed(0..6, 0, 0..raw_instances.len() as u32);
            }
        }
    }

    /// Full render: scene + submit. Used when no overlay is needed.
    pub fn render(
        &self,
        gpu: &GpuContext,
        camera_uniform: &CameraUniform,
        scene: &Scene,
    ) {
        let surface_texture = match gpu.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(st) => st,
            wgpu::CurrentSurfaceTexture::Suboptimal(st) => st,
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return;
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                gpu.surface.configure(&gpu.device, &gpu.config);
                return;
            }
            _ => return,
        };
        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        self.render_scene(
            gpu,
            camera_uniform,
            scene,
            &mut encoder,
            &view,
            None,
            &[],
            crate::scene::particle::ParticleBlendMode::Alpha,
            crate::scene::particle::ParticleRenderType::GlowCircle,
        );

        gpu.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();
    }
}
