// SketchRenderer — draws sketch entities on face planes.
// Confirmed entities = green, preview (pending) = orange, endpoints = yellow dots.

use glam::Vec3;
use wgpu::util::DeviceExt;

use super::camera::Camera;
use super::gpu_state::MSAA_SAMPLE_COUNT;
use crate::sketch::{Sketch, SnapType};
use crate::ui::SketchTool;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct SketchVertex {
    position: [f32; 3],
    color: [f32; 3],
    alpha: f32,
    _pad: f32, // align to 32 bytes
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct SketchUniform {
    view_proj: [[f32; 4]; 4],
}

pub struct SketchRenderer {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    vertex_buffer: Option<wgpu::Buffer>,
    num_vertices: u32,
    region_fill_buffer: Option<wgpu::Buffer>,
    num_region_vertices: u32,
    overlay_buffer: Option<wgpu::Buffer>,
    num_overlay_vertices: u32,
}

impl SketchRenderer {
    pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration, camera: &Camera) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Sketch Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("sketch.wgsl").into()),
        });

        let uniform = SketchUniform { view_proj: camera.uniform().view_proj };
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sketch Uniform"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Sketch BGL"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Sketch BG"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Sketch Layout"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });

        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<SketchVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute { offset: 0, shader_location: 0, format: wgpu::VertexFormat::Float32x3 },
                wgpu::VertexAttribute { offset: 12, shader_location: 1, format: wgpu::VertexFormat::Float32x3 },
                wgpu::VertexAttribute { offset: 24, shader_location: 2, format: wgpu::VertexFormat::Float32 },
            ],
        };

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Sketch Pipeline"),
            layout: Some(&layout),
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
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Always, // ALWAYS on top — sketch is overlay
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: MSAA_SAMPLE_COUNT, mask: !0, alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        Self {
            pipeline, uniform_buffer, bind_group,
            vertex_buffer: None, num_vertices: 0,
            region_fill_buffer: None, num_region_vertices: 0,
            overlay_buffer: None, num_overlay_vertices: 0,
        }
    }

    pub fn update_camera(&mut self, queue: &wgpu::Queue, camera: &Camera) {
        let uniform = SketchUniform { view_proj: camera.uniform().view_proj };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniform]));
    }

    /// Rebuild vertex buffer from sketch state (confirmed + preview).
    /// Lines are rendered as thin quads FLAT on the sketch plane (not camera-facing).
    pub fn update_sketch(&mut self, device: &wgpu::Device, sketch: &Sketch, tool: SketchTool, _camera_eye: Vec3) {
        let line_width = 0.005;
        let dot_size = 0.008;

        let confirmed_color = [0.15, 0.85, 0.3];  // green
        let construction_color = [1.0, 0.6, 0.15]; // orange for construction lines
        let preview_color = [1.0, 0.55, 0.1];      // orange
        let dot_color = [0.95, 0.9, 0.2];          // yellow

        let normal = sketch.plane.normal;

        let mut verts = Vec::new();

        for (p0, p1) in sketch.confirmed_lines_3d() {
            self.push_line_on_plane(&mut verts, p0, p1, normal, line_width, confirmed_color);
        }

        // Construction lines rendered in orange, slightly thinner
        for (p0, p1) in sketch.construction_lines_3d() {
            self.push_line_on_plane(&mut verts, p0, p1, normal, line_width * 0.7, construction_color);
        }

        for (p0, p1) in sketch.preview_lines_3d(tool) {
            self.push_line_on_plane(&mut verts, p0, p1, normal, line_width, preview_color);
        }

        for p3d in sketch.all_endpoints_3d() {
            self.push_dot_on_plane(&mut verts, p3d, normal, dot_size, dot_color);
        }

        if let Some(start) = sketch.pending_start {
            let p3d = sketch.to_3d(start);
            self.push_dot_on_plane(&mut verts, p3d, normal, dot_size * 1.5, preview_color);
        }

        // Selected entity highlight — render on top in magenta, thicker
        if let Some(idx) = sketch.selected_entity {
            let highlight_color = [1.0, 0.2, 0.8]; // magenta
            let highlight_width = line_width * 2.5;
            for (p0, p1) in sketch.entity_lines_3d(idx) {
                self.push_line_on_plane(&mut verts, p0, p1, normal, highlight_width, highlight_color);
            }
        }

        // Snap indicator — show the snap target the cursor is near
        if let Some(snap) = sketch.active_snap_target() {
            let snap_3d = sketch.to_3d(snap.point);
            self.push_snap_indicator(&mut verts, snap_3d, normal, snap.snap_type);
        }

        self.num_vertices = verts.len() as u32;
        if verts.is_empty() {
            self.vertex_buffer = None;
        } else {
            self.vertex_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Sketch Vertices"),
                contents: bytemuck::cast_slice(&verts),
                usage: wgpu::BufferUsages::VERTEX,
            }));
        }
    }

    /// Render a line as a thin quad lying on the sketch plane.
    fn push_line_on_plane(&self, verts: &mut Vec<SketchVertex>, p0: Vec3, p1: Vec3, normal: Vec3, width: f32, color: [f32; 3]) {
        let line_dir = (p1 - p0).normalize_or_zero();
        if line_dir.length() < 0.001 { return; }

        // Width direction = perpendicular to line, IN the plane (not toward camera)
        let right = line_dir.cross(normal).normalize_or_zero() * width;
        if right.length() < 0.0001 { return; }

        // Offset slightly along normal to prevent z-fighting
        let offset = normal * 0.006;
        let a = p0 - right + offset;
        let b = p0 + right + offset;
        let c = p1 + right + offset;
        let d = p1 - right + offset;

        let sv = |pos: Vec3| SketchVertex { position: pos.into(), color, alpha: 1.0, _pad: 0.0 };
        verts.extend_from_slice(&[sv(a), sv(b), sv(c), sv(a), sv(c), sv(d)]);
    }

    /// Render a dot as a small CIRCLE lying on the sketch plane (fan triangulation).
    fn push_dot_on_plane(&self, verts: &mut Vec<SketchVertex>, pos: Vec3, normal: Vec3, size: f32, color: [f32; 3]) {
        let (u, v) = Self::plane_axes(normal);
        let offset = normal * 0.004;
        let center = pos + offset;
        let segments = 10;
        let sv = |p: Vec3| SketchVertex { position: p.into(), color, alpha: 1.0, _pad: 0.0 };

        for i in 0..segments {
            let a0 = std::f32::consts::TAU * i as f32 / segments as f32;
            let a1 = std::f32::consts::TAU * (i + 1) as f32 / segments as f32;
            let p0 = center + u * (size * a0.cos()) + v * (size * a0.sin());
            let p1 = center + u * (size * a1.cos()) + v * (size * a1.sin());
            verts.extend_from_slice(&[sv(center), sv(p0), sv(p1)]);
        }
    }

    /// Render a SQUARE indicator (for corner snap).
    fn push_square_on_plane(&self, verts: &mut Vec<SketchVertex>, pos: Vec3, normal: Vec3, size: f32, color: [f32; 3]) {
        let (u, v) = Self::plane_axes(normal);
        let offset = normal * 0.005;
        let center = pos + offset;
        let sv = |p: Vec3| SketchVertex { position: p.into(), color, alpha: 1.0, _pad: 0.0 };

        let a = center - u * size - v * size;
        let b = center + u * size - v * size;
        let c = center + u * size + v * size;
        let d = center - u * size + v * size;
        verts.extend_from_slice(&[sv(a), sv(b), sv(c), sv(a), sv(c), sv(d)]);
    }

    /// Render a TRIANGLE indicator (for midpoint snap).
    fn push_triangle_on_plane(&self, verts: &mut Vec<SketchVertex>, pos: Vec3, normal: Vec3, size: f32, color: [f32; 3]) {
        let (u, v) = Self::plane_axes(normal);
        let offset = normal * 0.005;
        let center = pos + offset;
        let sv = |p: Vec3| SketchVertex { position: p.into(), color, alpha: 1.0, _pad: 0.0 };

        let top = center + v * size * 1.2;
        let bl = center - u * size - v * size * 0.6;
        let br = center + u * size - v * size * 0.6;
        verts.extend_from_slice(&[sv(top), sv(bl), sv(br)]);
    }

    /// Render a DIAMOND indicator (for edge snap).
    fn push_diamond_on_plane(&self, verts: &mut Vec<SketchVertex>, pos: Vec3, normal: Vec3, size: f32, color: [f32; 3]) {
        let (u, v) = Self::plane_axes(normal);
        let offset = normal * 0.005;
        let center = pos + offset;
        let sv = |p: Vec3| SketchVertex { position: p.into(), color, alpha: 1.0, _pad: 0.0 };

        let top = center + v * size;
        let right = center + u * size;
        let bottom = center - v * size;
        let left = center - u * size;
        verts.extend_from_slice(&[sv(top), sv(right), sv(bottom), sv(top), sv(bottom), sv(left)]);
    }

    /// Render the appropriate snap indicator based on type.
    fn push_snap_indicator(&self, verts: &mut Vec<SketchVertex>, pos: Vec3, normal: Vec3, snap_type: SnapType) {
        let size = 0.012;
        match snap_type {
            SnapType::Endpoint | SnapType::Quadrant => {
                self.push_dot_on_plane(verts, pos, normal, size, [0.95, 0.9, 0.2]); // yellow
            }
            SnapType::Corner => {
                self.push_square_on_plane(verts, pos, normal, size, [1.0, 0.6, 0.1]); // orange
            }
            SnapType::Midpoint => {
                self.push_triangle_on_plane(verts, pos, normal, size, [0.2, 0.9, 0.9]); // cyan
            }
            SnapType::Edge | SnapType::Circumference => {
                self.push_diamond_on_plane(verts, pos, normal, size * 0.8, [0.9, 0.3, 0.9]); // magenta
            }
        }
    }

    fn plane_axes(normal: Vec3) -> (Vec3, Vec3) {
        let u = if normal.dot(Vec3::Y).abs() < 0.99 {
            normal.cross(Vec3::Y).normalize()
        } else {
            normal.cross(Vec3::X).normalize()
        };
        let v = normal.cross(u).normalize();
        (u, v)
    }

    /// Append region fill triangles to the vertex buffer.
    /// Called after update_sketch with region data computed externally.
    pub fn append_region_fills(
        &mut self,
        device: &wgpu::Device,
        regions: &[crate::sketch::SketchRegion],
        selected_region: Option<usize>,
        plane: &crate::sketch::SketchPlane,
    ) {
        let normal = plane.normal;
        let offset = normal * 0.005; // above the face, consistent with sketch lines
        let region_color = [0.3, 0.45, 0.7]; // subtle blue
        let selected_color = [0.35, 0.55, 0.85]; // brighter blue for selected
        let region_alpha = 0.15;
        let selected_alpha = 0.3;

        let mut verts = Vec::new();

        for (i, region) in regions.iter().enumerate() {
            let is_selected = selected_region == Some(i);
            let color = if is_selected { selected_color } else { region_color };
            let alpha = if is_selected { selected_alpha } else { region_alpha };

            let points_3d: Vec<Vec3> = region.boundary.iter()
                .map(|p| plane.to_3d(*p) + offset)
                .collect();

            for tri in &region.triangles {
                if tri[0] >= points_3d.len() || tri[1] >= points_3d.len() || tri[2] >= points_3d.len() {
                    continue;
                }
                let sv = |pos: Vec3| SketchVertex { position: pos.into(), color, alpha, _pad: 0.0 };
                verts.push(sv(points_3d[tri[0]]));
                verts.push(sv(points_3d[tri[1]]));
                verts.push(sv(points_3d[tri[2]]));
            }
        }

        if verts.is_empty() { return; }

        self.region_fill_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Region Fill Vertices"),
            contents: bytemuck::cast_slice(&verts),
            usage: wgpu::BufferUsages::VERTEX,
        }));
        self.num_region_vertices = verts.len() as u32;
    }

    /// Render only confirmed entities (for inactive sketches on other faces).
    pub fn update_sketch_confirmed_only(&mut self, device: &wgpu::Device, sketch: &Sketch, _camera_eye: Vec3) {
        let line_width = 0.005;
        let confirmed_color = [0.15, 0.85, 0.3];
        let dot_color = [0.95, 0.9, 0.2];
        let dot_size = 0.008;
        let normal = sketch.plane.normal;

        let mut verts = Vec::new();

        for (p0, p1) in sketch.confirmed_lines_3d() {
            self.push_line_on_plane(&mut verts, p0, p1, normal, line_width, confirmed_color);
        }
        for p3d in sketch.all_endpoints_3d() {
            self.push_dot_on_plane(&mut verts, p3d, normal, dot_size, dot_color);
        }

        self.num_vertices = verts.len() as u32;
        if verts.is_empty() {
            self.vertex_buffer = None;
        } else {
            self.vertex_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Sketch Vertices"),
                contents: bytemuck::cast_slice(&verts),
                usage: wgpu::BufferUsages::VERTEX,
            }));
        }
    }

    /// Build a camera-facing (billboard) line quad for 3D overlay rendering.
    fn push_line_billboard(verts: &mut Vec<SketchVertex>, p0: Vec3, p1: Vec3, camera_eye: Vec3, width: f32, color: [f32; 3]) {
        let line_dir = (p1 - p0).normalize_or_zero();
        if line_dir.length_squared() < 1e-8 { return; }

        let mid = (p0 + p1) * 0.5;
        let to_cam = (camera_eye - mid).normalize_or_zero();
        let right = line_dir.cross(to_cam).normalize_or_zero() * width;
        if right.length_squared() < 1e-10 { return; }

        let sv = |pos: Vec3| SketchVertex { position: pos.into(), color, alpha: 1.0, _pad: 0.0 };
        let a = p0 - right;
        let b = p0 + right;
        let c = p1 + right;
        let d = p1 - right;
        verts.extend_from_slice(&[sv(a), sv(b), sv(c), sv(a), sv(c), sv(d)]);
    }

    /// Build a camera-facing dot for 3D overlay rendering.
    fn push_dot_billboard(verts: &mut Vec<SketchVertex>, pos: Vec3, camera_eye: Vec3, size: f32, color: [f32; 3]) {
        let to_cam = (camera_eye - pos).normalize_or_zero();
        if to_cam.length_squared() < 1e-8 { return; }

        // Build orthonormal frame facing camera
        let up = if to_cam.dot(Vec3::Y).abs() < 0.99 {
            to_cam.cross(Vec3::Y).normalize_or_zero()
        } else {
            to_cam.cross(Vec3::X).normalize_or_zero()
        };
        let right = to_cam.cross(up).normalize_or_zero();

        let sv = |p: Vec3| SketchVertex { position: p.into(), color, alpha: 1.0, _pad: 0.0 };
        let segments = 8;
        for i in 0..segments {
            let a0 = std::f32::consts::TAU * i as f32 / segments as f32;
            let a1 = std::f32::consts::TAU * (i + 1) as f32 / segments as f32;
            let p0 = pos + up * (size * a0.cos()) + right * (size * a0.sin());
            let p1 = pos + up * (size * a1.cos()) + right * (size * a1.sin());
            verts.extend_from_slice(&[sv(pos), sv(p0), sv(p1)]);
        }
    }

    /// Update 3D overlay geometry: measurement lines, edge highlights.
    /// These use billboard quads (face the camera) instead of plane-based quads.
    pub fn update_overlays(
        &mut self,
        device: &wgpu::Device,
        camera_eye: Vec3,
        measurement: Option<&crate::ui::Measurement>,
        measure_first: Option<[f32; 3]>,
        selected_edge: Option<([f32; 3], [f32; 3])>,
    ) {
        let mut verts = Vec::new();

        // Measurement line + endpoints
        if let Some(m) = measurement {
            let pa = Vec3::from(m.point_a);
            let pb = Vec3::from(m.point_b);
            let line_color = [0.0, 0.9, 1.0]; // cyan
            let dot_color = [1.0, 1.0, 0.0];  // yellow
            Self::push_line_billboard(&mut verts, pa, pb, camera_eye, 0.004, line_color);
            Self::push_dot_billboard(&mut verts, pa, camera_eye, 0.01, dot_color);
            Self::push_dot_billboard(&mut verts, pb, camera_eye, 0.01, dot_color);
        } else if let Some(first) = measure_first {
            // Show first point as pulsing orange dot
            let p = Vec3::from(first);
            Self::push_dot_billboard(&mut verts, p, camera_eye, 0.012, [1.0, 0.6, 0.0]);
        }

        // Selected edge highlight
        if let Some((a, b)) = selected_edge {
            let pa = Vec3::from(a);
            let pb = Vec3::from(b);
            let edge_color = [1.0, 0.4, 0.0]; // bright orange
            Self::push_line_billboard(&mut verts, pa, pb, camera_eye, 0.006, edge_color);
            Self::push_dot_billboard(&mut verts, pa, camera_eye, 0.008, edge_color);
            Self::push_dot_billboard(&mut verts, pb, camera_eye, 0.008, edge_color);
        }

        self.num_overlay_vertices = verts.len() as u32;
        if verts.is_empty() {
            self.overlay_buffer = None;
        } else {
            self.overlay_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Overlay Vertices"),
                contents: bytemuck::cast_slice(&verts),
                usage: wgpu::BufferUsages::VERTEX,
            }));
        }
    }

    pub fn clear(&mut self) {
        self.vertex_buffer = None;
        self.num_vertices = 0;
        self.region_fill_buffer = None;
        self.num_region_vertices = 0;
        self.overlay_buffer = None;
        self.num_overlay_vertices = 0;
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        // Draw region fills first (behind sketch lines)
        if let Some(ref rb) = self.region_fill_buffer {
            if self.num_region_vertices > 0 {
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &self.bind_group, &[]);
                pass.set_vertex_buffer(0, rb.slice(..));
                pass.draw(0..self.num_region_vertices, 0..1);
            }
        }
        // Then draw sketch lines and dots (on top)
        if let Some(ref vb) = self.vertex_buffer {
            if self.num_vertices > 0 {
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &self.bind_group, &[]);
                pass.set_vertex_buffer(0, vb.slice(..));
                pass.draw(0..self.num_vertices, 0..1);
            }
        }
        // 3D overlays: measurement lines, edge highlights (camera-facing billboards)
        if let Some(ref ob) = self.overlay_buffer {
            if self.num_overlay_vertices > 0 {
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &self.bind_group, &[]);
                pass.set_vertex_buffer(0, ob.slice(..));
                pass.draw(0..self.num_overlay_vertices, 0..1);
            }
        }
    }
}
