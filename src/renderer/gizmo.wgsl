struct GizmoUniform {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> gizmo: GizmoUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = gizmo.view_proj * vec4<f32>(in.position, 1.0);
    out.color = in.color;
    out.normal = normalize(in.position);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Simple lighting for the gizmo
    let light_dir = normalize(vec3<f32>(0.5, 0.7, 0.5));
    let ndl = max(dot(normalize(in.normal), light_dir), 0.0);
    let lit = in.color * (0.4 + ndl * 0.6);
    return vec4<f32>(lit, 1.0);
}
