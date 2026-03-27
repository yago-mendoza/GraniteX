// Tessellation — convert BREP faces to GPU vertex/index buffers.
//
// Each face is triangulated (fan for convex quads, earcutr for complex polygons).
// Output matches the existing Vertex format: position + normal + face_id.

use glam::Vec3;
use super::{BrepMesh, FaceId};

/// GPU vertex matching the existing shader layout.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub face_id: u32,
    pub _pad: u32,
}

/// Tessellation output: vertex buffer + index buffer.
pub struct TessellatedMesh {
    pub vertices: Vec<GpuVertex>,
    pub indices: Vec<u32>,
    /// Maps GPU face_id back to BrepMesh FaceId.
    pub face_id_map: Vec<FaceId>,
}

impl BrepMesh {
    /// Convert the entire mesh to GPU buffers for rendering.
    pub fn tessellate(&self) -> TessellatedMesh {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut face_id_map = Vec::new();

        for (face_key, face) in &self.faces {
            let positions = self.face_positions(face_key);
            let normal = face.normal;
            let gpu_face_id = face_id_map.len() as u32;
            face_id_map.push(face_key);

            let base = vertices.len() as u32;

            // Push vertices
            for p in &positions {
                vertices.push(GpuVertex {
                    position: (*p).into(),
                    normal: normal.into(),
                    face_id: gpu_face_id,
                    _pad: 0,
                });
            }

            // Triangulate
            let tris = triangulate_face(&positions, normal);
            for tri in &tris {
                indices.push(base + tri[0] as u32);
                indices.push(base + tri[1] as u32);
                indices.push(base + tri[2] as u32);
            }
        }

        TessellatedMesh { vertices, indices, face_id_map }
    }

    /// Total triangle count (for stats).
    pub fn triangle_count(&self) -> usize {
        self.faces.keys()
            .map(|f| {
                let n = self.face_sides(f);
                if n < 3 { 0 } else { n - 2 }
            })
            .sum()
    }
}

/// Triangulate a single face polygon.
fn triangulate_face(positions: &[Vec3], normal: Vec3) -> Vec<[usize; 3]> {
    let n = positions.len();
    if n < 3 { return Vec::new(); }

    // Fast path: triangle
    if n == 3 {
        return vec![[0, 1, 2]];
    }

    // Fast path: convex quad (fan works)
    if n == 4 {
        return vec![[0, 1, 2], [0, 2, 3]];
    }

    // General case: project to 2D and use earcutr
    let u_axis = if normal.dot(Vec3::Y).abs() < 0.99 {
        normal.cross(Vec3::Y).normalize()
    } else {
        normal.cross(Vec3::X).normalize()
    };
    let v_axis = normal.cross(u_axis).normalize();
    let origin = positions[0];

    let coords: Vec<f64> = positions.iter()
        .flat_map(|p| {
            let d = *p - origin;
            [d.dot(u_axis) as f64, d.dot(v_axis) as f64]
        })
        .collect();

    match earcutr::earcut(&coords, &[], 2) {
        Ok(idx) => {
            idx.chunks_exact(3)
                .map(|t| [t[0], t[1], t[2]])
                .collect()
        }
        Err(_) => {
            // Fallback: fan
            (1..n - 1).map(|i| [0, i, i + 1]).collect()
        }
    }
}
