// Mesh operations: extrude, cut, inset, delete, add polygon.

use glam::Vec3;
use super::{Mesh, Vertex};

impl Mesh {
    /// Extrude a face along its normal by `distance`.
    /// Coplanar adjacent faces are merged (SolidWorks behavior).
    pub fn extrude_face(&mut self, face_id: u32, distance: f32) -> Option<u32> {
        let normal = self.face_normal(face_id)?;
        let offset = normal * distance;

        let corners = self.face_boundary_corners(face_id)?;
        let n = corners.len();
        if n < 3 { return None; }

        let old_positions: Vec<[f32; 3]> = corners.iter().map(|c| (*c).into()).collect();

        // Save original triangles, then remove them from the index buffer.
        // We'll re-add them as cap triangles after moving the vertices.
        let cap_tris: Vec<[u32; 3]> = self.indices.chunks_exact(3)
            .filter(|c| self.vertices[c[0] as usize].face_id == face_id)
            .map(|c| [c[0], c[1], c[2]])
            .collect();
        self.remove_face_indices(face_id);

        // Move original vertices to cap position
        let face_verts = self.face_vertex_indices(face_id);
        let cap_face_id = self.next_face_id;
        self.next_face_id += 1;
        for &vi in &face_verts {
            let pos = Vec3::from(self.vertices[vi].position) + offset;
            self.vertices[vi].position = pos.into();
            self.vertices[vi].normal = normal.into(); // preserve outward normal for cap
            self.vertices[vi].face_id = cap_face_id;
        }

        // Re-add cap triangles (same vertex indices, now at cap position)
        for tri in &cap_tris {
            self.indices.extend_from_slice(&[tri[0], tri[1], tri[2]]);
        }

        // Transfer stored boundary to cap face
        if let Some(boundary) = self.stored_boundaries.remove(&face_id) {
            let cap_boundary: Vec<Vec3> = boundary.iter().map(|p| *p + offset).collect();
            self.stored_boundaries.insert(cap_face_id, cap_boundary);
        }

        // Transfer stored holes to cap face
        let hole_boundaries = self.stored_holes.remove(&face_id);
        if let Some(ref holes) = hole_boundaries {
            let cap_holes: Vec<Vec<Vec3>> = holes.iter()
                .map(|hole| hole.iter().map(|p| *p + offset).collect())
                .collect();
            self.stored_holes.insert(cap_face_id, cap_holes);
        }

        let new_positions: Vec<[f32; 3]> = old_positions.iter()
            .map(|p| (Vec3::from(*p) + offset).into())
            .collect();

        // Create outer boundary side walls
        self.create_side_walls(&old_positions, &new_positions, normal, false);

        // Create inner (hole) side walls — reversed winding so normals face inward
        if let Some(ref holes) = hole_boundaries {
            for hole in holes {
                let hole_old: Vec<[f32; 3]> = hole.iter().map(|p| (*p).into()).collect();
                let hole_new: Vec<[f32; 3]> = hole.iter().map(|p| (*p + offset).into()).collect();
                self.create_side_walls(&hole_old, &hole_new, normal, true);
            }
        }

        Some(cap_face_id)
    }

