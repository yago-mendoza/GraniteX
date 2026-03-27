// Region solver — computes selectable regions from sketch contours.
//
// When sketch contours overlap (e.g., two rectangles), they define a
// planar subdivision with distinct regions. This module:
//   1. Extracts closed contours from sketch entities
//   2. Converts them to geo::Polygon for boolean operations
//   3. Computes all distinct regions via intersection/difference
//   4. Provides point-in-region queries for click selection

use geo::algorithm::bool_ops::BooleanOps;
use geo::algorithm::contains::Contains;
use geo::algorithm::area::Area;

use super::entities::{Point2D, SketchEntity};

/// A closed contour extracted from sketch entities.
#[derive(Debug, Clone)]
pub struct Contour {
    pub points: Vec<Point2D>,
}

impl Contour {
    fn to_geo_polygon(&self) -> geo::Polygon<f64> {
        let coords: Vec<geo::Coord<f64>> = self.points.iter()
            .map(|p| geo::Coord { x: p.x as f64, y: p.y as f64 })
            .collect();
        geo::Polygon::new(geo::LineString::from(coords), vec![])
    }
}

/// A region is a bounded planar area defined by contour arrangement.
#[derive(Debug, Clone)]
pub struct SketchRegion {
    /// Outer boundary vertices in 2D sketch coordinates.
    pub boundary: Vec<Point2D>,
    /// Interior holes (each is a closed ring of points).
    pub holes: Vec<Vec<Point2D>>,
    /// Triangulated indices (for rendering). Indexes into all_vertices() (boundary + holes).
    pub triangles: Vec<[usize; 3]>,
    /// Region area.
    pub area: f64,
}

impl SketchRegion {
    /// All vertices: boundary followed by hole vertices (for triangulation indexing).
    pub fn all_vertices(&self) -> Vec<Point2D> {
        let mut v = self.boundary.clone();
        for hole in &self.holes {
            v.extend(hole);
        }
        v
    }
}

impl SketchRegion {
    /// Average of boundary points (for inside-face-boundary checks).
    pub fn centroid(&self) -> Point2D {
        if self.boundary.is_empty() {
            return Point2D::new(0.0, 0.0);
        }
        let n = self.boundary.len() as f32;
        let sx: f32 = self.boundary.iter().map(|p| p.x).sum();
        let sy: f32 = self.boundary.iter().map(|p| p.y).sum();
        Point2D::new(sx / n, sy / n)
    }

    /// Test if a 2D point is inside this region (including holes).
    pub fn contains_point(&self, point: Point2D) -> bool {
        let geo_point = geo::Point::new(point.x as f64, point.y as f64);
        let poly = self.to_geo_polygon();
        poly.contains(&geo_point)
    }

    fn to_geo_polygon(&self) -> geo::Polygon<f64> {
        let exterior: Vec<geo::Coord<f64>> = self.boundary.iter()
            .map(|p| geo::Coord { x: p.x as f64, y: p.y as f64 })
            .collect();
        let interiors: Vec<geo::LineString<f64>> = self.holes.iter()
            .map(|hole| {
                let coords: Vec<geo::Coord<f64>> = hole.iter()
                    .map(|p| geo::Coord { x: p.x as f64, y: p.y as f64 })
                    .collect();
                geo::LineString::from(coords)
            })
            .collect();
        geo::Polygon::new(geo::LineString::from(exterior), interiors)
    }
}

/// Manages region computation from sketch entities.
pub struct RegionSolver {
    contours: Vec<Contour>,
    regions: Vec<SketchRegion>,
    dirty: bool,
}

