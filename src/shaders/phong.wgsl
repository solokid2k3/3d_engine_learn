// Enhanced Blinn-Phong lighting shader with visual effects
// Features: Fresnel rim glow, distance fog, environment color bleed, Reinhard tone mapping

// ============================================================
// Uniforms
// ============================================================

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

struct LightUniform {
    dir_direction: vec4<f32>,  // xyz=direction, w=intensity
    dir_color: vec4<f32>,      // xyz=color

    point_positions: array<vec4<f32>, 4>,   // xyz=pos, w=intensity
    point_colors: array<vec4<f32>, 4>,      // xyz=color
    point_attenuation: array<vec4<f32>, 4>, // x=const, y=linear, z=quadratic

    num_point_lights: vec4<f32>,  // x=count, y=time
};
@group(2) @binding(0) var<uniform> lights: LightUniform;

struct MaterialUniform {
    ambient: vec4<f32>,   // xyz=ambient, w=shininess
    diffuse: vec4<f32>,   // xyz=diffuse
    specular: vec4<f32>,  // xyz=specular
};
@group(3) @binding(0) var<uniform> material: MaterialUniform;
@group(3) @binding(1) var t_diffuse: texture_2d<f32>;
@group(3) @binding(2) var s_diffuse: sampler;

// ============================================================
// Constants
// ============================================================

const FOG_COLOR: vec3<f32> = vec3<f32>(0.02, 0.02, 0.05);
const FOG_DENSITY: f32 = 0.04;
const FOG_START: f32 = 8.0;
const RIM_COLOR: vec3<f32> = vec3<f32>(0.4, 0.6, 1.0);
const RIM_POWER: f32 = 3.0;
const RIM_STRENGTH: f32 = 0.6;

// ============================================================
// Vertex
// ============================================================

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coords: vec2<f32>,
    @location(3) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) tex_coords: vec2<f32>,
    @location(3) color: vec3<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let world_pos = transform.model * vec4<f32>(in.position, 1.0);
    out.clip_position = camera.view_proj * world_pos;
    out.world_position = world_pos.xyz;
    out.world_normal = normalize((transform.normal_mat * vec4<f32>(in.normal, 0.0)).xyz);
    out.tex_coords = in.tex_coords;
    out.color = in.color;
    return out;
}

// ============================================================
// Fragment — Enhanced Blinn-Phong
// ============================================================

fn calc_directional_light(
    normal: vec3<f32>,
    view_dir: vec3<f32>,
    mat_diffuse: vec3<f32>,
    mat_specular: vec3<f32>,
    shininess: f32,
) -> vec3<f32> {
    let light_dir = normalize(-lights.dir_direction.xyz);
    let intensity = lights.dir_direction.w;
    let light_color = lights.dir_color.xyz;

    let diff = max(dot(normal, light_dir), 0.0);
    let diffuse = light_color * diff * mat_diffuse * intensity;

    let halfway = normalize(light_dir + view_dir);
    let spec = pow(max(dot(normal, halfway), 0.0), shininess);
    let specular = light_color * spec * mat_specular * intensity;

    return diffuse + specular;
}

fn calc_point_light(
    index: u32,
    frag_pos: vec3<f32>,
    normal: vec3<f32>,
    view_dir: vec3<f32>,
    mat_diffuse: vec3<f32>,
    mat_specular: vec3<f32>,
    shininess: f32,
) -> vec3<f32> {
    let light_pos = lights.point_positions[index].xyz;
    let intensity = lights.point_positions[index].w;
    let light_color = lights.point_colors[index].xyz;
    let attenuation = lights.point_attenuation[index];

    let light_vec = light_pos - frag_pos;
    let distance = length(light_vec);
    let light_dir = normalize(light_vec);

    let atten = 1.0 / (attenuation.x + attenuation.y * distance + attenuation.z * distance * distance);

    let diff = max(dot(normal, light_dir), 0.0);
    let diffuse = light_color * diff * mat_diffuse * intensity * atten;

    let halfway = normalize(light_dir + view_dir);
    let spec = pow(max(dot(normal, halfway), 0.0), shininess);
    let specular = light_color * spec * mat_specular * intensity * atten;

    return diffuse + specular;
}

// Fresnel-Schlick approximation for rim lighting
fn fresnel_rim(normal: vec3<f32>, view_dir: vec3<f32>) -> f32 {
    let n_dot_v = max(dot(normal, view_dir), 0.0);
    return pow(1.0 - n_dot_v, RIM_POWER) * RIM_STRENGTH;
}

// Exponential distance fog
fn apply_fog(color: vec3<f32>, distance: f32) -> vec3<f32> {
    let fog_amount = 1.0 - exp(-FOG_DENSITY * max(distance - FOG_START, 0.0));
    return mix(color, FOG_COLOR, fog_amount);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(in.world_normal);
    let view_dir = normalize(camera.view_pos.xyz - in.world_position);
    let view_dist = length(camera.view_pos.xyz - in.world_position);

    // Sample texture and modulate with vertex color
    let tex_color = textureSample(t_diffuse, s_diffuse, in.tex_coords).xyz;
    let base_color = in.color * tex_color;

    let mat_ambient = material.ambient.xyz * base_color;
    let mat_diffuse = material.diffuse.xyz * base_color;
    let mat_specular = material.specular.xyz;
    let shininess = material.ambient.w;

    // Ambient (subtle, with slight upward sky contribution)
    let sky_ambient = mix(mat_ambient * 0.7, mat_ambient * 1.2, normal.y * 0.5 + 0.5);
    var result = sky_ambient;

    // Directional light
    result += calc_directional_light(normal, view_dir, mat_diffuse, mat_specular, shininess);

    // Point lights
    let num_points = u32(lights.num_point_lights.x);
    for (var i: u32 = 0u; i < num_points; i++) {
        result += calc_point_light(i, in.world_position, normal, view_dir, mat_diffuse, mat_specular, shininess);
    }

    // Fresnel rim glow — creates a beautiful glowing edge effect
    let rim = fresnel_rim(normal, view_dir);
    // Tint the rim based on the nearest point light color for colored rim
    var rim_tint = RIM_COLOR;
    if num_points > 0u {
        var closest_dist = 9999.0;
        for (var i: u32 = 0u; i < num_points; i++) {
            let d = length(lights.point_positions[i].xyz - in.world_position);
            if d < closest_dist {
                closest_dist = d;
                rim_tint = mix(RIM_COLOR, lights.point_colors[i].xyz, 0.5);
            }
        }
    }
    result += rim_tint * rim;

    // Tone mapping (ACES-inspired filmic)
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    let mapped = clamp((result * (a * result + b)) / (result * (c * result + d) + e), vec3<f32>(0.0), vec3<f32>(1.0));

    // Apply distance fog
    let final_color = apply_fog(mapped, view_dist);

    return vec4<f32>(final_color, 1.0);
}
