use crate::sketch::SketchPlane;
use crate::ui::Tool;
use super::App;

impl App {
    pub(super) fn is_drawing_tool(&self) -> bool {
        matches!(self.ui.active_tool, Tool::Line | Tool::Rect | Tool::Circle)
    }

    pub(super) fn handle_draw_click(&mut self, screen_x: f32, screen_y: f32) {
        let Some(renderer) = &self.renderer else { return };

        // If no sketch exists, create one on the clicked face
        if self.sketch.is_none() {
            let result = crate::renderer::picking::pick_face(
                screen_x, screen_y,
                renderer.gpu_width(), renderer.gpu_height(),
                renderer.view_proj(),
                &renderer.mesh,
            );
            let Some(pick) = result else { return };
            let face_id = pick.face_id;

            // Only allow sketching on PLANAR faces (not cylinders, spheres, etc.)
            if !renderer.mesh.is_face_planar(face_id) {
                self.ui.toasts.push(crate::ui::Toast::new(
                    "Cannot sketch on curved surfaces — select a flat face".into()
                ));
                return;
            }

            let Some(normal) = renderer.mesh.face_normal(face_id) else { return };
            let Some(center) = renderer.face_center(face_id) else { return };
            let plane = SketchPlane::from_face(normal, center);

            // Get parent face boundary in 2D for outer region computation
            let face_boundary_2d = renderer.mesh.face_boundary_corners(face_id).map(|pts| {
                pts.iter().map(|p| plane.world_to_2d(*p)).collect::<Vec<_>>()
            });

            self.sketch = Some(crate::sketch::Sketch::new(plane, face_id, face_boundary_2d));
        }

        let Some(renderer) = &self.renderer else { return };
        let Some(sketch) = &mut self.sketch else { return };

        let (ray_o, ray_d) = renderer.screen_to_ray(screen_x, screen_y);
        let Some(hit) = sketch.plane.ray_intersect(ray_o, ray_d) else { return };
        let mut pos = sketch.world_to_2d(hit);
        sketch.cursor_2d = Some(pos);

        // Snap to nearest target (endpoints, corners, midpoints, edges)
        if let Some(snap) = sketch.snap_to_target(pos, 0.05) {
            pos = snap.point;
        }

        let mut contour_closed = false;

        match self.ui.active_tool {
            Tool::Line => {
                if let Some(start) = sketch.pending_start.take() {
                    sketch.add_line(start, pos);
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
            if let Some(sketch) = &mut self.sketch {
                sketch.pending_start = None;
                sketch.chain_start = None;
                sketch.region_solver.mark_dirty();
                sketch.selected_region = None;
            }

            let n = self.sketch.as_mut()
                .map(|s| s.regions().len())
                .unwrap_or(0);

            if n == 1 {
                // Single region — auto-select it (NO mesh face created yet!)
                // The face will be created atomically when user applies extrude/cut.
                if let Some(sketch) = &mut self.sketch {
                    sketch.selected_region = Some(0);
                }
                self.ui.active_tool = Tool::Extrude;
                self.ui.toasts.push(crate::ui::Toast::new("Contour closed — ready to extrude".into()));
            } else if n > 1 {
                self.ui.active_tool = Tool::Select;
                self.ui.toasts.push(crate::ui::Toast::new(
                    format!("{} regions — click inside one to select", n)
                ));
            }
        }
    }

    /// Try to select a sketch region at screen coordinates.
    /// Does NOT create a mesh face — just sets selected_region.
    /// The face is created atomically when the user applies an operation.
    pub(super) fn try_select_region(&mut self, screen_x: f32, screen_y: f32) -> bool {
        let Some(sketch) = &mut self.sketch else { return false };
        let Some(renderer) = &self.renderer else { return false };

        let (ray_o, ray_d) = renderer.screen_to_ray(screen_x, screen_y);
        let Some(hit) = sketch.plane.ray_intersect(ray_o, ray_d) else { return false };
        let pos_2d = sketch.world_to_2d(hit);

        if !sketch.select_region_at(pos_2d) {
            return false;
        }

        self.ui.active_tool = Tool::Extrude;
        self.ui.toasts.push(crate::ui::Toast::new("Region selected — ready to extrude".into()));
        true
    }
}
