use std::path::Path;

use crate::renderer::vertex::Vertex;
use crate::scene::mesh::Mesh;

/// Load meshes from a glTF 2.0 file.
pub fn load_gltf(
    path: &Path,
    device: &wgpu::Device,
) -> Result<Vec<Mesh>, Box<dyn std::error::Error>> {
    let (document, buffers, _images) = gltf::import(path)?;

    let mut meshes = Vec::new();

    for mesh in document.meshes() {
        for primitive in mesh.primitives() {
            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

            // Read positions (required)
            let positions: Vec<[f32; 3]> = reader
                .read_positions()
                .expect("glTF mesh missing positions")
                .collect();

            // Read normals (optional, default to up)
            let normals: Vec<[f32; 3]> = reader
                .read_normals()
                .map(|n| n.collect())
                .unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; positions.len()]);

            // Read tex coords (optional, default to 0,0)
            let tex_coords: Vec<[f32; 2]> = reader
                .read_tex_coords(0)
                .map(|tc| tc.into_f32().collect())
                .unwrap_or_else(|| vec![[0.0, 0.0]; positions.len()]);

            // Build vertices
            let vertices: Vec<Vertex> = positions
                .iter()
                .enumerate()
                .map(|(i, pos)| Vertex {
                    position: *pos,
                    normal: normals[i],
                    tex_coords: tex_coords[i],
                    color: [0.8, 0.8, 0.8], // default gray
                })
                .collect();

            // Read indices
            let indices: Vec<u32> = reader
                .read_indices()
                .map(|idx| idx.into_u32().collect())
                .unwrap_or_else(|| (0..vertices.len() as u32).collect());

            let name = mesh
                .name()
                .unwrap_or("unnamed")
                .to_string();

            meshes.push(Mesh::new(device, &name, &vertices, &indices));
        }
    }

    log::info!("Loaded {} meshes from {:?}", meshes.len(), path);
    Ok(meshes)
}
