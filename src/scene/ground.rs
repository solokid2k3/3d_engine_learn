use crate::gpu::texture::Texture;
use crate::renderer::vertex::Vertex;
use crate::scene::material::MaterialUniform;
use crate::scene::mesh::Mesh;

/// Create the engine's default ground plane: a large quad textured with caro tiles.
///
/// Returns a `Mesh` that already has its `material_bind_group` set to the caro tile
/// texture, so the renderer will use it automatically.
pub fn create_ground_plane(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    material_bgl: &wgpu::BindGroupLayout,
) -> Mesh {
    use wgpu::util::DeviceExt;

    // --- Geometry: 200×200 unit quad centered at origin, Y=0, facing up ---
    let half = 100.0_f32;
    let tile_count = 50.0_f32; // UV repeats

    let vertices = vec![
        Vertex {
            position: [-half, 0.0, -half],
            normal: [0.0, 1.0, 0.0],
            tex_coords: [0.0, 0.0],
            color: [1.0, 1.0, 1.0],
        },
        Vertex {
            position: [half, 0.0, -half],
            normal: [0.0, 1.0, 0.0],
            tex_coords: [tile_count, 0.0],
            color: [1.0, 1.0, 1.0],
        },
        Vertex {
            position: [half, 0.0, half],
            normal: [0.0, 1.0, 0.0],
            tex_coords: [tile_count, tile_count],
            color: [1.0, 1.0, 1.0],
        },
        Vertex {
            position: [-half, 0.0, half],
            normal: [0.0, 1.0, 0.0],
            tex_coords: [0.0, tile_count],
            color: [1.0, 1.0, 1.0],
        },
    ];

    let indices: Vec<u32> = vec![0, 2, 1, 0, 3, 2];

    // --- Load caro tile texture ---
    let tile_bytes = include_bytes!("../assets/caro_tiles/Tiles074_2K-JPG_Color.jpg");
    let img = image::load_from_memory(tile_bytes)
        .expect("Failed to load caro tile texture")
        .to_rgba8();
    let (w, h) = img.dimensions();
    let tile_texture = Texture::from_rgba8(device, queue, &img, w, h, "Caro Tile Texture");

    // --- Material: neutral white so the tile texture shows through ---
    let mat_uniform = MaterialUniform {
        ambient: [0.15, 0.15, 0.15, 64.0], // w = shininess
        diffuse: [1.0, 1.0, 1.0, 0.0],
        specular: [0.3, 0.3, 0.3, 0.0],
    };

    let mat_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Ground Material Buffer"),
        contents: bytemuck::cast_slice(&[mat_uniform]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let material_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Ground Material Bind Group"),
        layout: material_bgl,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: mat_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&tile_texture.view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&tile_texture.sampler),
            },
        ],
    });

    // --- Build mesh ---
    let mut mesh = Mesh::new(device, "Ground Plane", &vertices, &indices);
    mesh.material_bind_group = Some(material_bind_group);
    mesh
}
