// Selection outline shader — inverted-hull technique.
// Pushes vertices outward along normals, then renders solid orange
// with front-face culling so only the outer shell is visible.

// Same camera + transform bind groups as phong shader.

struct CameraUniform {
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    view_pos: vec4<f32>,
};
@group(0) @binding(0) var<uniform> camera: CameraUniform;

struct TransformUniform {
    model: mat4x4<f32>,
    normal_mat: mat4x4<f32>,
};
@group(1) @binding(0) var<uniform> transform: TransformUniform;

// ── Vertex ──

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coords: vec2<f32>,
    @location(3) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

const OUTLINE_THICKNESS: f32 = 0.025;
const OUTLINE_COLOR: vec3<f32> = vec3<f32>(1.0, 0.6, 0.15); // Blender orange

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    // Push vertex outward along its normal in model space.
    let expanded_pos = in.position + normalize(in.normal) * OUTLINE_THICKNESS;
    let world_pos = transform.model * vec4<f32>(expanded_pos, 1.0);
    out.clip_position = camera.view_proj * world_pos;

    return out;
}

// ── Fragment ──

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(OUTLINE_COLOR, 1.0);
}
