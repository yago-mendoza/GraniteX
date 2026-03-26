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
}

/// A sketch entity — something drawn on the sketch plane.
#[derive(Debug, Clone)]
pub enum SketchEntity {
    Line { start: Point2D, end: Point2D },
    Circle { center: Point2D, radius: f32 },
}

impl SketchEntity {
    /// Get the control points/endpoints of this entity (for snapping).
    pub fn endpoints(&self) -> Vec<Point2D> {
        match self {
            SketchEntity::Line { start, end } => vec![*start, *end],
            SketchEntity::Circle { center, .. } => vec![*center],
        }
    }
}
