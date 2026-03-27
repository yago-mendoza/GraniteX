// Picking — CPU raycasting for face selection.
// Uses Möller-Trumbore ray-triangle intersection.
// Reads face_id from vertex data so it works with any mesh topology.

use glam::{Mat4, Vec3, Vec4};

use super::mesh::Mesh;

#[derive(Debug, Clone, Copy)]
pub struct PickResult {
    pub face_id: u32,
    pub distance: f32,
    #[allow(dead_code)]
    pub hit_point: Vec3,
}

/// Cast a ray from screen coordinates and find the nearest face.
pub fn pick_face(
    screen_x: f32,
    screen_y: f32,
    screen_width: f32,
    screen_height: f32,
    view_proj: Mat4,
    mesh: &Mesh,
) -> Option<PickResult> {
    // Guard against zero-size viewport (crash bugs 1-2)
    if screen_width < 1.0 || screen_height < 1.0 { return None; }

    let ndc_x = (screen_x / screen_width) * 2.0 - 1.0;
    let ndc_y = 1.0 - (screen_y / screen_height) * 2.0;

    let inv_vp = view_proj.inverse();

    // Unproject near/far with proper homogeneous divide
    let near_h = inv_vp * Vec4::new(ndc_x, ndc_y, 0.0, 1.0);
    let far_h = inv_vp * Vec4::new(ndc_x, ndc_y, 1.0, 1.0);
    if near_h.w.abs() < 1e-10 || far_h.w.abs() < 1e-10 { return None; }
    let near = near_h.truncate() / near_h.w;
    let far = far_h.truncate() / far_h.w;

    let ray_origin = near;
    let ray_dir = (far - near).normalize_or_zero();
    if ray_dir.length_squared() < 1e-10 { return None; }

    let mut best: Option<PickResult> = None;
    let indices = &mesh.indices;
    let verts = &mesh.vertices;

    for tri in indices.chunks_exact(3) {
        let i0 = tri[0] as usize;
        let i1 = tri[1] as usize;
        let i2 = tri[2] as usize;
        if i0 >= verts.len() || i1 >= verts.len() || i2 >= verts.len() { continue; }

        let v0 = Vec3::from(verts[i0].position);
        let v1 = Vec3::from(verts[i1].position);
        let v2 = Vec3::from(verts[i2].position);

        // Test both windings — required for two-sided picking in CAD
        // (cut pocket walls face inward, but still need to be selectable)
        let t = ray_triangle_intersect(ray_origin, ray_dir, v0, v1, v2)
            .or_else(|| ray_triangle_intersect(ray_origin, ray_dir, v0, v2, v1));

        if let Some(t) = t {
            if t > 0.0 {
                // Read face_id from the vertex — this is the ground truth
                let face_id = verts[i0].face_id;

                if best.as_ref().map_or(true, |b| t < b.distance) {
                    best = Some(PickResult {
                        face_id,
                        distance: t,
                        hit_point: ray_origin + ray_dir * t,
                    });
                }
            }
        }
    }

    best
}

/// Möller–Trumbore ray-triangle intersection.
fn ray_triangle_intersect(
    ray_origin: Vec3,
    ray_dir: Vec3,
    v0: Vec3,
    v1: Vec3,
    v2: Vec3,
) -> Option<f32> {
    let edge1 = v1 - v0;
    let edge2 = v2 - v0;
    let h = ray_dir.cross(edge2);
    let a = edge1.dot(h);

    if a.abs() < 1e-6 {
        return None;
    }

    let f = 1.0 / a;
    let s = ray_origin - v0;
    let u = f * s.dot(h);

    if !(0.0..=1.0).contains(&u) {
        return None;
    }

    let q = s.cross(edge1);
    let v = f * ray_dir.dot(q);

    if v < 0.0 || u + v > 1.0 {
        return None;
    }

    let t = f * edge2.dot(q);
    if t > 1e-6 { Some(t) } else { None }
}

