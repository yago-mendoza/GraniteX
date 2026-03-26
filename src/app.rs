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
    /// Active sketch (only one at a time — on the face being drawn on).
    sketch: Option<Sketch>,
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
            sketch: None,
            history: CommandHistory::new(),
        }
    }

    fn is_drawing_tool(&self) -> bool {
        matches!(self.ui.active_tool, Tool::Line | Tool::Rect | Tool::Circle)
    }

    fn handle_draw_click(&mut self, screen_x: f32, screen_y: f32) {
        let Some(renderer) = &self.renderer else { return };

        // Pick the face under cursor
        let result = crate::renderer::picking::pick_face(
            screen_x, screen_y,
            renderer.gpu_width(), renderer.gpu_height(),
            renderer.view_proj(),
            &renderer.mesh,
        );
        let Some(pick) = result else { return };
        let face_id = pick.face_id;

        // If no sketch or sketch is on a different face, create new one
        let need_new = match &self.sketch {
            None => true,
            Some(s) => s.face_id != face_id,
        };
        if need_new {
            let Some(normal) = renderer.mesh.face_normal(face_id) else { return };
            let Some(center) = renderer.face_center(face_id) else { return };
            let plane = SketchPlane::from_face(normal, center);
            self.sketch = Some(Sketch::new(plane, face_id));
        }

        let Some(renderer) = &self.renderer else { return };
        let Some(sketch) = &mut self.sketch else { return };

        // Project click onto sketch plane
        let (ray_o, ray_d) = renderer.screen_to_ray(screen_x, screen_y);
        let Some(hit) = sketch.plane.ray_intersect(ray_o, ray_d) else { return };
        let mut pos = sketch.world_to_2d(hit);
        sketch.cursor_2d = Some(pos);

        // Snap to existing endpoints
        if let Some(snapped) = sketch.snap_to_endpoint(pos, 0.05) {
            pos = snapped;
        }

        let mut contour_closed = false;

        match self.ui.active_tool {
            Tool::Line => {
                if let Some(start) = sketch.pending_start.take() {
                    sketch.add_line(start, pos);
                    // Check if contour closed
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
                    contour_closed = true;
                } else {
                    sketch.pending_start = Some(pos);
                }
            }
            Tool::Circle => {
                if let Some(center) = sketch.pending_start.take() {
                    sketch.add_circle(center, center.distance_to(pos));
                    contour_closed = true;
                } else {
                    sketch.pending_start = Some(pos);
                }
            }
            _ => {}
        }

        if contour_closed {
            self.convert_contour_to_face();
            self.sketch = None; // clear sketch after conversion
            self.ui.active_tool = Tool::Select;
        }
    }

    fn convert_contour_to_face(&mut self) {
        let Some(sketch) = &self.sketch else { return };
        let Some(renderer) = &mut self.renderer else { return };

        self.history.save_state(&renderer.mesh);

        let entities = &sketch.entities;
        if entities.is_empty() { return; }

        let mut contour_points: Vec<crate::sketch::Point2D> = Vec::new();

        let last = &entities[entities.len() - 1];
        match last {
            crate::sketch::SketchEntity::Circle { center, radius } => {
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
                // Walk backward to find chain start
                let mut start_idx = entities.len() - 1;
                while start_idx > 0 {
                    if let (
                        crate::sketch::SketchEntity::Line { end: prev_end, .. },
                        crate::sketch::SketchEntity::Line { start: curr_start, .. }
                    ) = (&entities[start_idx - 1], &entities[start_idx]) {
                        if prev_end.distance_to(*curr_start) < 0.02 {
                            start_idx -= 1;
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }

                // Walk forward collecting points
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

        let points_3d: Vec<glam::Vec3> = contour_points.iter()
            .map(|p| sketch.to_3d(*p))
            .collect();
        let normal = sketch.plane.normal;

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
                                    if let Some(r) = &mut self.renderer {
                                        r.try_select_face(sx, sy);
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
                            if let Some(sketch) = &mut self.sketch {
                                if sketch.pending_start.is_some() {
                                    sketch.cancel_pending();
                                } else {
                                    self.sketch = None;
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
                            if let Some(r) = &mut self.renderer {
                                if self.input.middle_pressed {
                                    if self.input.modifiers.control_key() {
                                        r.pan(dx, dy);
                                    } else {
                                        r.orbit(dx, dy);
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
                if let Some(r) = &mut self.renderer { r.zoom(scroll); }
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
                        if let Some(sketch) = &mut self.sketch {
                            if sketch.pending_start.is_some() {
                                sketch.cancel_pending();
                            } else {
                                self.sketch = None;
                            }
                        }
                        self.ui.active_tool = Tool::Select;
                    }
                    Key::Character(c) if c.as_str() == "z" && self.input.modifiers.control_key() => {
                        // Undo sketch entity first, then mesh
                        if let Some(sketch) = &mut self.sketch {
                            if !sketch.entities.is_empty() {
                                sketch.undo_last();
                                return;
                            }
                        }
                        if let Some(r) = &mut self.renderer {
                            if self.history.undo(&mut r.mesh) {
                                r.mesh_pipeline.rebuild_buffers(&r.gpu.device, &r.mesh);
                            }
                        }
                    }
                    Key::Character(c) if c.as_str() == "y" && self.input.modifiers.control_key() => {
                        if let Some(r) = &mut self.renderer {
                            if self.history.redo(&mut r.mesh) {
                                r.mesh_pipeline.rebuild_buffers(&r.gpu.device, &r.mesh);
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

        // Extrude
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
