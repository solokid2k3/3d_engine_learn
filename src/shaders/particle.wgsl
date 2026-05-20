struct CameraUniform {
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    view_pos: vec4<f32>,
    camera_right: vec4<f32>,
    camera_up: vec4<f32>,
};
@group(0) @binding(0) var<uniform> camera: CameraUniform;

// ── Vertex ──

struct VertexInput {
    @location(0) position: vec2<f32>, // Quad coords [-0.5, 0.5]
    @location(1) uv: vec2<f32>,
};

struct InstanceInput {
    @location(2) center_pos: vec3<f32>,
    @location(3) size: f32,
    @location(4) color: vec4<f32>,
    @location(5) angle: f32,
    @location(6) render_type: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) render_type: f32,
};

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;

    // Billboard offset computation with rotation
    let c = cos(instance.angle);
    let s = sin(instance.angle);
    
    // Rotate local quad coordinates
    let rotated_x = model.position.x * c - model.position.y * s;
    let rotated_y = model.position.x * s + model.position.y * c;
    
    // Align with camera right and up vectors
    let world_pos = instance.center_pos 
                  + camera.camera_right.xyz * (rotated_x * instance.size)
                  + camera.camera_up.xyz * (rotated_y * instance.size);

    out.clip_position = camera.view_proj * vec4<f32>(world_pos, 1.0);
    out.uv = model.uv;
    out.color = instance.color;
    out.render_type = instance.render_type;

    return out;
}

// ── Fragment ──

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let dist = length(uv - vec2<f32>(0.5, 0.5));
    
    var alpha: f32 = 0.0;
    let r_type = i32(in.render_type + 0.5);
    
    if (r_type == 0) {
        // GlowCircle
        alpha = 1.0 - smoothstep(0.0, 0.5, dist);
    } else if (r_type == 1) {
        // Spark
        let dx = abs(uv.x - 0.5);
        let dy = abs(uv.y - 0.5);
        let flare_x = 1.0 - smoothstep(0.0, 0.03, dy);
        let flare_y = 1.0 - smoothstep(0.0, 0.03, dx);
        let radial = 1.0 - smoothstep(0.0, 0.45, dist);
        alpha = max(radial * 0.25, max(flare_x * (1.0 - dy * 2.0), flare_y * (1.0 - dx * 2.0)));
        alpha = alpha * (1.0 - smoothstep(0.42, 0.5, dist));
    } else if (r_type == 2) {
        // Flame
        let flame_uv = vec2<f32>(uv.x, uv.y + (0.5 - uv.y) * 0.3);
        let flame_dist = length(flame_uv - vec2<f32>(0.5, 0.4));
        alpha = 1.0 - smoothstep(0.0, 0.48, flame_dist);
    } else if (r_type == 3) {
        // Smoke
        let angle = atan2(uv.y - 0.5, uv.x - 0.5);
        let variation = sin(angle * 5.0) * 0.10 + cos(angle * 3.0) * 0.06;
        alpha = 1.0 - smoothstep(0.0, 0.45 + variation, dist);
    } else if (r_type == 4) {
        // Star (4-pointed glow)
        let dx = abs(uv.x - 0.5);
        let dy = abs(uv.y - 0.5);
        let star_val = 0.015 / (dx * dy + 0.001);
        alpha = star_val * (1.0 - smoothstep(0.35, 0.5, dist));
        alpha = clamp(alpha, 0.0, 1.0);
    } else {
        alpha = 1.0 - smoothstep(0.0, 0.5, dist);
    }
    
    let final_color = vec4<f32>(in.color.rgb, in.color.a * alpha);
    if (final_color.a < 0.005) {
        discard;
    }
    return final_color;
}
