// SketchRenderer — draws sketch entities on face planes.
// Confirmed entities = green, preview (pending) = orange, endpoints = yellow dots.

use glam::Vec3;
use wgpu::util::DeviceExt;

use super::camera::Camera;
use super::gpu_state::MSAA_SAMPLE_COUNT;
use crate::sketch::Sketch;
use crate::ui::SketchTool;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct SketchVertex {
    position: [f32; 3],
    color: [f32; 3],
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

        Self { pipeline, uniform_buffer, bind_group, vertex_buffer: None, num_vertices: 0 }
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
        let preview_color = [1.0, 0.55, 0.1];      // orange
        let dot_color = [0.95, 0.9, 0.2];          // yellow

        let normal = sketch.plane.normal;

        let mut verts = Vec::new();

        for (p0, p1) in sketch.confirmed_lines_3d() {
            self.push_line_on_plane(&mut verts, p0, p1, normal, line_width, confirmed_color);
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
        let offset = normal * 0.003;
        let a = p0 - right + offset;
        let b = p0 + right + offset;
        let c = p1 + right + offset;
        let d = p1 - right + offset;

        let sv = |pos: Vec3| SketchVertex { position: pos.into(), color };
        verts.extend_from_slice(&[sv(a), sv(b), sv(c), sv(a), sv(c), sv(d)]);
    }

    /// Render a dot as a small CIRCLE lying on the sketch plane (fan triangulation).
    fn push_dot_on_plane(&self, verts: &mut Vec<SketchVertex>, pos: Vec3, normal: Vec3, size: f32, color: [f32; 3]) {
        let u = if normal.dot(Vec3::Y).abs() < 0.99 {
            normal.cross(Vec3::Y).normalize()
        } else {
            normal.cross(Vec3::X).normalize()
        };
        let v = normal.cross(u).normalize();

        let offset = normal * 0.004; // slightly more offset than lines
        let center = pos + offset;
        let segments = 10;
        let sv = |p: Vec3| SketchVertex { position: p.into(), color };

        for i in 0..segments {
            let a0 = std::f32::consts::TAU * i as f32 / segments as f32;
            let a1 = std::f32::consts::TAU * (i + 1) as f32 / segments as f32;
            let p0 = center + u * (size * a0.cos()) + v * (size * a0.sin());
            let p1 = center + u * (size * a1.cos()) + v * (size * a1.sin());
            verts.extend_from_slice(&[sv(center), sv(p0), sv(p1)]);
        }
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

    pub fn clear(&mut self) {
        self.vertex_buffer = None;
        self.num_vertices = 0;
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        if let Some(ref vb) = self.vertex_buffer {
            if self.num_vertices > 0 {
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &self.bind_group, &[]);
                pass.set_vertex_buffer(0, vb.slice(..));
                pass.draw(0..self.num_vertices, 0..1);
            }
        }
    }
}
