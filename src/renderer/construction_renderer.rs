// ConstructionRenderer — draws reference planes and axes in the 3D viewport.
//
// Reuses sketch.wgsl shader (same vertex format: position + color + alpha).
// Planes: semi-transparent quads with border edges.
// Axes: thin line-quads extending through origin.

use glam::Vec3;
use wgpu::util::DeviceExt;

use super::camera::Camera;
use super::gpu_state::MSAA_SAMPLE_COUNT;
use crate::construction::{ConstructionGeometry, ConstructionId};

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
    alpha: f32,
    _pad: f32,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniform {
    view_proj: [[f32; 4]; 4],
}

pub struct ConstructionRenderer {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    vertex_buffer: Option<wgpu::Buffer>,
    num_vertices: u32,
}

impl ConstructionRenderer {
    pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration, camera: &Camera) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Construction Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("sketch.wgsl").into()),
        });

        let uniform = Uniform { view_proj: camera.uniform().view_proj };
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Construction Uniform"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Construction BGL"),
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
            label: Some("Construction BG"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Construction Layout"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });

        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute { offset: 0, shader_location: 0, format: wgpu::VertexFormat::Float32x3 },
                wgpu::VertexAttribute { offset: 12, shader_location: 1, format: wgpu::VertexFormat::Float32x3 },
                wgpu::VertexAttribute { offset: 24, shader_location: 2, format: wgpu::VertexFormat::Float32 },
            ],
        };

        // Use LessEqual (not Always) so planes sit behind mesh but in front of grid
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Construction Pipeline"),
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
                cull_mode: None, // render both sides of planes
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::LessEqual,
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
        }
    }

    pub fn update_camera(&self, queue: &wgpu::Queue, camera: &Camera) {
        let uniform = Uniform { view_proj: camera.uniform().view_proj };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniform]));
    }

    /// Rebuild vertex buffer from construction geometry state.
    pub fn update(&mut self, device: &wgpu::Device, cg: &ConstructionGeometry, camera_distance: f32) {
        let extent = (camera_distance * 0.6).clamp(0.5, 10.0);
        let line_width = (camera_distance * 0.002).clamp(0.002, 0.03);
        let edge_width = (camera_distance * 0.001).clamp(0.001, 0.015);

        let mut verts = Vec::new();

        // Render planes
        for (i, plane) in cg.planes.iter().enumerate() {
            if !plane.visible { continue; }

            let id = ConstructionId::Plane(i);
            let alpha = if cg.selected == Some(id) { 0.18 }
                else if cg.hovered == Some(id) { 0.12 }
                else { 0.06 };
            let edge_alpha = if cg.selected == Some(id) { 0.6 }
                else if cg.hovered == Some(id) { 0.45 }
                else { 0.25 };

            self.push_plane_quad(&mut verts, plane.origin, plane.u_axis, plane.v_axis, extent, plane.color, alpha);
            self.push_plane_edges(&mut verts, plane.origin, plane.u_axis, plane.v_axis, plane.normal, extent, edge_width, plane.color, edge_alpha);
        }

        // Render axes
        for (i, axis) in cg.axes.iter().enumerate() {
            if !axis.visible { continue; }

            let id = ConstructionId::Axis(i);
            let alpha = if cg.selected == Some(id) { 0.8 }
                else if cg.hovered == Some(id) { 0.6 }
                else { 0.35 };

            self.push_axis_line(&mut verts, axis.origin, axis.direction, extent, line_width, axis.color, alpha);
        }

        self.num_vertices = verts.len() as u32;
        if verts.is_empty() {
            self.vertex_buffer = None;
        } else {
            self.vertex_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Construction Vertices"),
                contents: bytemuck::cast_slice(&verts),
                usage: wgpu::BufferUsages::VERTEX,
            }));
        }
    }

    fn push_plane_quad(&self, verts: &mut Vec<Vertex>, origin: Vec3, u: Vec3, v: Vec3, extent: f32, color: [f32; 3], alpha: f32) {
        let a = origin - u * extent - v * extent;
        let b = origin + u * extent - v * extent;
        let c = origin + u * extent + v * extent;
        let d = origin - u * extent + v * extent;

        let sv = |pos: Vec3| Vertex { position: pos.into(), color, alpha, _pad: 0.0 };
        verts.extend_from_slice(&[sv(a), sv(b), sv(c), sv(a), sv(c), sv(d)]);
    }

    fn push_plane_edges(&self, verts: &mut Vec<Vertex>, origin: Vec3, u: Vec3, v: Vec3, normal: Vec3, extent: f32, width: f32, color: [f32; 3], alpha: f32) {
        let a = origin - u * extent - v * extent;
        let b = origin + u * extent - v * extent;
        let c = origin + u * extent + v * extent;
        let d = origin - u * extent + v * extent;

        // Four edges of the quad
        for (p0, p1) in [(a, b), (b, c), (c, d), (d, a)] {
            let dir = (p1 - p0).normalize_or_zero();
            let right = dir.cross(normal).normalize_or_zero() * width;
            let offset = normal * 0.001; // slight offset to prevent z-fighting

            let v0 = p0 - right + offset;
            let v1 = p0 + right + offset;
            let v2 = p1 + right + offset;
            let v3 = p1 - right + offset;

            let sv = |pos: Vec3| Vertex { position: pos.into(), color, alpha, _pad: 0.0 };
            verts.extend_from_slice(&[sv(v0), sv(v1), sv(v2), sv(v0), sv(v2), sv(v3)]);
        }
    }

    fn push_axis_line(&self, verts: &mut Vec<Vertex>, origin: Vec3, direction: Vec3, extent: f32, width: f32, color: [f32; 3], alpha: f32) {
        let p0 = origin - direction * extent;
        let p1 = origin + direction * extent;

        // Build a camera-independent width by using a perpendicular in world space
        let perp = if direction.dot(Vec3::Y).abs() < 0.99 {
            direction.cross(Vec3::Y).normalize()
        } else {
            direction.cross(Vec3::X).normalize()
        };

        let right = perp * width;

        let v0 = p0 - right;
        let v1 = p0 + right;
        let v2 = p1 + right;
        let v3 = p1 - right;

        let sv = |pos: Vec3| Vertex { position: pos.into(), color, alpha, _pad: 0.0 };
        verts.extend_from_slice(&[sv(v0), sv(v1), sv(v2), sv(v0), sv(v2), sv(v3)]);
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
