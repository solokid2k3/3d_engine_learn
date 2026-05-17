use std::path::Path;

use wgpu::util::DeviceExt;

use crate::gpu::texture::Texture;
use crate::renderer::vertex::Vertex;
use crate::scene::material::{Material, MaterialUniform};
use crate::scene::mesh::Mesh;

/// Load meshes from a glTF 2.0 file, including textures and materials.
///
/// Each returned `Mesh` carries its own `material_bind_group` when the glTF
/// primitive has a base color texture. Otherwise falls back to a default
/// white texture with vertex colors.
pub fn load_gltf(
    path: &Path,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    material_bgl: &wgpu::BindGroupLayout,
) -> Result<Vec<Mesh>, Box<dyn std::error::Error>> {
    let (document, buffers, images) = gltf::import(path)?;

    // Pre-upload all images as GPU textures
    let gpu_textures: Vec<Option<Texture>> = images
        .iter()
        .map(|img_data| {
            let rgba = match img_data.format {
                gltf::image::Format::R8G8B8A8 => img_data.pixels.clone(),
                gltf::image::Format::R8G8B8 => {
                    // Convert RGB → RGBA
                    let mut rgba = Vec::with_capacity(img_data.pixels.len() / 3 * 4);
                    for chunk in img_data.pixels.chunks(3) {
                        rgba.extend_from_slice(chunk);
                        rgba.push(255);
                    }
                    rgba
                }
                gltf::image::Format::R8 => {
                    // Grayscale → RGBA
                    let mut rgba = Vec::with_capacity(img_data.pixels.len() * 4);
                    for &v in &img_data.pixels {
                        rgba.extend_from_slice(&[v, v, v, 255]);
                    }
                    rgba
                }
                gltf::image::Format::R8G8 => {
                    // RG → RGBA (use R as intensity, G as alpha)
                    let mut rgba = Vec::with_capacity(img_data.pixels.len() / 2 * 4);
                    for chunk in img_data.pixels.chunks(2) {
                        rgba.extend_from_slice(&[chunk[0], chunk[0], chunk[0], chunk[1]]);
                    }
                    rgba
                }
                _ => {
                    log::warn!("Unsupported glTF image format: {:?}, skipping", img_data.format);
                    return None;
                }
            };

            Some(Texture::from_rgba8(
                device,
                queue,
                &rgba,
                img_data.width,
                img_data.height,
                "glTF Texture",
            ))
        })
        .collect();

    // Fallback white texture for primitives without a base color texture
    let white_texture = Texture::create_default_white(device, queue);

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

            // Read vertex colors if present (optional, default to white)
            let colors: Vec<[f32; 3]> = reader
                .read_colors(0)
                .map(|c| c.into_rgb_f32().collect())
                .unwrap_or_else(|| vec![[1.0, 1.0, 1.0]; positions.len()]);

            // Build vertices
            let vertices: Vec<Vertex> = positions
                .iter()
                .enumerate()
                .map(|(i, pos)| Vertex {
                    position: *pos,
                    normal: normals[i],
                    tex_coords: tex_coords[i],
                    color: colors[i],
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

            let mut m = Mesh::new(device, &name, &vertices, &indices);

            // --- Build per-mesh material bind group ---
            // Try to find the base color texture from the material
            let tex_ref: &Texture = primitive
                .material()
                .pbr_metallic_roughness()
                .base_color_texture()
                .and_then(|info| {
                    let idx = info.texture().source().index();
                    gpu_textures.get(idx).and_then(|t| t.as_ref())
                })
                .unwrap_or(&white_texture);

            // Extract base color factor as material diffuse
            let base_color_factor = primitive
                .material()
                .pbr_metallic_roughness()
                .base_color_factor();

            let mat = Material {
                ambient: [
                    base_color_factor[0] * 0.15,
                    base_color_factor[1] * 0.15,
                    base_color_factor[2] * 0.15,
                ],
                diffuse: [base_color_factor[0], base_color_factor[1], base_color_factor[2]],
                specular: [0.5, 0.5, 0.5],
                shininess: 32.0,
            };
            let mat_uniform = MaterialUniform::from_material(&mat);
            let mat_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{name} Material Buffer")),
                contents: bytemuck::cast_slice(&[mat_uniform]),
                usage: wgpu::BufferUsages::UNIFORM,
            });

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(&format!("{name} Material BG")),
                layout: material_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: mat_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&tex_ref.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&tex_ref.sampler),
                    },
                ],
            });

            m.material_bind_group = Some(bind_group);

            meshes.push(m);
        }
    }

    log::info!("Loaded {} meshes from {:?}", meshes.len(), path);
    Ok(meshes)
}
