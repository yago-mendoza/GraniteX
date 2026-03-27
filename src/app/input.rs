use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

use crate::ui::{Tool, SelectionMode};
use super::App;

impl App {
    pub(super) fn handle_input(&mut self, event: &WindowEvent, egui_consumed: bool) {
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
                            self.input.left_press_pos = Some(self.input.cursor_pos);

                            // Start drag-to-extrude/cut if tool is active + face selected
                            let is_op_tool = matches!(self.ui.active_tool, Tool::Extrude | Tool::Cut);
                            let has_selection = self.renderer.as_ref()
                                .map(|r| r.selected_face.is_some())
                                .unwrap_or(false);
                            let has_region = self.sketch.as_ref()
                                .map(|s| s.selected_region.is_some())
                                .unwrap_or(false);

                            if is_op_tool && (has_selection || has_region) {
                                self.input.operation_dragging = true;
                                self.input.drag_start_y = self.input.cursor_pos.1;
                                self.input.drag_accumulated = 0.0;
                            }
                        } else {
                            // End drag-to-extrude: apply the operation
                            if self.input.operation_dragging {
                                let dist = self.input.drag_accumulated;
                                if dist.abs() > 0.01 {
                                    match self.ui.active_tool {
                                        Tool::Extrude => {
                                            self.ui.extrude_distance = dist;
                                            self.ui.extrude_request = Some(dist);
                                        }
                                        Tool::Cut => {
                                            let depth = dist.max(0.0);
                                            self.ui.cut_depth = depth;
                                            self.ui.cut_request = Some(depth);
                                        }
                                        _ => {}
                                    }
                                }
                                self.input.operation_dragging = false;
                                self.input.left_pressed = false;
                                self.input.left_was_drag = false;
                                return;
                            }

                            // Normal click handling
                            if let Some(press) = self.input.left_press_pos {
                                let dx = self.input.cursor_pos.0 - press.0;
                                let dy = self.input.cursor_pos.1 - press.1;
                                if dx * dx + dy * dy > 25.0 {
                                    self.input.left_was_drag = true;
                                }
                            }
                            if !self.input.left_was_drag {
                                let sx = self.input.cursor_pos.0 as f32;
                                let sy = self.input.cursor_pos.1 as f32;
                                if self.ui.active_tool == Tool::Measure {
                                    self.handle_measure_click(sx, sy);
                                } else if self.is_drawing_tool() {
                                    self.handle_draw_click(sx, sy);
                                } else if self.ui.selection_mode == SelectionMode::Edge {
                                    self.handle_edge_click(sx, sy);
                                } else if !self.try_select_region(sx, sy) {
                                    // Try construction geometry first, then mesh faces
                                    if !self.try_select_construction(sx, sy) {
                                        let shift = self.input.modifiers.shift_key();
                                        if let Some(r) = &mut self.renderer {
                                            r.try_select_face_multi(sx, sy, shift);
                                            // Clear construction selection when a mesh face is selected
                                            if r.selected_face.is_some() {
                                                self.ui.construction_selected = None;
                                            }
                                        }
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
                            // Cancel drag-to-extrude/cut on right-click
                            if self.input.operation_dragging {
                                self.input.operation_dragging = false;
                                self.input.drag_accumulated = 0.0;
                                self.input.left_pressed = false;
                                self.input.left_was_drag = false;
                                // Restore slider to original value (0 = no operation)
                                match self.ui.active_tool {
                                    Tool::Extrude => self.ui.extrude_distance = 0.0,
                                    Tool::Cut => self.ui.cut_depth = 0.0,
                                    _ => {}
                                }
                            } else if self.is_drawing_tool() {
                                // In drawing mode: right-click cancels
                                if let Some(sketch) = &mut self.sketch {
                                    if sketch.pending_start.is_some() {
                                        sketch.cancel_pending();
                                    } else {
                                        self.sketch = None;
                                    }
                                }
                            } else {
                                // Not drawing: show context menu on face
                                let sx = self.input.cursor_pos.0 as f32;
                                let sy = self.input.cursor_pos.1 as f32;
                                if let Some(r) = &mut self.renderer {
                                    // Select the face under cursor first
                                    r.try_select_face(sx, sy);
                                    if let Some(fid) = r.selected_face {
                                        self.ui.context_menu_pos = Some(egui::pos2(sx, sy));
                                        self.ui.context_menu_face = Some(fid);
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
                self.input.cursor_moved = true;

                // Drag-to-extrude: update distance from vertical mouse movement
                if self.input.operation_dragging {
                    let dy = (self.input.drag_start_y - current.1) as f32; // up = positive
                    // Scale by camera distance for consistent feel
                    let scale = self.renderer.as_ref()
                        .map(|r| r.camera_distance() * 0.003)
                        .unwrap_or(0.01);
                    self.input.drag_accumulated = dy * scale;
                    // Update UI slider to match drag
                    match self.ui.active_tool {
                        Tool::Extrude => self.ui.extrude_distance = self.input.drag_accumulated,
                        Tool::Cut => self.ui.cut_depth = self.input.drag_accumulated.max(0.0),
                        _ => {}
                    }
                }

                if !self.egui_ctx.wants_pointer_input() {
                    if let Some(last) = self.input.last_mouse {
                        let dx = (current.0 - last.0) as f32;
                        let dy = (current.1 - last.1) as f32;
                        if dx.abs() > 0.5 || dy.abs() > 0.5 {
                            if let Some(r) = &mut self.renderer {
                                if self.input.middle_pressed {
                                    if self.input.modifiers.control_key() {
                                        // Ctrl + Middle = Pan (SolidWorks)
                                        r.pan(dx, dy);
                                    } else if self.input.modifiers.shift_key() {
                                        // Shift + Middle = Zoom (SolidWorks)
                                        r.zoom(dy * 0.05);
                                    } else {
                                        // Middle = Orbit (SolidWorks)
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
                if let Some(r) = &mut self.renderer {
                    // SolidWorks-style: zoom toward cursor position
                    let sx = self.input.cursor_pos.0 as f32;
                    let sy = self.input.cursor_pos.1 as f32;
                    r.zoom_toward_screen(scroll, sx, sy);
                }
            }

            _ => {}
        }
    }

    pub(super) fn handle_keyboard(&mut self, event: &WindowEvent) {
        // Don't process shortcuts if egui wants keyboard input (typing in chat, etc.)
        if self.egui_ctx.wants_keyboard_input() {
            return;
        }

        if let WindowEvent::KeyboardInput { event: key_event, .. } = event {
            if key_event.state == ElementState::Pressed {
                use winit::keyboard::{Key, NamedKey};

                // Block tool shortcuts during drag-to-extrude/cut (except Escape)
                if self.input.operation_dragging {
                    if !matches!(&key_event.logical_key, Key::Named(NamedKey::Escape)) {
                        return;
                    }
                }

                let ctrl = self.input.modifiers.control_key();

                match &key_event.logical_key {
                    // --- Global shortcuts ---
                    Key::Named(NamedKey::Escape) => {
                        if let Some(sketch) = &mut self.sketch {
                            if sketch.pending_start.is_some() {
                                sketch.cancel_pending();
                            } else {
                                self.sketch = None;
                            }
                        }
                        // Clear all active state
                        self.ui.measure_first_point = None;
                        self.ui.active_measurement = None;
                        self.ui.selected_edge = None;
                        self.input.operation_dragging = false;
                        self.ui.active_tool = Tool::Select;
                    }

                    // Ctrl+Z = Undo
                    // Priority: sketch undo only if actively drawing, else mesh undo
                    Key::Character(c) if c.as_str() == "z" && ctrl => {
                        if self.is_drawing_tool() {
                            if let Some(sketch) = &mut self.sketch {
                                if !sketch.entities.is_empty() {
                                    sketch.undo_last();
                                    return;
                                }
                            }
                        }
                        if let Some(r) = &mut self.renderer {
                            if self.history.undo(&mut r.mesh) {
                                r.mesh_pipeline.rebuild_buffers(&r.gpu.device, &r.mesh);
                                // Invalidate sketch if its parent face no longer exists
                                if let Some(sketch) = &self.sketch {
                                    if let Some(fid) = sketch.face_id {
                                        if r.mesh.face_normal(fid).is_none() {
                                            self.sketch = None;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Ctrl+Y or Ctrl+Shift+Z = Redo
                    Key::Character(c) if c.as_str() == "y" && ctrl => {
                        if let Some(r) = &mut self.renderer {
                            if self.history.redo(&mut r.mesh) {
                                r.mesh_pipeline.rebuild_buffers(&r.gpu.device, &r.mesh);
                                if let Some(sketch) = &self.sketch {
                                    if let Some(fid) = sketch.face_id {
                                        if r.mesh.face_normal(fid).is_none() {
                                            self.sketch = None;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Key::Character(c) if c.as_str() == "Z" && ctrl && self.input.modifiers.shift_key() => {
                        if let Some(r) = &mut self.renderer {
                            if self.history.redo(&mut r.mesh) {
                                r.mesh_pipeline.rebuild_buffers(&r.gpu.device, &r.mesh);
                                if let Some(sketch) = &self.sketch {
                                    if let Some(fid) = sketch.face_id {
                                        if r.mesh.face_normal(fid).is_none() {
                                            self.sketch = None;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Ctrl+N = New scene
                    Key::Character(c) if c.as_str() == "n" && ctrl => {
                        self.ui.new_scene_request = true;
                    }

                    // Ctrl+S = Save, Ctrl+Shift+S = Save As
                    Key::Character(c) if (c.as_str() == "s" || c.as_str() == "S") && ctrl => {
                        if self.input.modifiers.shift_key() {
                            self.ui.save_as_request = true;
                        } else {
                            self.ui.save_request = true;
                        }
                    }

                    // Ctrl+O = Open project
                    Key::Character(c) if c.as_str() == "o" && ctrl => {
                        self.ui.open_project_request = true;
                    }

                    // Ctrl+E = Export STL
                    Key::Character(c) if c.as_str() == "e" && ctrl => {
                        self.ui.export_stl_request = true;
                    }

                    // Delete = Delete selected sketch entity, or selected face
                    Key::Named(NamedKey::Delete) => {
                        let mut handled = false;
                        if let Some(sketch) = &mut self.sketch {
                            if sketch.selected_entity.is_some() {
                                sketch.delete_selected_entity();
                                handled = true;
                            }
                        }
                        if !handled {
                            if self.ui.selection_mode == SelectionMode::Edge {
                                self.ui.toasts.push(crate::ui::Toast::new("Edge deletion not supported".into()));
                            } else {
                                self.delete_selected_face();
                            }
                        }
                    }

                    // Tab = Toggle face/edge selection mode
                    Key::Named(NamedKey::Tab) => {
                        self.ui.selection_mode = match self.ui.selection_mode {
                            SelectionMode::Face => SelectionMode::Edge,
                            SelectionMode::Edge => SelectionMode::Face,
                        };
                    }

                    // Home = Zoom to fit (SolidWorks: F key)
                    Key::Named(NamedKey::Home) => {
                        if let Some(r) = &mut self.renderer {
                            r.fit_camera();
                        }
                    }

                    // --- Tool shortcuts (SolidWorks-style) ---
                    Key::Character(c) if !ctrl => {
                        let prev_tool = self.ui.active_tool;
                        match c.as_str() {
                            "s" => self.ui.active_tool = Tool::Select,
                            "l" => self.ui.active_tool = Tool::Line,
                            "r" => self.ui.active_tool = Tool::Rect,
                            "c" => self.ui.active_tool = Tool::Circle,
                            "e" => self.ui.active_tool = Tool::Extrude,
                            "x" => self.ui.active_tool = Tool::Cut,
                            "i" => self.ui.active_tool = Tool::Inset,
                            "f" => {
                                self.ui.toasts.push(crate::ui::Toast::new("Fillet — not yet implemented".into()));
                            }
                            "m" => {
                                self.ui.active_tool = Tool::Measure;
                                self.ui.measure_first_point = None;
                                self.ui.active_measurement = None;
                            }
                            "w" => self.ui.show_wireframe = !self.ui.show_wireframe,
                            "g" => self.ui.show_grid = !self.ui.show_grid,
                            _ => {}
                        }
                        // Clean up state when switching away from tools
                        if prev_tool != self.ui.active_tool {
                            // Cancel any drag-to-extrude/cut in progress
                            self.input.operation_dragging = false;
                            self.input.drag_accumulated = 0.0;
                            // Clear sketch pending state when leaving a drawing tool
                            let was_drawing = matches!(prev_tool, Tool::Line | Tool::Rect | Tool::Circle | Tool::CLine);
                            let now_drawing = matches!(self.ui.active_tool, Tool::Line | Tool::Rect | Tool::Circle | Tool::CLine);
                            if was_drawing && !now_drawing {
                                if let Some(sketch) = &mut self.sketch {
                                    sketch.cancel_pending();
                                }
                            }
                            if prev_tool == Tool::Measure {
                                self.ui.measure_first_point = None;
                            }
                        }
                    }

                    _ => {}
                }

                // Numpad view shortcuts (match physical keys for numpad)
                let ctrl = self.input.modifiers.control_key();
                match key_event.physical_key {
                    PhysicalKey::Code(KeyCode::Numpad1) => {
                        self.ui.view_request = Some(if ctrl { crate::ui::ViewPreset::Back } else { crate::ui::ViewPreset::Front });
                    }
                    PhysicalKey::Code(KeyCode::Numpad3) => {
                        self.ui.view_request = Some(if ctrl { crate::ui::ViewPreset::Left } else { crate::ui::ViewPreset::Right });
                    }
                    PhysicalKey::Code(KeyCode::Numpad7) => {
                        self.ui.view_request = Some(if ctrl { crate::ui::ViewPreset::Bottom } else { crate::ui::ViewPreset::Top });
                    }
                    PhysicalKey::Code(KeyCode::Numpad0) => {
                        self.ui.view_request = Some(crate::ui::ViewPreset::Isometric);
                    }
                    _ => {}
                }
            }
        }
    }

    /// Try to select construction geometry (plane or axis) at screen coords.
    /// Returns true if something was selected.
    /// BUG 7 fix: Only select construction if it's closer than any mesh face.
    fn try_select_construction(&mut self, screen_x: f32, screen_y: f32) -> bool {
        let Some(renderer) = &self.renderer else { return false };
        let (ray_o, ray_d) = renderer.screen_to_ray(screen_x, screen_y);
        let extent = (renderer.camera_distance() * 0.6).clamp(0.5, 10.0);

        let construction_hit = self.construction.pick(ray_o, ray_d, extent);

        if let Some((id, cg_dist)) = construction_hit {
            // Check if a mesh face is closer — if so, let face picking win
            let view_proj = renderer.view_proj();
            let mesh_hit = crate::renderer::picking::pick_face(
                screen_x, screen_y,
                renderer.gpu_width(), renderer.gpu_height(),
                view_proj, &renderer.mesh,
            );
            if let Some(face_hit) = mesh_hit {
                if face_hit.distance < cg_dist {
                    // Mesh is closer — don't select construction
                    self.ui.construction_selected = None;
                    return false;
                }
            }

            self.ui.construction_selected = Some(id);
            if let Some(r) = &mut self.renderer {
                r.selected_face = None;
                r.mesh_pipeline.set_selected_face(&r.gpu.queue, None);
            }
            true
        } else {
            // BUG 2 fix: Don't clear construction_selected here.
            // It gets cleared when the user clicks a mesh face (via face selection),
            // or when they click the feature tree. Clearing here would also clear
            // when clicking empty space, which breaks the "select plane then draw" workflow.
            false
        }
    }

}