impl RegionSolver {
    pub fn new() -> Self {
        Self {
            contours: Vec::new(),
            regions: Vec::new(),
            dirty: true,
        }
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Get computed regions (recomputes if dirty).
    pub fn regions(&mut self, entities: &[SketchEntity], face_boundary: Option<&[Point2D]>) -> &[SketchRegion] {
        if self.dirty {
            self.compute(entities, face_boundary);
            self.dirty = false;
        }
        &self.regions
    }

    /// Find which region contains a point (returns index).
    pub fn region_at_point(&mut self, entities: &[SketchEntity], point: Point2D, face_boundary: Option<&[Point2D]>) -> Option<usize> {
        let regions = self.regions(entities, face_boundary);
        for (i, region) in regions.iter().enumerate() {
            if region.contains_point(point) {
                return Some(i);
            }
        }
        None
    }

    fn compute(&mut self, entities: &[SketchEntity], face_boundary: Option<&[Point2D]>) {
        self.contours = extract_contours(entities);

        // Include parent face boundary as first contour when sketch contours exist.
        // This creates the "outer region" (face minus sketch shapes).
        if !self.contours.is_empty() {
            if let Some(boundary) = face_boundary {
                if boundary.len() >= 3 {
                    self.contours.insert(0, Contour { points: boundary.to_vec() });
                }
            }
        }

        self.regions = compute_regions(&self.contours);
    }
}

/// Extract closed contours from sketch entities.
/// A contour is either:
///   - A circle (tessellated to polygon)
///   - A chain of connected lines that closes back to its start
fn extract_contours(entities: &[SketchEntity]) -> Vec<Contour> {
    let mut contours = Vec::new();
    let mut used = vec![false; entities.len()];

    // Pass 1: Extract circles (always closed). Skip construction entities.
    for (i, entity) in entities.iter().enumerate() {
        if entity.is_construction() {
            used[i] = true; // mark as used so Pass 2 skips them too
            continue;
        }
        if let SketchEntity::Circle { center, radius } = entity {
            let segments = 64;
            let points: Vec<Point2D> = (0..segments)
                .map(|j| {
                    let angle = std::f32::consts::TAU * j as f32 / segments as f32;
                    Point2D::new(center.x + radius * angle.cos(), center.y + radius * angle.sin())
                })
                .collect();
            contours.push(Contour { points });
            used[i] = true;
        }
    }

    // Pass 2: Extract connected line chains that form closed loops
    for start_idx in 0..entities.len() {
        if used[start_idx] { continue; }
        let SketchEntity::Line { start, .. } = &entities[start_idx] else { continue };

        // Try to build a chain starting from this line
        let chain_start = *start;
        let mut chain = Vec::new();
        let mut current_idx = start_idx;
        let mut chain_used = vec![false; entities.len()];

        loop {
            if chain_used[current_idx] { break; }
            let SketchEntity::Line { start: ls, end: le } = &entities[current_idx] else { break };

            chain_used[current_idx] = true;
            if chain.is_empty() {
                chain.push(*ls);
            }
            chain.push(*le);

            // Check if chain closes
            if chain.len() >= 3 && le.distance_to(chain_start) < 0.02 {
                // Closed! Remove duplicate endpoint
                if chain.last().unwrap().distance_to(chain[0]) < 0.02 {
                    chain.pop();
                }
                if chain.len() >= 3 {
                    for (i, u) in chain_used.iter().enumerate() {
                        if *u { used[i] = true; }
                    }
                    contours.push(Contour { points: chain });
                }
                break;
            }

            // Find next connected line
            let mut found = false;
            for next_idx in 0..entities.len() {
                if used[next_idx] || chain_used[next_idx] { continue; }
                if let SketchEntity::Line { start: ns, .. } = &entities[next_idx] {
                    if ns.distance_to(*le) < 0.02 {
                        current_idx = next_idx;
                        found = true;
                        break;
                    }
                }
            }
            if !found { break; }
        }
    }

    contours
}

/// Compute all distinct regions from a set of closed contours.
/// Uses geo boolean operations for polygon overlay.
fn compute_regions(contours: &[Contour]) -> Vec<SketchRegion> {
    if contours.is_empty() {
        return Vec::new();
    }

    let geo_polys: Vec<geo::MultiPolygon<f64>> = contours.iter()
        .map(|c| geo::MultiPolygon::new(vec![c.to_geo_polygon()]))
        .collect();

    if geo_polys.len() == 1 {
        // Single contour — one region
        return geo_multi_to_regions(&geo_polys[0]);
    }

    // Multiple contours: compute all atomic regions via polygon overlay.
    // For N polygons, the atomic regions are:
    //   - The intersection of all subsets
    //   - The differences
    //
    // Simplified approach for 2-5 contours:
    // Start with all individual polygons, then split them at intersections.
    let mut regions: Vec<geo::MultiPolygon<f64>> = Vec::new();

    // Build incrementally: add each polygon, splitting existing regions
    regions.push(geo_polys[0].clone());

    for poly in &geo_polys[1..] {
        let mut new_regions = Vec::new();

        for existing in &regions {
            // Part of existing that doesn't overlap with new poly
            let diff = existing.difference(poly);
            if diff.unsigned_area() > 1e-8 {
                new_regions.push(diff);
            }

            // Intersection of existing with new poly
            let inter = existing.intersection(poly);
            if inter.unsigned_area() > 1e-8 {
                new_regions.push(inter);
            }
        }

        // Part of new poly that doesn't overlap with any existing region
        let mut uncovered = poly.clone();
        for existing in &regions {
            uncovered = uncovered.difference(existing);
        }
        if uncovered.unsigned_area() > 1e-8 {
            new_regions.push(uncovered);
        }

        regions = new_regions;
    }

    // Convert geo regions to SketchRegions
    let mut result = Vec::new();
    for mp in &regions {
        result.extend(geo_multi_to_regions(mp));
    }
    result
}

/// Convert a geo::MultiPolygon to SketchRegions (one per polygon in the multi).
fn geo_multi_to_regions(mp: &geo::MultiPolygon<f64>) -> Vec<SketchRegion> {
    let mut regions = Vec::new();
    for poly in mp.iter() {
        let mut boundary: Vec<Point2D> = poly.exterior().points()
            .map(|p| Point2D::new(p.x() as f32, p.y() as f32))
            .collect();

        // Remove duplicate closing point
        if boundary.len() >= 2 {
            if let (Some(first), Some(last)) = (boundary.first(), boundary.last()) {
                if first.distance_to(*last) < 0.001 {
                    boundary.pop();
                }
            }
        }
        if boundary.len() < 3 { continue; }

        // Extract interior rings (holes)
        let holes: Vec<Vec<Point2D>> = poly.interiors().iter()
            .filter_map(|ring| {
                let mut pts: Vec<Point2D> = ring.points()
                    .map(|p| Point2D::new(p.x() as f32, p.y() as f32))
                    .collect();
                if pts.len() >= 2 {
                    if let (Some(first), Some(last)) = (pts.first(), pts.last()) {
                        if first.distance_to(*last) < 0.001 { pts.pop(); }
                    }
                }
                if pts.len() >= 3 { Some(pts) } else { None }
            })
            .collect();

        let area = poly.unsigned_area();
        let triangles = triangulate_polygon_with_holes(&boundary, &holes);

        regions.push(SketchRegion {
            boundary,
            holes,
            triangles,
            area,
        });
    }
    regions
}

/// Triangulate a polygon with optional holes using ear clipping.
fn triangulate_polygon_with_holes(outer: &[Point2D], holes: &[Vec<Point2D>]) -> Vec<[usize; 3]> {
    if outer.len() < 3 { return Vec::new(); }

    // Flatten: outer boundary + all holes into [x, y, x, y, ...]
    let mut vertices: Vec<f64> = outer.iter()
        .flat_map(|p| [p.x as f64, p.y as f64])
        .collect();

    let mut hole_indices: Vec<usize> = Vec::new();
    for hole in holes {
        hole_indices.push(vertices.len() / 2); // start index of this hole
        for p in hole {
            vertices.push(p.x as f64);
            vertices.push(p.y as f64);
        }
    }

    match earcutr::earcut(&vertices, &hole_indices, 2) {
        Ok(indices) => {
            indices.chunks_exact(3)
                .map(|tri| [tri[0], tri[1], tri[2]])
                .collect()
        }
        Err(_) => {
            // Fallback: fan triangulation (no holes, works for convex)
            (1..outer.len() - 1)
                .map(|i| [0, i, i + 1])
                .collect()
        }
    }
}
