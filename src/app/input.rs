use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};

use crate::ui::Tool;
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
                            if self.is_drawing_tool() {
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

                if !self.egui_ctx.wants_pointer_input() {
                    if let Some(last) = self.input.last_mouse {
                        let dx = (current.0 - last.0) as f32;
                        let dy = (current.1 - last.1) as f32;
                        if dx.abs() > 1.0 || dy.abs() > 1.0 {
                            if self.input.left_pressed { self.input.left_was_drag = true; }
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
                if let Some(r) = &mut self.renderer { r.zoom(scroll); }
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
                        self.ui.active_tool = Tool::Select;
                    }

                    // Ctrl+Z = Undo
                    Key::Character(c) if c.as_str() == "z" && ctrl => {
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

                    // Ctrl+Y or Ctrl+Shift+Z = Redo
                    Key::Character(c) if c.as_str() == "y" && ctrl => {
                        if let Some(r) = &mut self.renderer {
                            if self.history.redo(&mut r.mesh) {
                                r.mesh_pipeline.rebuild_buffers(&r.gpu.device, &r.mesh);
                            }
                        }
                    }
                    Key::Character(c) if c.as_str() == "Z" && ctrl && self.input.modifiers.shift_key() => {
                        if let Some(r) = &mut self.renderer {
                            if self.history.redo(&mut r.mesh) {
                                r.mesh_pipeline.rebuild_buffers(&r.gpu.device, &r.mesh);
                            }
                        }
                    }

                    // Ctrl+O = Import file
                    Key::Character(c) if c.as_str() == "o" && ctrl => {
                        self.ui.import_request = true;
                    }

                    // Delete = Delete selected face
                    Key::Named(NamedKey::Delete) => {
                        self.delete_selected_face();
                    }

                    // Home = Zoom to fit (SolidWorks: F key)
                    Key::Named(NamedKey::Home) => {
                        if let Some(r) = &mut self.renderer {
                            r.fit_camera();
                        }
                    }

                    // --- Tool shortcuts (SolidWorks-style) ---
                    Key::Character(c) if !ctrl => {
                        match c.as_str() {
                            "s" => self.ui.active_tool = Tool::Select,
                            "l" => self.ui.active_tool = Tool::Line,
                            "r" => self.ui.active_tool = Tool::Rect,
                            "c" => self.ui.active_tool = Tool::Circle,
                            "e" => self.ui.active_tool = Tool::Extrude,
                            "x" => self.ui.active_tool = Tool::Cut,
                            "i" => self.ui.active_tool = Tool::Inset,
                            "f" => self.ui.active_tool = Tool::Fillet,
                            "w" => self.ui.show_wireframe = !self.ui.show_wireframe,
                            "g" => self.ui.show_grid = !self.ui.show_grid,
                            _ => {}
                        }
                    }

                    _ => {}
                }
            }
        }
    }
}
