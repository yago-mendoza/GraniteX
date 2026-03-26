use anyhow::Result;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::ModifiersState;
use winit::window::{Window, WindowId};

use crate::commands::CommandHistory;
use crate::renderer::Renderer;
use crate::sketch::{Sketch, SketchPlane};
use crate::ui::{Tool, UiState, ViewPreset};

#[derive(Default)]
struct InputState {
    left_pressed: bool,
    middle_pressed: bool,
    left_was_drag: bool,
    last_mouse: Option<(f64, f64)>,
    cursor_pos: (f64, f64),
    modifiers: ModifiersState,
}

struct App {
    window: Option<Window>,
    renderer: Option<Renderer>,
    egui_ctx: egui::Context,
    egui_state: Option<egui_winit::State>,
    ui: UiState,
    input: InputState,
    /// Per-face sketches. Key = face_id. Geometry persists.
    sketches: std::collections::HashMap<u32, Sketch>,
    /// Which face we're currently drawing on (if any).
    active_sketch_face: Option<u32>,
    /// Undo/redo history for mesh operations.
    history: CommandHistory,
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
            sketches: std::collections::HashMap::new(),
            active_sketch_face: None,
            history: CommandHistory::new(),
        }
    }

    /// Returns true if a drawing tool is active.
    fn is_drawing_tool(&self) -> bool {
        matches!(self.ui.active_tool, Tool::Line | Tool::Rect | Tool::Circle)
    }

    /// Get or create a sketch for the given face.
    fn get_or_create_sketch(&mut self, face_id: u32) -> bool {
        let Some(renderer) = &self.renderer else { return false };
        if self.sketches.contains_key(&face_id) {
            self.active_sketch_face = Some(face_id);
            return true;
        }
        let Some(normal) = renderer.mesh.face_normal(face_id) else { return false };
        let Some(center) = renderer.face_center(face_id) else { return false };

        let plane = SketchPlane::from_face(normal, center);
        self.sketches.insert(face_id, Sketch::new(plane, face_id));
        self.active_sketch_face = Some(face_id);
        true
    }

    fn handle_draw_click(&mut self, screen_x: f32, screen_y: f32) {
        let Some(renderer) = &self.renderer else { return };

        // Always pick the face under cursor — auto-detect which face to draw on
        let view_proj = renderer.view_proj();
        let result = crate::renderer::picking::pick_face(
            screen_x, screen_y,
            renderer.gpu_width(), renderer.gpu_height(),
            view_proj,
            &renderer.mesh,
        );
        let Some(pick) = result else { return };
        let face_id = pick.face_id;

        // If clicking a different face, cancel pending on old face and switch
        if self.active_sketch_face != Some(face_id) {
            if let Some(old_face) = self.active_sketch_face {
                if let Some(old_sketch) = self.sketches.get_mut(&old_face) {
                    old_sketch.cancel_pending();
                }
            }
            self.get_or_create_sketch(face_id);
        }
        // Ensure sketch exists for this face
        if !self.sketches.contains_key(&face_id) {
            self.get_or_create_sketch(face_id);
        }
        self.active_sketch_face = Some(face_id);

        let Some(renderer) = &self.renderer else { return };
        let Some(sketch) = self.sketches.get_mut(&face_id) else { return };

        // Cast ray to sketch plane
        let (ray_o, ray_d) = renderer.screen_to_ray(screen_x, screen_y);
        let Some(hit) = sketch.plane.ray_intersect(ray_o, ray_d) else { return };
        let mut pos = sketch.world_to_2d(hit);
        sketch.cursor_2d = Some(pos); // ensure cursor is set on first click

        // Snap to existing endpoints
        if let Some(snapped) = sketch.snap_to_endpoint(pos, 0.05) {
            pos = snapped;
        }

        let mut contour_closed = false;

        match self.ui.active_tool {
            Tool::Line => {
                if let Some(start) = sketch.pending_start.take() {
                    sketch.add_line(start, pos);

                    // Check if contour is closed (pos snapped back to chain_start)
                    if let Some(chain_start) = sketch.chain_start {
                        if pos.distance_to(chain_start) < 0.01 {
                            contour_closed = true;
                            sketch.pending_start = None;
                            sketch.chain_start = None;
                        } else {
                            sketch.pending_start = Some(pos);
                        }
                    } else {
                        sketch.pending_start = Some(pos);
                    }
                } else {
                    sketch.pending_start = Some(pos);
                    sketch.chain_start = Some(pos);
                }
            }
            Tool::Rect => {
                if let Some(start) = sketch.pending_start.take() {
                    sketch.add_rect(start, pos);
                    contour_closed = true; // rect is always a closed contour
                } else {
                    sketch.pending_start = Some(pos);
                }
            }
            Tool::Circle => {
                if let Some(center) = sketch.pending_start.take() {
                    sketch.add_circle(center, center.distance_to(pos));
                    contour_closed = true; // circle is always closed
                } else {
                    sketch.pending_start = Some(pos);
                }
            }
            _ => {}
        }

        // Closed contour → convert to mesh face, auto-switch to Select
        if contour_closed {
            self.convert_contour_to_face(face_id);
            self.ui.active_tool = Tool::Select;
            self.active_sketch_face = None;
        }
    }

    /// Convert the last closed contour on a face into actual mesh geometry.
    fn convert_contour_to_face(&mut self, face_id: u32) {
        let Some(sketch) = self.sketches.get(&face_id) else { return };
        let Some(renderer) = &mut self.renderer else { return };

        self.history.save_state(&renderer.mesh);

        let entities = &sketch.entities;
        if entities.is_empty() { return; }

        let mut contour_points: Vec<crate::sketch::Point2D> = Vec::new();

        // Check last entity type for simple cases
        let last = &entities[entities.len() - 1];
        match last {
            crate::sketch::SketchEntity::Circle { center, radius } => {
                // Circle → polygon with 24 segments
                let segments = 48;
                for j in 0..segments {
                    let angle = std::f32::consts::TAU * j as f32 / segments as f32;
                    contour_points.push(crate::sketch::Point2D::new(
                        center.x + radius * angle.cos(),
                        center.y + radius * angle.sin(),
                    ));
                }
            }
            crate::sketch::SketchEntity::Line { .. } => {
                // Walk backward through lines to build a connected chain
                // Lines are added in order: line1.end = line2.start (chaining)
                // So walking forward through connected lines gives the contour.

                // Find where the last contour starts by walking backwards
                let mut start_idx = entities.len() - 1;
                while start_idx > 0 {
                    let prev = &entities[start_idx - 1];
                    let curr = &entities[start_idx];
                    if let (
                        crate::sketch::SketchEntity::Line { end: prev_end, .. },
                        crate::sketch::SketchEntity::Line { start: curr_start, .. }
                    ) = (prev, curr) {
                        if prev_end.distance_to(*curr_start) < 0.02 {
                            start_idx -= 1;
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }

                // Now walk forward from start_idx, collecting unique points
                for idx in start_idx..entities.len() {
                    if let crate::sketch::SketchEntity::Line { start, end } = &entities[idx] {
                        if contour_points.is_empty() || start.distance_to(*contour_points.last().unwrap()) > 0.01 {
                            contour_points.push(*start);
                        }
                        contour_points.push(*end);
                    }
                }
            }
        }

        if contour_points.len() < 3 { return; }

        // Remove duplicate closing point
        if contour_points.first().unwrap().distance_to(*contour_points.last().unwrap()) < 0.02 {
            contour_points.pop();
        }

        if contour_points.len() < 3 { return; }

        // Convert 2D contour points to 3D
        let points_3d: Vec<glam::Vec3> = contour_points.iter()
            .map(|p| sketch.to_3d(*p))
            .collect();

        let normal = sketch.plane.normal;

        // Add as a new face to the mesh using fan triangulation
        renderer.mesh.add_polygon_face(&points_3d, normal);
        renderer.mesh_pipeline.rebuild_buffers(&renderer.gpu.device, &renderer.mesh);
    }

    fn handle_input(&mut self, event: &WindowEvent, egui_consumed: bool) {
        match event {
            WindowEvent::ModifiersChanged(mods) => {
                self.input.modifiers = mods.state();
            }

            WindowEvent::MouseInput { state, button, .. } if !egui_consumed => {
                let pressed = *state == ElementState::Pressed;
                match button {
                    MouseButton::Left => {
                        if pressed {
                            self.input.left_pressed = true;
                            self.input.left_was_drag = false;
                        } else {
                            if !self.input.left_was_drag {
                                let sx = self.input.cursor_pos.0 as f32;
                                let sy = self.input.cursor_pos.1 as f32;

                                if self.is_drawing_tool() {
                                    self.handle_draw_click(sx, sy);
                                } else {
                                    if let Some(renderer) = &mut self.renderer {
                                        renderer.try_select_face(sx, sy);
                                    }
                                }
                            }
                            self.input.left_pressed = false;
                            self.input.left_was_drag = false;
                        }
                    }
                    MouseButton::Middle => {
                        self.input.middle_pressed = pressed;
                        if !pressed { self.input.last_mouse = None; }
                    }
                    MouseButton::Right => {
                        if !pressed {
                            // Right-click: cancel pending draw operation
                            if let Some(face_id) = self.active_sketch_face {
                                if let Some(sketch) = self.sketches.get_mut(&face_id) {
                                    if sketch.pending_start.is_some() {
                                        sketch.cancel_pending();
                                    } else {
                                        // No pending → deactivate this sketch face
                                        self.active_sketch_face = None;
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                let current = (position.x, position.y);
                self.input.cursor_pos = current;

                if !self.egui_ctx.wants_pointer_input() {
                    if let Some(last) = self.input.last_mouse {
                        let dx = (current.0 - last.0) as f32;
                        let dy = (current.1 - last.1) as f32;
                        if dx.abs() > 1.0 || dy.abs() > 1.0 {
                            if self.input.left_pressed { self.input.left_was_drag = true; }
                            if let Some(renderer) = &mut self.renderer {
                                if self.input.middle_pressed {
                                    if self.input.modifiers.control_key() {
                                        renderer.pan(dx, dy);
                                    } else {
                                        renderer.orbit(dx, dy);
                                    }
                                }
                            }
                        }
                    }
                }
                self.input.last_mouse = Some(current);
            }

            WindowEvent::MouseWheel { delta, .. } if !egui_consumed => {
                let scroll = match delta {
                    MouseScrollDelta::LineDelta(_, y) => *y,
                    MouseScrollDelta::PixelDelta(pos) => pos.y as f32 * 0.01,
                };
                if let Some(renderer) = &mut self.renderer { renderer.zoom(scroll); }
            }

            _ => {}
        }
    }

    fn handle_keyboard(&mut self, event: &WindowEvent) {
        if let WindowEvent::KeyboardInput { event: key_event, .. } = event {
            if key_event.state == ElementState::Pressed {
                use winit::keyboard::{Key, NamedKey};
                match &key_event.logical_key {
                    Key::Named(NamedKey::Escape) => {
                        if let Some(face_id) = self.active_sketch_face {
                            if let Some(sketch) = self.sketches.get_mut(&face_id) {
                                if sketch.pending_start.is_some() {
                                    sketch.cancel_pending();
                                } else {
                                    self.active_sketch_face = None;
                                    self.ui.active_tool = Tool::Select;
                                }
                            }
                        } else {
                            self.ui.active_tool = Tool::Select;
                        }
                    }
                    Key::Character(c) if c.as_str() == "z" && self.input.modifiers.control_key() => {
                        // Ctrl+Z: undo sketch entity first, then mesh operations
                        if let Some(face_id) = self.active_sketch_face {
                            if let Some(sketch) = self.sketches.get_mut(&face_id) {
                                if !sketch.entities.is_empty() {
                                    sketch.undo_last();
                                    return;
                                }
                            }
                        }
                        // Undo mesh operation
                        if let Some(renderer) = &mut self.renderer {
                            if self.history.undo(&mut renderer.mesh) {
                                renderer.mesh_pipeline.rebuild_buffers(&renderer.gpu.device, &renderer.mesh);
                            }
                        }
                    }
                    Key::Character(c) if c.as_str() == "y" && self.input.modifiers.control_key() => {
                        // Ctrl+Y: redo
                        if let Some(renderer) = &mut self.renderer {
                            if self.history.redo(&mut renderer.mesh) {
                                renderer.mesh_pipeline.rebuild_buffers(&renderer.gpu.device, &renderer.mesh);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn apply_ui_state(&mut self) {
        let Some(renderer) = &mut self.renderer else { return };
        renderer.show_grid = self.ui.show_grid;

        // Extrude (with undo)
        if let Some(distance) = self.ui.extrude_request.take() {
            self.history.save_state(&renderer.mesh);
            renderer.extrude_selected(distance);
            renderer.clear_preview();
        }

        // Extrude preview
        if self.ui.active_tool == Tool::Extrude && renderer.selected_face.is_some() {
            renderer.update_extrude_preview(self.ui.extrude_distance);
        } else {
            renderer.clear_preview();
        }

        // Update cursor on active sketch plane
        if let Some(face_id) = self.active_sketch_face {
            if let Some(sketch) = self.sketches.get_mut(&face_id) {
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
            }
        }

        // Count total sketch entities
        let total: usize = self.sketches.values().map(|s| s.entities.len()).sum();
        self.ui.sketch_entity_count = total;

        // Update sketch rendering — render ALL sketches
        renderer.clear_sketch();
        let tool = match self.ui.active_tool {
            Tool::Line => crate::ui::Tool::Line,
            Tool::Rect => crate::ui::Tool::Rect,
            Tool::Circle => crate::ui::Tool::Circle,
            _ => crate::ui::Tool::Select,
        };
        for (face_id, sketch) in &self.sketches {
            let is_active = self.active_sketch_face == Some(*face_id);
            renderer.update_sketch_multi(sketch, tool, is_active);
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
