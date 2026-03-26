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
    pub indices: Vec<u32>,
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

            // Use per-face normal if provided, otherwise compute from triangle
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

        // Always sort by angle to ensure correct winding order.
        // Without this, 4-vertex faces can have crossed quads on extrude.
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

            let base_idx = self.vertices.len() as u32;

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

        let base_idx = self.vertices.len() as u32;

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
        for i in 1..(points.len() as u32 - 1) {
            self.indices.push(base_idx);
            self.indices.push(base_idx + i);
            self.indices.push(base_idx + i + 1);
        }

        face_id
    }

    /// Cut into a face along its negative normal by `depth`.
    /// Creates a pocket: the face moves inward, side walls are added.
    /// No coplanar merging (walls form pocket interior).
    /// Returns the new floor face_id.
    pub fn cut_face(&mut self, face_id: u32, depth: f32) -> Option<u32> {
        let normal = self.face_normal(face_id)?;
        let offset = normal * (-depth); // inward

        let corners = self.face_boundary_corners(face_id)?;
        let n = corners.len();
        if n < 3 { return None; }

        let face_verts = self.face_vertex_indices(face_id);
        let old_positions: Vec<[f32; 3]> = corners.iter().map(|c| (*c).into()).collect();

        // Move face inward → becomes floor of pocket
        let floor_face_id = self.next_face_id;
        self.next_face_id += 1;
        for &vi in &face_verts {
            let pos = Vec3::from(self.vertices[vi].position) + offset;
            self.vertices[vi].position = pos.into();
            self.vertices[vi].normal = normal.into(); // floor faces up (same as original)
            self.vertices[vi].face_id = floor_face_id;
        }

        let new_positions: Vec<[f32; 3]> = old_positions.iter()
            .map(|p| (Vec3::from(*p) + offset).into())
            .collect();

        // Create N side quads (pocket walls). Each gets a unique face_id.
        // Winding: old (top) to new (bottom), normals point inward (into pocket).
        for i in 0..n {
            let j = (i + 1) % n;

            let top0 = old_positions[i];
            let top1 = old_positions[j];
            let bot0 = new_positions[i];
            let bot1 = new_positions[j];

            // Normal points inward (opposite of extrude)
            let edge_h = Vec3::from(bot0) - Vec3::from(top0);
            let edge_w = Vec3::from(top1) - Vec3::from(top0);
            let side_normal = edge_h.cross(edge_w).normalize();

            let wall_face_id = self.next_face_id;
            self.next_face_id += 1;

            let base_idx = self.vertices.len() as u32;

            let v = |pos: [f32; 3]| Vertex {
                position: pos,
                normal: side_normal.into(),
                face_id: wall_face_id,
                _pad: 0,
            };

            // Quad: top0, top1, bot1, bot0
            self.vertices.push(v(top0));
            self.vertices.push(v(top1));
            self.vertices.push(v(bot1));
            self.vertices.push(v(bot0));

            self.indices.push(base_idx);
            self.indices.push(base_idx + 1);
            self.indices.push(base_idx + 2);
            self.indices.push(base_idx);
            self.indices.push(base_idx + 2);
            self.indices.push(base_idx + 3);
        }

        Some(floor_face_id)
    }

    pub fn next_face_id(&self) -> u32 {
        self.next_face_id
    }

    pub fn set_next_face_id(&mut self, id: u32) {
        self.next_face_id = id;
    }

    /// Delete a face and compact the mesh (removes orphaned vertices).
    /// Returns true if the face was found and deleted.
    pub fn delete_face(&mut self, face_id: u32) -> bool {
        // Remove triangles belonging to this face
        let mut new_indices: Vec<u32> = Vec::new();
        for chunk in self.indices.chunks(3) {
            if self.vertices[chunk[0] as usize].face_id != face_id {
                new_indices.extend_from_slice(chunk);
            }
        }

        if new_indices.len() == self.indices.len() {
            return false;
        }

        // Find which vertices are still referenced
        let mut used = vec![false; self.vertices.len()];
        for &idx in &new_indices {
            used[idx as usize] = true;
        }

        // Build remap table and compact vertices
        let mut remap = vec![0u32; self.vertices.len()];
        let mut new_verts = Vec::new();
        for (old_idx, vertex) in self.vertices.iter().enumerate() {
            if used[old_idx] {
                remap[old_idx] = new_verts.len() as u32;
                new_verts.push(*vertex);
            }
        }

        // Remap indices
        for idx in &mut new_indices {
            *idx = remap[*idx as usize];
        }

        self.vertices = new_verts;
        self.indices = new_indices;
        true
    }

    /// Inset a face: shrinks the boundary toward its center, creating
    /// a smaller inner face and connecting quad strips around the edge.
    /// `amount` is the absolute inset distance.
    /// Returns the inner face_id if successful.
    pub fn inset_face(&mut self, face_id: u32, amount: f32) -> Option<u32> {
        let normal = self.face_normal(face_id)?;
        let corners = self.face_boundary_corners(face_id)?;
        let n = corners.len();
        if n < 3 { return None; }

        let center: Vec3 = corners.iter().copied().sum::<Vec3>() / n as f32;

        // Compute inner corners by moving each corner toward center
        let inner_corners: Vec<Vec3> = corners.iter().map(|c| {
            let dir = (center - *c).normalize_or_zero();
            *c + dir * amount
        }).collect();

        // Delete original face geometry
        self.delete_face(face_id);

        // Add inner face (same normal)
        let inner_face_id = self.next_face_id;
        self.next_face_id += 1;

        let inner_base = self.vertices.len() as u32;
        for p in &inner_corners {
            self.vertices.push(Vertex {
                position: (*p).into(),
                normal: normal.into(),
                face_id: inner_face_id,
                _pad: 0,
            });
        }
        // Fan triangulation for inner face
        for i in 1..(n as u32 - 1) {
            self.indices.push(inner_base);
            self.indices.push(inner_base + i);
            self.indices.push(inner_base + i + 1);
        }

        // Add N connecting quads between outer boundary and inner boundary
        for i in 0..n {
            let j = (i + 1) % n;

            let quad_face_id = self.next_face_id;
            self.next_face_id += 1;

            let base = self.vertices.len() as u32;
            let quad_verts = [
                corners[i], corners[j], inner_corners[j], inner_corners[i],
            ];

            for p in &quad_verts {
                self.vertices.push(Vertex {
                    position: (*p).into(),
                    normal: normal.into(),
                    face_id: quad_face_id,
                    _pad: 0,
                });
            }

            // Two triangles per quad
            self.indices.push(base);
            self.indices.push(base + 1);
            self.indices.push(base + 2);
            self.indices.push(base);
            self.indices.push(base + 2);
            self.indices.push(base + 3);
        }

        Some(inner_face_id)
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
