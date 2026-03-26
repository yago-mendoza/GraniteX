// Mesh — dynamic geometry with face-aware operations.
//
// Key concept: a "face" is a planar region, not a single quad.
// When extruding, new side quads that are coplanar with existing faces
// get merged into the same face (same face_id). This matches SolidWorks
// behavior where extruding the top of a cube gives 6 faces, not 10.

mod ops;

use glam::Vec3;
use super::vertex::Vertex;

pub(crate) const COPLANAR_THRESHOLD: f32 = 0.999;
pub(crate) const EDGE_EPSILON: f32 = 1e-4;

pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub(super) next_face_id: u32,
}

impl Mesh {
    pub fn cube() -> Self {
        let v = |pos: [f32; 3], normal: [f32; 3], face_id: u32| Vertex {
            position: pos, normal, face_id, _pad: 0,
        };

        let vertices = vec![
            // Face 0: Front (z+)
            v([-0.5, -0.5,  0.5], [0.0, 0.0, 1.0], 0),
            v([ 0.5, -0.5,  0.5], [0.0, 0.0, 1.0], 0),
            v([ 0.5,  0.5,  0.5], [0.0, 0.0, 1.0], 0),
            v([-0.5,  0.5,  0.5], [0.0, 0.0, 1.0], 0),
            // Face 1: Back (z-)
            v([-0.5, -0.5, -0.5], [0.0, 0.0, -1.0], 1),
            v([ 0.5, -0.5, -0.5], [0.0, 0.0, -1.0], 1),
            v([ 0.5,  0.5, -0.5], [0.0, 0.0, -1.0], 1),
            v([-0.5,  0.5, -0.5], [0.0, 0.0, -1.0], 1),
            // Face 2: Top (y+)
            v([-0.5,  0.5, -0.5], [0.0, 1.0, 0.0], 2),
            v([ 0.5,  0.5, -0.5], [0.0, 1.0, 0.0], 2),
            v([ 0.5,  0.5,  0.5], [0.0, 1.0, 0.0], 2),
            v([-0.5,  0.5,  0.5], [0.0, 1.0, 0.0], 2),
            // Face 3: Bottom (y-)
            v([-0.5, -0.5, -0.5], [0.0, -1.0, 0.0], 3),
            v([ 0.5, -0.5, -0.5], [0.0, -1.0, 0.0], 3),
            v([ 0.5, -0.5,  0.5], [0.0, -1.0, 0.0], 3),
            v([-0.5, -0.5,  0.5], [0.0, -1.0, 0.0], 3),
            // Face 4: Right (x+)
            v([ 0.5, -0.5, -0.5], [1.0, 0.0, 0.0], 4),
            v([ 0.5,  0.5, -0.5], [1.0, 0.0, 0.0], 4),
            v([ 0.5,  0.5,  0.5], [1.0, 0.0, 0.0], 4),
            v([ 0.5, -0.5,  0.5], [1.0, 0.0, 0.0], 4),
            // Face 5: Left (x-)
            v([-0.5, -0.5, -0.5], [-1.0, 0.0, 0.0], 5),
            v([-0.5,  0.5, -0.5], [-1.0, 0.0, 0.0], 5),
            v([-0.5,  0.5,  0.5], [-1.0, 0.0, 0.0], 5),
            v([-0.5, -0.5,  0.5], [-1.0, 0.0, 0.0], 5),
        ];

        #[rustfmt::skip]
        let indices: Vec<u32> = vec![
            0,  1,  2,  0,  2,  3,
            4,  6,  5,  4,  7,  6,
            8,  9,  10, 8,  10, 11,
            12, 14, 13, 12, 15, 14,
            16, 17, 18, 16, 18, 19,
            20, 22, 21, 20, 23, 22,
        ];

        Self { vertices, indices, next_face_id: 6 }
    }

    #[allow(dead_code)]
    pub fn empty() -> Self {
        Self { vertices: Vec::new(), indices: Vec::new(), next_face_id: 0 }
    }

    /// Construct mesh from raw triangle data (for importers).
    /// Each triangle gets its own face_id (flat shading).
    pub fn from_triangles(positions: &[Vec3], normals: &[Vec3], indices: &[u32]) -> Self {
        let mut vertices = Vec::with_capacity(indices.len());
        let mut out_indices = Vec::with_capacity(indices.len());
        let mut face_id = 0u32;

        for tri in indices.chunks(3) {
            let i0 = tri[0] as usize;
            let i1 = tri[1] as usize;
            let i2 = tri[2] as usize;

            let normal = if !normals.is_empty() && normals.len() > i0 {
                normals[i0]
            } else {
                let e1 = positions[i1] - positions[i0];
                let e2 = positions[i2] - positions[i0];
                e1.cross(e2).normalize_or_zero()
            };

            let base = vertices.len() as u32;
            for &idx in tri {
                let p = positions[idx as usize];
                vertices.push(Vertex {
                    position: p.into(),
                    normal: normal.into(),
                    face_id,
                    _pad: 0,
                });
            }
            out_indices.push(base);
            out_indices.push(base + 1);
            out_indices.push(base + 2);
            face_id += 1;
        }

        Self { vertices, indices: out_indices, next_face_id: face_id }
    }

