// Preview renderer — shows a transparent blue ghost of pending operations.
// Used for extrude preview, cut preview, etc.

use glam::Vec3;
use wgpu::util::DeviceExt;

use super::camera::Camera;
use super::gpu_state::MSAA_SAMPLE_COUNT;
use super::mesh::Mesh;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct PreviewVertex {
    position: [f32; 3],
    _pad: f32,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct PreviewUniform {
    view_proj: [[f32; 4]; 4],
    color: [f32; 4],
}

const EXTRUDE_COLOR: [f32; 4] = [0.3, 0.5, 0.9, 0.45];
const CUT_COLOR: [f32; 4] = [0.9, 0.25, 0.2, 0.45];

pub struct PreviewPipeline {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    vertex_buffer: Option<wgpu::Buffer>,
    num_vertices: u32,
    cached_view_proj: [[f32; 4]; 4],
    cached_color: [f32; 4],
}

impl PreviewPipeline {
    pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration, camera: &Camera) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Preview Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("preview.wgsl").into()),
        });

        let uniform = PreviewUniform {
            view_proj: camera.uniform().view_proj,
            color: EXTRUDE_COLOR,
        };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Preview Uniform"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Preview BGL"),
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
            label: Some("Preview BG"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Preview Layout"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });

        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<PreviewVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x3,
            }],
        };

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Preview Pipeline"),
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
                depth_write_enabled: false, // don't write depth — ghost is transparent
                depth_compare: wgpu::CompareFunction::LessEqual,
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
            uniform_buffer,
            bind_group,
            vertex_buffer: None,
            num_vertices: 0,
            cached_view_proj: uniform.view_proj,
            cached_color: EXTRUDE_COLOR,
        }
    }

    pub fn update_camera(&mut self, queue: &wgpu::Queue, camera: &Camera) {
        self.cached_view_proj = camera.uniform().view_proj;
        let uniform = PreviewUniform { view_proj: self.cached_view_proj, color: self.cached_color };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniform]));
    }

    fn write_color(&mut self, queue: &wgpu::Queue, color: [f32; 4]) {
        self.cached_color = color;
        let uniform = PreviewUniform { view_proj: self.cached_view_proj, color };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniform]));
    }

    /// Generate preview geometry for an extrude operation.
    pub fn set_extrude_preview(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        mesh: &Mesh,
        face_id: u32,
        distance: f32,
    ) {
        let Some(normal) = mesh.face_normal(face_id) else { return };
        let offset = normal * distance;

        // Get the face boundary corners (works for merged faces)
        let Some(corners) = mesh.face_boundary_corners(face_id) else {
            self.clear();
            return;
        };

        // Offset base face slightly along normal to prevent z-fighting
        let z_bias = normal * 0.001;
        let face_positions: Vec<Vec3> = corners.iter().map(|p| *p + z_bias).collect();
        let new_positions: Vec<Vec3> = corners.iter().map(|p| *p + offset).collect();

        let mut verts = Vec::new();
        let pv = |p: Vec3| PreviewVertex { position: p.into(), _pad: 0.0 };

        // Cap face (ear-clipping for concave polygon support)
        let n = new_positions.len();
        let cap_tris = super::mesh::triangulate_3d_polygon(&new_positions, normal);
        for tri in &cap_tris {
            verts.push(pv(new_positions[tri[0]]));
            verts.push(pv(new_positions[tri[1]]));
            verts.push(pv(new_positions[tri[2]]));
        }

        // N side faces
        for i in 0..n {
            let j = (i + 1) % n;
            let b0 = face_positions[i];
            let b1 = face_positions[j];
            let t0 = new_positions[i];
            let t1 = new_positions[j];

            verts.push(pv(b0));
            verts.push(pv(b1));
            verts.push(pv(t1));
            verts.push(pv(b0));
            verts.push(pv(t1));
            verts.push(pv(t0));
        }

        self.write_color(queue, EXTRUDE_COLOR);
        self.num_vertices = verts.len() as u32;
        self.vertex_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Preview Vertices"),
            contents: bytemuck::cast_slice(&verts),
            usage: wgpu::BufferUsages::VERTEX,
        }));
    }

    /// Generate preview geometry for a cut operation (inward, red ghost).
    pub fn set_cut_preview(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        mesh: &Mesh,
        face_id: u32,
        depth: f32,
    ) {
        let Some(normal) = mesh.face_normal(face_id) else { return };
        let offset = normal * (-depth); // inward

        let Some(corners) = mesh.face_boundary_corners(face_id) else {
            self.clear();
            return;
        };

        // Offset top face slightly inward to prevent z-fighting
        let z_bias = normal * (-0.001);
        let face_positions: Vec<Vec3> = corners.iter().map(|p| *p + z_bias).collect();
        let new_positions: Vec<Vec3> = corners.iter().map(|p| *p + offset).collect();

        let mut verts = Vec::new();
        let pv = |p: Vec3| PreviewVertex { position: p.into(), _pad: 0.0 };

        // Floor face (ear-clipping for concave support)
        let n = new_positions.len();
        let floor_tris = super::mesh::triangulate_3d_polygon(&new_positions, normal);
        for tri in &floor_tris {
            verts.push(pv(new_positions[tri[0]]));
            verts.push(pv(new_positions[tri[1]]));
            verts.push(pv(new_positions[tri[2]]));
        }

        // N side wall faces
        for i in 0..n {
            let j = (i + 1) % n;
            let t0 = face_positions[i];
            let t1 = face_positions[j];
            let b0 = new_positions[i];
            let b1 = new_positions[j];

            verts.push(pv(t0));
            verts.push(pv(t1));
            verts.push(pv(b1));
            verts.push(pv(t0));
            verts.push(pv(b1));
            verts.push(pv(b0));
        }

        self.write_color(queue, CUT_COLOR);
        self.num_vertices = verts.len() as u32;
        self.vertex_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cut Preview Vertices"),
            contents: bytemuck::cast_slice(&verts),
            usage: wgpu::BufferUsages::VERTEX,
        }));
    }

    /// Generate preview geometry for an inset operation (teal ghost).
    /// Shows the inner face and connecting quads.
    pub fn set_inset_preview(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        mesh: &Mesh,
        face_id: u32,
        amount: f32,
    ) {
        let Some(normal) = mesh.face_normal(face_id) else { return };
        let Some(corners) = mesh.face_boundary_corners(face_id) else {
            self.clear();
            return;
        };

        let n = corners.len();
        if n < 3 { self.clear(); return; }

        let center: Vec3 = corners.iter().copied().sum::<Vec3>() / n as f32;
        let inner: Vec<Vec3> = corners.iter().map(|c| {
            let dir = (center - *c).normalize_or_zero();
            *c + dir * amount
        }).collect();

        let mut verts = Vec::new();
        let pv = |p: Vec3| PreviewVertex { position: (p + normal * 0.01).into(), _pad: 0.0 };

        // Inner face (ear-clipping for concave support)
        let inner_tris = super::mesh::triangulate_3d_polygon(&inner, normal);
        for tri in &inner_tris {
            verts.push(pv(inner[tri[0]]));
            verts.push(pv(inner[tri[1]]));
            verts.push(pv(inner[tri[2]]));
        }

        // Connecting quads between outer and inner
        for i in 0..n {
            let j = (i + 1) % n;
            verts.push(pv(corners[i]));
            verts.push(pv(corners[j]));
            verts.push(pv(inner[j]));
            verts.push(pv(corners[i]));
            verts.push(pv(inner[j]));
            verts.push(pv(inner[i]));
        }

        const INSET_COLOR: [f32; 4] = [0.2, 0.7, 0.7, 0.3];
        self.write_color(queue, INSET_COLOR);
        self.num_vertices = verts.len() as u32;
        self.vertex_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Inset Preview Vertices"),
            contents: bytemuck::cast_slice(&verts),
            usage: wgpu::BufferUsages::VERTEX,
        }));
    }

    /// Generate extrude preview directly from points (no mesh face needed).
    /// Used for sketch regions before the face is actually created.
    pub fn set_extrude_preview_from_points(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        face_positions: &[Vec3],
        normal: Vec3,
        distance: f32,
    ) {
        let n = face_positions.len();
        if n < 3 { self.clear(); return; }

        let offset = normal * distance;
        let new_positions: Vec<Vec3> = face_positions.iter().map(|p| *p + offset).collect();

        let mut verts = Vec::new();
        let pv = |p: Vec3| PreviewVertex { position: p.into(), _pad: 0.0 };

        // Cap face
        let cap_tris = super::mesh::triangulate_3d_polygon(&new_positions, normal);
        for tri in &cap_tris {
            verts.push(pv(new_positions[tri[0]]));
            verts.push(pv(new_positions[tri[1]]));
            verts.push(pv(new_positions[tri[2]]));
        }

        // Side faces
        for i in 0..n {
            let j = (i + 1) % n;
            verts.push(pv(face_positions[i]));
            verts.push(pv(face_positions[j]));
            verts.push(pv(new_positions[j]));
            verts.push(pv(face_positions[i]));
            verts.push(pv(new_positions[j]));
            verts.push(pv(new_positions[i]));
        }

        self.write_color(queue, EXTRUDE_COLOR);
        self.num_vertices = verts.len() as u32;
        self.vertex_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Extrude Preview (points)"),
            contents: bytemuck::cast_slice(&verts),
            usage: wgpu::BufferUsages::VERTEX,
        }));
    }

    /// Generate cut preview directly from points.
    pub fn set_cut_preview_from_points(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        face_positions: &[Vec3],
        normal: Vec3,
        depth: f32,
    ) {
        let n = face_positions.len();
        if n < 3 { self.clear(); return; }

        let offset = normal * (-depth);
        let new_positions: Vec<Vec3> = face_positions.iter().map(|p| *p + offset).collect();

        let mut verts = Vec::new();
        let pv = |p: Vec3| PreviewVertex { position: p.into(), _pad: 0.0 };

        // Floor face
        let floor_tris = super::mesh::triangulate_3d_polygon(&new_positions, normal);
        for tri in &floor_tris {
            verts.push(pv(new_positions[tri[0]]));
            verts.push(pv(new_positions[tri[1]]));
            verts.push(pv(new_positions[tri[2]]));
        }

        // Side walls
        for i in 0..n {
            let j = (i + 1) % n;
            verts.push(pv(face_positions[i]));
            verts.push(pv(face_positions[j]));
            verts.push(pv(new_positions[j]));
            verts.push(pv(face_positions[i]));
            verts.push(pv(new_positions[j]));
            verts.push(pv(new_positions[i]));
        }

        self.write_color(queue, CUT_COLOR);
        self.num_vertices = verts.len() as u32;
        self.vertex_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cut Preview (points)"),
            contents: bytemuck::cast_slice(&verts),
            usage: wgpu::BufferUsages::VERTEX,
        }));
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