/// Pick the nearest mesh boundary edge by screen-space distance.
/// Returns the two endpoints of the closest edge, or None if nothing is within threshold.
pub fn pick_edge(
    screen_x: f32,
    screen_y: f32,
    screen_width: f32,
    screen_height: f32,
    view_proj: Mat4,
    mesh: &Mesh,
    threshold_px: f32,
) -> Option<(Vec3, Vec3)> {
    use std::collections::{HashMap, HashSet};

    // Project a 3D point to screen space
    let project = |p: Vec3| -> Option<(f32, f32)> {
        let clip = view_proj * p.extend(1.0);
        if clip.w.abs() < 1e-6 { return None; }
        let ndc = clip.truncate() / clip.w;
        let sx = (ndc.x * 0.5 + 0.5) * screen_width;
        let sy = (0.5 - ndc.y * 0.5) * screen_height;
        Some((sx, sy))
    };

    // Distance from point to line segment in 2D
    let point_to_segment_dist = |px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32| -> f32 {
        let dx = bx - ax;
        let dy = by - ay;
        let len_sq = dx * dx + dy * dy;
        if len_sq < 1e-12 { return ((px - ax).powi(2) + (py - ay).powi(2)).sqrt(); }
        let t = ((px - ax) * dx + (py - ay) * dy) / len_sq;
        let t = t.clamp(0.0, 1.0);
        let cx = ax + t * dx;
        let cy = ay + t * dy;
        ((px - cx).powi(2) + (py - cy).powi(2)).sqrt()
    };

    // Collect boundary edges (same logic as pipeline edge extraction)
    type PosKey = (i64, i64, i64);
    let pos_key = |p: [f32; 3]| -> PosKey {
        ((p[0] * 10000.0).round() as i64,
         (p[1] * 10000.0).round() as i64,
         (p[2] * 10000.0).round() as i64)
    };
    type EdgeKey = (PosKey, PosKey);
    let edge_key = |a: [f32; 3], b: [f32; 3]| -> EdgeKey {
        let ka = pos_key(a);
        let kb = pos_key(b);
        if ka <= kb { (ka, kb) } else { (kb, ka) }
    };

    struct EdgeInfo {
        faces: HashSet<u32>,
        tri_count: u32,
        pa: [f32; 3],
        pb: [f32; 3],
    }
    let mut edge_faces: HashMap<EdgeKey, EdgeInfo> = HashMap::new();

    for tri in mesh.indices.chunks_exact(3) {
        let (t0, t1, t2) = (tri[0] as usize, tri[1] as usize, tri[2] as usize);
        if t0 >= mesh.vertices.len() || t1 >= mesh.vertices.len() || t2 >= mesh.vertices.len() { continue; }
        let face_id = mesh.vertices[t0].face_id;
        for &(a, b) in &[(t0, t1), (t1, t2), (t2, t0)] {
            let pa = mesh.vertices[a].position;
            let pb = mesh.vertices[b].position;
            let key = edge_key(pa, pb);
            edge_faces.entry(key)
                .and_modify(|info| { info.faces.insert(face_id); info.tri_count += 1; })
                .or_insert_with(|| {
                    let mut s = HashSet::new();
                    s.insert(face_id);
                    EdgeInfo { faces: s, tri_count: 1, pa, pb }
                });
        }
    }

    // Compute the depth of the nearest face at the click position for occlusion
    let face_hit = pick_face(screen_x, screen_y, screen_width, screen_height, view_proj, mesh);
    let max_depth = face_hit.map(|h| h.distance + 0.1).unwrap_or(f32::MAX);

    // Compute camera position from inverse view_proj
    let inv_vp = view_proj.inverse();
    let cam_h = inv_vp * Vec4::new(0.0, 0.0, 0.0, 1.0);
    if cam_h.w.abs() < 1e-10 { return None; }
    let cam_pos = cam_h.truncate() / cam_h.w;

    let mut best: Option<(f32, Vec3, Vec3)> = None;

    for info in edge_faces.values() {
        let is_boundary = info.faces.len() > 1 || info.tri_count == 1;
        if !is_boundary { continue; }

        let p0 = Vec3::from(info.pa);
        let p1 = Vec3::from(info.pb);

        // Depth check: edge midpoint should be closer than the face behind it
        let mid = (p0 + p1) * 0.5;
        let edge_dist = (mid - cam_pos).length();
        if edge_dist > max_depth { continue; }

        let Some((sx0, sy0)) = project(p0) else { continue };
        let Some((sx1, sy1)) = project(p1) else { continue };

        let dist = point_to_segment_dist(screen_x, screen_y, sx0, sy0, sx1, sy1);
        if dist < threshold_px {
            if best.as_ref().map_or(true, |b| dist < b.0) {
                best = Some((dist, p0, p1));
            }
        }
    }

    best.map(|(_, p0, p1)| (p0, p1))
}
