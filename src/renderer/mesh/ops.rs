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

            let side_face_id = self
                .find_coplanar_adjacent_face(side_normal, &side_positions)
                .unwrap_or_else(|| {
                    let id = self.next_face_id;
                    self.next_face_id += 1;
                    id
                });

            self.push_quad(side_positions, side_normal, side_face_id);
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

        for i in 0..n {
            let j = (i + 1) % n;

            let top0 = old_positions[i];
            let top1 = old_positions[j];
            let bot0 = new_positions[i];
            let bot1 = new_positions[j];

            let edge_h = Vec3::from(bot0) - Vec3::from(top0);
            let edge_w = Vec3::from(top1) - Vec3::from(top0);
            let side_normal = edge_h.cross(edge_w).normalize();

            let wall_face_id = self.next_face_id;
            self.next_face_id += 1;

            self.push_quad([top0, top1, bot1, bot0], side_normal, wall_face_id);
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
        let face_id = self.next_face_id;
        self.next_face_id += 1;

        if points.len() < 3 { return face_id; }

        let base_idx = self.vertices.len() as u32;
        let offset = normal * 0.002;

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
    pub fn boundary_edges(&self) -> Vec<[f32; 3]> {
        use std::collections::HashMap;

        let mut edge_faces: HashMap<(u32, u32), (u32, Option<u32>)> = HashMap::new();

        for chunk in self.indices.chunks(3) {
            if chunk.len() < 3 { continue; }
            let face_id = self.vertices[chunk[0] as usize].face_id;

            for &(a, b) in &[(chunk[0], chunk[1]), (chunk[1], chunk[2]), (chunk[2], chunk[0])] {
                let key = if a < b { (a, b) } else { (b, a) };
                edge_faces.entry(key)
                    .and_modify(|(_, second)| { *second = Some(face_id); })
                    .or_insert((face_id, None));
            }
        }

        let mut lines = Vec::new();
        for ((a, b), (face1, face2)) in &edge_faces {
            let is_boundary = match face2 {
                None => true,
                Some(f2) => *f2 != *face1,
            };
            if is_boundary {
                lines.push(self.vertices[*a as usize].position);
                lines.push(self.vertices[*b as usize].position);
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
