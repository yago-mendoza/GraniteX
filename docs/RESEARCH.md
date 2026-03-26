# GraniteX — Research Notes

Last updated: 2026-03-26

## Rust CAD Ecosystem Scan

### Geometry Kernels
| Project | Type | Maturity | Notes |
|---|---|---|---|
| truck | Pure Rust BREP | Medium | NURBS, STEP I/O. Sporadic updates. Best pure-Rust option. |
| opencascade-rs | FFI to C++ OCCT | Good | Industrial-grade. Heavy dependency. Fallback option. |
| curvo | NURBS library | Early | Appeared ~2024. Worth watching for curve/surface evaluation. |
| geop | Exact BREP | Very early | Rational arithmetic focus. Experimental. |

### CAD Applications
| Project | Approach | Status | Learnings |
|---|---|---|---|
| Fornjot | Code-first CAD | Paused/rethinking | Study kernel architecture |
| Blackjack | Node-based mesh | Dormant | Study wgpu rendering pipeline, half-edge mesh |
| CADara | Parametric CAD | Early | Uses opencascade-rs. Watch for progress. |

### Constraint Solvers
| Project | Type | Notes |
|---|---|---|
| rust_slvs | FFI to SolveSpace | Mature solver, Rust bindings vary in quality |
| geo-aid | Pure Rust | Active 2024-2025. Euclidean geometry focus. |
| argmin | Optimization framework | Build custom solver on top. LM algorithm available. |

### Key Libraries
| Crate | Purpose | Notes |
|---|---|---|
| parry3d | Collision/raycasting | Essential for selection (ray-mesh intersection) |
| lyon | 2D tessellation | For sketch mode rendering |
| spade | Delaunay triangulation | For meshing operations |
| robust | Exact predicates | For reliable geometry tests |
| ruststep | STEP file I/O | Same org as truck |

## Papers & References to Read
- "Boundary Representation Modelling Techniques" — Stroud (textbook)
- "Geometric Constraint Solving" — survey papers on arxiv
- "Half-Edge Data Structure" — various implementations and tradeoffs
- SolveSpace source code (C++) — well-documented constraint solver
- Blender BMesh source — gold standard mesh editing data structure

---

## Sketch-on-Face Architecture Research (2026-03-26)

Deep dive into how SolidWorks-style "sketch on face" works, algorithms involved,
and how to implement it in GraniteX (Rust, wgpu, glam).

### 1. Projecting a 3D Face to a 2D Coordinate System

The fundamental problem: the user selects a 3D face, and we need a local 2D coordinate
system on that face so they can draw in a plane.

**Algorithm: Construct a Local Frame (origin + U + V + normal)**

Given a planar face with vertices, derive:

1. **Origin** — pick a reference point. SolidWorks uses the centroid of the face or the
   first vertex. For consistency, use the face centroid:
   `origin = average(face_vertices)`

2. **Normal (N)** — the face normal. For a planar face, this is the cross product of two
   edges. For a non-planar face (curved surface), use the surface normal at the centroid.
   `N = normalize((v1 - v0) x (v2 - v0))`

3. **U axis** — pick a reference direction in the face plane. SolidWorks aligns U with
   the projection of a "preferred" world axis onto the plane. The algorithm:
   - Try projecting world X onto the plane: `U_candidate = X - dot(X, N) * N`
   - If that's degenerate (face normal is parallel to X), try world Y, then Z.
   - Normalize: `U = normalize(U_candidate)`

4. **V axis** — complete the right-handed frame: `V = N x U`

**Transformation matrices:**

- **3D-to-2D (world to sketch):**
  Given a 3D point P, its 2D sketch coordinates are:
  `u = dot(P - origin, U)`
  `v = dot(P - origin, V)`
  This is just a dot product projection — no matrix inversion needed.

- **2D-to-3D (sketch to world):**
  `P = origin + u * U + v * V`

These form a `SketchPlane` struct:
```
struct SketchPlane {
    origin: Vec3,
    u_axis: Vec3,   // local X in world space
    v_axis: Vec3,   // local Y in world space
    normal: Vec3,   // local Z (into/out of sketch)
}
```

**For non-planar faces:** SolidWorks still defines a sketch plane — it uses the tangent
plane at the point where the user clicked. The sketch is on a flat plane tangent to the
surface. The 2D profile is then projected back onto the curved surface when used for
operations. For GraniteX Phase 7, restrict to planar faces only. Curved face support
is a Phase 9+ concern.

