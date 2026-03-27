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
pub use entities::{SketchEntity, Point2D, SnapType, SnapTarget, InferenceType};
pub use region::{RegionSolver, SketchRegion};

use glam::Vec3;

/// Active sketch state.
pub struct Sketch {
    pub plane: SketchPlane,
    pub entities: Vec<SketchEntity>,
    pub face_id: Option<u32>,

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

    /// Current H/V inference state (updated per frame for preview).
    pub active_inference: InferenceType,
    /// Preview line length (for dimension display).
    pub preview_length: Option<f32>,
    /// Preview line angle in degrees (for dimension display).
    pub preview_angle: Option<f32>,
    /// Currently selected/hovered entity index (for editing/deletion).
    pub selected_entity: Option<usize>,
}

impl Sketch {
    pub fn new(plane: SketchPlane, face_id: Option<u32>, face_boundary_2d: Option<Vec<Point2D>>) -> Self {
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
            active_inference: InferenceType::None,
            preview_length: None,
            preview_angle: None,
            selected_entity: None,
        }
    }

    pub fn world_to_2d(&self, world_pos: Vec3) -> Point2D {
        self.plane.world_to_2d(world_pos)
    }

    /// Apply H/V inference to a line endpoint.
    /// With shift_held (ortho mode), always constrains to nearest cardinal direction.
    /// Without shift, only snaps if within 5° of horizontal or vertical.
    pub fn infer_constraint(start: Point2D, end: Point2D, shift_held: bool) -> (Point2D, InferenceType) {
        let dx = end.x - start.x;
        let dy = end.y - start.y;
        if dx.abs() < 1e-6 && dy.abs() < 1e-6 {
            return (end, InferenceType::None);
        }

        let angle = dy.atan2(dx).abs().to_degrees();
        let threshold = if shift_held { 45.0 } else { 5.0 };

        // Horizontal: angle near 0° or 180°
        if angle < threshold || (180.0 - angle) < threshold {
            return (Point2D::new(end.x, start.y), InferenceType::Horizontal);
        }
        // Vertical: angle near 90°
        if (angle - 90.0).abs() < threshold {
            return (Point2D::new(start.x, end.y), InferenceType::Vertical);
        }

        (end, InferenceType::None)
    }

    /// Grid spacing for grid snap (0 = disabled). Set from UI.
    pub fn grid_spacing(&self) -> f32 {
        0.05 // default 5cm grid
    }

    /// Snap a point to the nearest grid intersection.
    fn snap_to_grid(p: Point2D, grid: f32) -> Point2D {
        if grid <= 0.0 { return p; }
        Point2D::new(
            (p.x / grid).round() * grid,
            (p.y / grid).round() * grid,
        )
    }

    /// Resolve cursor position through snap + inference pipeline.
    /// `line_mode`: true only when the Line tool is active. H/V inference is
    /// harmful for Rect (can zero out a dimension) and Circle (warps radius),
    /// so it is skipped for those tools.
    pub fn resolve_cursor(&self, raw_2d: Point2D, shift_held: bool, line_mode: bool) -> (Point2D, Option<SnapTarget>, InferenceType) {
        // Priority 1: Snap targets (endpoints, corners, midpoints, edges)
        if let Some(snap) = self.snap_to_target(raw_2d, 0.05) {
            return (snap.point, Some(snap), InferenceType::None);
        }

        // Priority 2: H/V inference (only when actively drawing a LINE)
        if line_mode {
            if let Some(start) = self.pending_start {
                let (pos, inf) = Self::infer_constraint(start, raw_2d, shift_held);
                let grid = self.grid_spacing();
                let pos = if grid > 0.0 { Self::snap_to_grid(pos, grid) } else { pos };
                return (pos, None, inf);
            }
        }

        // Priority 3: Grid snap
        let grid = self.grid_spacing();
        let pos = if grid > 0.0 { Self::snap_to_grid(raw_2d, grid) } else { raw_2d };
        (pos, None, InferenceType::None)
    }

    /// Update preview dimensions (call after resolving cursor).
    pub fn update_preview_dimensions(&mut self) {
        if let (Some(start), Some(cursor)) = (self.pending_start, self.cursor_2d) {
            let dx = cursor.x - start.x;
            let dy = cursor.y - start.y;
            let length = (dx * dx + dy * dy).sqrt();
            let angle = dy.atan2(dx).to_degrees();
            self.preview_length = Some(length);
            self.preview_angle = Some(angle);
        } else {
            self.preview_length = None;
            self.preview_angle = None;
        }
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

    pub fn add_construction_line(&mut self, start: Point2D, end: Point2D) {
        if start.distance_to(end) > 0.005 {
            self.entities.push(SketchEntity::ConstructionLine { start, end });
            // No region_solver.mark_dirty() — construction lines don't affect regions
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
    /// Also restores chain state so the next click connects correctly.
    pub fn undo_last(&mut self) {
        self.entities.pop();
        self.region_solver.mark_dirty();
        self.selected_region = None;
        self.selected_entity = None;

        // Restore pending_start to the last entity's endpoint so the chain
        // stays connected. If no entities remain, reset to idle state.
        if let Some(last) = self.entities.last() {
            match last {
                SketchEntity::Line { end, .. } => {
                    self.pending_start = Some(*end);
                }
                SketchEntity::Circle { .. }
                | SketchEntity::ConstructionLine { .. } => {
                    self.pending_start = None;
                    self.chain_start = None;
                }
            }
        } else {
            self.pending_start = None;
            self.chain_start = None;
        }
    }

    /// Select the nearest entity within threshold. Returns true if found.
    pub fn select_entity_near(&mut self, pos: Point2D, threshold: f32) -> bool {
        let mut best: Option<(usize, f32)> = None;
        for (i, entity) in self.entities.iter().enumerate() {
            let dist = entity.distance_to_point(pos);
            if dist < threshold && (best.is_none() || dist < best.unwrap().1) {
                best = Some((i, dist));
            }
        }
        self.selected_entity = best.map(|(i, _)| i);
        self.selected_entity.is_some()
    }

    /// Delete the currently selected entity.
    pub fn delete_selected_entity(&mut self) {
        if let Some(idx) = self.selected_entity.take() {
            if idx < self.entities.len() {
                self.entities.remove(idx);
                self.region_solver.mark_dirty();
                self.selected_region = None;
            }
        }
    }

    /// Get all CONFIRMED line segments as 3D pairs (for rendering).
    /// Get confirmed line segments (excludes construction lines).
    pub fn confirmed_lines_3d(&self) -> Vec<(Vec3, Vec3)> {
        let mut lines = Vec::new();
        for entity in &self.entities {
            if !entity.is_construction() {
                self.entity_to_lines_3d(entity, &mut lines);
            }
        }
        lines
    }

    /// Get construction line segments (for rendering in different color).
    pub fn construction_lines_3d(&self) -> Vec<(Vec3, Vec3)> {
        let mut lines = Vec::new();
        for entity in &self.entities {
            if entity.is_construction() {
                self.entity_to_lines_3d(entity, &mut lines);
            }
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
            SketchEntity::Line { start, end }
            | SketchEntity::ConstructionLine { start, end } => {
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

        // Priority 1a: Sketch endpoints + circle quadrants (existing geometry snaps)
        for entity in &self.entities {
            for p in entity.endpoints() {
                let snap_type = match entity {
                    SketchEntity::Circle { center, .. } if p.distance_to(*center) > 0.001 => SnapType::Quadrant,
                    _ => SnapType::Endpoint,
                };
                consider(&mut best, p.distance_to(pos), SnapTarget { point: p, snap_type });
            }
        }

        // Priority 1b: Nearest point on circle circumference (lower priority than quadrants)
        let check_circumference = match &best {
            None => true,
            Some((d, _)) => *d > threshold * 0.5,
        };
        if check_circumference {
            for entity in &self.entities {
                if let SketchEntity::Circle { center, radius } = entity {
                    let dx = pos.x - center.x;
                    let dy = pos.y - center.y;
                    let dist_to_center = (dx * dx + dy * dy).sqrt();
                    if dist_to_center > 1e-6 {
                        let nearest = Point2D::new(
                            center.x + dx / dist_to_center * radius,
                            center.y + dy / dist_to_center * radius,
                        );
                        let dist = nearest.distance_to(pos);
                        if dist < threshold * 0.7 {
                            consider(&mut best, dist, SnapTarget { point: nearest, snap_type: SnapType::Circumference });
                        }
                    }
                }
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
            SketchEntity::ConstructionLine { .. } => return None,
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
    /// Rejects regions that lie outside the parent face boundary (would create
    /// floating geometry disconnected from the mesh).
    pub fn select_region_at(&mut self, point: Point2D) -> bool {
        self.selected_region = self.region_solver.region_at_point(&self.entities, point, self.face_boundary_2d.as_deref());

        // If sketching on a mesh face, verify the region is inside the face boundary
        if let (Some(idx), Some(ref boundary)) = (self.selected_region, &self.face_boundary_2d) {
            if boundary.len() >= 3 {
                let regions = self.region_solver.regions(&self.entities, self.face_boundary_2d.as_deref());
                if let Some(region) = regions.get(idx) {
                    // Check that the region centroid is inside the face boundary
                    let centroid = region.centroid();
                    let face_poly = geo::Polygon::new(
                        geo::LineString::from(
                            boundary.iter()
                                .map(|p| geo::Coord { x: p.x as f64, y: p.y as f64 })
                                .collect::<Vec<_>>()
                        ),
                        vec![],
                    );
                    let geo_point = geo::Point::new(centroid.x as f64, centroid.y as f64);
                    use geo::algorithm::contains::Contains;
                    if !face_poly.contains(&geo_point) {
                        self.selected_region = None;
                    }
                }
            }
        }

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
