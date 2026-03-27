use crate::ui::Measurement;
use super::App;

impl App {
    /// Handle a click in Measure mode.
    pub(super) fn handle_measure_click(&mut self, screen_x: f32, screen_y: f32) {
        let Some(renderer) = &self.renderer else { return };

        let view_proj = renderer.view_proj();
        let result = crate::renderer::picking::pick_face(
            screen_x, screen_y,
            renderer.gpu_width(), renderer.gpu_height(),
            view_proj,
            &renderer.mesh,
        );

        let Some(pick) = result else { return };
        let hit = [pick.hit_point.x, pick.hit_point.y, pick.hit_point.z];

        if let Some(first) = self.ui.measure_first_point {
            // Second click — compute distance
            let dx = hit[0] - first[0];
            let dy = hit[1] - first[1];
            let dz = hit[2] - first[2];
            let distance = (dx * dx + dy * dy + dz * dz).sqrt();

            self.ui.active_measurement = Some(Measurement {
                point_a: first,
                point_b: hit,
                distance,
            });
            self.ui.measure_first_point = None;
            self.ui.toasts.push(crate::ui::Toast::new(format!("Distance: {:.4} m", distance)));
        } else {
            // First click — store point
            self.ui.measure_first_point = Some(hit);
        }
    }
}
