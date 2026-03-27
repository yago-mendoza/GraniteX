// Mesh — dynamic geometry with face-aware operations.
//
// Key concept: a "face" is a planar region, not a single quad.
// When extruding, new side quads that are coplanar with existing faces
// get merged into the same face (same face_id). This matches SolidWorks
// behavior where extruding the top of a cube gives 6 faces, not 10.

mod ops;
mod smooth;

use glam::Vec3;
use super::vertex::Vertex;

pub(crate) const COPLANAR_THRESHOLD: f32 = 0.999;
pub(crate) const EDGE_EPSILON: f32 = 1e-4;

/// Triangulate a 3D polygon using ear clipping (earcutr).
/// Projects points onto the polygon's plane for 2D triangulation.
/// Returns triangle indices into the input points array.
/// Falls back to fan triangulation for convex-only if earcutr fails.
pub fn triangulate_3d_polygon(points: &[Vec3], normal: Vec3) -> Vec<[usize; 3]> {
    if points.len() < 3 || normal.length_squared() < 1e-10 { return Vec::new(); }

    // Build 2D projection axes on the polygon plane
    let u_axis = if normal.dot(Vec3::Y).abs() < 0.99 {
        normal.cross(Vec3::Y).normalize()
    } else {
        normal.cross(Vec3::X).normalize()
    };
    let v_axis = normal.cross(u_axis).normalize();
    let origin = points[0];

    // Project to 2D
    let coords_2d: Vec<f64> = points.iter()
        .flat_map(|p| {
            let d = *p - origin;
            [d.dot(u_axis) as f64, d.dot(v_axis) as f64]
        })
        .collect();

    match earcutr::earcut(&coords_2d, &[], 2) {
        Ok(indices) => {
            indices.chunks_exact(3)
                .map(|tri| [tri[0], tri[1], tri[2]])
                .collect()
        }
        Err(_) => {
            // Fallback: fan (convex only)
            (1..points.len() - 1)
                .map(|i| [0, i, i + 1])
                .collect()
        }
    }
}

/// Triangulate a 3D polygon with holes by projecting to 2D.
pub fn triangulate_3d_polygon_with_holes(outer: &[Vec3], holes: &[Vec<Vec3>], normal: Vec3) -> Vec<[usize; 3]> {
    if outer.len() < 3 || normal.length_squared() < 1e-10 { return Vec::new(); }

    let u_axis = if normal.dot(Vec3::Y).abs() < 0.99 {
        normal.cross(Vec3::Y).normalize()
    } else {
        normal.cross(Vec3::X).normalize()
    };
    let v_axis = normal.cross(u_axis).normalize();
    let origin = outer[0];

    let project = |p: Vec3| {
        let d = p - origin;
        [d.dot(u_axis) as f64, d.dot(v_axis) as f64]
    };

    // Outer boundary
    let mut coords: Vec<f64> = outer.iter().flat_map(|p| project(*p)).collect();

    // Holes
    let mut hole_indices: Vec<usize> = Vec::new();
    for hole in holes {
        hole_indices.push(coords.len() / 2);
        for p in hole {
            let [u, v] = project(*p);
            coords.push(u);
            coords.push(v);
        }
    }

    match earcutr::earcut(&coords, &hole_indices, 2) {
        Ok(indices) => {
            indices.chunks_exact(3)
                .map(|tri| [tri[0], tri[1], tri[2]])
                .collect()
        }
        Err(_) => {
            (1..outer.len() - 1)
                .map(|i| [0, i, i + 1])
                .collect()
        }
    }
}

pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub(super) next_face_id: u32,
    /// Stored boundary orderings for faces created from ordered point lists
    /// (e.g., sketch regions with concave shapes). Keyed by face_id.
    /// face_boundary_corners() checks this first before angle-sorting.
    stored_boundaries: std::collections::HashMap<u32, Vec<Vec3>>,
    /// Stored hole boundaries for faces with holes (e.g., outer region with punched shapes).
    /// Each face_id maps to a list of hole boundaries (each hole is a Vec of 3D points).
    stored_holes: std::collections::HashMap<u32, Vec<Vec<Vec3>>>,
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

        // Winding: all faces CCW when viewed from outside (outward normal = cross(e1,e2)).
        #[rustfmt::skip]
        let indices: Vec<u32> = vec![
            0,  1,  2,  0,  2,  3,    // Front  (+Z): outward = (0,0,1)  ✓
            4,  6,  5,  4,  7,  6,    // Back   (-Z): outward = (0,0,-1) ✓
            8,  10, 9,  8,  11, 10,   // Top    (+Y): outward = (0,1,0)  ✓ (was inverted)
            12, 13, 14, 12, 14, 15,   // Bottom (-Y): outward = (0,-1,0) ✓ (was inverted)
            16, 17, 18, 16, 18, 19,   // Right  (+X): outward = (1,0,0)  ✓
            20, 22, 21, 20, 23, 22,   // Left   (-X): outward = (-1,0,0) ✓
        ];

        Self { vertices, indices, next_face_id: 6, stored_boundaries: std::collections::HashMap::new(), stored_holes: std::collections::HashMap::new() }
    }

    #[allow(dead_code)]
    pub fn empty() -> Self {
        Self { vertices: Vec::new(), indices: Vec::new(), next_face_id: 0, stored_boundaries: std::collections::HashMap::new(), stored_holes: std::collections::HashMap::new() }
    }

    /// Construct mesh from raw triangle data (for importers).
    /// Applies smooth shading: merges coplanar faces, averages normals, welds vertices.
    pub fn from_triangles(positions: &[Vec3], _normals: &[Vec3], indices: &[u32]) -> Self {
        let mut vertices = Vec::with_capacity(indices.len());
        let mut out_indices = Vec::with_capacity(indices.len());
        let mut face_id = 0u32;

        for tri in indices.chunks_exact(3) {
            let i0 = tri[0] as usize;
            let i1 = tri[1] as usize;
            let i2 = tri[2] as usize;

            // Compute geometric face normal (ignore file normals — we recompute)
            let e1 = positions[i1] - positions[i0];
            let e2 = positions[i2] - positions[i0];
            let normal = e1.cross(e2).normalize_or_zero();

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

        let mut mesh = Self { vertices, indices: out_indices, next_face_id: face_id, stored_boundaries: std::collections::HashMap::new(), stored_holes: std::collections::HashMap::new() };
        mesh.apply_smooth_shading(30.0);
        mesh
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
        for chunk in self.indices.chunks_exact(3) {
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
        // If we have a stored boundary (from region-created faces), use it directly.
        // This preserves correct ordering for concave polygons.
        if let Some(boundary) = self.stored_boundaries.get(&face_id) {
            return Some(boundary.clone());
        }

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

        // Sort by angle around center for merged/cube faces.
        // This works for convex faces but NOT for concave ones
        // (concave faces should use stored_boundaries instead).
        if positions.is_empty() { return None; }
        let center: Vec3 = positions.iter().copied().sum::<Vec3>() / positions.len() as f32;

        let u_axis = if normal.dot(Vec3::Y).abs() < 0.99 {
            normal.cross(Vec3::Y).normalize_or_zero()
        } else {
            normal.cross(Vec3::X).normalize_or_zero()
        };
        if u_axis.length_squared() < 1e-10 { return None; }
        let v_axis = normal.cross(u_axis).normalize_or_zero();

        positions.sort_by(|a, b| {
            let da = *a - center;
            let db = *b - center;
            let angle_a = da.dot(v_axis).atan2(da.dot(u_axis));
            let angle_b = db.dot(v_axis).atan2(db.dot(u_axis));
            angle_a.partial_cmp(&angle_b).unwrap_or(std::cmp::Ordering::Equal)
        });

        Some(positions)
    }

    /// Construct a mesh from raw data (for project loading).
    pub fn from_raw(
        vertices: Vec<Vertex>,
        indices: Vec<u32>,
        next_face_id: u32,
        stored_boundaries: std::collections::HashMap<u32, Vec<Vec3>>,
        stored_holes: std::collections::HashMap<u32, Vec<Vec<Vec3>>>,
    ) -> Self {
        Self { vertices, indices, next_face_id, stored_boundaries, stored_holes }
    }

    // --- Accessors ---

    pub fn next_face_id(&self) -> u32 {
        self.next_face_id
    }

    pub fn set_next_face_id(&mut self, id: u32) {
        self.next_face_id = id;
    }

    /// Store a face boundary ordering (for concave regions from sketches).
    #[allow(dead_code)]
    pub fn store_boundary(&mut self, face_id: u32, boundary: Vec<Vec3>) {
        self.stored_boundaries.insert(face_id, boundary);
    }

    pub fn stored_boundaries(&self) -> &std::collections::HashMap<u32, Vec<Vec3>> {
        &self.stored_boundaries
    }

    pub fn set_stored_boundaries(&mut self, boundaries: std::collections::HashMap<u32, Vec<Vec3>>) {
        self.stored_boundaries = boundaries;
    }

    pub fn stored_holes(&self) -> &std::collections::HashMap<u32, Vec<Vec<Vec3>>> {
        &self.stored_holes
    }

    pub fn set_stored_holes(&mut self, holes: std::collections::HashMap<u32, Vec<Vec<Vec3>>>) {
        self.stored_holes = holes;
    }

    /// Check if a face is planar (all vertices have approximately the same normal).
    /// Curved faces (cylinders, spheres) return false.
    pub fn is_face_planar(&self, face_id: u32) -> bool {
        let normals: Vec<glam::Vec3> = self.vertices.iter()
            .filter(|v| v.face_id == face_id)
            .map(|v| glam::Vec3::from(v.normal))
            .collect();
        if normals.len() < 2 { return true; }
        let first = normals[0];
        normals.iter().all(|n| n.dot(first).abs() > 0.99)
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

// =============================================================================
// Tests — "triangle-soup era" safety net.
// These protect the current mesh ops during the transition to BrepMesh.
// They will be replaced when the BREP kernel takes over.
// =============================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    fn approx_eq(a: f32, b: f32) -> bool { (a - b).abs() < 1e-4 }
    fn v3_approx_eq(a: Vec3, b: Vec3) -> bool {
        approx_eq(a.x, b.x) && approx_eq(a.y, b.y) && approx_eq(a.z, b.z)
    }

    // ---- Test 1: Cube invariants ----
    #[test]
    fn cube_has_correct_topology() {
        let mesh = Mesh::cube();
        assert_eq!(mesh.face_count(), 6, "cube should have 6 faces");
        assert_eq!(mesh.vertex_count(), 24, "cube should have 24 vertices (4 per face, no sharing)");
        assert_eq!(mesh.triangle_count(), 12, "cube should have 12 triangles (2 per face)");
        assert_eq!(mesh.indices.len() % 3, 0, "index count must be divisible by 3");
    }

    #[test]
    fn cube_normals_point_outward() {
        let mesh = Mesh::cube();
        for face_id in 0..6u32 {
            let normal = mesh.face_normal(face_id).expect("face should exist");
            assert!(approx_eq(normal.length(), 1.0), "normal should be unit length");

            // Normal should point AWAY from origin (outward)
            let center: Vec3 = mesh.vertices.iter()
                .filter(|v| v.face_id == face_id)
                .map(|v| Vec3::from(v.position))
                .sum::<Vec3>() / 4.0;
            assert!(normal.dot(center) > 0.0, "face {} normal should point outward (away from origin)", face_id);
        }
    }

    #[test]
    fn cube_bounding_box() {
        let mesh = Mesh::cube();
        let (min, max) = mesh.bounding_box();
        assert!(v3_approx_eq(min, Vec3::splat(-0.5)));
        assert!(v3_approx_eq(max, Vec3::splat(0.5)));
    }

    // ---- Test 2: Extrude produces correct topology ----
    #[test]
    fn extrude_cube_top_increases_faces() {
        let mut mesh = Mesh::cube();
        let top_face = 2; // top face (Y+)
        let original_faces = mesh.face_count();

        let cap = mesh.extrude_face(top_face, 1.0);
        assert!(cap.is_some(), "extrude should succeed");

        let new_faces = mesh.face_count();
        // Side walls merge with coplanar existing faces, so we get:
        // original 6 - 1 (top becomes cap) + 1 (cap) = still 6 if all 4 sides merge
        // At minimum, the cap face_id should be different from any original
        assert!(new_faces >= original_faces,
            "face count should not decrease: was {}, now {}", original_faces, new_faces);
        assert!(cap.unwrap() != top_face, "cap should have a new face_id");
    }

    #[test]
    fn extrude_moves_bounding_box() {
        let mut mesh = Mesh::cube();
        let top_face = 2;
        mesh.extrude_face(top_face, 1.0);

        let (_, max) = mesh.bounding_box();
        assert!(approx_eq(max.y, 1.5), "top should be at 0.5 + 1.0 = 1.5, got {}", max.y);
    }

    #[test]
    fn extrude_cap_normal_preserved() {
        let mut mesh = Mesh::cube();
        let top_face = 2;
        let original_normal = mesh.face_normal(top_face).unwrap();
        let cap = mesh.extrude_face(top_face, 1.0).unwrap();

        let cap_normal = mesh.face_normal(cap).unwrap();
        assert!(v3_approx_eq(original_normal, cap_normal),
            "cap normal should match original face normal");
    }

    // ---- Test 3: Cut produces correct topology ----
    #[test]
    fn cut_cube_top_preserves_bounding_box() {
        let mut mesh = Mesh::cube();
        let top_face = 2;
        let (orig_min, orig_max) = mesh.bounding_box();

        let floor = mesh.cut_face(top_face, 0.3);
        assert!(floor.is_some(), "cut should succeed");

        // Cut goes inward — bounding box should NOT change (pocket is inside the cube)
        let (min, max) = mesh.bounding_box();
        assert!(v3_approx_eq(min, orig_min), "min shouldn't change after cut");
        assert!(v3_approx_eq(max, orig_max), "max shouldn't change after cut");
    }

    // ---- Test 4: Delete face compacts mesh ----
    #[test]
    fn delete_face_reduces_count() {
        let mut mesh = Mesh::cube();
        assert!(mesh.delete_face(0), "delete should succeed");
        assert_eq!(mesh.face_count(), 5, "should have 5 faces after deleting one");

        // All indices should be in bounds
        for &idx in &mesh.indices {
            assert!((idx as usize) < mesh.vertices.len(),
                "index {} out of bounds (verts: {})", idx, mesh.vertices.len());
        }
    }

    // ---- Test 5: face_boundary_corners returns correct count ----
    #[test]
    fn cube_face_boundary_has_4_corners() {
        let mesh = Mesh::cube();
        for face_id in 0..6u32 {
            let corners = mesh.face_boundary_corners(face_id);
            assert!(corners.is_some(), "face {} should have boundary", face_id);
            let corners = corners.unwrap();
            assert_eq!(corners.len(), 4, "cube face {} should have 4 boundary corners, got {}", face_id, corners.len());
        }
    }

    // ---- Test 6: Indices always valid ----
    #[test]
    fn indices_always_in_bounds() {
        let mut mesh = Mesh::cube();
        // Do several operations
        mesh.extrude_face(2, 0.5);  // extrude top
        mesh.extrude_face(0, 0.3);  // extrude front
        mesh.delete_face(1);         // delete back

        assert_eq!(mesh.indices.len() % 3, 0, "index count must be divisible by 3");
        for &idx in &mesh.indices {
            assert!((idx as usize) < mesh.vertices.len(),
                "index {} >= vertex count {}", idx, mesh.vertices.len());
        }
    }

    // ---- Test 7: Extrude chain doesn't corrupt ----
    #[test]
    fn extrude_chain_3_deep() {
        let mut mesh = Mesh::cube();
        let cap1 = mesh.extrude_face(2, 0.5).expect("extrude 1");
        let cap2 = mesh.extrude_face(cap1, 0.3).expect("extrude 2");
        let cap3 = mesh.extrude_face(cap2, 0.2).expect("extrude 3");

        // Cap should exist and be queryable
        assert!(mesh.face_normal(cap3).is_some(), "cap3 should have a normal");

        // Bounding box should reflect all extrusions
        let (_, max) = mesh.bounding_box();
        assert!(approx_eq(max.y, 0.5 + 0.5 + 0.3 + 0.2),
            "max.y should be 1.5, got {}", max.y);

        // All indices valid
        for &idx in &mesh.indices {
            assert!((idx as usize) < mesh.vertices.len());
        }
    }

    // ---- Test 8: Cylindrical detection ----
    #[test]
    fn is_face_planar_on_cube() {
        let mesh = Mesh::cube();
        for face_id in 0..6u32 {
            assert!(mesh.is_face_planar(face_id), "cube face {} should be planar", face_id);
        }
    }
}
