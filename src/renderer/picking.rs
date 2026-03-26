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
    let ndc_x = (screen_x / screen_width) * 2.0 - 1.0;
    let ndc_y = 1.0 - (screen_y / screen_height) * 2.0;

    let inv_vp = view_proj.inverse();

    // Unproject near/far with proper homogeneous divide
    let near_h = inv_vp * Vec4::new(ndc_x, ndc_y, 0.0, 1.0);
    let far_h = inv_vp * Vec4::new(ndc_x, ndc_y, 1.0, 1.0);
    let near = near_h.truncate() / near_h.w;
    let far = far_h.truncate() / far_h.w;

    let ray_origin = near;
    let ray_dir = (far - near).normalize();

    let mut best: Option<PickResult> = None;
    let indices = &mesh.indices;
    let verts = &mesh.vertices;

    for tri_start in (0..indices.len()).step_by(3) {
        let i0 = indices[tri_start] as usize;
        let i1 = indices[tri_start + 1] as usize;
        let i2 = indices[tri_start + 2] as usize;

        let v0 = Vec3::from(verts[i0].position);
        let v1 = Vec3::from(verts[i1].position);
        let v2 = Vec3::from(verts[i2].position);

        // Test both windings (mesh has inconsistent winding on some faces)
        let t = ray_triangle_intersect(ray_origin, ray_dir, v0, v1, v2)
            .or_else(|| ray_triangle_intersect(ray_origin, ray_dir, v0, v2, v1));

        if let Some(t) = t {
            if t > 0.0 {
                // Read face_id from the vertex — this is the ground truth
                let face_id = verts[i0].face_id;

                if best.is_none() || t < best.unwrap().distance {
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
