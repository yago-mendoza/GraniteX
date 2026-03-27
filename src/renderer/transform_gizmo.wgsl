struct Uniform {
    view_proj: mat4x4<f32>,
    model: mat4x4<f32>,
    highlight_axis: i32,
    _pad1: f32,
    _pad2: f32,
    _pad3: f32,
};

@group(0) @binding(0) var<uniform> u: Uniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
};

@vertex fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let world_pos = u.model * vec4<f32>(in.position, 1.0);
    out.clip_position = u.view_proj * world_pos;
    out.color = in.color;
    out.world_normal = normalize(in.position);
    return out;
}

@fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let light_dir = normalize(vec3<f32>(0.5, 0.7, 0.3));
    let ndl = max(abs(dot(in.world_normal, light_dir)), 0.0);
    var color = in.color * (0.45 + ndl * 0.55);
    return vec4<f32>(color, 1.0);
}
