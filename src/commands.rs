// Command system — every operation is a reversible command.
// This enables Ctrl+Z undo and Ctrl+Y redo for ALL operations.
//
// Architecture: Command pattern.
// Each operation (extrude, add sketch entity, etc.) creates a Command
// that knows how to execute AND undo itself. Commands are stored in
// a history stack.

use crate::renderer::mesh::Mesh;
use crate::renderer::vertex::Vertex;

/// A snapshot of the mesh state (for undo).
/// We store the full mesh state because mesh operations are complex
/// and partial undo is error-prone. For a few thousand vertices,
/// this is cheap (< 1MB per snapshot).
#[derive(Clone)]
pub struct MeshSnapshot {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub next_face_id: u32,
    pub stored_boundaries: std::collections::HashMap<u32, Vec<glam::Vec3>>,
    pub stored_holes: std::collections::HashMap<u32, Vec<Vec<glam::Vec3>>>,
}

impl MeshSnapshot {
    pub fn from_mesh(mesh: &Mesh) -> Self {
        Self {
            vertices: mesh.vertices.clone(),
            indices: mesh.indices.clone(),
            next_face_id: mesh.next_face_id(),
            stored_boundaries: mesh.stored_boundaries().clone(),
            stored_holes: mesh.stored_holes().clone(),
        }
    }

    pub fn apply_to_mesh(&self, mesh: &mut Mesh) {
        mesh.vertices = self.vertices.clone();
        mesh.indices = self.indices.clone();
        mesh.set_next_face_id(self.next_face_id);
        mesh.set_stored_boundaries(self.stored_boundaries.clone());
        mesh.set_stored_holes(self.stored_holes.clone());
    }
}

/// The undo/redo history.
pub struct CommandHistory {
    /// Stack of previous mesh states (for undo).
    undo_stack: Vec<MeshSnapshot>,
    /// Stack of undone states (for redo).
    redo_stack: Vec<MeshSnapshot>,
    /// Maximum number of undo steps.
    max_history: usize,
}

impl CommandHistory {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_history: 50,
        }
    }

    /// Save current mesh state before an operation.
    /// Call this BEFORE modifying the mesh.
    pub fn save_state(&mut self, mesh: &Mesh) {
        self.undo_stack.push(MeshSnapshot::from_mesh(mesh));
        // New action invalidates redo stack
        self.redo_stack.clear();
        // Limit history size
        if self.undo_stack.len() > self.max_history {
            self.undo_stack.remove(0);
        }
    }

    /// Undo: restore previous mesh state.
    /// Returns true if undo was performed.
    pub fn undo(&mut self, mesh: &mut Mesh) -> bool {
        if let Some(prev) = self.undo_stack.pop() {
            // Save current state to redo stack
            self.redo_stack.push(MeshSnapshot::from_mesh(mesh));
            prev.apply_to_mesh(mesh);
            true
        } else {
            false
        }
    }

    /// Clear all undo/redo history (for new scene).
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    /// Redo: restore next mesh state.
    /// Returns true if redo was performed.
    pub fn redo(&mut self, mesh: &mut Mesh) -> bool {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(MeshSnapshot::from_mesh(mesh));
            next.apply_to_mesh(mesh);
            true
        } else {
            false
        }
    }

}
