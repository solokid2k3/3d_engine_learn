/// CPU-side mouse picking via ray–AABB intersection.
///
/// Workflow:
///   1. `unproject_ray` converts screen coordinates → world-space ray.
///   2. `ray_aabb_intersect` tests the ray against one axis-aligned bounding box.
///   3. `pick_object` iterates all user-model meshes, transforms their local AABBs,
///      and returns the closest hit's `group_id`.

use glam::{Mat4, Vec3, Vec4};

use crate::scene::{MeshSource, Scene};

/// Convert a screen-space mouse position into a world-space ray (origin, direction).
pub fn unproject_ray(
    mouse_x: f32,
    mouse_y: f32,
    screen_width: f32,
    screen_height: f32,
    inv_view_proj: Mat4,
) -> (Vec3, Vec3) {
    // Normalize to NDC [-1, 1]
    let ndc_x = (2.0 * mouse_x) / screen_width - 1.0;
    let ndc_y = 1.0 - (2.0 * mouse_y) / screen_height; // flip Y

    // Near and far points in clip space
    let near_clip = Vec4::new(ndc_x, ndc_y, 0.0, 1.0);
    let far_clip = Vec4::new(ndc_x, ndc_y, 1.0, 1.0);

    // Unproject to world space
    let near_world = inv_view_proj * near_clip;
    let far_world = inv_view_proj * far_clip;

    let near = near_world.truncate() / near_world.w;
    let far = far_world.truncate() / far_world.w;

    let direction = (far - near).normalize();
    (near, direction)
}

/// Ray–AABB intersection using the slab method.
/// Returns `Some(t)` where `t` is the distance along the ray to the nearest hit,
/// or `None` if the ray misses the box.
pub fn ray_aabb_intersect(
    ray_origin: Vec3,
    ray_dir: Vec3,
    aabb_min: Vec3,
    aabb_max: Vec3,
) -> Option<f32> {
    let inv_dir = Vec3::new(
        if ray_dir.x.abs() > 1e-8 { 1.0 / ray_dir.x } else { f32::INFINITY * ray_dir.x.signum() },
        if ray_dir.y.abs() > 1e-8 { 1.0 / ray_dir.y } else { f32::INFINITY * ray_dir.y.signum() },
        if ray_dir.z.abs() > 1e-8 { 1.0 / ray_dir.z } else { f32::INFINITY * ray_dir.z.signum() },
    );

    let t1 = (aabb_min - ray_origin) * inv_dir;
    let t2 = (aabb_max - ray_origin) * inv_dir;

    let t_min = t1.min(t2);
    let t_max = t1.max(t2);

    let t_enter = t_min.x.max(t_min.y).max(t_min.z);
    let t_exit = t_max.x.min(t_max.y).min(t_max.z);

    if t_enter <= t_exit && t_exit >= 0.0 {
        Some(t_enter.max(0.0))
    } else {
        None
    }
}

/// Pick the closest user-model object under the mouse cursor.
/// Returns the `group_id` of the hit model, or `None`.
pub fn pick_object(
    mouse_x: f32,
    mouse_y: f32,
    screen_width: f32,
    screen_height: f32,
    inv_view_proj: Mat4,
    scene: &Scene,
) -> Option<u32> {
    let (ray_origin, ray_dir) =
        unproject_ray(mouse_x, mouse_y, screen_width, screen_height, inv_view_proj);

    let mut closest_t = f32::INFINITY;
    let mut closest_group: Option<u32> = None;

    for inst in &scene.meshes {
        // Only pick user-uploaded models (skip ground plane, demo geometry)
        let group_id = match &inst.source {
            MeshSource::UserModel { group_id, .. } => *group_id,
            _ => continue,
        };

        // Transform the local AABB into world space.
        // For an accurate test with non-uniform scale/rotation, we transform
        // all 8 corners of the local AABB and build a world-space AABB around them.
        let model_mat = inst.transform.to_model_matrix();
        let local_min = Vec3::from(inst.mesh.aabb_min);
        let local_max = Vec3::from(inst.mesh.aabb_max);

        // Generate all 8 corners
        let corners = [
            Vec3::new(local_min.x, local_min.y, local_min.z),
            Vec3::new(local_max.x, local_min.y, local_min.z),
            Vec3::new(local_min.x, local_max.y, local_min.z),
            Vec3::new(local_max.x, local_max.y, local_min.z),
            Vec3::new(local_min.x, local_min.y, local_max.z),
            Vec3::new(local_max.x, local_min.y, local_max.z),
            Vec3::new(local_min.x, local_max.y, local_max.z),
            Vec3::new(local_max.x, local_max.y, local_max.z),
        ];

        let mut world_min = Vec3::splat(f32::INFINITY);
        let mut world_max = Vec3::splat(f32::NEG_INFINITY);

        for corner in &corners {
            let world_corner = model_mat.transform_point3(*corner);
            world_min = world_min.min(world_corner);
            world_max = world_max.max(world_corner);
        }

        if let Some(t) = ray_aabb_intersect(ray_origin, ray_dir, world_min, world_max) {
            if t < closest_t {
                closest_t = t;
                closest_group = Some(group_id);
            }
        }
    }

    closest_group
}
