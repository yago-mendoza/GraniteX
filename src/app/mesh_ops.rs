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

    /// Inset the currently selected face (with undo support).
    /// Returns the new inner face_id if successful.
    pub(super) fn inset_selected_face(&mut self, amount: f32) -> Option<u32> {
        let r = self.renderer.as_mut()?;
        let face_id = r.selected_face?;
        self.history.save_state(&r.mesh);
        let inner = r.mesh.inset_face(face_id, amount)?;
        r.selected_face = Some(inner);
        r.mesh_pipeline.rebuild_buffers(&r.gpu.device, &r.mesh);
        r.mesh_pipeline.set_selected_face(&r.gpu.queue, Some(inner));
        Some(inner)
    }
}