    /// Create side walls between two boundary rings (old → new positions).
    /// If `reverse` is true, winding is flipped (for hole interiors).
    fn create_side_walls(
        &mut self,
        old_positions: &[[f32; 3]],
        new_positions: &[[f32; 3]],
        face_normal: Vec3,
        reverse: bool,
    ) {
        let n = old_positions.len();
        if n < 3 { return; }

        // Detect if this is a tessellated circle (many points, all ~equidistant from center)
        // vs a user-drawn polygon (few points, arbitrary shape).
        // Circles get smooth radial normals (cylinder). Polygons get flat faces (hard edges).
        let center = old_positions.iter()
            .map(|p| Vec3::from(*p))
            .sum::<Vec3>() / n as f32;

        let is_cylindrical = n > 16 && {
            // Check if all corners are approximately equidistant from center (within 5%)
            let distances: Vec<f32> = old_positions.iter()
                .map(|p| (Vec3::from(*p) - center).length())
                .collect();
            let avg = distances.iter().sum::<f32>() / distances.len() as f32;
            avg > 1e-6 && distances.iter().all(|d| (d - avg).abs() / avg < 0.05)
        };
        let cylinder_face_id = if is_cylindrical {
            let id = self.next_face_id;
            self.next_face_id += 1;
            Some(id)
        } else {
            None
        };

        let radial_normals: Vec<Vec3> = if cylinder_face_id.is_some() {
            old_positions.iter().map(|p| {
                let pos = Vec3::from(*p);
                let radial = (pos - center - face_normal * (pos - center).dot(face_normal)).normalize_or_zero();
                if reverse { -radial } else { radial }
            }).collect()
        } else {
            Vec::new()
        };

        for i in 0..n {
            let j = (i + 1) % n;

            // For reversed (hole) walls, swap i/j to flip winding
            let (a, b) = if reverse { (j, i) } else { (i, j) };

            let bottom0 = old_positions[a];
            let bottom1 = old_positions[b];
            let top0 = new_positions[a];
            let top1 = new_positions[b];

            let side_positions = [bottom0, bottom1, top1, top0];

            if let Some(cyl_id) = cylinder_face_id {
                let n0 = radial_normals[a];
                let n1 = radial_normals[b];

                let base = self.vertices.len() as u32;
                let v = |pos: [f32; 3], norm: Vec3| Vertex {
                    position: pos, normal: norm.into(), face_id: cyl_id, _pad: 0,
                };
                self.vertices.push(v(bottom0, n0));
                self.vertices.push(v(bottom1, n1));
                self.vertices.push(v(top1, n1));
                self.vertices.push(v(top0, n0));
                self.indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
            } else {
                let edge_h = Vec3::from(top0) - Vec3::from(bottom0);
                let edge_w = Vec3::from(bottom1) - Vec3::from(bottom0);
                let side_normal = edge_w.cross(edge_h).normalize_or_zero();

                let side_face_id = self
                    .find_coplanar_adjacent_face(side_normal, &side_positions)
                    .unwrap_or_else(|| {
                        let id = self.next_face_id;
                        self.next_face_id += 1;
                        id
                    });

                // Invalidate stored boundary for merged face — the old boundary
                // is stale after absorbing the new quad. face_boundary_corners()
                // will fall back to angle-sort (correct for convex merged faces).
                self.stored_boundaries.remove(&side_face_id);

                self.push_quad(side_positions, side_normal, side_face_id);
            }
        }
    }

    /// Cut into a face along its negative normal by `depth` (pocket).
    pub fn cut_face(&mut self, face_id: u32, depth: f32) -> Option<u32> {
        let normal = self.face_normal(face_id)?;
        let offset = normal * (-depth);

        let corners = self.face_boundary_corners(face_id)?;
        let n = corners.len();
        if n < 3 { return None; }

        let old_positions: Vec<[f32; 3]> = corners.iter().map(|c| (*c).into()).collect();

        // Save original triangles, then remove from index buffer
        let floor_tris: Vec<[u32; 3]> = self.indices.chunks_exact(3)
            .filter(|c| self.vertices[c[0] as usize].face_id == face_id)
            .map(|c| [c[0], c[1], c[2]])
            .collect();
        self.remove_face_indices(face_id);

        // Move original vertices to floor position
        let face_verts = self.face_vertex_indices(face_id);
        let floor_face_id = self.next_face_id;
        self.next_face_id += 1;
        for &vi in &face_verts {
            let pos = Vec3::from(self.vertices[vi].position) + offset;
            self.vertices[vi].position = pos.into();
            self.vertices[vi].normal = normal.into();
            self.vertices[vi].face_id = floor_face_id;
        }

        // Re-add floor triangles
        for tri in &floor_tris {
            self.indices.extend_from_slice(&[tri[0], tri[1], tri[2]]);
        }

        // Transfer stored boundary to floor face
        if let Some(boundary) = self.stored_boundaries.remove(&face_id) {
            let floor_boundary: Vec<Vec3> = boundary.iter().map(|p| *p + offset).collect();
            self.stored_boundaries.insert(floor_face_id, floor_boundary);
        }

        // Transfer stored holes to floor face
        let hole_boundaries = self.stored_holes.remove(&face_id);
        if let Some(ref holes) = hole_boundaries {
            let floor_holes: Vec<Vec<Vec3>> = holes.iter()
                .map(|hole| hole.iter().map(|p| *p + offset).collect())
                .collect();
            self.stored_holes.insert(floor_face_id, floor_holes);
        }

        let new_positions: Vec<[f32; 3]> = old_positions.iter()
            .map(|p| (Vec3::from(*p) + offset).into())
            .collect();

        // Create outer boundary walls (cut walls face inward, so reverse=true)
        self.create_side_walls(&old_positions, &new_positions, normal, true);

        // Create inner (hole) walls — these face outward (reverse=false) since it's a cut
        if let Some(ref holes) = hole_boundaries {
            for hole in holes {
                let hole_old: Vec<[f32; 3]> = hole.iter().map(|p| (*p).into()).collect();
                let hole_new: Vec<[f32; 3]> = hole.iter().map(|p| (*p + offset).into()).collect();
                self.create_side_walls(&hole_old, &hole_new, normal, false);
            }
        }

        Some(floor_face_id)
    }