    /// Axis-aligned bounding box: (min, max).
    pub fn bounding_box(&self) -> (Vec3, Vec3) {
        if self.vertices.is_empty() {
            return (Vec3::ZERO, Vec3::ZERO);
        }
        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);
        for v in &self.vertices {
            let p = Vec3::from(v.position);
            min = min.min(p);
            max = max.max(p);
        }
        (min, max)
    }

    // --- Queries ---

    pub fn face_normal(&self, face_id: u32) -> Option<Vec3> {
        self.vertices.iter()
            .find(|v| v.face_id == face_id)
            .map(|v| Vec3::from(v.normal))
    }

    /// Approximate face area (sum of triangle areas for this face_id).
    pub fn face_area(&self, face_id: u32) -> f32 {
        let mut area = 0.0;
        for chunk in self.indices.chunks(3) {
            if self.vertices[chunk[0] as usize].face_id != face_id { continue; }
            let p0 = Vec3::from(self.vertices[chunk[0] as usize].position);
            let p1 = Vec3::from(self.vertices[chunk[1] as usize].position);
            let p2 = Vec3::from(self.vertices[chunk[2] as usize].position);
            area += (p1 - p0).cross(p2 - p0).length() * 0.5;
        }
        area
    }

    fn face_vertex_indices(&self, face_id: u32) -> Vec<usize> {
        self.vertices.iter().enumerate()
            .filter(|(_, v)| v.face_id == face_id)
            .map(|(i, _)| i)
            .collect()
    }

    fn find_coplanar_adjacent_face(
        &self,
        side_normal: Vec3,
        side_positions: &[[f32; 3]; 4],
    ) -> Option<u32> {
        let mut face_ids: Vec<u32> = self.vertices.iter().map(|v| v.face_id).collect();
        face_ids.sort_unstable();
        face_ids.dedup();

        for &fid in &face_ids {
            let face_normal = match self.face_normal(fid) {
                Some(n) => n,
                None => continue,
            };

            if face_normal.dot(side_normal).abs() < COPLANAR_THRESHOLD {
                continue;
            }

            let face_positions: Vec<Vec3> = self.vertices.iter()
                .filter(|v| v.face_id == fid)
                .map(|v| Vec3::from(v.position))
                .collect();

            let mut shared_points = 0;
            for sp in side_positions {
                let sp = Vec3::from(*sp);
                for fp in &face_positions {
                    if (sp - *fp).length() < EDGE_EPSILON {
                        shared_points += 1;
                        break;
                    }
                }
            }

            if shared_points >= 2 {
                let face_d = face_normal.dot(face_positions[0]);
                let side_d = face_normal.dot(Vec3::from(side_positions[0]));
                if (face_d - side_d).abs() < EDGE_EPSILON {
                    return Some(fid);
                }
            }
        }

        None
    }

    pub fn face_boundary_corners(&self, face_id: u32) -> Option<Vec<Vec3>> {
        let normal = self.face_normal(face_id)?;

        let all_positions: Vec<Vec3> = self.vertices.iter()
            .filter(|v| v.face_id == face_id)
            .map(|v| Vec3::from(v.position))
            .collect();

        let mut positions: Vec<Vec3> = Vec::new();
        for p in &all_positions {
            if !positions.iter().any(|q| (*q - *p).length() < EDGE_EPSILON) {
                positions.push(*p);
            }
        }

        if positions.len() < 3 {
            return None;
        }

        // Always sort by angle around center — even for 3-4 vertex faces.
        // Without this, vertex buffer order can cause crossed quads on extrude.
        let center: Vec3 = positions.iter().copied().sum::<Vec3>() / positions.len() as f32;

        let u_axis = if normal.dot(Vec3::Y).abs() < 0.99 {
            normal.cross(Vec3::Y).normalize()
        } else {
            normal.cross(Vec3::X).normalize()
        };
        let v_axis = normal.cross(u_axis).normalize();

        positions.sort_by(|a, b| {
            let da = *a - center;
            let db = *b - center;
            let angle_a = da.dot(v_axis).atan2(da.dot(u_axis));
            let angle_b = db.dot(v_axis).atan2(db.dot(u_axis));
            angle_a.partial_cmp(&angle_b).unwrap_or(std::cmp::Ordering::Equal)
        });

        Some(positions)
    }

    // --- Accessors ---

    pub fn next_face_id(&self) -> u32 {
        self.next_face_id
    }

    pub fn set_next_face_id(&mut self, id: u32) {
        self.next_face_id = id;
    }

    pub fn face_count(&self) -> u32 {
        let mut ids: Vec<u32> = self.vertices.iter().map(|v| v.face_id).collect();
        ids.sort_unstable();
        ids.dedup();
        ids.len() as u32
    }

    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }
}
