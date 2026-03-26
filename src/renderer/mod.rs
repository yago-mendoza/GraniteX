// Renderer module — orchestrates all GPU rendering for GraniteX.
//
// Architecture:
//   GpuState  — wgpu device, surface, textures
//   Mesh      — dynamic geometry (vertices + indices), supports operations like extrude
//   Pipeline  — main mesh render pipeline (shader, buffers, uniforms)
//   Grid      — infinite XZ ground grid
//   Gizmo     — orientation indicator (bottom-left corner)
//   Picking   — CPU raycasting for face selection
//   Camera    — orbit/pan/zoom camera with view presets

mod gpu_state;
mod pipeline;
pub mod vertex;
mod camera;
mod grid;
mod gizmo;
pub mod mesh;
mod preview;
mod sketch_renderer;
pub(crate) mod picking;

use gpu_state::GpuState;
use pipeline::MeshPipeline;
use grid::GridPipeline;
use gizmo::GizmoPipeline;
use preview::PreviewPipeline;
use sketch_renderer::SketchRenderer;
use camera::Camera;
pub use mesh::Mesh;

use winit::dpi::PhysicalSize;
use winit::window::Window;

pub struct Renderer {
    pub gpu: GpuState,
    pub mesh_pipeline: MeshPipeline,
    grid: GridPipeline,
    gizmo: GizmoPipeline,
    preview: PreviewPipeline,
    sketch_renderer: SketchRenderer,
    camera: Camera,
    egui_renderer: egui_wgpu::Renderer,

    // Scene state
    pub mesh: Mesh,
    pub show_grid: bool,
    pub show_wireframe: bool,
    pub selected_face: Option<u32>,
}

