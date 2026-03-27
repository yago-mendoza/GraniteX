use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

use super::camera::Camera;
use super::gpu_state::MSAA_SAMPLE_COUNT;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct GizmoVertex {
    position: [f32; 3],
    color: [f32; 3],
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct GizmoUniform {
    view_proj: [[f32; 4]; 4],
}

pub struct GizmoPipeline {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    num_vertices: u32,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl GizmoPipeline {
    pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration, camera: &Camera) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Gizmo Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("gizmo.wgsl").into()),
        });

        // Build axis line vertices — 3 axes, each with an arrow head
        let vertices = Self::build_vertices();
        let num_vertices = vertices.len() as u32;

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Gizmo Vertices"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let uniform = Self::compute_uniform(camera);
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Gizmo Uniform"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Gizmo BGL"),
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
            label: Some("Gizmo BG"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Gizmo Pipeline Layout"),
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
            label: Some("Gizmo Pipeline"),
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
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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
                depth_compare: wgpu::CompareFunction::Always, // always on top
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: MSAA_SAMPLE_COUNT,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        Self {
            pipeline,
            vertex_buffer,
            num_vertices,
            uniform_buffer,
            bind_group,
        }
    }

    fn compute_uniform(camera: &Camera) -> GizmoUniform {
        // Use only the camera's rotation (no translation) — gizmo stays centered
        let eye = camera.eye();
        let target = camera.target;
        let view_dir = (target - eye).normalize();

        // Build rotation-only view matrix
        let right = view_dir.cross(Vec3::Y).normalize();
        let up = right.cross(view_dir).normalize();

        // View matrix that only rotates (camera at origin looking at -view_dir)
        let rotation_view = Mat4::look_at_rh(
            -view_dir * 2.5, // eye at a fixed distance
            Vec3::ZERO,
            up,
        );

        // Orthographic projection — fixed size
        let proj = Mat4::orthographic_rh(-1.8, 1.8, -1.8, 1.8, 0.1, 10.0);

        let view_proj = proj * rotation_view;
        GizmoUniform {
            view_proj: view_proj.to_cols_array_2d(),
        }
    }

    fn build_vertices() -> Vec<GizmoVertex> {
        let mut verts = Vec::new();

        // Axis colors
        let red = [0.9, 0.2, 0.2];   // X
        let green = [0.2, 0.8, 0.2];  // Y
        let blue = [0.3, 0.4, 0.9];   // Z

        let shaft_len = 0.85;
        let shaft_radius = 0.04;
        let head_len = 0.3;
        let head_radius = 0.1;
        let segments = 12;

        // Generate each axis arrow
        for (axis, color) in [
            (Vec3::X, red),
            (Vec3::Y, green),
            (Vec3::Z, blue),
        ] {
            // Build orthonormal basis for this axis
            let (u, v) = if axis.dot(Vec3::Y).abs() < 0.99 {
                let u = axis.cross(Vec3::Y).normalize();
                let v = u.cross(axis).normalize();
                (u, v)
            } else {
                let u = axis.cross(Vec3::X).normalize();
                let v = u.cross(axis).normalize();
                (u, v)
            };

            // Shaft (cylinder from origin to shaft_len along axis)
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

                // Two triangles per quad
                verts.push(GizmoVertex { position: bot0.into(), color });
                verts.push(GizmoVertex { position: top0.into(), color });
                verts.push(GizmoVertex { position: top1.into(), color });

                verts.push(GizmoVertex { position: bot0.into(), color });
                verts.push(GizmoVertex { position: top1.into(), color });
                verts.push(GizmoVertex { position: bot1.into(), color });
            }

            // Arrow head (cone from shaft_len to shaft_len + head_len)
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

        // Small sphere at origin (8 triangles, octahedron)
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

    pub fn update_camera(&self, queue: &wgpu::Queue, camera: &Camera) {
        let uniform = Self::compute_uniform(camera);
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniform]));
    }

    pub fn draw<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, width: u32, height: u32) {
        if width == 0 || height == 0 { return; }
        // Draw in bottom-left corner, offset past the left panel (~160px)
        let size = 85.0_f32.min(width as f32 * 0.1).min(height as f32 * 0.15);
        if size < 1.0 { return; }
        let x = 185.0; // past the fixed 150px left panel + generous margin
        let y = height as f32 - size - 25.0; // above status bar

        render_pass.set_viewport(x, y, size, size, 0.0, 1.0);
        render_pass.set_scissor_rect(x as u32, y as u32, size as u32, size as u32);
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.draw(0..self.num_vertices, 0..1);

        // Restore full viewport
        render_pass.set_viewport(0.0, 0.0, width as f32, height as f32, 0.0, 1.0);
        render_pass.set_scissor_rect(0, 0, width, height);
    }
}
