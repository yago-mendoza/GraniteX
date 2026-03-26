// Mesh — dynamic geometry with face-aware operations.
//
// Key concept: a "face" is a planar region, not a single quad.
// When extruding, new side quads that are coplanar with existing faces
// get merged into the same face (same face_id). This matches SolidWorks
// behavior where extruding the top of a cube gives 6 faces, not 10.

use glam::Vec3;
use super::vertex::Vertex;

const COPLANAR_THRESHOLD: f32 = 0.999; // dot product threshold for "same normal"
const EDGE_EPSILON: f32 = 1e-4;        // distance threshold for "same point"

pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
    next_face_id: u32,
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
        let indices: Vec<u16> = vec![
            0,  1,  2,  0,  2,  3,
            4,  6,  5,  4,  7,  6,
            8,  9,  10, 8,  10, 11,
            12, 14, 13, 12, 15, 14,
            16, 17, 18, 16, 18, 19,
            20, 22, 21, 20, 23, 22,
        ];

        Self { vertices, indices, next_face_id: 6 }
    }

    pub fn indices_u16(&self) -> &[u16] {
        &self.indices
    }

    pub fn face_normal(&self, face_id: u32) -> Option<Vec3> {
        self.vertices.iter()
            .find(|v| v.face_id == face_id)
            .map(|v| Vec3::from(v.normal))
    }

    /// Get vertex indices belonging to a face.
    fn face_vertex_indices(&self, face_id: u32) -> Vec<usize> {
        self.vertices.iter().enumerate()
            .filter(|(_, v)| v.face_id == face_id)
            .map(|(i, _)| i)
            .collect()
    }

    /// Find an existing face that is coplanar with `normal` and shares an edge
    /// with the given side quad (defined by its 4 positions).
    fn find_coplanar_adjacent_face(
        &self,
        side_normal: Vec3,
        side_positions: &[[f32; 3]; 4],
    ) -> Option<u32> {
        // Collect all unique face_ids
        let mut face_ids: Vec<u32> = self.vertices.iter().map(|v| v.face_id).collect();
        face_ids.sort_unstable();
        face_ids.dedup();

        for &fid in &face_ids {
            let face_normal = match self.face_normal(fid) {
                Some(n) => n,
                None => continue,
            };

            // Check coplanarity: same normal direction
            if face_normal.dot(side_normal).abs() < COPLANAR_THRESHOLD {
                continue;
            }

            // Check that they share at least one edge (2 common points).
            // Get all positions of this face.
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

            // Need at least 2 shared points (an edge) to be adjacent
            if shared_points >= 2 {
                // Verify they're on the same plane — use same normal for both
                let face_d = face_normal.dot(face_positions[0]);
                let side_d = face_normal.dot(Vec3::from(side_positions[0]));
                if (face_d - side_d).abs() < EDGE_EPSILON {
                    return Some(fid);
                }
            }
        }

        None
    }

    /// Get the ordered boundary vertices of a face.
    /// For simple faces (4 verts) returns them directly.
    /// For merged/polygon faces, collects unique positions and orders them
    /// by angle around the face center (convex hull on the plane).
    pub fn face_boundary_corners(&self, face_id: u32) -> Option<Vec<Vec3>> {
        let normal = self.face_normal(face_id)?;

        // Collect all unique positions for this face
        let all_positions: Vec<Vec3> = self.vertices.iter()
            .filter(|v| v.face_id == face_id)
            .map(|v| Vec3::from(v.position))
            .collect();

        // Deduplicate
        let mut positions: Vec<Vec3> = Vec::new();
        for p in &all_positions {
            if !positions.iter().any(|q| (*q - *p).length() < EDGE_EPSILON) {
                positions.push(*p);
            }
        }

        if positions.len() < 3 {
            return None;
        }

        if positions.len() <= 4 {
            return Some(positions);
        }

        // For >4 unique positions: sort by angle around center on the face plane.
        // This gives the correct polygon boundary for convex shapes (circles, merged rects).
        let center: Vec3 = positions.iter().copied().sum::<Vec3>() / positions.len() as f32;

        let u_axis = if normal.dot(Vec3::Y).abs() < 0.99 {
            normal.cross(Vec3::Y).normalize()
        } else {
            normal.cross(Vec3::X).normalize()
        };
        let v_axis = normal.cross(u_axis).normalize();

        // Sort by angle
        positions.sort_by(|a, b| {
            let da = *a - center;
            let db = *b - center;
            let angle_a = da.dot(v_axis).atan2(da.dot(u_axis));
            let angle_b = db.dot(v_axis).atan2(db.dot(u_axis));
            angle_a.partial_cmp(&angle_b).unwrap_or(std::cmp::Ordering::Equal)
        });

        Some(positions)
    }

    /// Extrude a face along its normal by `distance`.
    /// Coplanar adjacent faces are merged (SolidWorks behavior).
    /// Returns the new cap face_id.
    pub fn extrude_face(&mut self, face_id: u32, distance: f32) -> Option<u32> {
        let normal = self.face_normal(face_id)?;
        let offset = normal * distance;

        // Get boundary corners (works for quads, merged faces, and polygons like circles)
        let corners = self.face_boundary_corners(face_id)?;
        let n = corners.len();
        if n < 3 {
            return None;
        }

        let face_verts = self.face_vertex_indices(face_id);

        // Save original boundary positions
        let old_positions: Vec<[f32; 3]> = corners.iter().map(|c| (*c).into()).collect();

        // Move ALL face vertices along the normal (entire face moves → becomes cap)
        let cap_face_id = self.next_face_id;
        self.next_face_id += 1;
        for &vi in &face_verts {
            let pos = Vec3::from(self.vertices[vi].position) + offset;
            self.vertices[vi].position = pos.into();
            self.vertices[vi].face_id = cap_face_id;
        }

        // New boundary positions (after move)
        let new_positions: Vec<[f32; 3]> = old_positions.iter()
            .map(|p| (Vec3::from(*p) + offset).into())
            .collect();

        // Create N side quads (one per edge), merging with coplanar adjacent faces
        for i in 0..n {
            let j = (i + 1) % n;

            let bottom0 = old_positions[i];
            let bottom1 = old_positions[j];
            let top0 = new_positions[i];
            let top1 = new_positions[j];

            let edge_h = Vec3::from(top0) - Vec3::from(bottom0);
            let edge_w = Vec3::from(bottom1) - Vec3::from(bottom0);
            let side_normal = edge_w.cross(edge_h).normalize();

            let side_positions = [bottom0, bottom1, top1, top0];

            // Check if this side should merge with an existing coplanar face
            let side_face_id = self
                .find_coplanar_adjacent_face(side_normal, &side_positions)
                .unwrap_or_else(|| {
                    let id = self.next_face_id;
                    self.next_face_id += 1;
                    id
                });

            let base_idx = self.vertices.len() as u16;

            let v = |pos: [f32; 3]| Vertex {
                position: pos,
                normal: side_normal.into(),
                face_id: side_face_id,
                _pad: 0,
            };

            self.vertices.push(v(bottom0));
            self.vertices.push(v(bottom1));
            self.vertices.push(v(top1));
            self.vertices.push(v(top0));

            self.indices.push(base_idx);
            self.indices.push(base_idx + 1);
            self.indices.push(base_idx + 2);
            self.indices.push(base_idx);
            self.indices.push(base_idx + 2);
            self.indices.push(base_idx + 3);
        }

        Some(cap_face_id)
    }

    /// Add a polygon as a new face, slightly offset along normal to prevent
    /// z-fighting with the parent face underneath.
    /// Returns the new face_id.
    pub fn add_polygon_face(&mut self, points: &[Vec3], normal: Vec3) -> u32 {
        let face_id = self.next_face_id;
        self.next_face_id += 1;

        if points.len() < 3 { return face_id; }

        let base_idx = self.vertices.len() as u16;

        // Offset slightly along normal so contour renders on top of parent face
        let offset = normal * 0.002;

        for p in points {
            self.vertices.push(Vertex {
                position: (*p + offset).into(),
                normal: normal.into(),
                face_id,
                _pad: 0,
            });
        }

        // Fan triangulation
        for i in 1..(points.len() as u16 - 1) {
            self.indices.push(base_idx);
            self.indices.push(base_idx + i);
            self.indices.push(base_idx + i + 1);
        }

        face_id
    }

    pub fn next_face_id(&self) -> u32 {
        self.next_face_id
    }

    pub fn set_next_face_id(&mut self, id: u32) {
        self.next_face_id = id;
    }

    pub fn face_count(&self) -> u32 {
        // Count unique face_ids actually used
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
