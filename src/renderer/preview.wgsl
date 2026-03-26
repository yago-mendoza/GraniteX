struct PreviewUniform {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> preview: PreviewUniform;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

@vertex
fn vs_main(@location(0) position: vec3<f32>) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = preview.view_proj * vec4<f32>(position, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Transparent blue ghost
    return vec4<f32>(0.3, 0.5, 0.9, 0.25);
}