impl Renderer {
    pub async fn new(window: &Window) -> Self {
        let gpu = GpuState::new(window).await;
        let camera = Camera::new(gpu.config.width as f32 / gpu.config.height as f32);
        let mesh = Mesh::cube();
        let mesh_pipeline = MeshPipeline::new(&gpu.device, &gpu.config, &camera, &mesh, gpu.features);
        let grid = GridPipeline::new(&gpu.device, &gpu.config, &camera);
        let gizmo = GizmoPipeline::new(&gpu.device, &gpu.config, &camera);
        let preview = PreviewPipeline::new(&gpu.device, &gpu.config, &camera);
        let sketch_renderer = SketchRenderer::new(&gpu.device, &gpu.config, &camera);

        let egui_renderer = egui_wgpu::Renderer::new(
            &gpu.device,
            gpu.config.format,
            None,
            1,
            false,
        );

        Self {
            gpu,
            mesh_pipeline,
            grid,
            gizmo,
            preview,
            sketch_renderer,
            camera,
            egui_renderer,
            mesh,
            show_grid: true,
            show_wireframe: false,
            selected_face: None,
        }
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.gpu.resize(new_size);
            self.camera.set_aspect(new_size.width as f32 / new_size.height as f32);
            self.sync_camera();
        }
    }

    // --- Camera controls ---

    fn sync_camera(&mut self) {
        self.mesh_pipeline.update_camera(&self.gpu.queue, &self.camera);
        self.grid.update_camera(&self.gpu.queue, &self.camera);
        self.gizmo.update_camera(&self.gpu.queue, &self.camera);
        self.preview.update_camera(&self.gpu.queue, &self.camera);
        self.sketch_renderer.update_camera(&self.gpu.queue, &self.camera);
    }

    pub fn orbit(&mut self, dx: f32, dy: f32) {
        self.camera.orbit(dx, dy);
        self.sync_camera();
    }

    pub fn pan(&mut self, dx: f32, dy: f32) {
        self.camera.pan(dx, dy);
        self.sync_camera();
    }

    pub fn zoom(&mut self, delta: f32) {
        self.camera.zoom(delta);
        self.sync_camera();
    }

    pub fn set_view(&mut self, yaw: f32, pitch: f32) {
        self.camera.set_view(yaw, pitch);
        self.sync_camera();
    }

    // --- Selection ---

    pub fn try_select_face(&mut self, screen_x: f32, screen_y: f32) {
        let view_proj = self.camera.projection_matrix() * self.camera.view_matrix();

        let result = picking::pick_face(
            screen_x,
            screen_y,
            self.gpu.config.width as f32,
            self.gpu.config.height as f32,
            view_proj,
            &self.mesh,
        );

        self.selected_face = result.map(|r| r.face_id);
        self.mesh_pipeline.set_selected_face(&self.gpu.queue, self.selected_face);
    }

    // --- Preview ---

    /// Update the extrude preview ghost (transparent blue).
    pub fn update_extrude_preview(&mut self, distance: f32) {
        if let Some(face_id) = self.selected_face {
            self.preview.set_extrude_preview(&self.gpu.device, &self.gpu.queue, &self.mesh, face_id, distance);
        } else {
            self.preview.clear();
        }
    }

    /// Update the cut preview ghost (transparent red).
    pub fn update_cut_preview(&mut self, depth: f32) {
        if let Some(face_id) = self.selected_face {
            self.preview.set_cut_preview(&self.gpu.device, &self.gpu.queue, &self.mesh, face_id, depth);
        } else {
            self.preview.clear();
        }
    }

    pub fn clear_preview(&mut self) {
        self.preview.clear();
    }

    // --- Sketch ---

    /// Update sketch rendering for a single sketch (appends to existing).
    pub fn update_sketch_multi(&mut self, sketch: &crate::sketch::Sketch, tool: crate::ui::Tool, is_active: bool) {
        let sketch_tool = match tool {
            crate::ui::Tool::Line => crate::ui::SketchTool::Line,
            crate::ui::Tool::Rect => crate::ui::SketchTool::Rect,
            crate::ui::Tool::Circle => crate::ui::SketchTool::Circle,
            _ => crate::ui::SketchTool::Line,
        };
        // Only show preview for the active sketch
        if is_active {
            self.sketch_renderer.update_sketch(&self.gpu.device, sketch, sketch_tool, self.camera.eye());
        } else {
            self.sketch_renderer.update_sketch_confirmed_only(&self.gpu.device, sketch, self.camera.eye());
        }
    }

    pub fn clear_sketch(&mut self) {
        self.sketch_renderer.clear();
    }

    pub fn view_proj(&self) -> glam::Mat4 {
        self.camera.projection_matrix() * self.camera.view_matrix()
    }

    pub fn gpu_width(&self) -> f32 {
        self.gpu.config.width as f32
    }

    pub fn gpu_height(&self) -> f32 {
        self.gpu.config.height as f32
    }


    /// Unproject screen coordinates to a ray (origin, direction).
    pub fn screen_to_ray(&self, screen_x: f32, screen_y: f32) -> (glam::Vec3, glam::Vec3) {
        let ndc_x = (screen_x / self.gpu.config.width as f32) * 2.0 - 1.0;
        let ndc_y = 1.0 - (screen_y / self.gpu.config.height as f32) * 2.0;

        let view_proj = self.camera.projection_matrix() * self.camera.view_matrix();
        let inv_vp = view_proj.inverse();

        let near_h = inv_vp * glam::Vec4::new(ndc_x, ndc_y, 0.0, 1.0);
        let far_h = inv_vp * glam::Vec4::new(ndc_x, ndc_y, 1.0, 1.0);

        let near = near_h.truncate() / near_h.w;
        let far = far_h.truncate() / far_h.w;

        (near, (far - near).normalize())
    }

    /// Compute face center for sketch plane creation.
    pub fn face_center(&self, face_id: u32) -> Option<glam::Vec3> {
        let positions: Vec<glam::Vec3> = self.mesh.vertices.iter()
            .filter(|v| v.face_id == face_id)
            .map(|v| glam::Vec3::from(v.position))
            .collect();

        if positions.is_empty() { return None; }
        Some(positions.iter().copied().sum::<glam::Vec3>() / positions.len() as f32)
    }

    // --- Mesh loading ---

    /// Replace the current mesh with a new one (for file import).
    pub fn load_mesh(&mut self, mesh: Mesh) {
        self.mesh = mesh;
        self.selected_face = None;
        self.mesh_pipeline.rebuild_buffers(&self.gpu.device, &self.mesh);
        self.mesh_pipeline.set_selected_face(&self.gpu.queue, None);
        self.fit_camera();
    }

    /// Auto-fit camera to current mesh bounding box.
    pub fn fit_camera(&mut self) {
        let (min, max) = self.mesh.bounding_box();
        self.camera.fit_to_bounds(min, max);
        self.sync_camera();
    }

    pub fn has_wireframe(&self) -> bool {
        self.mesh_pipeline.has_wireframe()
    }

    // --- Operations ---

    /// Extrude the currently selected face by `distance` along its normal.
    /// Returns the new cap face_id if successful.
    pub fn extrude_selected(&mut self, distance: f32) -> Option<u32> {
        let face_id = self.selected_face?;
        let new_face = self.mesh.extrude_face(face_id, distance)?;

        // Rebuild GPU buffers with new mesh data
        self.mesh_pipeline.rebuild_buffers(&self.gpu.device, &self.mesh);

        // Select the new cap face
        self.selected_face = Some(new_face);
        self.mesh_pipeline.set_selected_face(&self.gpu.queue, self.selected_face);

        Some(new_face)
    }

    /// Cut the currently selected face inward by `depth`.
    /// Returns the new floor face_id if successful.
    pub fn cut_selected(&mut self, depth: f32) -> Option<u32> {
        let face_id = self.selected_face?;
        let new_face = self.mesh.cut_face(face_id, depth)?;

        self.mesh_pipeline.rebuild_buffers(&self.gpu.device, &self.mesh);

        self.selected_face = Some(new_face);
        self.mesh_pipeline.set_selected_face(&self.gpu.queue, self.selected_face);

        Some(new_face)
    }

    // --- Render ---

    pub fn render(
        &mut self,
        egui_textures_delta: egui::TexturesDelta,
        egui_primitives: Vec<egui::ClippedPrimitive>,
        screen_descriptor: egui_wgpu::ScreenDescriptor,
    ) {
        let output = match self.gpu.surface.get_current_texture() {
            Ok(t) => t,
            Err(wgpu::SurfaceError::Lost) => {
                let size = PhysicalSize::new(self.gpu.config.width, self.gpu.config.height);
                self.gpu.resize(size);
                return;
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                log::error!("Out of GPU memory");
                return;
            }
            Err(e) => {
                log::warn!("Surface error: {:?}", e);
                return;
            }
        };

        let surface_view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let msaa_view = self.gpu.msaa_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let depth_view = self.gpu.depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Update egui textures
        for (id, image_delta) in &egui_textures_delta.set {
            self.egui_renderer.update_texture(&self.gpu.device, &self.gpu.queue, *id, image_delta);
        }

        let mut encoder = self.gpu.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor { label: Some("Render Encoder") },
        );

        // Pass 1: 3D scene (MSAA)
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("3D Scene"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &msaa_view,
                    resolve_target: Some(&surface_view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.1, g: 0.1, b: 0.12, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            if self.show_grid {
                self.grid.draw(&mut pass);
            }
            if self.show_wireframe {
                self.mesh_pipeline.draw_wireframe(&mut pass);
            } else {
                self.mesh_pipeline.draw(&mut pass);
            }
            self.preview.draw(&mut pass);
            self.sketch_renderer.draw(&mut pass);
            self.gizmo.draw(&mut pass, self.gpu.config.width, self.gpu.config.height);
        }

        // Pass 2: egui overlay (no MSAA)
        self.egui_renderer.update_buffers(
            &self.gpu.device, &self.gpu.queue, &mut encoder,
            &egui_primitives, &screen_descriptor,
        );

        {
            let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            let mut pass = pass.forget_lifetime();
            self.egui_renderer.render(&mut pass, &egui_primitives, &screen_descriptor);
        }

        self.gpu.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        for id in &egui_textures_delta.free {
            self.egui_renderer.free_texture(id);
        }
    }
}
