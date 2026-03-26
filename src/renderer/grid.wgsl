struct CameraUniform {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let grid_size = 100.0;

    let grid_positions = array<vec3<f32>, 6>(
        vec3<f32>(-grid_size, 0.0, -grid_size),
        vec3<f32>( grid_size, 0.0, -grid_size),
        vec3<f32>( grid_size, 0.0,  grid_size),
        vec3<f32>(-grid_size, 0.0, -grid_size),
        vec3<f32>( grid_size, 0.0,  grid_size),
        vec3<f32>(-grid_size, 0.0,  grid_size),
    );

    let world_pos = grid_positions[vertex_index];

    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(world_pos, 1.0);
    out.world_pos = world_pos;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let pos = in.world_pos;
    let dist = length(pos.xz);

    // Single grid scale, very subtle
    let coord = pos.xz;
    let derivative = fwidth(coord);
    let grid_line = abs(fract(coord - 0.5) - 0.5) / derivative;
    let line = min(grid_line.x, grid_line.y);
    let grid_alpha = (1.0 - min(line, 1.0)) * 0.15;

    // Fade with distance
    let fade = 1.0 - smoothstep(15.0, 50.0, dist);

    // Base: very faint gray grid
    var alpha = grid_alpha * fade;

    // X axis — subtle red
    if abs(pos.z) < derivative.y * 1.0 {
        alpha = max(alpha, 0.5 * fade);
        let axis_color = vec4<f32>(0.7, 0.15, 0.15, alpha);
        return axis_color;
    }
    // Z axis — subtle blue
    if abs(pos.x) < derivative.x * 1.0 {
        alpha = max(alpha, 0.5 * fade);
        let axis_color = vec4<f32>(0.15, 0.2, 0.7, alpha);
        return axis_color;
    }

    if alpha < 0.005 {
        discard;
    }

    return vec4<f32>(0.4, 0.4, 0.45, alpha);
}
