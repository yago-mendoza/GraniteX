// SketchPlane — defines a 2D coordinate system on a 3D face.
//
// Derived from a face's normal and position. Provides bidirectional
// transforms between 2D sketch coordinates and 3D world positions.
// Also handles ray-plane intersection for mouse picking.

use glam::Vec3;
use super::Point2D;

#[derive(Clone)]
pub struct SketchPlane {
    pub origin: Vec3,
    pub u_axis: Vec3, // local X direction in world space
    pub v_axis: Vec3, // local Y direction in world space
    pub normal: Vec3,  // face normal (local Z, pointing out)
}

impl SketchPlane {
    /// Create a sketch plane from a face's normal and a point on the face.
    /// The U axis is derived by projecting a preferred world axis onto the plane.
    pub fn from_face(normal: Vec3, face_center: Vec3) -> Self {
        let normal = normal.normalize();

        // Pick U axis: project a world axis onto the plane
        let u_candidate = if normal.dot(Vec3::Y).abs() < 0.95 {
            Vec3::Y // prefer Y for horizontal-ish faces
        } else {
            Vec3::X // fallback for vertical faces
        };

        // Project onto plane: remove the normal component
        let u_axis = (u_candidate - normal * u_candidate.dot(normal)).normalize();
        let v_axis = normal.cross(u_axis).normalize();

        Self {
            origin: face_center,
            u_axis,
            v_axis,
            normal,
        }
    }

    /// Convert 3D world position to 2D sketch coordinates.
    pub fn world_to_2d(&self, world_pos: Vec3) -> Point2D {
        let d = world_pos - self.origin;
        Point2D {
            x: d.dot(self.u_axis),
            y: d.dot(self.v_axis),
        }
    }

    /// Convert 2D sketch coordinates to 3D world position.
    pub fn to_3d(&self, p: Point2D) -> Vec3 {
        self.origin + self.u_axis * p.x + self.v_axis * p.y
    }

    /// Intersect a ray with this plane. Returns the 3D hit point if it intersects.
    pub fn ray_intersect(&self, ray_origin: Vec3, ray_dir: Vec3) -> Option<Vec3> {
        let denom = self.normal.dot(ray_dir);
        if denom.abs() < 1e-6 {
            return None; // ray parallel to plane
        }

        let t = self.normal.dot(self.origin - ray_origin) / denom;
        if t < 0.0 {
            return None; // intersection behind ray
        }

        Some(ray_origin + ray_dir * t)
    }
}
