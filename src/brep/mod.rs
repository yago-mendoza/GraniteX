// BREP — Half-edge mesh with explicit topology.
//
// Every face knows its edges, every edge knows its faces.
// No heuristics, no angle-sorting, no stored_boundaries HashMap.
// Operations are local pointer rewiring, not global vertex scanning.

pub mod primitives;
pub mod tessellate;

use glam::Vec3;
use slotmap::{new_key_type, SlotMap};

new_key_type! {
    pub struct VertexId;
    pub struct HalfEdgeId;
    pub struct FaceId;
}

#[derive(Clone)]
pub struct HVertex {
    pub position: Vec3,
    /// One outgoing half-edge from this vertex (arbitrary choice, but stable).
    pub halfedge: Option<HalfEdgeId>,
}

#[derive(Clone, Copy)]
pub struct HalfEdge {
    /// The vertex this half-edge POINTS TO.
    pub vertex: VertexId,
    /// The twin (opposite direction on the same physical edge).
    /// None for boundary edges (mesh boundary, not a hole).
    pub twin: Option<HalfEdgeId>,
    /// Next half-edge CCW around the face.
    pub next: HalfEdgeId,
    /// The face to the LEFT of this half-edge (None = boundary).
    pub face: Option<FaceId>,
}

#[derive(Clone)]
pub struct HFace {
    /// One half-edge on this face's boundary.
    pub halfedge: HalfEdgeId,
    /// Cached face normal (recomputed on mutation).
    pub normal: Vec3,
}

/// Half-edge mesh with explicit topology.
#[derive(Clone)]
pub struct BrepMesh {
    pub vertices: SlotMap<VertexId, HVertex>,
    pub halfedges: SlotMap<HalfEdgeId, HalfEdge>,
    pub faces: SlotMap<FaceId, HFace>,
}

impl BrepMesh {
    pub fn new() -> Self {
        Self {
            vertices: SlotMap::with_key(),
            halfedges: SlotMap::with_key(),
            faces: SlotMap::with_key(),
        }
    }

    // --- Vertex operations ---

    pub fn add_vertex(&mut self, position: Vec3) -> VertexId {
        self.vertices.insert(HVertex { position, halfedge: None })
    }

    pub fn vertex_position(&self, v: VertexId) -> Vec3 {
        self.vertices[v].position
    }

    // --- Face traversal ---

    /// Walk the half-edge loop of a face, returning vertex IDs in order.
    pub fn face_vertices(&self, face: FaceId) -> Vec<VertexId> {
        let start = self.faces[face].halfedge;
        let mut verts = Vec::new();
        let mut he = start;
        loop {
            verts.push(self.halfedges[he].vertex);
            he = self.halfedges[he].next;
            if he == start { break; }
            if verts.len() > 1000 { break; } // safety
        }
        verts
    }

    /// Get face boundary positions in order.
    pub fn face_positions(&self, face: FaceId) -> Vec<Vec3> {
        self.face_vertices(face).iter()
            .map(|&v| self.vertices[v].position)
            .collect()
    }

    /// Number of edges/vertices in a face.
    pub fn face_sides(&self, face: FaceId) -> usize {
        self.face_vertices(face).len()
    }

    pub fn face_normal(&self, face: FaceId) -> Vec3 {
        self.faces[face].normal
    }

    /// Recompute face normal from vertex positions.
    pub fn recompute_normal(&mut self, face: FaceId) {
        let positions = self.face_positions(face);
        if positions.len() >= 3 {
            let e1 = positions[1] - positions[0];
            let e2 = positions[2] - positions[0];
            self.faces[face].normal = e1.cross(e2).normalize_or_zero();
        }
    }

    /// Find the half-edge from vertex `from` to vertex `to`, if it exists.
    pub fn find_halfedge(&self, from: VertexId, to: VertexId) -> Option<HalfEdgeId> {
        // Walk outgoing half-edges from `from`
        let start_he = self.vertices[from].halfedge?;
        let mut he = start_he;
        loop {
            if self.halfedges[he].vertex == to {
                return Some(he);
            }
            // Move to next outgoing edge: twin → next
            let twin = self.halfedges[he].twin?;
            he = self.halfedges[twin].next;
            if he == start_he { break; }
        }
        None
    }

    // --- Face construction ---

    /// Add a face from an ordered list of vertex IDs.
    /// Creates half-edges connecting them in a loop.
    /// Attempts to twin with existing half-edges.
    pub fn add_face(&mut self, vertex_ids: &[VertexId], normal: Vec3) -> FaceId {
        let n = vertex_ids.len();
        assert!(n >= 3, "Face needs at least 3 vertices");

        // Create a temporary face ID (need it before creating half-edges)
        let face_id = self.faces.insert(HFace {
            halfedge: HalfEdgeId::default(), // placeholder, fixed below
            normal,
        });

        // Create half-edges for the face loop
        let mut he_ids: Vec<HalfEdgeId> = Vec::with_capacity(n);
        for i in 0..n {
            let target = vertex_ids[(i + 1) % n];
            let he_id = self.halfedges.insert(HalfEdge {
                vertex: target,
                twin: None,
                next: HalfEdgeId::default(), // fixed below
                face: Some(face_id),
            });
            he_ids.push(he_id);
        }

        // Wire next pointers (each half-edge's next is the following one)
        for i in 0..n {
            self.halfedges[he_ids[i]].next = he_ids[(i + 1) % n];
        }

        // Set face's halfedge
        self.faces[face_id].halfedge = he_ids[0];

        // Set vertex outgoing half-edges (if not already set)
        for i in 0..n {
            let v = vertex_ids[i];
            if self.vertices[v].halfedge.is_none() {
                self.vertices[v].halfedge = Some(he_ids[i]);
            }
        }

        // Try to find twins for each new half-edge
        for i in 0..n {
            let from = vertex_ids[i];
            let to = vertex_ids[(i + 1) % n];
            // Look for existing half-edge from `to` to `from` (opposite direction)
            if let Some(twin_he) = self.find_opposite_halfedge(to, from, he_ids[i]) {
                self.halfedges[he_ids[i]].twin = Some(twin_he);
                self.halfedges[twin_he].twin = Some(he_ids[i]);
            }
        }

        face_id
    }

    /// Find a half-edge from `from` to `to` that is NOT `exclude`.
    fn find_opposite_halfedge(&self, from: VertexId, to: VertexId, exclude: HalfEdgeId) -> Option<HalfEdgeId> {
        let start_he = self.vertices[from].halfedge?;
        let mut he = start_he;
        let mut iterations = 0;
        loop {
            if self.halfedges[he].vertex == to && he != exclude {
                return Some(he);
            }
            // Move to next outgoing edge via twin
            if let Some(twin) = self.halfedges[he].twin {
                he = self.halfedges[twin].next;
            } else {
                break; // boundary — can't continue
            }
            iterations += 1;
            if he == start_he || iterations > 100 { break; }
        }
        None
    }

    // --- Statistics ---

    pub fn face_count(&self) -> usize {
        self.faces.len()
    }

    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }
}
