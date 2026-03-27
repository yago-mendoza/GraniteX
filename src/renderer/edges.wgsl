// Edge line shader — draws dark lines on face boundaries only.
// Uses LineList topology with position-only vertices (boundary edges computed on CPU).

struct SceneUniform {
    view_proj: mat4x4<f32>,
    camera_eye: vec4<f32>,
    selected_face: i32,
    hovered_face: i32,
    _pad0: f32,
    _pad1: f32,
};

@group(0) @binding(0)
var<uniform> scene: SceneUniform;

@vertex
fn vs_main(@location(0) position: vec3<f32>) -> @builtin(position) vec4<f32> {
    return scene.view_proj * vec4<f32>(position, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}
