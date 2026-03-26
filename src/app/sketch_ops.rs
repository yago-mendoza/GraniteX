use crate::sketch::SketchPlane;
use crate::ui::Tool;
use super::App;

impl App {
    pub(super) fn is_drawing_tool(&self) -> bool {
        matches!(self.ui.active_tool, Tool::Line | Tool::Rect | Tool::Circle)
    }

    pub(super) fn handle_draw_click(&mut self, screen_x: f32, screen_y: f32) {
        let Some(renderer) = &self.renderer else { return };

        let result = crate::renderer::picking::pick_face(
            screen_x, screen_y,
            renderer.gpu_width(), renderer.gpu_height(),
            renderer.view_proj(),
            &renderer.mesh,
        );
        let Some(pick) = result else { return };
        let face_id = pick.face_id;

        let need_new = match &self.sketch {
            None => true,
            Some(s) => s.face_id != face_id,
        };
        if need_new {
            let Some(normal) = renderer.mesh.face_normal(face_id) else { return };
            let Some(center) = renderer.face_center(face_id) else { return };
            let plane = SketchPlane::from_face(normal, center);
            self.sketch = Some(crate::sketch::Sketch::new(plane, face_id));
        }

        let Some(renderer) = &self.renderer else { return };
        let Some(sketch) = &mut self.sketch else { return };

        let (ray_o, ray_d) = renderer.screen_to_ray(screen_x, screen_y);
        let Some(hit) = sketch.plane.ray_intersect(ray_o, ray_d) else { return };
        let mut pos = sketch.world_to_2d(hit);
        sketch.cursor_2d = Some(pos);

        if let Some(snapped) = sketch.snap_to_endpoint(pos, 0.05) {
            pos = snapped;
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
            self.convert_contour_to_face();
            self.sketch = None;
            self.ui.active_tool = Tool::Select;
        }
    }

    pub(super) fn convert_contour_to_face(&mut self) {
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

                for entity in &entities[start_idx..] {
                    if let crate::sketch::SketchEntity::Line { start, end } = entity {
                        if contour_points.is_empty() || start.distance_to(*contour_points.last().unwrap()) > 0.01 {
                            contour_points.push(*start);
                        }
                        contour_points.push(*end);
                    }
                }
            }
        }

        if contour_points.len() < 3 { return; }

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
}
