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
}
