// MeshPipeline — renders the scene mesh with lighting and face selection highlight.

use wgpu::util::DeviceExt;

use super::camera::Camera;
use super::mesh::Mesh;
use super::vertex::Vertex;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct SceneUniform {
    view_proj: [[f32; 4]; 4],  // 64 bytes @ 0
    camera_eye: [f32; 4],       // 16 bytes @ 64 (vec4 for alignment, w unused)
    selected_face: i32,          // 4 bytes @ 80
    hovered_face: i32,           // 4 bytes @ 84
    _pad: [f32; 2],             // 8 bytes @ 88 (total 96, divisible by 16)
}

pub struct MeshPipeline {
    pipeline: wgpu::RenderPipeline,
    wireframe_pipeline: Option<wgpu::RenderPipeline>,
    edge_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    edge_vertex_buffer: Option<wgpu::Buffer>,
    num_edge_vertices: u32,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    selected_face: Option<u32>,
    hovered_face: Option<u32>,
    cached_view_proj: [[f32; 4]; 4],
    cached_eye: [f32; 4],
}

impl MeshPipeline {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        camera: &Camera,
        mesh: &Mesh,
        device_features: wgpu::Features,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Mesh Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let uniform = SceneUniform {
            view_proj: camera.uniform().view_proj,
            camera_eye: { let e = camera.eye(); [e.x, e.y, e.z, 0.0] },
            selected_face: -1,
            hovered_face: -1,
            _pad: [0.0; 2],
        };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Scene Uniform"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Scene BGL"),
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
            label: Some("Scene BG"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Mesh Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Mesh Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::layout()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
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
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: super::gpu_state::MSAA_SAMPLE_COUNT,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let wireframe_pipeline = if device_features.contains(wgpu::Features::POLYGON_MODE_LINE) {
            Some(device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Wireframe Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[Vertex::layout()],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    polygon_mode: wgpu::PolygonMode::Line,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: super::gpu_state::MSAA_SAMPLE_COUNT,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
                cache: None,
            }))
        } else {
            None
        };

        // Edge overlay pipeline — boundary-only lines (SolidWorks-style).
        // Uses LineList topology with position-only vertices. No GPU feature required.
        let edge_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Edge Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("edges.wgsl").into()),
        });

        // Edge vertex: position (vec3) + two face_ids packed as u32 (for selection highlighting)
        let edge_vertex_layout = wgpu::VertexBufferLayout {
            array_stride: (3 * 4 + 4 + 4) as wgpu::BufferAddress, // 3 floats + 2 u32s = 20 bytes
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3, // position
                },
                wgpu::VertexAttribute {
                    offset: 12,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Uint32, // face_id_a
                },
                wgpu::VertexAttribute {
                    offset: 16,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Uint32, // face_id_b
                },
            ],
        };

        let edge_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Edge Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &edge_shader,
                entry_point: Some("vs_main"),
                buffers: &[edge_vertex_layout],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &edge_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState {
                    constant: -8,       // push edges toward camera (in depth buffer units)
                    slope_scale: -1.5,  // scale with surface slope (less aggressive to avoid pop-in)
                    clamp: -0.0001,     // small clamp prevents excessive bias on near-parallel faces
                },
            }),
            multisample: wgpu::MultisampleState {
                count: super::gpu_state::MSAA_SAMPLE_COUNT,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let (vertex_buffer, index_buffer, num_indices) = Self::create_buffers(device, mesh);
        let (edge_vertex_buffer, num_edge_vertices) = Self::create_edge_buffer(device, mesh);

        Self {
            pipeline,
            wireframe_pipeline,
            edge_pipeline,
            vertex_buffer,
            index_buffer,
            num_indices,
            edge_vertex_buffer,
            num_edge_vertices,
            uniform_buffer,
            bind_group,
            selected_face: None,
            hovered_face: None,
            cached_view_proj: uniform.view_proj,
            cached_eye: { let e = camera.eye(); [e.x, e.y, e.z, 0.0] },
        }
    }

    fn create_buffers(device: &wgpu::Device, mesh: &Mesh) -> (wgpu::Buffer, wgpu::Buffer, u32) {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Mesh Vertices"),
            contents: bytemuck::cast_slice(&mesh.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Mesh Indices"),
            contents: bytemuck::cast_slice(&mesh.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        (vertex_buffer, index_buffer, mesh.indices.len() as u32)
    }

    /// Extract boundary edges (edges between different face_ids) and create a LineList buffer.
    /// Only draws edges where two different faces meet — internal triangle diagonals are skipped.
    /// Uses quantized position tuples as keys (collision-free, unlike XOR hashing).
    fn create_edge_buffer(device: &wgpu::Device, mesh: &Mesh) -> (Option<wgpu::Buffer>, u32) {
        use std::collections::{HashMap, HashSet};

        // Quantize a position to integer coords for reliable comparison.
        // 4 decimal places = 0.0001 precision, more than enough for CAD geometry.
        type PosKey = (i64, i64, i64);
        let pos_key = |p: [f32; 3]| -> PosKey {
            ((p[0] * 10000.0).round() as i64,
             (p[1] * 10000.0).round() as i64,
             (p[2] * 10000.0).round() as i64)
        };

        // Canonical edge key: sorted pair of position keys (order-independent).
        type EdgeKey = (PosKey, PosKey);
        let edge_key = |a: [f32; 3], b: [f32; 3]| -> EdgeKey {
            let ka = pos_key(a);
            let kb = pos_key(b);
            if ka <= kb { (ka, kb) } else { (kb, ka) }
        };

        // Pass 1: Collect face_ids AND triangle count per unique edge.
        // tri_count distinguishes internal edges (2 tris, same face) from mesh boundaries (1 tri).
        struct EdgeInfo {
            faces: HashSet<u32>,
            tri_count: u32,
            pa: [f32; 3],
            pb: [f32; 3],
        }
        let mut edge_faces: HashMap<EdgeKey, EdgeInfo> = HashMap::new();

        for tri in mesh.indices.chunks_exact(3) {
            let i0 = tri[0] as usize;
            let i1 = tri[1] as usize;
            let i2 = tri[2] as usize;
            let face_id = mesh.vertices[i0].face_id;

            for &(a, b) in &[(i0, i1), (i1, i2), (i2, i0)] {
                let pa = mesh.vertices[a].position;
                let pb = mesh.vertices[b].position;
                let key = edge_key(pa, pb);

                edge_faces.entry(key)
                    .and_modify(|info| {
                        info.faces.insert(face_id);
                        info.tri_count += 1;
                    })
                    .or_insert_with(|| {
                        let mut s = HashSet::new();
                        s.insert(face_id);
                        EdgeInfo { faces: s, tri_count: 1, pa, pb }
                    });
            }
        }

        // Pass 2: Emit edges with face_id info (for selection highlighting).
        // Each edge vertex = [f32; 3] position + [u32; 2] face_ids
        // Format: [px, py, pz, face_a_bits, face_b_bits] × 2 vertices per edge
        let mut data: Vec<u8> = Vec::new();
        let mut vertex_count = 0u32;

        let emit_edge = |data: &mut Vec<u8>, count: &mut u32, pos: &[f32; 3], fa: u32, fb: u32| {
            data.extend_from_slice(bytemuck::cast_slice(pos));        // 12 bytes
            data.extend_from_slice(&fa.to_le_bytes());                // 4 bytes
            data.extend_from_slice(&fb.to_le_bytes());                // 4 bytes = 20 total
            *count += 1;
        };

        for info in edge_faces.values() {
            let face_ids: Vec<u32> = info.faces.iter().copied().collect();
            let (fa, fb) = if face_ids.len() >= 2 {
                (face_ids[0], face_ids[1])
            } else {
                (face_ids[0], face_ids[0])
            };

            if info.faces.len() > 1 {
                emit_edge(&mut data, &mut vertex_count, &info.pa, fa, fb);
                emit_edge(&mut data, &mut vertex_count, &info.pb, fa, fb);
            } else if info.tri_count == 1 {
                emit_edge(&mut data, &mut vertex_count, &info.pa, fa, fb);
                emit_edge(&mut data, &mut vertex_count, &info.pb, fa, fb);
            }
        }

        if vertex_count == 0 {
            return (None, 0);
        }

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Edge Vertices"),
            contents: &data,
            usage: wgpu::BufferUsages::VERTEX,
        });

        (Some(buffer), vertex_count)
    }

    /// Recreate vertex/index buffers after mesh modification (extrude, etc.)
    pub fn rebuild_buffers(&mut self, device: &wgpu::Device, mesh: &Mesh) {
        let (vb, ib, count) = Self::create_buffers(device, mesh);
        self.vertex_buffer = vb;
        self.index_buffer = ib;
        self.num_indices = count;
        let (eb, ec) = Self::create_edge_buffer(device, mesh);
        self.edge_vertex_buffer = eb;
        self.num_edge_vertices = ec;
    }

    pub fn update_camera(&mut self, queue: &wgpu::Queue, camera: &Camera) {
        self.cached_view_proj = camera.uniform().view_proj;
        self.cached_eye = { let e = camera.eye(); [e.x, e.y, e.z, 0.0] };
        self.write_uniform(queue);
    }

    pub fn set_selected_face(&mut self, queue: &wgpu::Queue, face: Option<u32>) {
        self.selected_face = face;
        self.write_uniform(queue);
    }

    pub fn set_hovered_face(&mut self, queue: &wgpu::Queue, face: Option<u32>) {
        self.hovered_face = face;
        self.write_uniform(queue);
    }

    fn write_uniform(&self, queue: &wgpu::Queue) {
        let uniform = SceneUniform {
            view_proj: self.cached_view_proj,
            camera_eye: self.cached_eye,
            selected_face: self.selected_face.map(|f| f as i32).unwrap_or(-1),
            hovered_face: self.hovered_face.map(|f| f as i32).unwrap_or(-1),
            _pad: [0.0; 2],
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniform]));
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..self.num_indices, 0, 0..1);
    }

    pub fn draw_wireframe<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        if let Some(ref wf) = self.wireframe_pipeline {
            pass.set_pipeline(wf);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }
    }

    /// Draw dark edge lines on face boundaries only (SolidWorks-style).
    pub fn draw_edges<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        if let Some(ref eb) = self.edge_vertex_buffer {
            if self.num_edge_vertices > 0 {
                pass.set_pipeline(&self.edge_pipeline);
                pass.set_bind_group(0, &self.bind_group, &[]);
                pass.set_vertex_buffer(0, eb.slice(..));
                pass.draw(0..self.num_edge_vertices, 0..1);
            }
        }
    }

    pub fn has_wireframe(&self) -> bool {
        self.wireframe_pipeline.is_some()
    }
}