**Camera alignment:** When entering sketch mode, animate the camera to look down the
normal. Target = origin, eye = origin + normal * distance, up = v_axis. Use the existing
`Camera` struct — set `yaw`/`pitch` to match the normal direction, or switch to an
explicit `look_at` mode. SolidWorks also switches to orthographic projection in sketch
mode (configurable). Implement both:
- Perspective sketch mode (default for 3D context awareness)
- Ortho sketch mode (toggled with a key, useful for precise dimensioning)

### 2. The Constraint Solver

This is the most algorithmically complex part. The sketch constraint solver takes a set
of geometric entities (points, lines, circles, arcs) and constraints between them
(distance, angle, coincident, tangent, etc.) and finds positions for all entities that
satisfy all constraints simultaneously.

**The problem formulation:**

Variables: all the free parameters of the sketch entities.
- A point has 2 DOF: (x, y)
- A line segment has 4 DOF: two endpoints (x1, y1, x2, y2)
- A circle has 3 DOF: center (cx, cy) + radius r
- An arc has 5 DOF: center (cx, cy) + radius r + start_angle + end_angle

Constraints are equations that reduce DOF:
- Coincident (point-on-point): 2 equations: x1 = x2, y1 = y2
- Point-on-line: 1 equation (distance from point to line = 0)
- Horizontal line: 1 equation: y1 = y2
- Vertical line: 1 equation: x1 = x2
- Distance: 1 equation: sqrt((x2-x1)^2 + (y2-y1)^2) = d
- Perpendicular: 1 equation: dot product of direction vectors = 0
- Parallel: 1 equation: cross product of direction vectors = 0
- Tangent (line-circle): 1 equation: distance from center to line = radius
- Equal length: 1 equation
- Fix point: 2 equations: x = x0, y = y0
- Angle: 1 equation

**The system:** N unknowns (total DOF of all entities before constraints), M constraint
equations. The solver finds values that make all constraint equations equal zero (or as
close as possible).

This is a **nonlinear least-squares problem**: minimize sum of squared constraint
residuals. The constraints form a system F(x) = 0 where F: R^N -> R^M.

**Algorithm options:**

1. **Newton-Raphson** — classic. Compute Jacobian J, solve J * dx = -F for dx, update
   x += dx. Fast quadratic convergence near the solution. But: requires a good initial
   guess, can diverge, doesn't handle rank-deficient Jacobians (under-constrained).

2. **Levenberg-Marquardt (LM)** — the standard choice for geometric constraint solving.
   It's Newton-Raphson with a damping term: solve (J^T J + lambda * I) * dx = -J^T F.
   When lambda is large, it behaves like gradient descent (safe, slow). When lambda is
   small, it behaves like Newton (fast, near solution). The algorithm adapts lambda
   automatically. **This is what SolveSpace uses.** Also what most commercial CAD
   constraint solvers use.

3. **Gauss-Newton** — LM without the damping. Faster but less robust. Not recommended
   as the primary solver.

4. **Graph-based decomposition + sequential solving** — decompose the constraint graph
   into clusters that can be solved independently. Used in some academic solvers. More
   complex to implement, better for very large sketches. Not needed for Phase 7.

**SolveSpace's approach (reference implementation):**

SolveSpace (github.com/solvespace/solvespace, src/system.cpp) uses:

1. **Symbolic substitution first** — before numerical solving, it does algebraic
   simplification. If two points are coincident, it substitutes one for the other,
   reducing the system size. This handles simple constraints without iteration.

2. **Newton's method with rank detection** — it builds the Jacobian, does Gaussian
   elimination with partial pivoting, and detects rank deficiency. The rank tells you
   the DOF remaining (under-constrained) or redundancy (over-constrained).

3. **Dogleg trust-region** — SolveSpace actually uses a dogleg method (a trust-region
   variant related to LM). The key files:
   - `src/system.cpp` — the solver core
   - `src/constraint.cpp` — constraint equation definitions
   - `src/entity.cpp` — entity parameterization

4. **Dragging** — when the user drags an entity, SolveSpace re-solves with the dragged
   point's coordinates as additional "soft" constraints. This gives real-time feedback.
   The drag point gets extra weight so the solver moves it to where the cursor is while
   keeping other constraints satisfied.