    /// Delete a face and compact the mesh.
    pub fn delete_face(&mut self, face_id: u32) -> bool {
        self.stored_boundaries.remove(&face_id);
        self.stored_holes.remove(&face_id);
        let mut new_indices: Vec<u32> = Vec::new();
        for chunk in self.indices.chunks_exact(3) {
            if self.vertices[chunk[0] as usize].face_id != face_id {
                new_indices.extend_from_slice(chunk);
            }
        }

        if new_indices.len() == self.indices.len() {
            return false;
        }

        let mut used = vec![false; self.vertices.len()];
        for &idx in &new_indices {
            used[idx as usize] = true;
        }

        let mut remap = vec![0u32; self.vertices.len()];
        let mut new_verts = Vec::new();
        for (old_idx, vertex) in self.vertices.iter().enumerate() {
            if used[old_idx] {
                remap[old_idx] = new_verts.len() as u32;
                new_verts.push(*vertex);
            }
        }

        for idx in &mut new_indices {
            *idx = remap[*idx as usize];
        }

        self.vertices = new_verts;
        self.indices = new_indices;
        true
    }

    /// Inset a face: shrink boundary, create inner face + connecting quads.
    pub fn inset_face(&mut self, face_id: u32, amount: f32) -> Option<u32> {
        let normal = self.face_normal(face_id)?;
        let corners = self.face_boundary_corners(face_id)?;
        let n = corners.len();
        if n < 3 { return None; }

        let center: Vec3 = corners.iter().copied().sum::<Vec3>() / n as f32;

        let inner_corners: Vec<Vec3> = corners.iter().map(|c| {
            let dir = (center - *c).normalize_or_zero();
            *c + dir * amount
        }).collect();

        self.delete_face(face_id);

        // Inner face
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
        let tris = super::triangulate_3d_polygon(&inner_corners, normal);
        for tri in &tris {
            self.indices.push(inner_base + tri[0] as u32);
            self.indices.push(inner_base + tri[1] as u32);
            self.indices.push(inner_base + tri[2] as u32);
        }

        // Store boundary for inner face
        self.stored_boundaries.insert(inner_face_id, inner_corners.clone());

        // Connecting quads
        for i in 0..n {
            let j = (i + 1) % n;
            let quad_face_id = self.next_face_id;
            self.next_face_id += 1;
            self.push_quad(
                [corners[i].into(), corners[j].into(), inner_corners[j].into(), inner_corners[i].into()],
                normal,
                quad_face_id,
            );
        }

        Some(inner_face_id)
    }

    /// Add a polygon face, offset along normal to prevent z-fighting with parent.
    #[allow(dead_code)]
    pub fn add_polygon_face(&mut self, points: &[Vec3], normal: Vec3) -> u32 {
        self.add_polygon_face_inner(points, normal, normal * 0.008)
    }

    /// Add a polygon face flush with the surface (no z-offset).
    /// Use when the parent face has been deleted so there's nothing to z-fight with.
    #[allow(dead_code)]
    pub fn add_polygon_face_flush(&mut self, points: &[Vec3], normal: Vec3) -> u32 {
        self.add_polygon_face_inner(points, normal, Vec3::ZERO)
    }

    fn add_polygon_face_inner(&mut self, points: &[Vec3], normal: Vec3, offset: Vec3) -> u32 {
        let face_id = self.next_face_id;
        self.next_face_id += 1;

        if points.len() < 3 { return face_id; }

        let base_idx = self.vertices.len() as u32;

        for p in points {
            self.vertices.push(Vertex {
                position: (*p + offset).into(),
                normal: normal.into(),
                face_id,
                _pad: 0,
            });
        }

        // Ear-clipping triangulation (handles concave polygons correctly)
        let offset_points: Vec<Vec3> = points.iter().map(|p| *p + offset).collect();
        let tris = super::triangulate_3d_polygon(&offset_points, normal);
        for tri in &tris {
            self.indices.push(base_idx + tri[0] as u32);
            self.indices.push(base_idx + tri[1] as u32);
            self.indices.push(base_idx + tri[2] as u32);
        }

        // Store original boundary ordering for concave polygon support.
        // face_boundary_corners() will use this instead of angle-sorting.
        let stored: Vec<Vec3> = points.iter().map(|p| *p + offset).collect();
        self.stored_boundaries.insert(face_id, stored);

        face_id
    }

