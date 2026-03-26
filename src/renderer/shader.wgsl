struct SceneUniform {
    view_proj: mat4x4<f32>,
    selected_face: i32,
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

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(in.world_normal);

    let light_dir_1 = normalize(vec3<f32>(0.5, 0.8, 0.3));
    let light_dir_2 = normalize(vec3<f32>(-0.3, 0.4, -0.6));

    let ndl_1 = max(dot(normal, light_dir_1), 0.0);
    let ndl_2 = max(dot(normal, light_dir_2), 0.0);

    let base_color = vec3<f32>(0.45, 0.47, 0.50);

    let ambient = 0.15;
    let diffuse = ndl_1 * 0.55 + ndl_2 * 0.25;
    var color = base_color * (ambient + diffuse);

    // Fresnel edge darkening
    let view_dir = normalize(vec3<f32>(2.0, 1.5, 2.0) - in.world_position);
    let fresnel = pow(1.0 - max(dot(normal, view_dir), 0.0), 3.0);
    color = mix(color, vec3<f32>(0.2, 0.22, 0.25), fresnel * 0.3);

    // Selection highlight — blue tint
    if scene.selected_face >= 0 && in.face_id == u32(scene.selected_face) {
        let highlight = vec3<f32>(0.3, 0.5, 0.9);
        color = mix(color, highlight, 0.45);
    }

    return vec4<f32>(color, 1.0);
}
