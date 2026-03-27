// TransformGizmo — 3D translate manipulator rendered at the centroid of the selected face.
// Renders in the main viewport using the scene's perspective projection.
// Scales with camera distance to maintain constant screen size.

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

use super::gizmo::GizmoVertex;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum GizmoAxis {
    X,
    Y,
    Z,
}

impl GizmoAxis {
    pub fn direction(&self) -> Vec3 {
        match self {
            GizmoAxis::X => Vec3::X,
            GizmoAxis::Y => Vec3::Y,
            GizmoAxis::Z => Vec3::Z,
        }
    }

    pub fn color(&self) -> [f32; 3] {
        match self {
            GizmoAxis::X => [0.9, 0.2, 0.2],
            GizmoAxis::Y => [0.2, 0.8, 0.2],
            GizmoAxis::Z => [0.3, 0.4, 0.9],
        }
    }

    fn index(&self) -> i32 {
        match self {
            GizmoAxis::X => 1,
            GizmoAxis::Y => 2,
            GizmoAxis::Z => 3,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct TransformGizmoUniform {
    view_proj: [[f32; 4]; 4],
    model: [[f32; 4]; 4],
    highlight_axis: i32,
    _pad: [f32; 3],
}

pub(crate) struct TransformGizmo {
    vertex_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
    vertex_count: u32,
    // state
    pub position: Vec3,
    pub visible: bool,
    pub active_axis: Option<GizmoAxis>,
    pub dragging: bool,
    drag_start_point: Vec3,
    cached_scale: f32,
}

impl TransformGizmo {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat, msaa_count: u32) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Transform Gizmo Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("transform_gizmo.wgsl").into()),
        });

        let vertices = Self::build_vertices();
        let vertex_count = vertices.len() as u32;

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Transform Gizmo Vertices"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let uniform = TransformGizmoUniform {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            model: Mat4::IDENTITY.to_cols_array_2d(),
            highlight_axis: 0,
            _pad: [0.0; 3],
        };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Transform Gizmo Uniform"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Transform Gizmo BGL"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Transform Gizmo BG"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Transform Gizmo Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<GizmoVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 12,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        };

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Transform Gizmo Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[vertex_layout],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: msaa_count,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        Self {
            vertex_buffer,
            uniform_buffer,
            bind_group,
            pipeline,
            vertex_count,
            position: Vec3::ZERO,
            visible: false,
            active_axis: None,
            dragging: false,
            drag_start_point: Vec3::ZERO,
            cached_scale: 1.0,
        }
    }

    /// Update the model matrix uniform. Call when position or camera changes.
    /// Scale = camera_distance * 0.08 (constant screen size).
    pub fn update_uniform(&self, queue: &wgpu::Queue, view_proj: Mat4, camera_distance: f32) {
        let scale = camera_distance * 0.08;
        let model = Mat4::from_translation(self.position) * Mat4::from_scale(Vec3::splat(scale));

        let highlight = self.active_axis.map(|a| a.index()).unwrap_or(0);

        let uniform = TransformGizmoUniform {
            view_proj: view_proj.to_cols_array_2d(),
            model: model.to_cols_array_2d(),
            highlight_axis: highlight,
            _pad: [0.0; 3],
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniform]));
    }

    /// Draw the gizmo in the render pass (call after main mesh, before egui).
    pub fn draw<'a>(&'a self, rpass: &mut wgpu::RenderPass<'a>) {
        if !self.visible {
            return;
        }
        rpass.set_pipeline(&self.pipeline);
        rpass.set_bind_group(0, &self.bind_group, &[]);
        rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        rpass.draw(0..self.vertex_count, 0..1);
    }

    /// Pick: given a ray (origin, direction) in world space, test against each axis.
    /// Returns the axis if hit (within threshold distance from axis line).
    pub fn pick(&self, ray_origin: Vec3, ray_dir: Vec3, camera_distance: f32) -> Option<GizmoAxis> {
        let scale = camera_distance * 0.08;
        let shaft_length = 1.0 * scale;
        let threshold = 0.05 * scale;

        let mut best_axis: Option<GizmoAxis> = None;
        let mut best_dist = f32::MAX;

        for axis in [GizmoAxis::X, GizmoAxis::Y, GizmoAxis::Z] {
            let axis_dir = axis.direction();
            let axis_start = self.position;
            let axis_end = self.position + axis_dir * shaft_length;

            let d = closest_distance_ray_segment(ray_origin, ray_dir, axis_start, axis_end);

            if d < threshold && d < best_dist {
                best_dist = d;
                best_axis = Some(axis);
            }
        }

        best_axis
    }

    /// Start dragging on an axis. Records the initial intersection point.
    pub fn start_drag(&mut self, axis: GizmoAxis, ray_origin: Vec3, ray_dir: Vec3) {
        self.active_axis = Some(axis);
        self.dragging = true;
        self.drag_start_point = project_ray_onto_axis(ray_origin, ray_dir, self.position, axis.direction());
    }

    /// Continue dragging. Returns the world-space delta since drag started.
    pub fn update_drag(&mut self, ray_origin: Vec3, ray_dir: Vec3) -> Vec3 {
        let axis = match self.active_axis {
            Some(a) => a,
            None => return Vec3::ZERO,
        };

        let current = project_ray_onto_axis(ray_origin, ray_dir, self.position, axis.direction());
        let delta = current - self.drag_start_point;

        // Project delta onto the axis direction only
        let axis_dir = axis.direction();
        axis_dir * delta.dot(axis_dir)
    }

    /// End dragging. Returns final delta.
    pub fn end_drag(&mut self, ray_origin: Vec3, ray_dir: Vec3) -> Vec3 {
        let delta = self.update_drag(ray_origin, ray_dir);
        self.dragging = false;
        self.active_axis = None;
        delta
    }

    /// Set which axis to highlight (None = no highlight, during hover).
    pub fn set_highlight(&mut self, axis: Option<GizmoAxis>) {
        self.active_axis = axis;
    }

    /// Store the current scale for external use.
    pub fn update_scale(&mut self, camera_distance: f32) {
        self.cached_scale = camera_distance * 0.08;
    }

    fn build_vertices() -> Vec<GizmoVertex> {
        let mut verts = Vec::new();

        let red = [0.9, 0.2, 0.2];
        let green = [0.2, 0.8, 0.2];
        let blue = [0.3, 0.4, 0.9];

        let shaft_len = 1.0;
        let shaft_radius = 0.03;
        let head_len = 0.25;
        let head_radius = 0.08;
        let segments = 12;

        for (axis, color) in [
            (Vec3::X, red),
            (Vec3::Y, green),
            (Vec3::Z, blue),
        ] {
            let (u, v) = if axis.dot(Vec3::Y).abs() < 0.99 {
                let u = axis.cross(Vec3::Y).normalize();
                let v = u.cross(axis).normalize();
                (u, v)
            } else {
                let u = axis.cross(Vec3::X).normalize();
                let v = u.cross(axis).normalize();
                (u, v)
            };

            // Shaft cylinder
            for i in 0..segments {
                let a0 = std::f32::consts::TAU * i as f32 / segments as f32;
                let a1 = std::f32::consts::TAU * (i + 1) as f32 / segments as f32;

                let (c0, s0) = (a0.cos(), a0.sin());
                let (c1, s1) = (a1.cos(), a1.sin());

                let offset0 = (u * c0 + v * s0) * shaft_radius;
                let offset1 = (u * c1 + v * s1) * shaft_radius;

                let bot0 = offset0;
                let bot1 = offset1;
                let top0 = axis * shaft_len + offset0;
                let top1 = axis * shaft_len + offset1;

                verts.push(GizmoVertex { position: bot0.into(), color });
                verts.push(GizmoVertex { position: top0.into(), color });
                verts.push(GizmoVertex { position: top1.into(), color });

                verts.push(GizmoVertex { position: bot0.into(), color });
                verts.push(GizmoVertex { position: top1.into(), color });
                verts.push(GizmoVertex { position: bot1.into(), color });
            }

            // Cone arrowhead
            let tip = axis * (shaft_len + head_len);
            let base_center = axis * shaft_len;

            for i in 0..segments {
                let a0 = std::f32::consts::TAU * i as f32 / segments as f32;
                let a1 = std::f32::consts::TAU * (i + 1) as f32 / segments as f32;

                let (c0, s0) = (a0.cos(), a0.sin());
                let (c1, s1) = (a1.cos(), a1.sin());

                let base0 = base_center + (u * c0 + v * s0) * head_radius;
                let base1 = base_center + (u * c1 + v * s1) * head_radius;

                // Cone side
                verts.push(GizmoVertex { position: base0.into(), color });
                verts.push(GizmoVertex { position: tip.into(), color });
                verts.push(GizmoVertex { position: base1.into(), color });

                // Cone base cap
                verts.push(GizmoVertex { position: base_center.into(), color });
                verts.push(GizmoVertex { position: base1.into(), color });
                verts.push(GizmoVertex { position: base0.into(), color });
            }
        }

        // Small octahedron at center (gray)
        let r = 0.06;
        let gray = [0.6, 0.6, 0.6];
        let dirs = [Vec3::X, Vec3::NEG_X, Vec3::Y, Vec3::NEG_Y, Vec3::Z, Vec3::NEG_Z];
        let faces: [(usize, usize, usize); 8] = [
            (0, 2, 4), (0, 4, 3), (0, 3, 5), (0, 5, 2),
            (1, 4, 2), (1, 3, 4), (1, 5, 3), (1, 2, 5),
        ];
        for (a, b, c) in &faces {
            verts.push(GizmoVertex { position: (dirs[*a] * r).into(), color: gray });
            verts.push(GizmoVertex { position: (dirs[*b] * r).into(), color: gray });
            verts.push(GizmoVertex { position: (dirs[*c] * r).into(), color: gray });
        }

        verts
    }
}

