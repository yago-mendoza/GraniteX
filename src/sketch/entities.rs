// Sketch entities — the geometric primitives that can be drawn on a sketch plane.

/// A 2D point in sketch-local coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point2D {
    pub x: f32,
    pub y: f32,
}

impl Point2D {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn distance_to(&self, other: Point2D) -> f32 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }

    /// Closest point on line segment a→b to self.
    pub fn closest_on_segment(&self, a: Point2D, b: Point2D) -> Point2D {
        let ab = Point2D::new(b.x - a.x, b.y - a.y);
        let ap = Point2D::new(self.x - a.x, self.y - a.y);
        let t = (ap.x * ab.x + ap.y * ab.y) / (ab.x * ab.x + ab.y * ab.y + 1e-12);
        let t = t.clamp(0.0, 1.0);
        Point2D::new(a.x + t * ab.x, a.y + t * ab.y)
    }
}

/// A sketch entity — something drawn on the sketch plane.
#[derive(Debug, Clone)]
pub enum SketchEntity {
    Line { start: Point2D, end: Point2D },
    Circle { center: Point2D, radius: f32 },
    /// Construction line — reference geometry, not included in region computation.
    /// Used as revolve axes, dimension references, etc.
    ConstructionLine { start: Point2D, end: Point2D },
}

impl SketchEntity {
    /// Get the control points/endpoints of this entity (for snapping).
    /// For circles, includes center + 4 quadrant points (top/bottom/left/right)
    /// so that lines can connect to the circle's edge.
    pub fn endpoints(&self) -> Vec<Point2D> {
        match self {
            SketchEntity::Line { start, end } => vec![*start, *end],
            SketchEntity::Circle { center, radius } => vec![
                *center,
                Point2D::new(center.x + radius, center.y),  // right (0°)
                Point2D::new(center.x, center.y + radius),  // top (90°)
                Point2D::new(center.x - radius, center.y),  // left (180°)
                Point2D::new(center.x, center.y - radius),  // bottom (270°)
            ],
            SketchEntity::ConstructionLine { start, end } => vec![*start, *end],
        }
    }

    /// Whether this is a construction entity (not included in regions).
    pub fn is_construction(&self) -> bool {
        matches!(self, SketchEntity::ConstructionLine { .. })
    }

    /// Minimum distance from a 2D point to this entity.
    #[allow(dead_code)]
    pub fn distance_to_point(&self, p: Point2D) -> f32 {
        match self {
            SketchEntity::Line { start, end }
            | SketchEntity::ConstructionLine { start, end } => {
                let closest = p.closest_on_segment(*start, *end);
                p.distance_to(closest)
            }
            SketchEntity::Circle { center, radius } => {
                (p.distance_to(*center) - radius).abs()
            }
        }
    }
}

/// Type of snap target (affects visual indicator).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SnapType {
    Endpoint,   // existing sketch endpoint — yellow circle
    Quadrant,   // circle quadrant point (0°/90°/180°/270°) — yellow circle
    Corner,     // mesh face corner/vertex — orange square
    Midpoint,   // edge midpoint — cyan triangle
    Edge,       // nearest point on edge — magenta circle
    Circumference, // nearest point on circle circumference — magenta circle
}

/// A snap target with position and type.
#[derive(Debug, Clone, Copy)]
pub struct SnapTarget {
    pub point: Point2D,
    pub snap_type: SnapType,
}

/// H/V inference type for line drawing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InferenceType {
    None,
    Horizontal,
    Vertical,
}
