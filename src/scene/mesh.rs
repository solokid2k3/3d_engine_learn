/// A GPU-resident mesh with vertex and index buffers.
pub struct Mesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
    pub name: String,
}

impl Mesh {
    pub fn new(
        device: &wgpu::Device,
        name: &str,
        vertices: &[crate::renderer::vertex::Vertex],
        indices: &[u32],
    ) -> Self {
        use wgpu::util::DeviceExt;

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{name} Vertex Buffer")),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{name} Index Buffer")),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            vertex_buffer,
            index_buffer,
            num_indices: indices.len() as u32,
            name: name.to_string(),
        }
    }
}

/// Generate a unit cube mesh with proper normals for each face.
pub fn create_cube(device: &wgpu::Device) -> Mesh {
    use crate::renderer::vertex::Vertex;

    // Each face has 4 vertices (so normals are correct per-face)
    #[rustfmt::skip]
    let vertices = vec![
        // Front face (z = +0.5), normal = (0, 0, 1)
        Vertex { position: [-0.5, -0.5,  0.5], normal: [0.0, 0.0, 1.0], tex_coords: [0.0, 1.0], color: [1.0, 0.3, 0.3] },
        Vertex { position: [ 0.5, -0.5,  0.5], normal: [0.0, 0.0, 1.0], tex_coords: [1.0, 1.0], color: [1.0, 0.3, 0.3] },
        Vertex { position: [ 0.5,  0.5,  0.5], normal: [0.0, 0.0, 1.0], tex_coords: [1.0, 0.0], color: [1.0, 0.3, 0.3] },
        Vertex { position: [-0.5,  0.5,  0.5], normal: [0.0, 0.0, 1.0], tex_coords: [0.0, 0.0], color: [1.0, 0.3, 0.3] },

        // Back face (z = -0.5), normal = (0, 0, -1)
        Vertex { position: [ 0.5, -0.5, -0.5], normal: [0.0, 0.0, -1.0], tex_coords: [0.0, 1.0], color: [0.3, 0.3, 1.0] },
        Vertex { position: [-0.5, -0.5, -0.5], normal: [0.0, 0.0, -1.0], tex_coords: [1.0, 1.0], color: [0.3, 0.3, 1.0] },
        Vertex { position: [-0.5,  0.5, -0.5], normal: [0.0, 0.0, -1.0], tex_coords: [1.0, 0.0], color: [0.3, 0.3, 1.0] },
        Vertex { position: [ 0.5,  0.5, -0.5], normal: [0.0, 0.0, -1.0], tex_coords: [0.0, 0.0], color: [0.3, 0.3, 1.0] },

        // Top face (y = +0.5), normal = (0, 1, 0)
        Vertex { position: [-0.5,  0.5,  0.5], normal: [0.0, 1.0, 0.0], tex_coords: [0.0, 1.0], color: [0.3, 1.0, 0.3] },
        Vertex { position: [ 0.5,  0.5,  0.5], normal: [0.0, 1.0, 0.0], tex_coords: [1.0, 1.0], color: [0.3, 1.0, 0.3] },
        Vertex { position: [ 0.5,  0.5, -0.5], normal: [0.0, 1.0, 0.0], tex_coords: [1.0, 0.0], color: [0.3, 1.0, 0.3] },
        Vertex { position: [-0.5,  0.5, -0.5], normal: [0.0, 1.0, 0.0], tex_coords: [0.0, 0.0], color: [0.3, 1.0, 0.3] },

        // Bottom face (y = -0.5), normal = (0, -1, 0)
        Vertex { position: [-0.5, -0.5, -0.5], normal: [0.0, -1.0, 0.0], tex_coords: [0.0, 1.0], color: [1.0, 1.0, 0.3] },
        Vertex { position: [ 0.5, -0.5, -0.5], normal: [0.0, -1.0, 0.0], tex_coords: [1.0, 1.0], color: [1.0, 1.0, 0.3] },
        Vertex { position: [ 0.5, -0.5,  0.5], normal: [0.0, -1.0, 0.0], tex_coords: [1.0, 0.0], color: [1.0, 1.0, 0.3] },
        Vertex { position: [-0.5, -0.5,  0.5], normal: [0.0, -1.0, 0.0], tex_coords: [0.0, 0.0], color: [1.0, 1.0, 0.3] },

        // Right face (x = +0.5), normal = (1, 0, 0)
        Vertex { position: [ 0.5, -0.5,  0.5], normal: [1.0, 0.0, 0.0], tex_coords: [0.0, 1.0], color: [1.0, 0.3, 1.0] },
        Vertex { position: [ 0.5, -0.5, -0.5], normal: [1.0, 0.0, 0.0], tex_coords: [1.0, 1.0], color: [1.0, 0.3, 1.0] },
        Vertex { position: [ 0.5,  0.5, -0.5], normal: [1.0, 0.0, 0.0], tex_coords: [1.0, 0.0], color: [1.0, 0.3, 1.0] },
        Vertex { position: [ 0.5,  0.5,  0.5], normal: [1.0, 0.0, 0.0], tex_coords: [0.0, 0.0], color: [1.0, 0.3, 1.0] },

        // Left face (x = -0.5), normal = (-1, 0, 0)
        Vertex { position: [-0.5, -0.5, -0.5], normal: [-1.0, 0.0, 0.0], tex_coords: [0.0, 1.0], color: [0.3, 1.0, 1.0] },
        Vertex { position: [-0.5, -0.5,  0.5], normal: [-1.0, 0.0, 0.0], tex_coords: [1.0, 1.0], color: [0.3, 1.0, 1.0] },
        Vertex { position: [-0.5,  0.5,  0.5], normal: [-1.0, 0.0, 0.0], tex_coords: [1.0, 0.0], color: [0.3, 1.0, 1.0] },
        Vertex { position: [-0.5,  0.5, -0.5], normal: [-1.0, 0.0, 0.0], tex_coords: [0.0, 0.0], color: [0.3, 1.0, 1.0] },
    ];

    #[rustfmt::skip]
    let indices: Vec<u32> = vec![
         0,  1,  2,  0,  2,  3, // front
         4,  5,  6,  4,  6,  7, // back
         8,  9, 10,  8, 10, 11, // top
        12, 13, 14, 12, 14, 15, // bottom
        16, 17, 18, 16, 18, 19, // right
        20, 21, 22, 20, 22, 23, // left
    ];

    Mesh::new(device, "Cube", &vertices, &indices)
}