/// Closest distance between a ray and a line segment.
fn closest_distance_ray_segment(ray_origin: Vec3, ray_dir: Vec3, seg_a: Vec3, seg_b: Vec3) -> f32 {
    let seg_dir = seg_b - seg_a;
    let seg_len = seg_dir.length();
    if seg_len < 1e-8 {
        return (seg_a - ray_origin).cross(ray_dir).length();
    }
    let d1 = ray_dir.normalize();
    let d2 = seg_dir / seg_len;

    let cross = d1.cross(d2);
    let denom = cross.length_squared();

    if denom < 1e-10 {
        // Ray and segment are nearly parallel — use point-to-line distance
        let to_a = seg_a - ray_origin;
        return to_a.cross(d1).length();
    }

    let w = ray_origin - seg_a;

    // Parameters for closest points on each line
    let t_seg = w.cross(d1).dot(cross) / denom;
    let t_ray = w.cross(d2).dot(cross) / denom;

    // Clamp segment parameter to [0, seg_len]
    let t_seg_clamped = t_seg.clamp(0.0, seg_len);
    // Ray parameter must be positive (in front of camera)
    let t_ray_clamped = t_ray.max(0.0);

    let closest_on_ray = ray_origin + d1 * t_ray_clamped;
    let closest_on_seg = seg_a + d2 * t_seg_clamped;

    (closest_on_ray - closest_on_seg).length()
}

/// Project a ray onto a world-space axis line and return the closest point on the axis line.
fn project_ray_onto_axis(ray_origin: Vec3, ray_dir: Vec3, axis_origin: Vec3, axis_dir: Vec3) -> Vec3 {
    let d1 = ray_dir.normalize();
    let d2 = axis_dir.normalize();

    let cross = d1.cross(d2);
    let denom = cross.length_squared();

    if denom < 1e-10 {
        // Parallel — return the axis origin as fallback
        return axis_origin;
    }

    let w = ray_origin - axis_origin;
    let t = w.cross(d1).dot(cross) / denom;

    axis_origin + d2 * t
}
