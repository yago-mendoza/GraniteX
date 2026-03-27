// Edge line shader — draws dark lines on face boundaries.
// Edges adjacent to the selected/hovered face are highlighted (SolidWorks-style).

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

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) face_id_a: u32,
    @location(2) face_id_b: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) @interpolate(flat) face_id_a: u32,
    @location(1) @interpolate(flat) face_id_b: u32,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = scene.view_proj * vec4<f32>(in.position, 1.0);
    out.face_id_a = in.face_id_a;
    out.face_id_b = in.face_id_b;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Check if this edge touches the selected face
    let sel = u32(max(scene.selected_face, 0));
    let hov = u32(max(scene.hovered_face, 0));

    let is_selected = scene.selected_face >= 0 && (in.face_id_a == sel || in.face_id_b == sel);
    let is_hovered = scene.hovered_face >= 0 && (in.face_id_a == hov || in.face_id_b == hov);

    if is_selected {
        // Bright blue for selected face edges (SolidWorks green, we use blue)
        return vec4<f32>(0.15, 0.4, 0.9, 1.0);
    } else if is_hovered {
        // Subtle blue for hovered face edges
        return vec4<f32>(0.3, 0.45, 0.7, 1.0);
    } else {
        // Default: dark edge
        return vec4<f32>(0.12, 0.12, 0.14, 1.0);
    }
}
