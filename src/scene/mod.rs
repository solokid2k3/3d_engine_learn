pub mod ground;
pub mod light;
pub mod material;
pub mod mesh;
pub mod particle;
pub mod picking;
pub mod transform;

use crate::scene::light::LightUniform;
use crate::scene::mesh::Mesh;
use crate::scene::transform::Transform;

/// Distinguishes built-in demo geometry from user-uploaded models.
#[derive(Debug, Clone)]
pub enum MeshSource {
    /// Part of the hardcoded demo scene.
    Demo,
    /// Permanent engine fixture (ground plane, etc.) — never removed by user.
    EngineDefault,
    /// Loaded at runtime from a file. All primitives in one glTF share
    /// the same `group_id` so they can be removed as a unit.
    UserModel { group_id: u32, filename: String },
}

/// Container for all renderable objects in the scene.
pub struct Scene {
    pub meshes: Vec<MeshInstance>,
    pub light_uniform: LightUniform,
    /// Monotonically increasing counter for user-model groups.
    next_group_id: u32,
}

/// A mesh + its transform in the scene.
pub struct MeshInstance {
    pub mesh: Mesh,
    pub transform: Transform,
    pub material_index: usize,
    pub source: MeshSource,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            meshes: Vec::new(),
            light_uniform: LightUniform::default(),
            next_group_id: 0,
        }
    }

    /// Add a demo mesh (part of the built-in scene).
    pub fn add_mesh(&mut self, mesh: Mesh, transform: Transform, material_index: usize) {
        self.meshes.push(MeshInstance {
            mesh,
            transform,
            material_index,
            source: MeshSource::Demo,
        });
    }

    /// Add a set of meshes loaded from a user-uploaded glTF/GLB file.
    /// Returns the `group_id` assigned to this model.
    pub fn add_user_model(
        &mut self,
        meshes: Vec<Mesh>,
        transform: Transform,
        filename: &str,
    ) -> u32 {
        let group_id = self.next_group_id;
        self.next_group_id += 1;

        for m in meshes {
            self.meshes.push(MeshInstance {
                mesh: m,
                transform: transform.clone(),
                material_index: 0,
                source: MeshSource::UserModel {
                    group_id,
                    filename: filename.to_string(),
                },
            });
        }
        group_id
    }

    /// Remove all mesh instances belonging to a user-model group.
    pub fn remove_user_model(&mut self, group_id: u32) {
        self.meshes.retain(|inst| {
            !matches!(&inst.source, MeshSource::UserModel { group_id: gid, .. } if *gid == group_id)
        });
    }
}