/// Generate a UV sphere mesh.
pub fn create_sphere(device: &wgpu::Device, stacks: u32, slices: u32) -> Mesh {
    use crate::renderer::vertex::Vertex;
    use std::f32::consts::PI;

    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for i in 0..=stacks {
        let phi = PI * i as f32 / stacks as f32;
        let y = phi.cos();
        let ring_radius = phi.sin();

        for j in 0..=slices {
            let theta = 2.0 * PI * j as f32 / slices as f32;
            let x = ring_radius * theta.cos();
            let z = ring_radius * theta.sin();

            let u = j as f32 / slices as f32;
            let v = i as f32 / stacks as f32;

            // Color gradient based on position
            let r = (x * 0.5 + 0.5).clamp(0.2, 1.0);
            let g = (y * 0.5 + 0.5).clamp(0.2, 1.0);
            let b = (z * 0.5 + 0.5).clamp(0.2, 1.0);

            vertices.push(Vertex {
                position: [x * 0.5, y * 0.5, z * 0.5],
                normal: [x, y, z],
                tex_coords: [u, v],
                color: [r, g, b],
            });
        }
    }

    for i in 0..stacks {
        for j in 0..slices {
            let first = i * (slices + 1) + j;
            let second = first + slices + 1;

            indices.push(first);
            indices.push(second);
            indices.push(first + 1);

            indices.push(second);
            indices.push(second + 1);
            indices.push(first + 1);
        }
    }

    Mesh::new(device, "Sphere", &vertices, &indices)
}
