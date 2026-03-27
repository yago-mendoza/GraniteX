use super::App;

impl App {
    /// Delete the currently selected face (with undo support).
    pub(super) fn delete_selected_face(&mut self) {
        let Some(r) = &mut self.renderer else { return };
        let Some(face_id) = r.selected_face else { return };
        self.history.save_state(&r.mesh);
        if r.mesh.delete_face(face_id) {
            r.selected_face = None;
            r.mesh_pipeline.rebuild_buffers(&r.gpu.device, &r.mesh);
            r.mesh_pipeline.set_selected_face(&r.gpu.queue, None);
        }
    }

    /// Handle a click in edge selection mode.
    pub(super) fn handle_edge_click(&mut self, screen_x: f32, screen_y: f32) {
        let Some(r) = &self.renderer else { return };
        if let Some((p0, p1)) = r.try_pick_edge(screen_x, screen_y, 10.0) {
            self.ui.selected_edge = Some(([p0.x, p0.y, p0.z], [p1.x, p1.y, p1.z]));
            let length = (p1 - p0).length();
            self.ui.toasts.push(crate::ui::Toast::new(format!("Edge: {:.4} m", length)));
        } else {
            self.ui.selected_edge = None;
        }
    }
}