    /// Add a polygon face with holes (for region-with-hole shapes like washers).
    pub fn add_polygon_face_with_holes_flush(
        &mut self,
        outer: &[Vec3],
        holes: &[Vec<Vec3>],
        normal: Vec3,
    ) -> u32 {
        let face_id = self.next_face_id;
        self.next_face_id += 1;

        if outer.len() < 3 { return face_id; }

        let base_idx = self.vertices.len() as u32;

        // Push outer boundary vertices
        for p in outer {
            self.vertices.push(Vertex {
                position: (*p).into(),
                normal: normal.into(),
                face_id,
                _pad: 0,
            });
        }

        // Push hole vertices
        for hole in holes {
            for p in hole {
                self.vertices.push(Vertex {
                    position: (*p).into(),
                    normal: normal.into(),
                    face_id,
                    _pad: 0,
                });
            }
        }

        // Triangulate with earcutr (project to 2D for triangulation)
        let tris = super::triangulate_3d_polygon_with_holes(outer, holes, normal);
        for tri in &tris {
            self.indices.push(base_idx + tri[0] as u32);
            self.indices.push(base_idx + tri[1] as u32);
            self.indices.push(base_idx + tri[2] as u32);
        }

        // Store outer boundary for face_boundary_corners
        self.stored_boundaries.insert(face_id, outer.to_vec());

        // Store hole boundaries for extrude/cut to create inner walls
        if !holes.is_empty() {
            self.stored_holes.insert(face_id, holes.to_vec());
        }

        face_id
    }

    // --- Helpers ---

    /// Remove all triangles of a face from the index buffer WITHOUT compacting vertices.
    fn remove_face_indices(&mut self, face_id: u32) {
        let mut new_indices = Vec::with_capacity(self.indices.len());
        for chunk in self.indices.chunks_exact(3) {
            if chunk.len() == 3 && self.vertices[chunk[0] as usize].face_id != face_id {
                new_indices.extend_from_slice(chunk);
            }
        }
        self.indices = new_indices;
    }

    fn push_quad(&mut self, positions: [[f32; 3]; 4], normal: Vec3, face_id: u32) {
        let base = self.vertices.len() as u32;
        for pos in &positions {
            self.vertices.push(Vertex {
                position: *pos,
                normal: normal.into(),
                face_id,
                _pad: 0,
            });
        }
        self.indices.push(base);
        self.indices.push(base + 1);
        self.indices.push(base + 2);
        self.indices.push(base);
        self.indices.push(base + 2);
        self.indices.push(base + 3);
    }

    // --- Translate operations ---

    /// Translate all vertices belonging to a face by a delta vector.
    /// This moves the face in world space without changing its shape.
    #[allow(dead_code)]
    pub fn translate_face(&mut self, face_id: u32, delta: Vec3) {
        for v in &mut self.vertices {
            if v.face_id == face_id {
                let pos = Vec3::from(v.position) + delta;
                v.position = pos.into();
            }
        }

        if let Some(boundary) = self.stored_boundaries.get_mut(&face_id) {
            for p in boundary.iter_mut() {
                *p += delta;
            }
        }

        if let Some(holes) = self.stored_holes.get_mut(&face_id) {
            for hole in holes.iter_mut() {
                for p in hole.iter_mut() {
                    *p += delta;
                }
            }
        }
    }

    /// Translate multiple faces by the same delta.
    /// More efficient than calling translate_face in a loop (single pass over vertices).
    #[allow(dead_code)]
    pub fn translate_faces(&mut self, face_ids: &[u32], delta: Vec3) {
        let id_set: std::collections::HashSet<u32> = face_ids.iter().copied().collect();

        for v in &mut self.vertices {
            if id_set.contains(&v.face_id) {
                let pos = Vec3::from(v.position) + delta;
                v.position = pos.into();
            }
        }

        for &fid in face_ids {
            if let Some(boundary) = self.stored_boundaries.get_mut(&fid) {
                for p in boundary.iter_mut() {
                    *p += delta;
                }
            }

            if let Some(holes) = self.stored_holes.get_mut(&fid) {
                for hole in holes.iter_mut() {
                    for p in hole.iter_mut() {
                        *p += delta;
                    }
                }
            }
        }
    }

    /// Compute the centroid (center point) of a face.
    /// Returns the average position of all vertices belonging to this face.
    #[allow(dead_code)]
    pub fn face_centroid(&self, face_id: u32) -> Option<Vec3> {
        let mut sum = Vec3::ZERO;
        let mut count = 0u32;
        for v in &self.vertices {
            if v.face_id == face_id {
                sum += Vec3::from(v.position);
                count += 1;
            }
        }
        if count == 0 { None } else { Some(sum / count as f32) }
    }

    /// Compute the centroid of multiple faces.
    #[allow(dead_code)]
    pub fn faces_centroid(&self, face_ids: &[u32]) -> Option<Vec3> {
        let id_set: std::collections::HashSet<u32> = face_ids.iter().copied().collect();
        let mut sum = Vec3::ZERO;
        let mut count = 0u32;
        for v in &self.vertices {
            if id_set.contains(&v.face_id) {
                sum += Vec3::from(v.position);
                count += 1;
            }
        }
        if count == 0 { None } else { Some(sum / count as f32) }
    }
}
