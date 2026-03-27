// Construction geometry — reference planes, axes, and points.
//
// These are the foundation for parametric operations:
//   - Revolve needs an axis
//   - Mirror needs a plane
//   - Patterns need an axis/direction
//   - First sketch needs an origin plane (can't sketch on air)

use glam::Vec3;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ConstructionId {
    Plane(usize),
    Axis(usize),
}

pub struct ReferencePlane {
    pub name: String,
    pub origin: Vec3,
    pub normal: Vec3,
    pub u_axis: Vec3,
    pub v_axis: Vec3,
    pub visible: bool,
    pub color: [f32; 3],
}

pub struct ReferenceAxis {
    pub name: String,
    pub origin: Vec3,
    pub direction: Vec3,
    pub visible: bool,
    pub color: [f32; 3],
}

pub struct ConstructionGeometry {
    pub planes: Vec<ReferencePlane>,
    pub axes: Vec<ReferenceAxis>,
    pub selected: Option<ConstructionId>,
    pub hovered: Option<ConstructionId>,
}

impl ConstructionGeometry {
    pub fn new() -> Self {
        let planes = vec![
            // XY plane (normal = +Z, blue)
            ReferencePlane {
                name: "XY Plane".into(),
                origin: Vec3::ZERO,
                normal: Vec3::Z,
                u_axis: Vec3::X,
                v_axis: Vec3::Y,
                visible: true,
                color: [0.3, 0.3, 0.9],
            },
            // XZ plane (normal = +Y, green)
            ReferencePlane {
                name: "XZ Plane".into(),
                origin: Vec3::ZERO,
                normal: Vec3::Y,
                u_axis: Vec3::X,
                v_axis: Vec3::Z,
                visible: true,
                color: [0.2, 0.8, 0.2],
            },
            // YZ plane (normal = +X, red)
            ReferencePlane {
                name: "YZ Plane".into(),
                origin: Vec3::ZERO,
                normal: Vec3::X,
                u_axis: Vec3::Y,
                v_axis: Vec3::Z,
                visible: true,
                color: [0.9, 0.2, 0.2],
            },
        ];

        let axes = vec![
            ReferenceAxis {
                name: "X Axis".into(),
                origin: Vec3::ZERO,
                direction: Vec3::X,
                visible: true,
                color: [0.9, 0.2, 0.2],
            },
            ReferenceAxis {
                name: "Y Axis".into(),
                origin: Vec3::ZERO,
                direction: Vec3::Y,
                visible: true,
                color: [0.2, 0.8, 0.2],
            },
            ReferenceAxis {
                name: "Z Axis".into(),
                origin: Vec3::ZERO,
                direction: Vec3::Z,
                visible: true,
                color: [0.3, 0.4, 0.9],
            },
        ];

        Self {
            planes,
            axes,
            selected: None,
            hovered: None,
        }
    }

    /// Pick construction geometry with a ray. Returns the closest hit.
    /// `extent` is the rendered size of planes/axes (scales with camera distance).
    pub fn pick(&self, ray_origin: Vec3, ray_dir: Vec3, extent: f32) -> Option<(ConstructionId, f32)> {
        let mut best: Option<(ConstructionId, f32)> = None;

        // Pick planes
        for (i, plane) in self.planes.iter().enumerate() {
            if !plane.visible { continue; }
            if let Some(dist) = self.pick_plane(plane, ray_origin, ray_dir, extent) {
                if best.is_none() || dist < best.unwrap().1 {
                    best = Some((ConstructionId::Plane(i), dist));
                }
            }
        }

        // Pick axes (closer threshold wins over planes)
        for (i, axis) in self.axes.iter().enumerate() {
            if !axis.visible { continue; }
            if let Some(dist) = self.pick_axis(axis, ray_origin, ray_dir, extent) {
                if best.is_none() || dist < best.unwrap().1 {
                    best = Some((ConstructionId::Axis(i), dist));
                }
            }
        }

        best
    }

    fn pick_plane(&self, plane: &ReferencePlane, ray_o: Vec3, ray_d: Vec3, extent: f32) -> Option<f32> {
        let denom = plane.normal.dot(ray_d);
        if denom.abs() < 1e-6 { return None; }

        let t = plane.normal.dot(plane.origin - ray_o) / denom;
        if t < 0.0 { return None; }

        let hit = ray_o + ray_d * t;
        let local = hit - plane.origin;
        let u = local.dot(plane.u_axis);
        let v = local.dot(plane.v_axis);

        // Check if within the rendered quad
        if u.abs() <= extent && v.abs() <= extent {
            Some(t)
        } else {
            None
        }
    }

    fn pick_axis(&self, axis: &ReferenceAxis, ray_o: Vec3, ray_d: Vec3, extent: f32) -> Option<f32> {
        // Closest approach between ray and axis line
        let axis_o = axis.origin;
        let axis_d = axis.direction;

        let w = ray_o - axis_o;
        let a = ray_d.dot(ray_d);
        let b = ray_d.dot(axis_d);
        let c = axis_d.dot(axis_d);
        let d = ray_d.dot(w);
        let e = axis_d.dot(w);

        let denom = a * c - b * b;
        if denom.abs() < 1e-8 { return None; } // parallel

        let t_ray = (b * e - c * d) / denom;
        let t_axis = (a * e - b * d) / denom;

        if t_ray < 0.0 { return None; }
        if t_axis.abs() > extent { return None; } // beyond rendered length

        let closest_ray = ray_o + ray_d * t_ray;
        let closest_axis = axis_o + axis_d * t_axis;
        let distance = (closest_ray - closest_axis).length();

        // Threshold: pick if within ~5% of extent (feels about right for thin lines)
        let threshold = extent * 0.05;
        if distance < threshold {
            Some(t_ray)
        } else {
            None
        }
    }

    /// Get a plane as a SketchPlane for sketching on it.
    pub fn plane_as_sketch_plane(&self, idx: usize) -> Option<crate::sketch::SketchPlane> {
        let p = self.planes.get(idx)?;
        Some(crate::sketch::SketchPlane {
            origin: p.origin,
            u_axis: p.u_axis,
            v_axis: p.v_axis,
            normal: p.normal,
        })
    }
}