**Recommendation for GraniteX:**

Use **Levenberg-Marquardt** via the `argmin` crate (or implement it directly — LM is
~100 lines of code for the core loop). The algorithm:

```
function solve_sketch(params, constraints):
    x = current parameter values (initial guess = current entity positions)
    lambda = 1e-3  // initial damping

    for iteration in 0..MAX_ITER:
        F = evaluate all constraint residuals at x     // M-vector
        J = compute Jacobian (dF/dx) at x              // M x N matrix

        // LM step
        A = J^T * J + lambda * diag(J^T * J)
        b = -J^T * F
        solve A * dx = b for dx

        x_new = x + dx
        F_new = evaluate constraints at x_new

        if ||F_new|| < ||F||:  // improvement
            x = x_new
            lambda *= 0.1      // reduce damping (more Newton-like)
        else:
            lambda *= 10.0     // increase damping (more gradient-descent-like)

        if ||F|| < tolerance:
            return Solved(x)

    return FailedToConverge
```

**The Jacobian** can be computed analytically (each constraint type has known partial
derivatives) or via finite differences (simpler but slower). For production quality,
use analytical Jacobians — they're straightforward for geometric constraints.

**Sparse matrices:** For sketches with <100 entities (typical), dense matrices are fine.
For large sketches, the Jacobian is sparse (each constraint only references a few
entities). Use `nalgebra::DMatrix` for now, switch to `nalgebra::CsrMatrix` or
`sprs` if performance requires it.

### 3. Detecting Closed Contours (for Extrusion)

When the user exits sketch mode, the system needs to find closed loops (profiles) that
can be extruded. This is a fundamental 2D computational geometry problem.

**The algorithm: Planar Face Finding**

Given a set of 2D line segments, arcs, and curves, find all minimal closed regions.

**Step 1: Build a planar graph**

- Each sketch entity contributes edges to a 2D planar graph.
- Intersection points between entities become graph nodes.
- Split entities at intersection points into sub-edges.
- Coincident endpoints merge into single nodes.

**Step 2: Sort edges around each node**

At each node, sort the outgoing edges by angle (atan2 of the direction vector). This
angular ordering is critical.

**Step 3: Trace minimal faces using the "left-turn rule"**

For each directed edge (u -> v):
1. At node v, find the edge that comes *next* in the clockwise angular ordering after
   the reverse of edge (u -> v).
2. Follow that edge. Repeat until you return to the starting node.
3. This traces one minimal face (the face to the left of the walk).

