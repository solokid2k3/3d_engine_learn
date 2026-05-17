// Procedural sky + sun disc shader
// Renders as a fullscreen triangle — no vertex buffer needed.

// ============================================================
// Uniforms
// ============================================================

struct SkyUniforms {
    inv_view_proj: mat4x4<f32>,
    sun_direction: vec4<f32>,  // xyz = normalized direction TO sun, w = unused
    sun_color: vec4<f32>,      // xyz = sun color, w = sun intensity
};
@group(0) @binding(0) var<uniform> sky: SkyUniforms;

// ============================================================
// Vertex — fullscreen triangle from vertex_index
// ============================================================

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) ndc: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Generate a triangle that covers the entire screen
    // vertex 0: (-1, -1), vertex 1: (3, -1), vertex 2: (-1, 3)
    var out: VertexOutput;
    let x = f32(i32(vertex_index & 1u)) * 4.0 - 1.0;
    let y = f32(i32(vertex_index >> 1u)) * 4.0 - 1.0;
    out.clip_position = vec4<f32>(x, y, 0.9999, 1.0);
    out.ndc = vec2<f32>(x, y);
    return out;
}

// ============================================================
// Fragment — procedural sky gradient + sun disc
// ============================================================

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Reconstruct world-space ray direction from NDC
    let clip_pos = vec4<f32>(in.ndc.x, in.ndc.y, 1.0, 1.0);
    let world_pos = sky.inv_view_proj * clip_pos;
    let ray_dir = normalize(world_pos.xyz / world_pos.w);

    // --- Sky gradient ---
    let up_factor = ray_dir.y * 0.5 + 0.5; // 0 at horizon, 1 at zenith

    // Zenith: deep blue
    let zenith_color = vec3<f32>(0.15, 0.25, 0.55);
    // Horizon: warm orange-pink
    let horizon_color = vec3<f32>(0.65, 0.45, 0.35);
    // Below horizon: dark ground blend
    let nadir_color = vec3<f32>(0.08, 0.08, 0.12);

    var sky_color: vec3<f32>;
    if ray_dir.y >= 0.0 {
        // Smooth gradient from horizon to zenith using a power curve
        let t = pow(up_factor, 0.7);
        sky_color = mix(horizon_color, zenith_color, t);

        // Add a subtle warm glow near the horizon
        let horizon_glow = exp(-abs(ray_dir.y) * 8.0) * 0.3;
        sky_color += vec3<f32>(0.8, 0.5, 0.3) * horizon_glow;
    } else {
        // Below horizon: fade to dark
        let t = clamp(-ray_dir.y * 5.0, 0.0, 1.0);
        sky_color = mix(horizon_color, nadir_color, t);
    }

    // --- Sun disc ---
    let sun_dir = normalize(sky.sun_direction.xyz);
    let sun_dot = dot(ray_dir, sun_dir);
    let sun_intensity = sky.sun_color.w;

    // Hard sun disc (angular radius ~0.5 degrees → cos(0.5°) ≈ 0.99996, we use wider for visual impact)
    let sun_disc_threshold = 0.9995;
    let sun_disc = smoothstep(sun_disc_threshold - 0.0005, sun_disc_threshold, sun_dot);

    // Soft glow halo around the sun
    let glow_power = 512.0;
    let sun_glow = pow(max(sun_dot, 0.0), glow_power) * 2.0;

    // Medium halo
    let halo_power = 64.0;
    let sun_halo = pow(max(sun_dot, 0.0), halo_power) * 0.4;

    // Wide atmospheric scatter
    let scatter_power = 8.0;
    let sun_scatter = pow(max(sun_dot, 0.0), scatter_power) * 0.15;

    let sun_col = sky.sun_color.xyz * sun_intensity;

    // Combine
    var final_color = sky_color;
    final_color += sun_col * sun_scatter;    // wide atmospheric glow
    final_color += sun_col * sun_halo;       // medium halo
    final_color += sun_col * sun_glow;       // tight glow
    final_color += sun_col * sun_disc * 5.0; // bright disc

    // Simple tone mapping (Reinhard)
    final_color = final_color / (final_color + vec3<f32>(1.0));

    return vec4<f32>(final_color, 1.0);
}
