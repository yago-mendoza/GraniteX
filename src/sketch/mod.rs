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
pub mod region;

pub use plane::SketchPlane;
pub use entities::{SketchEntity, Point2D, SnapType, SnapTarget};
pub use region::{RegionSolver, SketchRegion};

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

    /// Region computation (lazy, recomputed when entities change).
    pub region_solver: RegionSolver,
    /// Currently selected region index (None = no region selected).
    pub selected_region: Option<usize>,
    /// Parent face boundary in 2D (for computing outer region = face minus sketch).
    pub face_boundary_2d: Option<Vec<Point2D>>,
}

impl Sketch {
    pub fn new(plane: SketchPlane, face_id: u32, face_boundary_2d: Option<Vec<Point2D>>) -> Self {
        Self {
            plane,
            entities: Vec::new(),
            face_id,
            pending_start: None,
            chain_start: None,
            cursor_2d: None,
            region_solver: RegionSolver::new(),
            selected_region: None,
            face_boundary_2d,
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
            self.region_solver.mark_dirty();
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
            self.region_solver.mark_dirty();
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
        self.region_solver.mark_dirty();
        self.selected_region = None;
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

    /// Enhanced snap: checks sketch endpoints, face corners, edge midpoints, edge nearest.
    /// Returns the best snap target within threshold.
    pub fn snap_to_target(&self, pos: Point2D, threshold: f32) -> Option<SnapTarget> {
        let mut best: Option<(f32, SnapTarget)> = None;

        let mut consider = |best: &mut Option<(f32, SnapTarget)>, dist: f32, target: SnapTarget| {
            if dist < threshold && (best.is_none() || dist < best.unwrap().0) {
                *best = Some((dist, target));
            }
        };

        // Priority 1: Sketch endpoints (existing geometry snaps)
        for entity in &self.entities {
            for p in entity.endpoints() {
                consider(&mut best, p.distance_to(pos), SnapTarget { point: p, snap_type: SnapType::Endpoint });
            }
        }

        // Priority 2: Face boundary corners (mesh vertices projected to 2D)
        if let Some(ref boundary) = self.face_boundary_2d {
            for p in boundary {
                consider(&mut best, p.distance_to(pos), SnapTarget { point: *p, snap_type: SnapType::Corner });
            }

            // Priority 3: Edge midpoints
            let n = boundary.len();
            for i in 0..n {
                let a = boundary[i];
                let b = boundary[(i + 1) % n];
                let mid = Point2D::new((a.x + b.x) * 0.5, (a.y + b.y) * 0.5);
                consider(&mut best, mid.distance_to(pos), SnapTarget { point: mid, snap_type: SnapType::Midpoint });
            }

            // Priority 4: Nearest point on edge (only if no higher-priority snap found)
            let check_edges = match &best {
                None => true,
                Some((d, _)) => *d > threshold * 0.5,
            };
            if check_edges {
                for i in 0..n {
                    let a = boundary[i];
                    let b = boundary[(i + 1) % n];
                    let closest = pos.closest_on_segment(a, b);
                    let dist = closest.distance_to(pos);
                    if dist < threshold * 0.7 {
                        consider(&mut best, dist, SnapTarget { point: closest, snap_type: SnapType::Edge });
                    }
                }
            }
        }

        best.map(|(_, t)| t)
    }

    /// Get the current active snap target (for rendering the visual indicator).
    /// Returns None if cursor is not near any snap target.
    pub fn active_snap_target(&self) -> Option<SnapTarget> {
        let cursor = self.cursor_2d?;
        self.snap_to_target(cursor, 0.08) // slightly larger threshold for visual display
    }

    /// Extract the closed contour points (2D) from the last completed shape.
    /// Returns None if there's no closed contour.
    #[allow(dead_code)]
    pub fn closed_contour_2d(&self) -> Option<Vec<Point2D>> {
        if self.entities.is_empty() { return None; }

        let last = &self.entities[self.entities.len() - 1];
        let mut points = Vec::new();

        match last {
            SketchEntity::Circle { center, radius } => {
                let segments = 64;
                for j in 0..segments {
                    let angle = std::f32::consts::TAU * j as f32 / segments as f32;
                    points.push(Point2D::new(
                        center.x + radius * angle.cos(),
                        center.y + radius * angle.sin(),
                    ));
                }
            }
            SketchEntity::Line { .. } => {
                // Walk backward to find chain start
                let mut start_idx = self.entities.len() - 1;
                while start_idx > 0 {
                    if let (
                        SketchEntity::Line { end: prev_end, .. },
                        SketchEntity::Line { start: curr_start, .. }
                    ) = (&self.entities[start_idx - 1], &self.entities[start_idx]) {
                        if prev_end.distance_to(*curr_start) < 0.02 {
                            start_idx -= 1;
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }

                for entity in &self.entities[start_idx..] {
                    if let SketchEntity::Line { start, end } = entity {
                        if points.is_empty() || start.distance_to(*points.last().unwrap()) > 0.01 {
                            points.push(*start);
                        }
                        points.push(*end);
                    }
                }
            }
        }

        if points.len() < 3 { return None; }

        // Remove duplicate closing point
        if let (Some(first), Some(last)) = (points.first(), points.last()) {
            if first.distance_to(*last) < 0.02 {
                points.pop();
            }
        }

        if points.len() < 3 { return None; }
        Some(points)
    }

    /// Extract closed contour as 3D points ready for extrusion.
    #[allow(dead_code)]
    pub fn closed_contour_3d(&self) -> Option<Vec<Vec3>> {
        self.closed_contour_2d().map(|pts| {
            pts.iter().map(|p| self.to_3d(*p)).collect()
        })
    }

    /// Get computed regions (recomputes if dirty).
    pub fn regions(&mut self) -> &[SketchRegion] {
        self.region_solver.regions(&self.entities, self.face_boundary_2d.as_deref())
    }

    /// Select the region at a 2D point. Returns true if a region was found.
    pub fn select_region_at(&mut self, point: Point2D) -> bool {
        self.selected_region = self.region_solver.region_at_point(&self.entities, point, self.face_boundary_2d.as_deref());
        self.selected_region.is_some()
    }

    /// Get the selected region's boundary as 3D world points.
    pub fn selected_region_3d(&mut self) -> Option<Vec<Vec3>> {
        let idx = self.selected_region?;
        let regions = self.region_solver.regions(&self.entities, self.face_boundary_2d.as_deref());
        let region = regions.get(idx)?;
        let boundary: Vec<Point2D> = region.boundary.clone();
        Some(boundary.iter().map(|p| self.plane.to_3d(*p)).collect())
    }

    /// Get the selected region's holes as 3D world points.
    pub fn selected_region_holes_3d(&mut self) -> Vec<Vec<Vec3>> {
        let Some(idx) = self.selected_region else { return Vec::new() };
        let regions = self.region_solver.regions(&self.entities, self.face_boundary_2d.as_deref());
        let Some(region) = regions.get(idx) else { return Vec::new() };
        let holes: Vec<Vec<Point2D>> = region.holes.clone();
        holes.iter()
            .map(|hole| hole.iter().map(|p| self.plane.to_3d(*p)).collect())
            .collect()
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
