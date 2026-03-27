struct SceneUniform {
    view_proj: mat4x4<f32>,
    camera_eye: vec4<f32>,      // xyz = eye position, w = unused
    selected_face: i32,
    hovered_face: i32,
    _pad0: f32,
    _pad1: f32,
};

@group(0) @binding(0)
var<uniform> scene: SceneUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) face_id: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) world_position: vec3<f32>,
    @location(2) @interpolate(flat) face_id: u32,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = scene.view_proj * vec4<f32>(in.position, 1.0);
    out.world_normal = in.normal;
    out.world_position = in.position;
    out.face_id = in.face_id;
    return out;
}

// sRGB ↔ linear conversions for physically correct lighting
fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    return pow(c, vec3<f32>(2.2));
}

fn linear_to_srgb(c: vec3<f32>) -> vec3<f32> {
    return pow(max(c, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.2));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(in.world_normal);
    let view_dir = normalize(scene.camera_eye.xyz - in.world_position);

    // Work in linear color space for physically correct math
    let base_color = srgb_to_linear(vec3<f32>(0.45, 0.47, 0.50));

    // Three-light setup for industrial CAD look
    let light_dir_1 = normalize(vec3<f32>(0.5, 0.8, 0.3));   // key light
    let light_dir_2 = normalize(vec3<f32>(-0.3, 0.4, -0.6));  // fill light
    let light_dir_3 = normalize(vec3<f32>(0.0, -0.5, 0.8));   // rim/back light

    // Diffuse (Lambertian) — abs() for two-sided lighting (pocket interiors)
    let ndl_1 = abs(dot(normal, light_dir_1));
    let ndl_2 = abs(dot(normal, light_dir_2));
    let ndl_3 = abs(dot(normal, light_dir_3));

    let ambient = srgb_to_linear(vec3<f32>(0.10, 0.10, 0.12));
    let diffuse = base_color * (ndl_1 * 0.55 + ndl_2 * 0.20 + ndl_3 * 0.08);

    // Specular (Blinn-Phong) — gives metallic/plastic shine like SolidWorks
    let shininess = 48.0;
    let spec_strength = 0.35;
    let half_1 = normalize(light_dir_1 + view_dir);
    let half_2 = normalize(light_dir_2 + view_dir);
    let spec_1 = pow(max(dot(normal, half_1), 0.0), shininess) * ndl_1;
    let spec_2 = pow(max(dot(normal, half_2), 0.0), shininess) * ndl_2;
    let specular = vec3<f32>(1.0) * spec_strength * (spec_1 * 0.7 + spec_2 * 0.3);

    var color = ambient + diffuse + specular;

    // Fresnel rim darkening — abs() for two-sided
    let fresnel = pow(1.0 - abs(dot(normal, view_dir)), 4.0);
    color = mix(color, srgb_to_linear(vec3<f32>(0.15, 0.16, 0.18)), fresnel * 0.25);

    // Selection highlight — blue tint
    if scene.selected_face >= 0 && in.face_id == u32(scene.selected_face) {
        let highlight = srgb_to_linear(vec3<f32>(0.3, 0.5, 0.9));
        color = mix(color, highlight, 0.4);
    }
    // Hover pre-highlight — subtle warm tint
    else if scene.hovered_face >= 0 && in.face_id == u32(scene.hovered_face) {
        let hover_color = srgb_to_linear(vec3<f32>(0.5, 0.6, 0.8));
        color = mix(color, hover_color, 0.15);
    }

    // Convert back to sRGB for display
    color = linear_to_srgb(color);

    return vec4<f32>(color, 1.0);
}