This is the **planar subdivision face-finding algorithm** (also called the "half-edge
traversal" on the planar graph). It finds ALL minimal faces, including the outer
(infinite) face.

**Step 4: Classify faces**

- Discard the outer (unbounded) face — it has the largest area or a negative signed area
  depending on winding convention.
- The remaining faces are closed contour candidates.
- The user can click inside a contour to select it, or the system auto-selects the
  innermost contour.

**Simpler approach for Phase 7:**

Skip the full planar subdivision. Instead:
1. Build an adjacency graph of sketch entities (connected at shared endpoints).
2. Find cycles in the graph using DFS.
3. For each cycle, compute the signed area. Positive = CCW = valid profile.
4. Reject self-intersecting cycles.
5. Present valid cycles to the user for selection.

This simpler approach breaks down with overlapping geometry but works for clean sketches.
Implement the full planar subdivision when needed (Phase 8-9).

**Winding direction matters:** Extrude operations need to know which side is "inside."
Use the signed area (shoelace formula). CCW = material side, CW = void (hole). A profile
with a CW inner loop creates a pocket (extrude-cut with an island).

### 4. Rendering the 2D Sketch Overlaid on the 3D Face

The sketch lives on a plane in 3D space. Rendering it involves:

**Approach A: Render as 3D geometry on the sketch plane (recommended)**

Transform all 2D sketch entities to 3D using the SketchPlane's 2D-to-3D mapping, then
render them as 3D line geometry. This is the simplest approach and what SolidWorks does.

- Lines become 3D line segments (two vertices each).
- Circles/arcs become polylines (tessellated into N segments).
- Constraint icons (dimension arrows, perpendicular symbols) are billboarded sprites
  or small 3D gizmos placed on the sketch plane.

Rendering details:
- Use a separate render pass or draw call with depth testing against the 3D body but
  a slight depth bias (polygon offset / depth bias in wgpu) to prevent z-fighting
  with the face surface.
- Use `wgpu::PrimitiveTopology::LineList` or `LineStrip` for sketch entities.
- Color code: fully constrained entities = black/dark, under-constrained = blue,
  over-constrained = red, selected = green. (This matches SolidWorks convention.)
- Line width: wgpu doesn't support wide lines natively. Options:
  - Use wgpu line rendering (1px lines) — acceptable for initial implementation.
  - Generate screen-space quads for each line segment (2 triangles per segment) for
    anti-aliased wide lines. This is what production apps do.
  - Use the `lyon` crate to tessellate lines into triangle strips with configurable width.

**Approach B: Render in screen space with egui**

Project sketch entities to screen coordinates and draw them using egui's `Painter`. This
is simpler for constraint annotations (text, dimension numbers) but loses 3D depth
integration.

**Hybrid approach (recommended for GraniteX):**

- Sketch entities (lines, arcs, circles): Render as 3D geometry on the sketch plane
  (Approach A). This keeps them integrated with the 3D scene.
- Constraint annotations (dimension values, constraint icons): Render as egui overlays
  in screen space. Project the annotation anchor point to screen coords, then draw text
  with egui. This avoids having to do text rendering in wgpu.

**Sketch mode visual cues:**
- Dim the 3D body (reduce opacity or desaturate) to focus attention on the sketch.
- Show the sketch plane as a semi-transparent rectangle extending beyond the face.
- Highlight the face boundary (the edges of the selected face) as a thick outline.
- Show a "grid" on the sketch plane (like SolidWorks' sketch grid).
- Show origin and axes of the sketch coordinate system.

### 5. Under-Constrained vs Over-Constrained Sketches

SolidWorks' handling of constraint states is central to its UX. Here's how it works:

**Degrees of Freedom (DOF) tracking:**

- Each entity starts with N DOF (point = 2, line = 4, circle = 3).
- Each constraint removes DOF (coincident = 2, dimension = 1, etc.).
- Total remaining DOF = sum(entity DOF) - sum(constraint DOF consumed).

But this is only an approximation — redundant constraints can make the count wrong. The
real DOF is determined by the **rank of the Jacobian matrix** at the current solution.

`true_DOF = N_params - rank(J)`

Where N_params is the total number of unknown parameters and rank(J) is the rank of the
constraint Jacobian.

**Three states:**

1. **Under-constrained (DOF > 0):** The sketch has remaining freedom. Entities that are
   under-constrained can be dragged. SolidWorks colors these entities **blue**.
   - The solver still works — it finds *a* solution, keeping free entities at their
     current positions (minimum displacement from the initial guess).
   - Under-constrained sketches CAN be used for extrusion (SolidWorks allows it).
   - The status bar shows "Under Defined" with the DOF count.

2. **Fully constrained (DOF = 0):** Every entity's position is uniquely determined by
   the constraints. SolidWorks colors these entities **black** (or dark gray).
   - This is the ideal state. The sketch is "locked down."
   - Status bar shows "Fully Defined."

3. **Over-constrained (DOF < 0 or contradictory):** Too many constraints, or
   contradictory constraints. SolidWorks colors conflicting constraints **red** and
   refuses to add the constraint (shows an error dialog).
   - Detection: after adding a constraint, check if rank(J) increased. If not, the new
     constraint is redundant. If the residual is nonzero after solving, the constraints
     are contradictory.
   - SolidWorks distinguishes "redundant" (consistent but unnecessary, shown as a
     warning) from "contradictory" (impossible, shown as an error).

**Per-entity DOF detection (which entities are free?):**

After solving, compute the null space of the Jacobian. Each null space vector represents
a direction of freedom. The entities whose parameters appear with nonzero coefficients
in the null space vectors are the under-constrained ones. In practice, a simpler
approach:
- Try to perturb each entity slightly and re-solve. If the solver finds a different
  solution, that entity is under-constrained.
- Or: for each parameter, check if its column in the Jacobian is in the column space.
  If not, it's free.

SolveSpace's approach: after solving, it examines the Jacobian's rank and identifies
which parameters are in the null space, then reports them as "free" with arrows showing
the direction of freedom (you can see this in SolveSpace's UI — free points show little
green arrows).

**Recommendation for GraniteX:**

1. Track DOF approximately (entity DOF - constraint DOF) for the status bar.
2. Use the Jacobian rank from the LM solver for ground truth.
3. Color entities by state: blue (under), black (fully), red (over).
4. Allow extrusion from under-constrained sketches (SolidWorks does, it's pragmatic).
5. On adding a constraint: solve immediately, check for contradiction (nonzero residual)
   or redundancy (rank didn't increase). Show error/warning accordingly.

### 6. Data Structures for the 2D Sketch

**Core data model:**

```
// A complete sketch attached to a face/plane
struct Sketch {
    id: SketchId,
    plane: SketchPlane,              // the 3D plane this sketch lives on
    entities: SlotMap<EntityId, SketchEntity>,
    constraints: SlotMap<ConstraintId, Constraint>,
    solve_result: SolveResult,       // cached: Solved / UnderConstrained(dof) / OverConstrained / Failed
}

// Sketch entities — the geometric primitives
enum SketchEntity {
    Point(SketchPoint),
    Line(SketchLine),
    Circle(SketchCircle),
    Arc(SketchArc),
    // Future: Spline, Ellipse, Construction geometry
}

struct SketchPoint {
    pos: Vec2,                       // sketch-local coordinates
    construction: bool,              // construction geometry (not used for profiles)
    fixed: bool,                     // pinned in place
}

struct SketchLine {
    start: EntityId,                 // reference to a SketchPoint
    end: EntityId,                   // reference to a SketchPoint
    construction: bool,
}

struct SketchCircle {
    center: EntityId,                // reference to a SketchPoint
    radius: f32,
    construction: bool,
}

struct SketchArc {
    center: EntityId,                // reference to a SketchPoint
    start: EntityId,                 // reference to a SketchPoint on the arc
    end: EntityId,                   // reference to a SketchPoint on the arc
    // Arcs go CCW from start to end
    construction: bool,
}

// Constraints between entities
struct Constraint {
    kind: ConstraintKind,
    entities: Vec<EntityId>,         // the entities this constraint references
    value: Option<f64>,              // for dimensional constraints (distance, angle, radius)
    driven: bool,                    // "reference" dimension — display only, not enforced
}

enum ConstraintKind {
    // Geometric
    Coincident,          // point-point or point-on-line or point-on-circle
    Horizontal,          // line is horizontal
    Vertical,            // line is vertical
    Parallel,            // line-line
    Perpendicular,       // line-line
    Tangent,             // line-circle or circle-circle
    Equal,               // equal length (line-line) or equal radius (circle-circle)
    Symmetric,           // two points symmetric about a line
    Midpoint,            // point at midpoint of line
    Collinear,           // two lines on the same infinite line
    Concentric,          // two circles share center

    // Dimensional
    Distance,            // point-point, point-line, or line-line distance
    Angle,               // angle between two lines
    Radius,              // circle/arc radius
    Diameter,            // circle/arc diameter

    // Fix
    FixPoint,            // lock point to specific coordinates
    FixAngle,            // lock line to specific angle
}
```

**Key design decisions:**

1. **Points are shared, not duplicated.** Lines and arcs reference point entities by ID.
   A coincident constraint between two lines means their endpoint IDs are the same point
   (or a coincident constraint links two separate points). SolveSpace uses the "merge
   points" approach; SolidWorks keeps separate points with coincident constraints.
   Recommendation: keep separate points with constraints — it's more flexible and easier
   to undo.

2. **Use a SlotMap (or generational arena) for entity storage.** Entities are frequently
   created and deleted. A SlotMap gives O(1) insert/delete/lookup and stable IDs that
   don't break when other entities are deleted. Crate: `slotmap`.

3. **Construction geometry** is a boolean flag on entities. Construction lines/circles
   are used for reference (guide geometry) but are excluded from profile detection.
   SolidWorks has a "construction geometry" toggle — one click converts a line to
   construction and back.

4. **Solver parameters array:** The solver operates on a flat `Vec<f64>` of parameters.
   Each entity knows its range of indices into this array. E.g., a point owns indices
   [i, i+1] for (x, y). The solver doesn't know about entities — it only sees parameters
   and constraint equations. This separation is clean and makes the solver reusable.

5. **Serialization:** The sketch must be serializable (for save/load and undo/redo).
   `serde` with the entity/constraint enums works naturally. The SlotMap keys serialize
   as (index, generation) pairs.

**Solver interface:**

```
struct SketchSolver {
    // Flattened parameter vector
    params: Vec<f64>,
    // Mapping: EntityId -> range of indices in params
    param_map: HashMap<EntityId, Range<usize>>,
    // Constraint equations (closures or trait objects)
    equations: Vec<Box<dyn ConstraintEquation>>,
}

trait ConstraintEquation {
    // Evaluate the residual (0 = satisfied)
    fn residual(&self, params: &[f64]) -> f64;
    // Evaluate partial derivatives w.r.t. referenced parameters
    fn jacobian_row(&self, params: &[f64], row: &mut [f64]);
}
```

The solver builds this from the Sketch, runs LM, then writes the solution back to the
entity positions.

### 7. End-to-End Workflow Summary

1. **User selects face** → compute SketchPlane (origin, U, V, normal).
2. **Enter sketch mode** → animate camera to face-on view, dim 3D body, show sketch
   plane grid and axes.
3. **User draws** → mouse events are unprojected from screen to sketch plane using
   ray-plane intersection:
   `t = dot(origin - ray_origin, normal) / dot(ray_dir, normal)`
   `hit_3d = ray_origin + t * ray_dir`
   `u = dot(hit_3d - origin, U), v = dot(hit_3d - origin, V)`
   → (u, v) are the sketch coordinates of the mouse.
4. **Drawing creates entities** → each click/drag creates SketchPoints, SketchLines, etc.
   Smart snapping: snap to existing points, midpoints, line extensions, grid points.
5. **Adding constraints** → user clicks entities and selects constraint type from a
   toolbar or right-click menu. Auto-constraints: when drawing a nearly-horizontal line,
   auto-suggest a Horizontal constraint. SolidWorks does this aggressively.
6. **Solver runs** → after each constraint change, run the LM solver. Update entity
   positions. Re-color entities by constraint state. Show DOF in status bar.
7. **User exits sketch** → detect closed contours (planar face-finding algorithm).
   Highlight valid profiles. Store the sketch as part of the feature tree.
8. **Extrude/cut** → user selects a profile and an operation. The 2D profile is swept
   along the sketch plane normal (or a user-specified direction) to create a 3D solid.
   The 2D-to-3D mapping converts the profile edges to 3D edges that define the new
   BREP faces.

### 8. Crate Recommendations for Implementation

| Need | Crate | Notes |
|---|---|---|
| LM solver | `argmin` + `argmin-math` | Has LevenbergMarquardt solver. Or hand-roll (~100 LOC). |
| Linear algebra for solver | `nalgebra` | Dense matrices for Jacobian, SVD for rank detection. Already in stack. |
| Entity storage | `slotmap` | Generational arena with stable IDs. Widely used in game/ECS code. |
| 2D tessellation (arcs/circles to polylines) | `lyon` | Already noted in RESEARCH.md. |
| 2D boolean ops (contour detection) | `geo` | Rust geo library. Has polygon operations. |
| Robust predicates | `robust` | For reliable orientation/incircle tests in contour detection. |

### 9. Risks and Open Questions

- **Performance of LM on large sketches:** Typical SolidWorks sketches have 10-50
  entities. LM with dense matrices handles this in <1ms. Only becomes a problem with
  hundreds of entities. Not a near-term risk.

- **Automatic constraint inference:** SolidWorks aggressively auto-constrains (horizontal,
  vertical, coincident) as you draw. This is a UX feature, not a solver feature. Implement
  as a separate "inference" pass that proposes constraints based on proximity/angle
  thresholds. Let the user accept/reject.

- **Multiple solutions:** A constraint system can have multiple valid solutions (e.g., a
  triangle with given side lengths can be reflected). LM converges to the solution nearest
  the initial guess, which is the current entity positions. This is correct behavior —
  the sketch "remembers" its shape and only moves entities as needed. But it means the
  initial positions when drawing matter.

- **Circular arcs are tricky:** Their parameterization (center + radius + angles) creates
  nonlinear constraint equations that are harder to converge. SolveSpace parameterizes
  arcs differently (center + two points on the arc) to improve convergence.

- **When to re-solve:** SolidWorks re-solves after every constraint change and during
  dragging (every mouse move event). The solver must be fast enough for 60fps dragging.
  For LM with <50 entities, this is achievable. Cache the Jacobian structure and reuse
  allocated matrices between solves.
