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

        let face_verts = self.face_vertex_indices(face_id);
        let old_positions: Vec<[f32; 3]> = corners.iter().map(|c| (*c).into()).collect();

        let cap_face_id = self.next_face_id;
        self.next_face_id += 1;
        for &vi in &face_verts {
            let pos = Vec3::from(self.vertices[vi].position) + offset;
            self.vertices[vi].position = pos.into();
            self.vertices[vi].face_id = cap_face_id;
        }

        let new_positions: Vec<[f32; 3]> = old_positions.iter()
            .map(|p| (Vec3::from(*p) + offset).into())
            .collect();

        // For cylindrical extrusions (many sides), use a single face_id with smooth normals.
        // For simple extrusions (4 sides = box), use individual face_ids with coplanar merging.
        let is_cylindrical = n > 6;
        let cylinder_face_id = if is_cylindrical {
            let id = self.next_face_id;
            self.next_face_id += 1;
            Some(id)
        } else {
            None
        };

        // Compute center of the face (for radial normals on cylinders)
        let center = old_positions.iter()
            .map(|p| Vec3::from(*p))
            .sum::<Vec3>() / n as f32;

        for i in 0..n {
            let j = (i + 1) % n;

            let bottom0 = old_positions[i];
            let bottom1 = old_positions[j];
            let top0 = new_positions[i];
            let top1 = new_positions[j];

            let side_positions = [bottom0, bottom1, top1, top0];

            if let Some(cyl_id) = cylinder_face_id {
                // Smooth normals: radial direction from the extrude axis
                let axis = normal;
                let mid0 = Vec3::from(bottom0);
                let mid1 = Vec3::from(bottom1);
                // Project position onto the axis-perpendicular plane for radial normal
                let radial0 = (mid0 - center - axis * (mid0 - center).dot(axis)).normalize();
                let radial1 = (mid1 - center - axis * (mid1 - center).dot(axis)).normalize();

                let base = self.vertices.len() as u32;
                let v = |pos: [f32; 3], n: Vec3| Vertex {
                    position: pos, normal: n.into(), face_id: cyl_id, _pad: 0,
                };
                self.vertices.push(v(bottom0, radial0));
                self.vertices.push(v(bottom1, radial1));
                self.vertices.push(v(top1, radial1));
                self.vertices.push(v(top0, radial0));
                self.indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
            } else {
                // Flat normals with coplanar face merging
                let edge_h = Vec3::from(top0) - Vec3::from(bottom0);
                let edge_w = Vec3::from(bottom1) - Vec3::from(bottom0);
                let side_normal = edge_w.cross(edge_h).normalize();

                let side_face_id = self
                    .find_coplanar_adjacent_face(side_normal, &side_positions)
                    .unwrap_or_else(|| {
                        let id = self.next_face_id;
                        self.next_face_id += 1;
                        id
                    });

                self.push_quad(side_positions, side_normal, side_face_id);
            }
        }

        Some(cap_face_id)
    }

    /// Cut into a face along its negative normal by `depth` (pocket).
    pub fn cut_face(&mut self, face_id: u32, depth: f32) -> Option<u32> {
        let normal = self.face_normal(face_id)?;
        let offset = normal * (-depth);

        let corners = self.face_boundary_corners(face_id)?;
        let n = corners.len();
        if n < 3 { return None; }

        let face_verts = self.face_vertex_indices(face_id);
        let old_positions: Vec<[f32; 3]> = corners.iter().map(|c| (*c).into()).collect();

        let floor_face_id = self.next_face_id;
        self.next_face_id += 1;
        for &vi in &face_verts {
            let pos = Vec3::from(self.vertices[vi].position) + offset;
            self.vertices[vi].position = pos.into();
            self.vertices[vi].normal = normal.into();
            self.vertices[vi].face_id = floor_face_id;
        }

        let new_positions: Vec<[f32; 3]> = old_positions.iter()
            .map(|p| (Vec3::from(*p) + offset).into())
            .collect();

        let is_cylindrical = n > 6;
        let cylinder_face_id = if is_cylindrical {
            let id = self.next_face_id;
            self.next_face_id += 1;
            Some(id)
        } else {
            None
        };

        let center = old_positions.iter()
            .map(|p| Vec3::from(*p))
            .sum::<Vec3>() / n as f32;

        for i in 0..n {
            let j = (i + 1) % n;

            let top0 = old_positions[i];
            let top1 = old_positions[j];
            let bot0 = new_positions[i];
            let bot1 = new_positions[j];

            if let Some(cyl_id) = cylinder_face_id {
                let mid0 = Vec3::from(top0);
                let mid1 = Vec3::from(top1);
                let radial0 = (mid0 - center - normal * (mid0 - center).dot(normal)).normalize();
                let radial1 = (mid1 - center - normal * (mid1 - center).dot(normal)).normalize();
                // Inward-facing for cut
                let n0 = -radial0;
                let n1 = -radial1;

                let base = self.vertices.len() as u32;
                let v = |pos: [f32; 3], norm: Vec3| Vertex {
                    position: pos, normal: norm.into(), face_id: cyl_id, _pad: 0,
                };
                self.vertices.push(v(top0, n0));
                self.vertices.push(v(top1, n1));
                self.vertices.push(v(bot1, n1));
                self.vertices.push(v(bot0, n0));
                self.indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
            } else {
                let edge_h = Vec3::from(bot0) - Vec3::from(top0);
                let edge_w = Vec3::from(top1) - Vec3::from(top0);
                let side_normal = edge_h.cross(edge_w).normalize();

                let wall_face_id = self.next_face_id;
                self.next_face_id += 1;

                self.push_quad([top0, top1, bot1, bot0], side_normal, wall_face_id);
            }
        }

        Some(floor_face_id)
    }

    /// Delete a face and compact the mesh.
    pub fn delete_face(&mut self, face_id: u32) -> bool {
        let mut new_indices: Vec<u32> = Vec::new();
        for chunk in self.indices.chunks(3) {
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
        for i in 1..(n as u32 - 1) {
            self.indices.push(inner_base);
            self.indices.push(inner_base + i);
            self.indices.push(inner_base + i + 1);
        }

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

    /// Add a polygon face, slightly offset along normal to prevent z-fighting.
    pub fn add_polygon_face(&mut self, points: &[Vec3], normal: Vec3) -> u32 {
        self.add_polygon_face_inner(points, normal, normal * 0.0003)
    }

    /// Add a polygon face flush with the surface (no z-offset).
    /// Use when the parent face has been deleted so there's nothing to z-fight with.
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

        for i in 1..(points.len() as u32 - 1) {
            self.indices.push(base_idx);
            self.indices.push(base_idx + i);
            self.indices.push(base_idx + i + 1);
        }

        face_id
    }

    /// Compute boundary edges for line rendering.
    #[allow(dead_code)]
    /// Extract boundary edges — edges where adjacent triangles have DIFFERENT face_ids.
    /// Uses position-based hashing (not vertex indices) because faces don't share vertices.
    pub fn boundary_edges(&self) -> Vec<[f32; 3]> {
        use std::collections::HashMap;

        // Hash a position to a canonical integer key for dedup.
        let hash_pos = |p: [f32; 3]| -> i64 {
            let x = (p[0] * 10000.0).round() as i64;
            let y = (p[1] * 10000.0).round() as i64;
            let z = (p[2] * 10000.0).round() as i64;
            x.wrapping_mul(73856093) ^ y.wrapping_mul(19349663) ^ z.wrapping_mul(83492791)
        };

        // Map: (pos_hash_a, pos_hash_b) → set of face_ids touching this edge.
        // Also store the actual positions for rendering.
        let mut edge_data: HashMap<(i64, i64), (Vec<u32>, [f32; 3], [f32; 3])> = HashMap::new();

        for chunk in self.indices.chunks(3) {
            if chunk.len() < 3 { continue; }
            let face_id = self.vertices[chunk[0] as usize].face_id;

            let idx = [chunk[0] as usize, chunk[1] as usize, chunk[2] as usize];
            for &(ai, bi) in &[(idx[0], idx[1]), (idx[1], idx[2]), (idx[2], idx[0])] {
                let pa = self.vertices[ai].position;
                let pb = self.vertices[bi].position;
                let ha = hash_pos(pa);
                let hb = hash_pos(pb);
                let key = if ha <= hb { (ha, hb) } else { (hb, ha) };

                edge_data.entry(key)
                    .and_modify(|(faces, _, _)| {
                        if !faces.contains(&face_id) {
                            faces.push(face_id);
                        }
                    })
                    .or_insert((vec![face_id], pa, pb));
            }
        }

        let mut lines = Vec::new();
        for (_, (faces, pa, pb)) in &edge_data {
            // Boundary = edge touching 2+ different faces, or edge on mesh boundary (1 face)
            let is_boundary = faces.len() != 1 || {
                // Check if this is a mesh boundary (silhouette) edge
                // For now, only draw edges between different faces
                false
            };
            if is_boundary && faces.len() > 1 {
                lines.push(*pa);
                lines.push(*pb);
            }
        }
        lines
    }

    // --- Helpers ---

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
}
