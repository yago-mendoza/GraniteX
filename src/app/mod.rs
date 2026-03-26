mod input;
mod mesh_ops;
mod sketch_ops;

use anyhow::Result;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::ModifiersState;
use winit::window::{Window, WindowId};

use crate::commands::CommandHistory;
use crate::renderer::Renderer;
use crate::sketch::Sketch;
use crate::ui::{Tool, UiState, ViewPreset};

#[derive(Default)]
pub(super) struct InputState {
    pub(super) left_pressed: bool,
    pub(super) middle_pressed: bool,
    pub(super) left_was_drag: bool,
    pub(super) last_mouse: Option<(f64, f64)>,
    pub(super) cursor_pos: (f64, f64),
    pub(super) modifiers: ModifiersState,
}

pub(super) struct App {
    pub(super) window: Option<Window>,
    pub(super) renderer: Option<Renderer>,
    pub(super) egui_ctx: egui::Context,
    pub(super) egui_state: Option<egui_winit::State>,
    pub(super) ui: UiState,
    pub(super) input: InputState,
    pub(super) sketch: Option<Sketch>,
    pub(super) history: CommandHistory,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            renderer: None,
            egui_ctx: egui::Context::default(),
            egui_state: None,
            ui: UiState::new(),
            input: InputState::default(),
            sketch: None,
            history: CommandHistory::new(),
        }
    }

    fn apply_ui_state(&mut self) {
        // Handle import before borrowing renderer (avoids double-borrow)
        if self.ui.import_request {
            self.ui.import_request = false;
            self.open_file_dialog();
        }

        let Some(renderer) = &mut self.renderer else { return };
        renderer.show_grid = self.ui.show_grid;
        renderer.show_wireframe = self.ui.show_wireframe;
        self.ui.wireframe_supported = renderer.has_wireframe();

        // Extrude
        if let Some(distance) = self.ui.extrude_request.take() {
            self.history.save_state(&renderer.mesh);
            renderer.extrude_selected(distance);
            renderer.clear_preview();
        }

        // Cut
        if let Some(depth) = self.ui.cut_request.take() {
            self.history.save_state(&renderer.mesh);
            renderer.cut_selected(depth);
            renderer.clear_preview();
        }

        // Inset
        if let Some(amount) = self.ui.inset_request.take() {
            if let Some(face_id) = renderer.selected_face {
                self.history.save_state(&renderer.mesh);
                if let Some(inner) = renderer.mesh.inset_face(face_id, amount) {
                    renderer.selected_face = Some(inner);
                    renderer.mesh_pipeline.rebuild_buffers(&renderer.gpu.device, &renderer.mesh);
                    renderer.mesh_pipeline.set_selected_face(&renderer.gpu.queue, Some(inner));
                }
            }
        }

        // Preview (extrude or cut)
        if renderer.selected_face.is_some() {
            match self.ui.active_tool {
                Tool::Extrude => renderer.update_extrude_preview(self.ui.extrude_distance),
                Tool::Cut => renderer.update_cut_preview(self.ui.cut_depth),
                _ => renderer.clear_preview(),
            }
        } else {
            renderer.clear_preview();
        }

        // Update sketch cursor
        if let Some(sketch) = &mut self.sketch {
            let (ray_o, ray_d) = renderer.screen_to_ray(
                self.input.cursor_pos.0 as f32,
                self.input.cursor_pos.1 as f32,
            );
            if let Some(hit) = sketch.plane.ray_intersect(ray_o, ray_d) {
                let mut pos = sketch.world_to_2d(hit);
                if let Some(snapped) = sketch.snap_to_endpoint(pos, 0.05) {
                    pos = snapped;
                }
                sketch.cursor_2d = Some(pos);
            }
            self.ui.sketch_entity_count = sketch.entities.len();
        } else {
            self.ui.sketch_entity_count = 0;
        }

        // Render sketch
        renderer.clear_sketch();
        if let Some(sketch) = &self.sketch {
            let tool = match self.ui.active_tool {
                Tool::Line => crate::ui::Tool::Line,
                Tool::Rect => crate::ui::Tool::Rect,
                Tool::Circle => crate::ui::Tool::Circle,
                _ => crate::ui::Tool::Select,
            };
            renderer.update_sketch_multi(sketch, tool, true);
        }

        // View presets
        if let Some(preset) = self.ui.view_request.take() {
            use std::f32::consts::*;
            let (yaw, pitch) = match preset {
                ViewPreset::Front     => (0.0, 0.0),
                ViewPreset::Back      => (PI, 0.0),
                ViewPreset::Top       => (0.0, FRAC_PI_2 - 0.01),
                ViewPreset::Bottom    => (0.0, -(FRAC_PI_2 - 0.01)),
                ViewPreset::Right     => (FRAC_PI_2, 0.0),
                ViewPreset::Left      => (-FRAC_PI_2, 0.0),
                ViewPreset::Isometric => (FRAC_PI_4, FRAC_PI_6),
            };
            renderer.set_view(yaw, pitch);
        }

        // Mesh stats
        self.ui.mesh_faces = renderer.mesh.face_count();
        self.ui.mesh_verts = renderer.mesh.vertex_count();
        self.ui.mesh_tris = renderer.mesh.triangle_count();
    }

    fn open_file_dialog(&mut self) {
        let file = rfd::FileDialog::new()
            .add_filter("3D Models", crate::import::supported_extensions())
            .add_filter("STL Files", &["stl"])
            .add_filter("OBJ Files", &["obj"])
            .pick_file();

        if let Some(path) = file {
            match crate::import::load_file(&path) {
                Ok(mesh) => {
                    if let Some(r) = &mut self.renderer {
                        self.history.save_state(&r.mesh);
                        r.load_mesh(mesh);
                    }
                    log::info!("Imported: {}", path.display());
                }
                Err(e) => {
                    log::error!("Import failed: {}", e);
                }
            }
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() { return; }

        let attrs = Window::default_attributes()
            .with_title("GraniteX")
            .with_inner_size(winit::dpi::LogicalSize::new(1280, 720));

        let window = event_loop.create_window(attrs).expect("Failed to create window");
        let egui_state = egui_winit::State::new(
            self.egui_ctx.clone(), self.egui_ctx.viewport_id(),
            &window, None, None, None,
        );

        self.egui_ctx.set_visuals(egui::Visuals::dark());
        self.renderer = Some(pollster::block_on(Renderer::new(&window)));
        self.egui_state = Some(egui_state);
        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let egui_consumed = self.egui_state.as_mut()
            .map(|s| s.on_window_event(self.window.as_ref().unwrap(), &event).consumed)
            .unwrap_or(false);

        match &event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(r) = &mut self.renderer { r.resize(*size); }
            }
            WindowEvent::DroppedFile(path) => {
                match crate::import::load_file(path) {
                    Ok(mesh) => {
                        if let Some(r) = &mut self.renderer {
                            self.history.save_state(&r.mesh);
                            r.load_mesh(mesh);
                        }
                        log::info!("Imported (drag-and-drop): {}", path.display());
                    }
                    Err(e) => {
                        log::error!("Import failed: {}", e);
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                self.apply_ui_state();

                let window = self.window.as_ref().unwrap();
                let egui_state = self.egui_state.as_mut().unwrap();
                let raw_input = egui_state.take_egui_input(window);
                let full_output = self.egui_ctx.run(raw_input, |ctx| { self.ui.draw(ctx); });
                egui_state.handle_platform_output(window, full_output.platform_output);

                let primitives = self.egui_ctx.tessellate(full_output.shapes, full_output.pixels_per_point);
                let size = window.inner_size();
                let screen = egui_wgpu::ScreenDescriptor {
                    size_in_pixels: [size.width, size.height],
                    pixels_per_point: full_output.pixels_per_point,
                };

                if let Some(r) = &mut self.renderer {
                    r.render(full_output.textures_delta, primitives, screen);
                }
                window.request_redraw();
            }
            _ => {}
        }

        self.handle_keyboard(&event);
        self.handle_input(&event, egui_consumed);
    }
}

pub fn run() -> Result<()> {
    let event_loop = EventLoop::new()?;
    event_loop.run_app(&mut App::new())?;
    Ok(())
}
