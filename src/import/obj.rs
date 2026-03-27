// OBJ importer — uses tobj crate for parsing.

use std::path::Path;
use glam::Vec3;
use crate::renderer::Mesh;
use super::ImportError;

pub fn load(path: &Path) -> Result<Mesh, ImportError> {
    let (models, _materials) = tobj::load_obj(path, &tobj::GPU_LOAD_OPTIONS)
        .map_err(|e| ImportError::Parse(format!("OBJ parse error: {}", e)))?;

    if models.is_empty() {
        return Err(ImportError::Parse("No meshes found in OBJ file".into()));
    }

    // Merge all models into one mesh
    let mut all_positions = Vec::new();
    let mut all_normals = Vec::new();
    let mut all_indices = Vec::new();

    for model in &models {
        let m = &model.mesh;
        let base_vertex = all_positions.len() as u32;

        // Positions
        for chunk in m.positions.chunks_exact(3) {
            all_positions.push(Vec3::new(chunk[0], chunk[1], chunk[2]));
        }

        // Normals (may be empty)
        for chunk in m.normals.chunks_exact(3) {
            all_normals.push(Vec3::new(chunk[0], chunk[1], chunk[2]));
        }

        // Indices (offset by base_vertex)
        for &idx in &m.indices {
            all_indices.push(base_vertex + idx);
        }
    }

    if all_positions.is_empty() || all_indices.is_empty() {
        return Err(ImportError::Parse("OBJ file has no geometry".into()));
    }

    Ok(Mesh::from_triangles(&all_positions, &all_normals, &all_indices))
}
