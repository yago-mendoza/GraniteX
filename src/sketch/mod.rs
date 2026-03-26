// Sketch module — 2D drawing on face planes.
//
// Interaction model (matches SolidWorks):
//   Line:   click start → move mouse (preview follows) → click end → chains to next
//   Rect:   click corner1 → move mouse (rect preview follows) → click corner2
//   Circle: click center → move mouse (radius preview follows) → click edge
//
// All entities are stored in 2D sketch-local coordinates.
// The SketchPlane handles 2D↔3D transforms.

mod plane;
mod entities;

pub use plane::SketchPlane;
pub use entities::{SketchEntity, Point2D};

use glam::Vec3;

/// Active sketch state.
pub struct Sketch {
    pub plane: SketchPlane,
    pub entities: Vec<SketchEntity>,
    pub face_id: u32,

    /// First click point for current drawing operation (None = not drawing)
    pub pending_start: Option<Point2D>,
    /// First point of the current line chain (for closed contour detection)
    pub chain_start: Option<Point2D>,
    /// Current cursor position in 2D sketch space (updated every frame)
    pub cursor_2d: Option<Point2D>,
}

impl Sketch {
    pub fn new(plane: SketchPlane, face_id: u32) -> Self {
        Self {
            plane,
            entities: Vec::new(),
            face_id,
            pending_start: None,
            chain_start: None,
            cursor_2d: None,
        }
    }

    pub fn world_to_2d(&self, world_pos: Vec3) -> Point2D {
        self.plane.world_to_2d(world_pos)
    }

    pub fn to_3d(&self, p: Point2D) -> Vec3 {
        self.plane.to_3d(p)
    }

    pub fn add_line(&mut self, start: Point2D, end: Point2D) {
        if start.distance_to(end) > 0.005 {
            self.entities.push(SketchEntity::Line { start, end });
        }
    }

    pub fn add_rect(&mut self, corner1: Point2D, corner2: Point2D) {
        if (corner1.x - corner2.x).abs() < 0.005 || (corner1.y - corner2.y).abs() < 0.005 {
            return;
        }
        let min_x = corner1.x.min(corner2.x);
        let max_x = corner1.x.max(corner2.x);
        let min_y = corner1.y.min(corner2.y);
        let max_y = corner1.y.max(corner2.y);

        let bl = Point2D::new(min_x, min_y);
        let br = Point2D::new(max_x, min_y);
        let tr = Point2D::new(max_x, max_y);
        let tl = Point2D::new(min_x, max_y);

        self.entities.push(SketchEntity::Line { start: bl, end: br });
        self.entities.push(SketchEntity::Line { start: br, end: tr });
        self.entities.push(SketchEntity::Line { start: tr, end: tl });
        self.entities.push(SketchEntity::Line { start: tl, end: bl });
    }

    pub fn add_circle(&mut self, center: Point2D, radius: f32) {
        if radius > 0.005 {
            self.entities.push(SketchEntity::Circle { center, radius });
        }
    }

    /// Cancel the current pending operation.
    pub fn cancel_pending(&mut self) {
        self.pending_start = None;
        self.chain_start = None;
    }

    /// Delete the last entity (undo).
    pub fn undo_last(&mut self) {
        self.entities.pop();
    }

    /// Get all CONFIRMED line segments as 3D pairs (for rendering).
    pub fn confirmed_lines_3d(&self) -> Vec<(Vec3, Vec3)> {
        let mut lines = Vec::new();
        for entity in &self.entities {
            self.entity_to_lines_3d(entity, &mut lines);
        }
        lines
    }

    /// Get the PREVIEW line segments (pending operation following cursor).
    pub fn preview_lines_3d(&self, tool: crate::ui::SketchTool) -> Vec<(Vec3, Vec3)> {
        let Some(start) = self.pending_start else { return Vec::new() };
        let Some(cursor) = self.cursor_2d else { return Vec::new() };

        use crate::ui::SketchTool;

        let preview_entity = match tool {
            SketchTool::Line => SketchEntity::Line { start, end: cursor },
            SketchTool::Rect => {
                // Generate 4 lines for rectangle preview
                let mut lines = Vec::new();
                let min_x = start.x.min(cursor.x);
                let max_x = start.x.max(cursor.x);
                let min_y = start.y.min(cursor.y);
                let max_y = start.y.max(cursor.y);
                let bl = Point2D::new(min_x, min_y);
                let br = Point2D::new(max_x, min_y);
                let tr = Point2D::new(max_x, max_y);
                let tl = Point2D::new(min_x, max_y);
                lines.push((self.to_3d(bl), self.to_3d(br)));
                lines.push((self.to_3d(br), self.to_3d(tr)));
                lines.push((self.to_3d(tr), self.to_3d(tl)));
                lines.push((self.to_3d(tl), self.to_3d(bl)));
                return lines;
            }
            SketchTool::Circle => {
                SketchEntity::Circle {
                    center: start,
                    radius: start.distance_to(cursor),
                }
            }
        };

        let mut lines = Vec::new();
        self.entity_to_lines_3d(&preview_entity, &mut lines);
        lines
    }

    fn entity_to_lines_3d(&self, entity: &SketchEntity, lines: &mut Vec<(Vec3, Vec3)>) {
        match entity {
            SketchEntity::Line { start, end } => {
                lines.push((self.to_3d(*start), self.to_3d(*end)));
            }
            SketchEntity::Circle { center, radius } => {
                let segments = 64;
                for i in 0..segments {
                    let a0 = std::f32::consts::TAU * i as f32 / segments as f32;
                    let a1 = std::f32::consts::TAU * (i + 1) as f32 / segments as f32;
                    let p0 = Point2D::new(center.x + radius * a0.cos(), center.y + radius * a0.sin());
                    let p1 = Point2D::new(center.x + radius * a1.cos(), center.y + radius * a1.sin());
                    lines.push((self.to_3d(p0), self.to_3d(p1)));
                }
            }
        }
    }

    /// Snap to existing endpoints.
    pub fn snap_to_endpoint(&self, pos: Point2D, threshold: f32) -> Option<Point2D> {
        let mut best: Option<(f32, Point2D)> = None;
        for entity in &self.entities {
            for p in entity.endpoints() {
                let dist = p.distance_to(pos);
                if dist < threshold && (best.is_none() || dist < best.unwrap().0) {
                    best = Some((dist, p));
                }
            }
        }
        best.map(|(_, p)| p)
    }

    /// Get all unique endpoints for rendering dots.
    pub fn all_endpoints_3d(&self) -> Vec<Vec3> {
        let mut points = Vec::new();
        for entity in &self.entities {
            for p in entity.endpoints() {
                let p3d = self.to_3d(p);
                if !points.iter().any(|q: &Vec3| (*q - p3d).length() < 0.001) {
                    points.push(p3d);
                }
            }
        }
        points
    }
}
