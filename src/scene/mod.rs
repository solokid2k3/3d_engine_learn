pub mod light;
pub mod material;
pub mod mesh;
pub mod transform;

use crate::scene::light::LightUniform;
use crate::scene::mesh::Mesh;
use crate::scene::transform::Transform;

/// Container for all renderable objects in the scene.
pub struct Scene {
    pub meshes: Vec<MeshInstance>,
    pub light_uniform: LightUniform,
}

/// A mesh + its transform in the scene.
pub struct MeshInstance {
    pub mesh: Mesh,
    pub transform: Transform,
    pub material_index: usize,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            meshes: Vec::new(),
            light_uniform: LightUniform::default(),
        }
    }

    pub fn add_mesh(&mut self, mesh: Mesh, transform: Transform, material_index: usize) {
        self.meshes.push(MeshInstance {
            mesh,
            transform,
            material_index,
        });
    }
}
