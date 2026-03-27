# GraniteX — Research Notes

Last updated: 2026-03-27

---

## Reference & Construction Geometry — Professional CAD UX Research (2026-03-27)

Research into how SolidWorks, Fusion 360, and Onshape handle reference geometry, construction geometry, and advanced feature workflows (revolve, sweep, mirror, pattern). Focus on UX/interaction patterns for GraniteX implementation planning.

### 1. Default Planes (Front / Top / Right)

**SolidWorks behavior:**
- Three default planes exist at the origin: Front, Top, Right. They appear in the FeatureManager Design Tree before the Origin node.
- **Cannot be deleted or reordered** — they are permanent fixtures.
- **Hidden by default** in the 3D viewport. They only appear automatically when:
  - Starting the very first sketch in a new part (so the user can pick which plane to sketch on).
  - The user selects a plane in the FeatureManager tree (temporary highlight).
- **Manual show/hide:** Right-click plane in FeatureManager > click eyeball icon (Show/Hide). Or View > Planes to toggle all.
- **Visual appearance when shown:** Semi-transparent rectangles with labeled borders. They extend a finite visual distance from the origin (not infinite). Color-coded edges. They fade when not actively selected.
- **Keyboard shortcut:** Can be assigned via Tools > Customize > Keyboard for "Show/Hide Planes."

**Fusion 360 behavior:**
- Origin folder contains XY, XZ, YZ planes plus X, Y, Z axes and a single origin point.
- Hidden by default; click the lightbulb icon next to Origin folder or individual items to show.
- Planes display as semi-transparent rectangles similar to SolidWorks.

**GraniteX implication:** We need 3 default planes at origin, hidden by default, shown on demand. They should be semi-transparent rectangles with labeled edges. They must appear automatically when user enters "pick a plane" mode for sketch creation.

### 2. Reference Plane Creation Methods

**Access:** Insert > Reference Geometry > Plane (or Features toolbar dropdown > Reference Geometry > Plane)

**The UI is adaptive** — what the user selects determines what type of plane gets created. Up to 3 references can be selected.

| Method | User Selects | Result |
|--------|-------------|--------|
| **Offset** | 1 face or plane | Plane parallel at specified distance. Can create multiple equally-spaced planes at once. |
| **At Angle** | 1 face/plane + 1 edge/line | Plane rotated by specified angle around the edge. Can create multiple at angle intervals. |
| **Through 3 Points** | 3 points/vertices | Plane passing through all 3 points. |
| **Parallel at Point** | 1 plane/face + 1 point | Plane parallel to the face, passing through the point. |
| **Mid-Plane** | 2 faces (parallel or not) | Plane halfway between the two faces. Most common for mirror operations. |
| **Normal to Curve** | 1 curve/edge (+ optional point) | Plane perpendicular to curve at its closest endpoint. |
| **On Surface** | Point on non-planar surface | Tangent plane at that point. |
| **Tangent to Cylinder** | Cylindrical face + plane | Plane tangent to cylinder, at angle from reference plane. |

**Key UX pattern:** The PropertyManager dynamically updates as the user makes selections. The system infers the plane type from the combination of references. The user doesn't explicitly choose "offset" vs "angle" — it's automatic based on what geometry they pick.

**Caution noted in docs:** If you click the Sketch button while you have a part edge selected, SolidWorks silently creates a reference plane and starts a sketch on it. This is a convenience shortcut but can confuse users.

### 3. Reference Axes

**Access:** Insert > Reference Geometry > Axis

**Creation methods:**

| Method | User Selects | Result |
|--------|-------------|--------|
| **One Line/Edge/Axis** | Any single straight element | Axis along that line |
| **Two Planes** | 2 planes | Axis at their intersection line |
| **Two Points/Vertices** | 2 points | Axis through both points |
| **Cylindrical/Conical Face** | 1 cylindrical or conical face | Axis through center of the cylinder/cone |
| **Point + Face/Plane** | 1 point + 1 face | Axis normal to face, through point |

**Temporary Axes:**
- SolidWorks **automatically** creates a temporary axis for every cylindrical and conical feature in the model (holes, fillets on round parts, etc.).
- **Hidden by default.** Toggle via View > Temporary Axes.
- These are extremely useful for circular patterns and revolve operations — users can reference them without explicitly creating reference axes.
- Display: Shown as dashed lines through the model when visible.

**Default origin axes:** X, Y, Z axes exist at the origin alongside the default planes. Also hidden by default, toggled from the FeatureManager Origin folder.

### 4. Construction Lines / Centerlines in Sketches

**Two ways to create construction geometry in sketches:**
1. Draw a regular line, then check "For construction" / "Construction Geometry" checkbox in the PropertyManager.
2. Use the dedicated **Centerline** tool (Tools > Sketch Entities > Centerline) which creates construction lines directly.

**Visual difference:** Construction lines display as dashed/dot-dash lines (vs solid lines for real geometry). They are NOT included in the profile used for extrude/revolve — they serve purely as references.

**Centerline special properties:**
- Acts as an axis of revolution for the Revolve feature.
- Enables **diameter dimensioning**: when you dimension a line relative to a centerline, the dimension automatically shows as a diameter (with the "dia" symbol) rather than a radius.
- If there is exactly 1 centerline in the sketch, Revolve **automatically selects it** as the axis. If multiple centerlines exist, user must manually pick one.

**Key UX pattern for revolve sketches:**
1. Start sketch on a plane.
2. Draw centerline FIRST (best practice — establishes the axis of revolution).
3. Draw the profile on ONE SIDE of the centerline only.
4. When adding dimensions, select both the horizontal line and the centerline → dimension appears as diameter (cursor position above/below centerline toggles radius vs diameter display).
5. Fully constrain the sketch.
6. Exit sketch or directly click Revolve.

### 5. Revolve Operation — Complete UX Flow

**Access:** Insert > Boss/Base > Revolve, or Features tab > Revolved Boss/Base button.

**Pre-requisite:** A sketch containing a closed (or open for thin-feature) profile AND a centerline/axis.

**Step-by-step:**
1. **If sketch is active:** Click Revolved Boss/Base. SW auto-detects the profile and centerline.
2. **If no sketch active:** Click Revolved Boss/Base. SW prompts to select a sketch, or to create a new one.
3. **PropertyManager opens with these fields:**
   - **Axis of Revolution** — auto-filled if single centerline exists. Blue selection box; user can click to change.
   - **Direction type dropdown:**
     - One-Direction (default) — revolves in one angular direction
     - Mid-Plane — revolves symmetrically around sketch plane (e.g., 180 means 90 each side)
     - Two-Direction — separate angle controls for each direction
   - **Angle** — degrees of revolution (default 360). For Two-Direction: Angle1 and Angle2 separately.
   - **Thin Feature checkbox** — if checked, adds wall thickness field. Converts solid revolve to hollow shell.
   - **Selected Contours** — if sketch has multiple closed regions, user picks which to revolve.
4. **Preview** updates live in the viewport as settings change.
5. **Click green checkmark** (OK) to confirm, or red X to cancel.

**Post-creation editing:** Right-click the feature in FeatureManager > Edit Feature (changes revolve parameters) or Edit Sketch (changes the 2D profile). Feature auto-updates when sketch changes.

### 6. Sweep Operation — Complete UX Flow

**Access:** Insert > Boss/Base > Sweep, or Features tab > Swept Boss/Base.

**Pre-requisite:** Two separate sketches:
1. **Path sketch** — the trajectory the profile follows (can be open or closed curve).
2. **Profile sketch** — the cross-section shape to sweep along the path.

**Critical constraint: Pierce Relation**
The profile sketch MUST touch the path sketch. This is enforced by adding a "Pierce" sketch relation between a point on the profile and the path line. Pierce means "constrain this point to lie on this 3D curve where it pierces the sketch plane."

**Step-by-step:**
1. **Create path sketch first** (e.g., on Right Plane — a spline, arc, or series of lines). Exit sketch.
2. **Create profile sketch second** (e.g., on Front Plane — a circle, rectangle, or complex profile).
   - While in the profile sketch, select the center point of the profile and the path line, then add a Pierce relation.
   - This snaps the profile's origin to where the path pierces the profile's sketch plane.
3. Exit sketch.
4. Click **Swept Boss/Base** from Features tab.
5. **PropertyManager opens:**
   - **Profile** selection box (highlighted blue) — click the profile sketch.
   - **Path** selection box — click the path sketch.
   - **Options:**
     - Orientation/Twist Control (Follow Path, Keep Normal Constant, etc.)
     - Guide Curves (for varying cross-section along path)
     - Merge Tangent Faces
     - Thin Feature option
6. Live preview shows the swept solid.
7. Click OK.

**Shortcut for circular profiles:** If you just need a round cross-section (like a pipe), the sweep tool has a built-in circular profile option — you only need the path sketch, and specify a diameter. No second sketch needed.

### 7. Mirror Operation — Complete UX Flow

**Access:** Insert > Pattern/Mirror > Mirror Feature.

**PropertyManager fields:**

| Field | Description |
|-------|-------------|
| **Mirror Face/Plane** | Select a plane or planar face as the mirror symmetry plane. Can be a default plane, reference plane, or any flat face. |
| **Features to Mirror** | Pick features from the FeatureManager tree or click them in the viewport. Only these features get mirrored. |
| **Faces to Mirror** | Alternative: pick individual faces instead of entire features. |
| **Bodies to Mirror** | Alternative: mirror entire solid bodies. Select from graphics area only (not tree). |

**Options checkboxes:**
- Geometry Pattern — faster computation, doesn't re-solve each mirrored feature individually.
- Propagate Visual Properties — copies appearance/color to mirrored features.
- Full Preview / Partial Preview — controls how much of the result shows during setup.

**Two distinct mirror workflows:**

**A. Mirror Feature (within same part):**
1. Click Insert > Pattern/Mirror > Mirror Feature.
2. Select the mirror plane (commonly the Mid-Plane or a default plane like Right Plane).
3. Select features/faces/bodies to mirror.
4. Click OK. Mirrored geometry appears as a new feature in the FeatureManager.

**B. Mirror Part (creates new part file):**
1. Select a planar face on the part.
2. Insert > Mirror Part.
3. A new part file is created that is the mirror image.
4. Optionally linked — changes to the original propagate to the mirror.

### 8. Pattern Operations — Complete UX Flow

#### Linear Pattern

**Access:** Insert > Pattern/Mirror > Linear Pattern.

**PropertyManager fields:**
- **Direction 1:**
  - Direction reference — select an edge, axis, or linear sketch entity to define the pattern direction.
  - Reverse Direction button (flips the pattern direction).
  - Spacing — distance between instances.
  - Number of Instances — total count including the original.
- **Direction 2** (optional, for 2D grid patterns):
  - Same fields as Direction 1 but for the perpendicular direction.
- **Features and Faces:**
  - Features to Pattern — select which features to repeat.
  - Faces to Pattern — alternative to features.
- **Options:**
  - Geometry Pattern — performance optimization.
  - Vary Sketch — allows pattern instances to adapt to local geometry.
  - Instances to Skip — click specific pattern positions to exclude them.

**Step-by-step:**
1. Click Linear Pattern.
2. Select a linear edge for Direction 1 (or it auto-fills if you pre-selected a feature with an obvious direction).
3. Set spacing (e.g., 25mm) and instance count (e.g., 5).
4. Optionally enable Direction 2 for a grid.
5. Select features to pattern.
6. Preview shows all instances. Click any instance dot to skip it.
7. Click OK.

#### Circular Pattern

**Access:** Insert > Pattern/Mirror > Circular Pattern.

**PropertyManager fields:**
- **Pattern Axis** — select a temporary axis, reference axis, edge, or cylindrical face. This defines the center of rotation.
- **Angle** — total angle to span (360 for full circle).
- **Number of Instances** — total count including original.
- **Equal Spacing checkbox** — when checked, instances are evenly distributed over the angle. When unchecked, user specifies angle between each instance.
- **Features to Pattern / Faces to Pattern / Bodies to Pattern** — what to repeat.
- **Instances to Skip** — click dots to exclude.

**Step-by-step:**
1. Click Circular Pattern.
2. Select the axis of rotation (commonly a temporary axis from a cylindrical feature, or an explicit reference axis).
3. Set total angle (typically 360) and instance count (e.g., 6 for hex pattern).
4. Check Equal Spacing for uniform distribution.
5. Select features to pattern.
6. Preview shows circular arrangement. Click dots to skip any.
7. Click OK.

### Key UX Patterns Across All Features

1. **PropertyManager is the control panel** — always appears on the left when a feature is active. Blue highlight = active selection box. Click different boxes to switch what you're selecting.
2. **Live preview** — 3D viewport updates in real-time as you change parameters.
3. **Auto-detection** — SW tries to infer references automatically (single centerline = revolve axis, pre-selected sketch = active sketch, etc.).
4. **Selection boxes** — each PropertyManager has multiple selection boxes. Only one is "active" (blue) at a time. Clicking geometry fills the active box. Click a different box to switch context.
5. **Green checkmark / Red X** — universal confirm/cancel in PropertyManager.
6. **Feature tree is history** — every operation becomes a node in the FeatureManager tree, editable at any time. Right-click > Edit Feature or Edit Sketch.
7. **Temporary axes are free** — every cylinder auto-generates a usable axis. Users rarely need to create explicit reference axes.

### GraniteX Implementation Priority Notes

For our implementation, the most impactful features to build first are:
1. **Default planes + origin display** — needed for literally everything.
2. **Revolve** — highest bang-for-buck new feature (just needs centerline support in sketcher).
3. **Mirror** — conceptually simple (reflect geometry across plane), massively useful.
4. **Linear/Circular Pattern** — transformational for mechanical parts.
5. **Sweep** — complex but powerful; needs the pierce relation concept.
6. **Reference plane creation** — offset and mid-plane are the most commonly used.
7. **Reference axes** — mostly handled by temporary axes; explicit creation is lower priority.

---

## Rust BREP Kernel Evaluation (2026-03-27)

### Candidates Evaluated

**truck (1.4k GitHub stars, pure Rust)**
- NURBS + BREP + tessellation + wgpu rendering
- STEP I/O exists
- **FATAL FLAWS: No fillet/chamfer. Fragile boolean operations. Development stalled since Sept 2024.**
- CADmium (Rust CAD project) was archived partly because truck lacked these capabilities.

**opencascade-rs (228 stars, wraps C++ OCCT)**
- Full fillet, chamfer, booleans, extrude, revolve, loft, sweep — all battle-tested (30+ years)
- STEP/STL/SVG/DXF I/O
- Uses glam (same as GraniteX)
- **Downsides:** C++ dependency (~17GB during build on Windows), LGPL-2.1, solo maintainer, CMake + MSVC required
- **RECOMMENDED.**

**Fornjot (2.5k stars, pure Rust)**
- Self-described as "early-stage and experimental," "unsuited for real-world use cases"
- Development paused. Not viable.

**Others (monstertruck, cadk, chijin, breprs, hedron)**
- All <10 stars or abandoned. None viable.

### Decision
**Use opencascade-rs.** It's the only option that provides the operations needed for SolidWorks-class modeling. Isolate behind a trait boundary so the kernel can be swapped if a pure-Rust option matures.

### Architecture
```
GraniteX UI (egui + winit)  ← existing
Feature Tree + Sketch Solver ← new
CadKernel trait boundary     ← abstraction
impl OcctKernel              ← opencascade-rs
Tessellation → wgpu Renderer ← existing (adapted)
```

---

## CAD Kernel Internals & BREP Architecture (2026-03-26)

Deep-dive research into how professional CAD systems (SolidWorks/Parasolid, ACIS, OpenCASCADE)
work internally. Covers BREP data structures, surface representation, tessellation, boolean
operations, edge rendering, and what mesh-based CAD gets wrong.

### 1. BREP (Boundary Representation) Kernel Architecture

#### 1.1 Core Data Model

Professional CAD kernels (Parasolid used by SolidWorks/NX/Solid Edge, ACIS used by AutoCAD/
Fusion 360, OpenCASCADE used by FreeCAD) all use Boundary Representation. BREP separates
**topology** (connectivity/structure) from **geometry** (shape/position).

**Topological entities (the graph):**
- **Solid** — top-level container, a complete 3D body
- **Shell** — connected set of faces forming a closed (or open) boundary. A solid has one or
  more shells (outer shell + optional inner voids)
- **Face** — bounded region on a surface. References one underlying surface geometry + one or
  more loops
- **Loop** — closed circuit of edges bounding a face. Outer loop = boundary; inner loops = holes
- **Coedge (Half-edge)** — oriented use of an edge within a specific loop. Each edge has exactly
  two coedges (one per adjacent face). Stores: next/prev coedge in loop, mating coedge on
  adjacent face, parent face, parent edge
- **Edge** — bounded piece of a curve where two faces meet. References curve geometry + two
  endpoint vertices
- **Vertex** — point in 3D space where edges meet

**Geometric entities (the math):**
- **Point** — 3D coordinates (vertices)
- **Curve** — parametric curve: line, circle, ellipse, B-spline, NURBS (edges)
- **Surface** — parametric surface: plane, cylinder, cone, sphere, torus, NURBS (faces)

#### 1.2 The Topology/Geometry Separation

This is the critical insight:
- **Topology** = *what connects to what*. A graph. "Face A is adjacent to Face B via Edge 1."
- **Geometry** = *where in space*. Math equations. "Face A lies on plane z=5."

A single surface can be shared by multiple faces (trimmed differently). A single curve can be
shared by multiple edges. Adjacency queries ("which faces share this edge?") are O(1) — no
geometric computation needed.

#### 1.3 The Half-Edge Data Structure

Standard implementation for BREP topology:

```
HalfEdge {
    twin: HalfEdge,      // opposite half-edge (on adjacent face)
    next: HalfEdge,      // next half-edge in this face's loop
    prev: HalfEdge,      // previous (optional, derivable from next)
    vertex: Vertex,      // vertex this half-edge points TO
    edge: Edge,          // parent edge (shared with twin)
    face: Face,          // face this half-edge belongs to
}

Face {
    outer_loop: HalfEdge,      // any half-edge on outer boundary
    inner_loops: Vec<HalfEdge>, // holes
    surface: Surface,           // underlying surface geometry
}

Edge {
    half_edge: HalfEdge,   // one of the two half-edges
    curve: Curve,          // underlying curve geometry
}

Vertex {
    half_edge: HalfEdge,   // any outgoing half-edge
    point: Point,          // 3D position
}
```

**Impact on GraniteX:** Our current mesh uses flat vertex/index arrays with face_ids. No adjacency
information. Finding which faces share an edge = linear scan. Half-edge gives O(1) adjacency.

#### 1.4 Parasolid Specifics

Parasolid (SolidWorks' kernel) uses hierarchical organization:
Body > Region > Shell > Face > Loop > Coedge/Fin > Edge > Vertex

Supports solid bodies, sheet bodies (open surfaces), wireframe, and mixed models. All topology
modifications use Euler operations maintaining: `V - E + F - (L - F) - 2(S - G) = 0`

### 2. Surface Representation & Tessellation

#### 2.1 How Faces Are Stored (NOT as triangles)

Each face = underlying **parametric surface** + **trimming loops**

Common surface types:
- **Plane**: `P(u,v) = origin + u*dir_u + v*dir_v`
- **Cylinder**: `P(u,v) = center + r*cos(u)*x + r*sin(u)*y + v*z`
- **Sphere**: `P(u,v) = center + r*cos(v)*cos(u)*x + r*cos(v)*sin(u)*y + r*sin(v)*z`
- **NURBS Surface**: General freeform via control points, weights, knot vectors

#### 2.2 Tessellation for Rendering

BREP is tessellated to triangles ONLY for display. The BREP stays authoritative. Parameters:

- **Chord height tolerance**: Max perpendicular distance from triangle edge midpoint to true
  surface curve. Default ~0.1mm. If exceeded, triangle subdivides.
- **Angular tolerance**: Max angle between adjacent triangle normals.
- **Edge length tolerance**: Max triangle edge length.

Algorithm (adaptive):
1. Start with coarse triangulation of each face
2. For each triangle edge, measure chord height deviation from true surface
3. If deviation > tolerance, subdivide
4. Repeat until all triangles meet criteria
5. Result: flat regions → few large triangles, curved regions → many small triangles

**Why CAD models render clean:**
- Each face tessellated independently on its own surface
- Shared edge vertices synchronized (no cracks)
- Normals from analytical surface, not triangle geometry
- No coplanar overlapping triangles — each face is a unique spatial region

#### 2.3 Normal Computation in BREP

Normals come from parametric surface, NOT triangles:
- At any face point, evaluate surface partial derivatives dS/du and dS/dv
- Normal = normalize(dS/du x dS/dv)
- Mathematically perfect smooth normals everywhere, even on coarse mesh
- No faceted vs smooth trade-off — normals are always exact

### 3. Solid Modeling Operations

#### 3.1 How Extrude Works

Extrude = sweep operation. Process:
1. **Input**: Face/wire/sketch profile + direction vector
2. **Sweep topology**: Each entity generates a higher-dimensional entity:
   - Vertex → Edge (side edge)
   - Edge → Face (side wall)
   - Face → Solid (extruded volume)
3. **Create geometry**: Side edges get line curves; side faces get ruled surfaces (plane for
   straight edges, cylinder for circular edges, NURBS for freeform)
4. **Build topology**: New faces/edges/vertices, loops formed, half-edge connectivity established
5. **Boolean with existing body**: Union for boss, subtract for cut

Every step produces valid topology. Euler-Poincare holds throughout.

#### 3.2 Boolean Operations (Union, Subtract, Intersect)

Most complex operations in a BREP kernel:
1. **Surface-surface intersection**: Find all curves where surfaces of A intersect surfaces of B
2. **Curve classification**: Which portions lie on the other solid's boundary
3. **Face splitting**: Split faces along intersection curves
4. **Face classification**: Each face → inside/outside/on-boundary of other body
5. **Selection**: Union = outside + shared; Subtract = A-outside-B + B-inside-A-reversed;
   Intersect = inside both
6. **Sewing**: Connect selected faces into valid shell, build half-edge connectivity
7. **Validation**: Euler-Poincare check, manifoldness check

**Why booleans are hard:**
- Surface-surface intersection numerically delicate (tangent intersections, near-misses)
- Floating point causes gaps/overlaps
- Degenerate cases (edge-on-edge, vertex-on-face, coplanar) need special handling
- Result must be watertight

**Watertight boolean framework** (from academic research): Three-stage process — parametric
space analysis, reparameterization, model space update — ensuring gap-free results.

#### 3.3 Euler Operations

All BREP topology modifications go through atomic Euler operators that maintain the
Euler-Poincare invariant:
- **MEV** (Make Edge, Vertex): Insert vertex, splitting edge
- **MEF** (Make Edge, Face): Connect two vertices in a face, splitting it
- **MVFS** (Make Vertex, Face, Shell): Create new body
- **KEV/KEF**: Inverse operations (kill edge/vertex, kill edge/face)
- **KFMRH**: Create hole in a face

Every valid solid = initial solid + finite sequence of Euler operations. Guarantees topological
validity at every step.

### 4. Edge Rendering in CAD Applications

#### 4.1 Edge Types

1. **Sharp/Feature edges**: From BREP topology where faces meet at significant angle (>15-30°).
   Stored explicitly — they ARE the edges in the half-edge structure.
2. **Silhouette edges**: Front-facing meets back-facing relative to camera. Computed per frame:
   for each edge, check `dot(face_A_normal, view_dir)` vs `dot(face_B_normal, view_dir)`.
   If signs differ → silhouette.
3. **Boundary edges**: Only one adjacent face (open surfaces). Half-edge has no twin.
4. **Smooth edges**: Tangent edges at fillet/blend transitions. May or may not render.

#### 4.2 How SolidWorks Renders Edges

- **Source**: Directly from BREP topology curves
- **Tessellation**: Edge curves → polylines using same chord height tolerance as faces
- **Rendering**: Lines with depth offset (polygon offset) to avoid z-fighting with face triangles
- **Width**: 1-2px, line primitives or screen-space quads
- **Silhouettes**: Per-frame iteration of BREP edges testing face orientation

#### 4.3 Why Edge Rendering Is Hard for Mesh-Based Apps

In triangle mesh, every triangle edge is an edge. Must distinguish:
- Feature edges (real geometric) from mesh edges (tessellation artifacts)
- Heuristic: dihedral angle between adjacent triangles > threshold (e.g., 30°)
- Approximate — misses subtle features, creates false edges on curved surfaces

### 5. Mesh vs BREP — Fundamental Problems

#### 5.1 Why Mesh-Based CAD Has Artifacts

**Loss of information**: Triangle mesh is lossy. Once converted, cannot recover:
- Flat face vs slightly curved face distinction
- True edge locations (feature vs tessellation)
- Parametric surface/curve definitions
- Design intent

**Z-fighting**: Coplanar faces from operations (e.g., two extrudes to same height on adjacent
faces) produce triangles in the same plane. Depth buffer can't distinguish → flickering. BREP
avoids this because kernel merges coplanar faces into one.

**Boolean operations on meshes**: Extremely fragile:
- Intersection detection O(n^2) without spatial acceleration
- New intersection vertices don't align with existing topology
- Retriangulation error-prone
- No watertightness guarantee — tiny gaps/overlaps accumulate
- Coplanar faces = degenerate intersections
- Each operation degrades mesh quality

**One-way conversion**: BREP → mesh is trivial (tessellate). Mesh → BREP is nearly impossible
in general. Can recognize primitives (planes, cylinders) but freeform surfaces are lost.

#### 5.2 Artifact Comparison Table

| Problem | Cause | BREP Solution |
|---------|-------|---------------|
| Faceted cylinders | Finite polygon approximation | Analytical surface + adaptive tessellation |
| Z-fighting coplanar faces | Same depth values | Kernel merges coplanar regions |
| Gaps at boolean intersections | FP imprecision in mesh cutting | Parametric intersection + gap-free reparameterization |
| Jagged edge lines | Triangle edges != true curves | Edge curves tessellated independently |
| Normal seam breaks | Incorrect averaging across features | Normals from analytical surface |
| Quality degradation over operations | Accumulated triangle errors | Operations on exact geometry, retessellate for display |

#### 5.3 Progressive Strategy for GraniteX

**Short-term (current mesh architecture):**
1. Store "feature edges" explicitly alongside mesh (not just triangle edges)
2. Smooth normals with crease angle threshold — share across smooth edges, split at sharp
3. Depth bias (polygon offset) for edge lines over faces
4. Edge extraction: unique edges with dihedral angle > threshold = feature edges
5. Detect and merge coplanar adjacent faces after operations

**Medium-term (hybrid architecture):**
1. Store lightweight BREP alongside mesh — faces know surface type (plane, cylinder, etc.)
2. Use BREP for operations, regenerate mesh for display
3. Analytical normals where possible (planes, cylinders) instead of averaged vertex normals
4. Proper boolean operations via `truck` crate

**Long-term (full BREP):**
1. Full BREP kernel (truck-rs or custom)
2. Mesh = purely display artifact, regenerated per model change
3. All operations on exact geometry
4. STEP import/export for interoperability

### 6. Normal Interpolation & Smooth Shading

#### 6.1 The Problem

Cylinder approximated by 12 flat faces looks faceted. Each triangle has single face normal →
visible edges between faces (flat shading).

#### 6.2 How Smooth Shading Works

Gouraud/Phong smooth shading (1971):
1. At each vertex, compute **vertex normal** = average of adjacent face normals (for faces
   sharing smoothing group)
2. During rasterization, interpolate vertex normal across triangle via barycentric coords
3. Compute lighting per-pixel with interpolated normal

Makes 12-sided cylinder look nearly round.

#### 6.3 Smooth Groups / Crease Angles

Not all edges should be smooth (cube needs hard edges):
- **Crease angle threshold**: If dihedral angle between adjacent faces > threshold (30-45°),
  edge is "hard" — normals NOT shared across it
- **Implementation**: At hard edges, duplicate vertices with different normals ("splitting")
- **In BREP**: Kernel knows which edges are smooth (tangent continuity) vs sharp. Exact, no
  threshold needed.

#### 6.4 Recommendations for GraniteX

**Immediate fix for cylinder faceting:**
1. When generating primitives (extrude creates cylinders), store intended surface type per-face
2. Cylindrical faces: vertex normals = radial direction from cylinder axis (not triangle average)
3. Planar faces: use plane normal (no interpolation)
4. General: crease-angle-based vertex normal computation, ~30° threshold

**Vertex format change:**
```
Current:  position + normal + face_id  (flat shading only)
Proposed: position + smooth_normal + face_id + flags
```
Vertices at hard edges duplicated with different normals.

### 7. Rust BREP Kernel Options

#### 7.1 truck-rs (github.com/ricosjp/truck)
- Pure Rust, modern. Topology: vertex/edge/wire/face/shell/solid. Meshing built-in. STEP I/O.
- Still maturing. Boolean robustness uncertain. Smaller community.

#### 7.2 opencascade-rs (github.com/bschwind/opencascade-rs)
- Wraps most battle-tested open-source BREP kernel. Full features.
- 3.6M lines C++ dependency. Build complexity. Not idiomatic Rust.

#### 7.3 Fornjot (fornjot.app)
- Pure Rust b-rep. Clean architecture. Very early stage, not production-ready.

#### 7.4 Recommendation
Start with **truck-rs** in Phase 9. Pure Rust, right abstractions, aligns with architecture.
Use initially for STEP I/O, clean tessellation with analytical normals, validation layer.
Keep mesh ops as primary for now, architect code so BREP backend can progressively take over.

### Research Sources

- [Boundary Representation — Wikipedia](https://en.wikipedia.org/wiki/Boundary_representation)
- [What is B-Rep? — Shapr3D](https://www.shapr3d.com/content-library/what-is-b-rep)
- [What is B-Rep? — Tech Soft 3D](https://www.techsoft3d.com/resources/blog/what-is-boundary-representation-b-rep-/)
- [Mesh vs BRep — CAD Exchanger](https://cadexchanger.com/blog/crash-course-on-cad-data-part-3/)
- [Parasolid — Wikipedia](https://en.wikipedia.org/wiki/Parasolid)
- [Geometric Modeling Kernel — Wikipedia](https://en.wikipedia.org/wiki/Geometric_modeling_kernel)
- [OpenCASCADE Modeling Algorithms](https://dev.opencascade.org/doc/overview/html/occt_user_guides__modeling_algos.html)
- [Watertight Boolean Operations — ScienceDirect](https://www.sciencedirect.com/science/article/abs/pii/S0010448519302106)
- [Topology in CAD — Novedge](https://novedge.com/blogs/design-news/design-software-history-topology-in-cad-manifoldness-genus-and-the-earliest-b-rep-formalisms)
- [Adaptive Tessellation for Trimmed NURBS](https://www.academia.edu/802844/Adaptive_Tessellation_for_Trimmed_NURBS_Surface)
- [Tessellation — Buerli.io](https://buerli.io/docs/training-for-designers/HowTos/Tessellation/)
- [NURBS — Wikipedia](https://en.wikipedia.org/wiki/Non-uniform_rational_B-spline)
- [Z-fighting — Wikipedia](https://en.wikipedia.org/wiki/Z-fighting)
- [Polygon Offset — OpenGL FAQ](https://www.opengl.org/archives/resources/faq/technical/polygonoffset.htm)
- [Silhouette Edges — Real-Time Rendering](https://www.realtimerendering.com/blog/drawing-silhouette-edges/)
- [Silhouette Edge — Wikipedia](https://en.wikipedia.org/wiki/Silhouette_edge)
- [Real-Time Feature Edges — Marc ten Bosch](https://marctenbosch.com/npr_edges/)
- [Half-Edge Data Structure — Stanford](http://graphics.stanford.edu/courses/cs268-16-fall/Notes/halfedge.pdf)
- [Winged Edge — Wikipedia](https://en.wikipedia.org/wiki/Winged_edge)
- [Half-Edge — CS418 Illinois](https://cs418.cs.illinois.edu/website/text/halfedge.html)
- [Euler Operations — Ohio State](https://ohiostate.pressbooks.pub/app/uploads/sites/45/2020/01/GM-Euler.pdf)
- [Euler-Poincare Formula — MTU](https://pages.mtu.edu/~shene/COURSES/cs3621/NOTES/model/euler.html)
- [truck-rs — GitHub](https://github.com/ricosjp/truck)
- [opencascade-rs — GitHub](https://github.com/bschwind/opencascade-rs)
- [Fornjot](https://www.fornjot.app/)
- [CADmium — Matt Ferraro](https://mattferraro.dev/posts/cadmium)
- [B-Rep vs Implicit — nTopology](https://ntopology.com/blog/2019/03/12/understanding-the-basics-of-b-reps-and-implicits/)
- [Smooth Shading — Scratchapixel](https://www.scratchapixel.com/lessons/3d-basic-rendering/introduction-to-shading/shading-normals.html)
- [Mixed Modeling — Onshape](https://www.onshape.com/en/blog/mixed-modeling-brep-geometry-mesh-data)

---

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

---

## CAD Rendering & Kernel Internals — Deep Research (2026-03-26)

Comprehensive technical research on how professional CAD systems handle rendering,
geometry representation, and constraint solving. Focused on practical implementation
details, specific algorithms, and data structures.

### 1. B-REP (Boundary Representation) — Data Structures

B-REP is THE core data structure of every serious CAD kernel (Parasolid, ACIS,
OpenCascade, C3D). It was independently developed by Ian Braid (Cambridge) and Bruce
Baumgart (Stanford) in the early 1970s.

**The fundamental idea:** A solid is defined not by its volume, but by its boundary
surfaces. The boundary is decomposed into a hierarchy of topological and geometric
entities.

**Topological hierarchy (from top to bottom):**

```
CompSolid (optional: collection of solids sharing faces)
  └─ Solid (a single watertight volume)
       └─ Shell (connected set of faces bounding a region)
            └─ Face (bounded portion of a surface)
                 └─ Loop / Wire (circuit of edges bounding a face)
                      └─ Edge (bounded piece of a curve)
                           └─ Vertex (point in space)
```

**The topology/geometry split is critical:**

| Topology (connectivity) | Geometry (shape)          |
|--------------------------|---------------------------|
| Face                     | Surface (plane, cylinder, NURBS, ...) |
| Edge                     | Curve (line, circle, B-spline, ...)   |
| Vertex                   | Point (x, y, z)                       |
| Loop                     | (no geometry — pure connectivity)     |
| Shell                    | (no geometry — pure connectivity)     |

Topology says HOW things connect. Geometry says WHERE they are in space. This
separation is what makes B-REP powerful — you can perform topological operations
(boolean union, fillet) without changing the geometric representations until
necessary.

**How B-REP differs from indexed triangle meshes:**

| Aspect          | B-REP                              | Triangle Mesh              |
|-----------------|-------------------------------------|----------------------------|
| Geometry        | Exact (NURBS, analytic surfaces)   | Approximate (flat triangles) |
| Topology        | Rich (face-edge-vertex + adjacency)| Minimal (index buffer)     |
| Operations      | Boolean, fillet, chamfer, offset   | Very limited               |
| Storage         | Compact (few faces for a cylinder) | Verbose (many triangles)   |
| Rendering       | Requires tessellation first        | Direct GPU rendering       |
| Precision       | Exact to floating point limits     | Depends on triangle count  |

A cylinder in B-REP: 3 faces (top, bottom, barrel), 3 edges, 0 or 2 vertices.
A cylinder in a triangle mesh: hundreds of triangles.

**ACIS-specific:** Uses "FACE -> LOOP -> COEDGE -> EDGE -> VERTEX" with COEDGE
being their version of a half-edge (a directed use of an edge within a loop).
Each edge stores two pcurves (parameter-space curves) — one for each adjacent face.

**OpenCascade-specific:** Uses the TopoDS framework with shapes classified as
TopoDS_Vertex, TopoDS_Edge, TopoDS_Wire, TopoDS_Face, TopoDS_Shell, TopoDS_Solid,
TopoDS_CompSolid, TopoDS_Compound. Each shape has an "orientation" (Forward/Reversed)
that determines which side is material.

**Implication for GraniteX:** Our current mesh-based approach (indexed triangles with
face metadata) is fine for rendering but will NOT support real CAD operations. When
we add boolean operations, fillets, or STEP export, we need a real B-REP kernel.
Options: build our own (huge effort), use truck (Rust-native), or FFI to OpenCascade.

### 2. Surface Tessellation — NURBS to Triangles

Every CAD system must convert its exact B-REP geometry to triangles for GPU display.
This is called tessellation or meshing.

**The pipeline:**

```
B-REP Face (exact surface)
  -> Sample points on surface respecting curvature
  -> Triangulate in parameter space (u,v domain)
  -> Map triangles back to 3D
  -> Result: Poly_Triangulation per face
```

**Chordal deviation (linear deflection):**

The maximum allowed distance between the exact surface and its triangulated
approximation. If a triangle's midpoint is more than epsilon away from the true
surface, that triangle must be subdivided.

```
                true curve
               /         \
              /    d       \     d = chordal deviation
             /     |        \   (distance from chord midpoint to curve)
            *------+--------*
            chord (triangle edge)
```

Typical values: 0.1mm for coarse display, 0.01mm for high quality, 0.001mm for
export/printing.

**Angular deflection:**

The maximum angle (in radians) between adjacent triangle edge segments approximating
a curve. Limits the angular "jump" between facets. Typical: 0.5 rad (coarse) to
0.1 rad (fine).

**The algorithm (OpenCascade's BRepMesh_IncrementalMesh):**

1. **Discretize edges first:** Each edge of the face is tessellated into a polyline
   respecting both linear and angular deflection. This gives boundary nodes.

2. **Initial triangulation:** Create an initial Delaunay triangulation of the face's
   parameter domain (u,v space) using boundary nodes. OpenCascade uses Watson's
   incremental Delaunay algorithm.

3. **Refinement loop:** For each triangle, check if the 3D midpoint deviates from the
   true surface by more than the linear deflection tolerance. If yes, insert a new
   point at the surface midpoint and re-triangulate. Repeat until all triangles
   satisfy the tolerance. This is BRepMesh_DelaunayDeflectionControlMeshAlgo.

4. **Trimming:** For trimmed NURBS surfaces (surfaces with holes or non-rectangular
   boundaries), use Constrained Delaunay Triangulation (CDT) to respect the trim
   curves as forced edges.

**Adaptive tessellation:** High-curvature regions get more triangles, flat regions
get fewer. This is automatic from the deflection-based refinement. A flat face might
get 2 triangles; a small fillet might get 50.

**For GraniteX:** We currently generate fixed-segment cylinders. We should move toward
deflection-based adaptive tessellation. The spade crate provides Delaunay
triangulation in Rust. For parametric surfaces, sample at (u,v) points and use
CDT in the parameter domain.

### 3. Edge Rendering — Sharp Edges and Wireframe Overlay

CAD systems render edges as lines overlaid on the shaded solid. This is the
"Shaded with Edges" display mode that's the default in every CAD tool.

**Three types of edges in CAD rendering:**

1. **Sharp/crease edges:** Where two faces meet at a dihedral angle above a threshold
   (typically > 30 degrees). These are the B-REP edges — they come directly from
   the topology. Examples: edges of a box, edge between a cylinder top cap and barrel.

2. **Silhouette edges:** Where a front-facing triangle meets a back-facing triangle
   (relative to the camera). These are view-dependent — they change as the camera
   rotates. They give curved surfaces their "outline." SolidWorks has explicit
   silhouette edge computation and display.

3. **Boundary edges:** Edges shared by only one face (open mesh boundaries). In a
   valid solid B-REP, there are none. Only appear with sheet bodies or incomplete
   geometry.

**How SolidWorks detects sharp edges:**

Sharp edges are pre-computed from the B-REP topology. Every edge in the B-REP
that separates two faces with a dihedral angle above the crease threshold is
marked as "sharp" and rendered as a visible line. This is NOT a runtime
computation — it's metadata on the edge.

The dihedral angle is computed from the face normals at the edge:

```
cos(dihedral) = dot(normal_face1, normal_face2)
if cos(dihedral) < cos(crease_threshold):
    edge is SHARP -> draw it
```

**How edges are rendered without z-fighting:**

The critical technique is **polygon offset / depth bias**. When rendering edge
lines on top of filled faces:

1. Render the shaded solid normally (depth test ON, depth write ON).
2. Apply a depth bias (polygon offset) that pushes filled polygons slightly
   AWAY from the camera in depth buffer space.
3. Render edges as lines — they now pass the depth test because the faces
   behind them have been pushed back.

In OpenGL: `glPolygonOffset(1.0, 1.0)` applied during the face render pass.
In wgpu: set `depth_bias` and `depth_bias_slope_scale` in the
`DepthStencilState` of the face render pipeline.

**The formula:**

```
offset = factor * max_depth_slope + units * min_resolvable_depth
```

Where factor scales with the polygon's depth slope (steeper polygons get more
offset) and units is a fixed bias. Typical values: factor=1.0, units=1.0 for
the FACE pass (push faces back), edges rendered with factor=0, units=0.

**Alternative: render edges in a separate pass with depth test but no depth write,
using a slight depth offset.** This is cleaner and what most modern CAD renderers do.

**For silhouette edges:** These require a per-frame computation. For each edge,
check if one adjacent face is front-facing and the other is back-facing:

```
for each edge with faces f1, f2:
    d1 = dot(f1.normal, view_direction)
    d2 = dot(f2.normal, view_direction)
    if d1 * d2 < 0:  // one positive, one negative
        edge is a silhouette edge
```

SolidWorks 2024 added GPU-accelerated silhouette edge computation for performance.

**For GraniteX:** Our current edge rendering approach uses depth bias, which is
correct. The key improvement needed: extract sharp edges from B-REP topology
(we already know which edges are "sharp" from face adjacency), and implement
silhouette edge detection for curved surfaces.

### 4. Shading Models — Smooth Curved Surfaces

CAD systems use **Phong shading** (per-pixel normal interpolation) to make faceted
meshes look like smooth curved surfaces.

**The three shading approaches, in order of quality:**

1. **Flat shading:** One normal per triangle. Each triangle is a uniform color.
   Produces visible faceting. Never used in modern CAD display.

2. **Gouraud shading:** Compute lighting at vertices, interpolate colors across
   the triangle. Fast but misses specular highlights in triangle interiors.
   Used in older CAD systems.

3. **Phong shading:** Interpolate NORMALS across the triangle (not colors), then
   compute lighting per-pixel using the interpolated normal. Produces smooth
   shading with correct specular highlights. This is what every modern CAD
   system uses, and what GPUs do natively in the fragment shader.

**Per-vertex normal computation for smooth shading:**

For a tessellated B-REP surface, the vertex normal should be the TRUE surface
normal at that point, not an average of face normals. Since the B-REP has the
exact surface definition, we can evaluate the surface normal analytically:

```
For a point at parameter (u, v) on a NURBS surface S:
    normal = normalize(dS/du x dS/dv)
```

For analytic surfaces:
- **Plane:** normal is constant (face normal).
- **Cylinder:** normal = normalize(point - axis_projection). Points radially
  outward from the axis, independent of height.
- **Sphere:** normal = normalize(point - center).
- **Cone:** normal = computed from cone geometry.

**When exact normals are NOT available (e.g., our current mesh-based approach):**

Average the face normals of adjacent triangles that share a vertex, BUT only
average across SMOOTH edges (dihedral angle below crease threshold):

```
for each vertex v:
    normal = vec3(0)
    for each face f adjacent to v:
        if all edges connecting f to v's other adjacent faces are SMOOTH:
            normal += f.face_normal * f.area  // area-weighted
    vertex_normal = normalize(normal)
```

**The smooth-to-sharp transition:**

At a sharp edge, a vertex has DIFFERENT normals for each side. This means the
vertex must be DUPLICATED in the vertex buffer — one copy for each face group
on each side of the sharp edge.

```
Sharp edge between face A and face B:
    vertex_for_face_A.normal = computed from face A's smooth group
    vertex_for_face_B.normal = computed from face B's smooth group
    Same position, different normals -> vertex duplication
```

This is why our cylinder vertices need to be duplicated at the cap edges —
the barrel wants smooth normals pointing radially, the cap wants flat normals
pointing along the axis.

**For GraniteX:** We already do this correctly for cylinders (radial normals on
barrel, axis normals on caps). The general solution is: when tessellating a B-REP,
set each vertex's normal to the true surface normal at that point. At sharp edges
(B-REP edges), duplicate vertices with different normals for each face.

### 5. Z-Fighting Prevention

Z-fighting occurs when two primitives have nearly identical depth values, causing
flickering as the depth test alternates between them depending on floating point
rounding.

**Where it happens in CAD:**

1. **Sketch overlay on face:** 2D sketch geometry lies exactly on the 3D face.
2. **Edge lines on filled faces:** Wireframe edges are coplanar with faces.
3. **Coincident faces:** Boolean operation results, imported geometry.
4. **Decals/labels on surfaces.**

**Prevention techniques (ordered by effectiveness):**

**A. Reversed floating-point depth buffer (BEST):**

Map near plane to depth=1.0, far plane to depth=0.0, using a 32-bit float depth
buffer. This exploits the fact that floating-point has more precision near zero,
which cancels the 1/z nonlinearity perfectly.

Result: near-uniform depth precision from near to far plane. "Ridiculously more
accurate" than standard depth mapping. Makes z-fighting essentially disappear
for normal geometry.

In wgpu: use `TextureFormat::Depth32Float`, configure projection matrix for
reversed Z, set depth compare to `Greater` instead of `Less`.

**B. Polygon offset / depth bias:**

Push one set of geometry slightly in depth. Used for edge-on-face rendering.

wgpu `DepthBiasState`:
```rust
depth_bias: 2,           // constant bias in depth units
depth_bias_slope_scale: 1.0,  // scale by polygon slope
depth_bias_clamp: 0.0,   // max bias
```

Apply to the FACE pass (push faces back) so edges pass the depth test.

**C. Separate render passes with depth manipulation:**

For sketch overlays:
1. Render the 3D solid (depth test ON, depth write ON).
2. Clear depth or write a constant depth for the sketch plane region.
3. Render sketch geometry (depth test ON against sketch plane only).

Alternatively: render sketch with depth test OFF (always on top) but this
breaks when the sketch plane is partially behind other geometry.

**D. Logarithmic depth buffer:**

Use `log2(z)` instead of `1/z` for depth values. Gives approximately uniform
precision. Used in large-scale scenes (flight simulators). Overkill for CAD
but worth knowing.

**E. Stencil buffer tricks:**

Mark regions with the stencil buffer, then use stencil test to control which
geometry renders where. Useful for complex overlay scenarios.

**Near/far plane ratio matters enormously:**

With standard depth buffers, precision scales with `far/near` ratio. A ratio
of 1000:1 is acceptable. 100000:1 causes severe z-fighting. Solutions:
- Keep near plane as far as possible (0.1m, not 0.001m).
- Keep far plane as close as possible (1000m, not 1000000m).
- Or use reversed-Z float depth, which makes the ratio nearly irrelevant.

**For GraniteX:** We should implement reversed-Z depth buffer (it's a one-time
setup change with huge benefit) and use polygon offset for edge rendering. For
sketch overlays, use a small depth bias plus reversed-Z.

### 6. Fan Triangulation vs Ear Clipping

**Fan triangulation:** Pick one vertex (the "fan vertex"), draw diagonals to all
other non-adjacent vertices. Creates n-2 triangles from an n-gon.

```
Fan from vertex 0:
    Triangle(0, 1, 2)
    Triangle(0, 2, 3)
    Triangle(0, 3, 4)
    ...
```

**Why fan triangulation FAILS for concave polygons:**

If the fan vertex is a convex vertex and the polygon has a concavity elsewhere,
some fan triangles will extend OUTSIDE the polygon boundary. The resulting
triangulation covers area that isn't part of the original polygon.

```
Concave polygon:         Fan triangulation:
    1---2                    1---2
   /     \                  /|  / \
  0   .   3                0 | /   3    <- triangle 0-2-3
   \     /                  \|/   /        extends OUTSIDE
    5---4                    5---4          the polygon!
```

Only polygons with exactly one concave vertex can be fan-triangulated (from that
concave vertex). Any polygon with 2+ concavities cannot be correctly fan-
triangulated from ANY vertex.

**FBX SDK notoriously uses fan triangulation**, producing incorrect results for
concave faces. This is a known bug that has persisted for years.

**Ear clipping algorithm:**

An "ear" is a triangle formed by three consecutive polygon vertices where:
1. The triangle is entirely inside the polygon.
2. No other vertices are inside the triangle.

The algorithm:
1. Find an ear (a convex vertex whose triangle contains no other vertices).
2. Remove it (output the triangle, remove the middle vertex from the polygon).
3. Repeat until only one triangle remains.

Complexity: O(n^2) with convex/concave vertex lists. O(n^3) naive. Works for ALL
simple polygons (convex, concave, with holes via bridge edges). The `earcut` crate
(Rust port of Mapbox's earcut.js) is fast and robust.

**Constrained Delaunay Triangulation (CDT):**

The gold standard for CAD tessellation. Delaunay triangulation maximizes the minimum
angle (avoids skinny triangles), and the "constrained" variant forces specific edges
(polygon boundaries, holes) into the triangulation.

Algorithm: Start with a Delaunay triangulation of all vertices, then insert
constrained edges by flipping adjacent triangles. The `spade` crate provides CDT
in Rust.

**What CAD systems actually use:**

- OpenCascade: Watson's incremental Delaunay + deflection-based refinement.
- Most production CAD: CDT for face tessellation (best triangle quality).
- Game engines: earcut (fast, handles holes, acceptable quality).

**For GraniteX:** Use earcut for simple cases (convex polygons, sketch profiles).
Move to CDT (spade crate) for production tessellation of B-REP faces.

### 7. Smooth Cylinder Rendering

**The problem:** A cylinder is tessellated into N rectangular strips around its
circumference, each split into 2 triangles. With flat shading, you see N distinct
facets. The goal is to make it look perfectly smooth.

**The solution: per-vertex normals from the exact surface.**

For a cylinder with axis along Y, center at origin, radius R:

```
For a vertex at position (x, y, z) on the barrel:
    normal = normalize(x, 0, z)  // radial direction, ignoring Y
```

This is the TRUE surface normal of the mathematical cylinder at that point.
It does NOT depend on the tessellation — it's computed from the analytic surface.

When the GPU interpolates these normals across each triangle (Phong shading /
per-fragment lighting), the lighting varies smoothly as if the surface were truly
curved.

**How many segments are needed?**

| Segments | Degrees per facet | Visual quality                   |
|----------|-------------------|----------------------------------|
| 8        | 45                | Obviously faceted                |
| 16       | 22.5              | Acceptable for distant objects   |
| 24       | 15                | Good for most CAD display        |
| 32       | 11.25             | Very smooth                      |
| 48       | 7.5               | Near-perfect                     |
| 64       | 5.6               | Overkill for display             |

With proper normal interpolation, 24-32 segments look smooth to the eye. The
silhouette (outline) is the giveaway — it's always polygonal. More segments
improve the silhouette at the cost of more triangles.

SolidWorks uses adaptive tessellation: large cylinders get more segments, small
ones get fewer. The chordal deviation tolerance controls this automatically.

**Cap handling:**

The cap faces (top/bottom circles) need FLAT normals (0, 1, 0) or (0, -1, 0).
The cap vertices must be DUPLICATED — same position but different normal than the
barrel vertices at the same location. This creates the sharp edge between barrel
and cap.

```
At the edge between barrel and cap:
    barrel_vertex.pos  = (x, y, z)
    barrel_vertex.norm = normalize(x, 0, z)   // radial
    cap_vertex.pos     = (x, y, z)            // SAME position
    cap_vertex.norm    = (0, 1, 0)            // axis-aligned
```

**For GraniteX:** We already handle this correctly. The key insight: vertex normals
should come from the mathematical surface, not from averaging face normals. When
tessellating ANY analytic surface (plane, cylinder, cone, sphere, torus), compute
the normal analytically. Only fall back to face-normal averaging for arbitrary
meshes or imported geometry.

### 8. OpenCascade Technology (OCCT) — Rendering Architecture

OpenCascade is the open-source CAD kernel used by FreeCAD, CADRays, and many
commercial products. It's written in C++ with ~4M lines of code.

**Architecture overview:**

```
Application Level
    +-- OCAF (Application Framework) — document model, undo/redo
Modeling Level
    +-- BRep (exact geometry) — TopoDS shapes, Geom surfaces/curves
    +-- Boolean operations (BRepAlgo)
    +-- Fillets, chamfers (BRepFilletAPI)
Visualization Level
    +-- AIS (Application Interactive Services) — interactive 3D display
    +-- Prs3d (Presentation) — builds GPU-ready presentations
    +-- BRepMesh — tessellation engine
    +-- OpenGL / Vulkan backend (TKOpenGl)
```

**Tessellation pipeline (BRepMesh):**

1. **Input:** A TopoDS_Shape (B-REP solid).

2. **Edge discretization:** Each edge is discretized into a polyline. Deflection
   parameters control the accuracy. The PCurve (parameter-space curve) is used
   to get consistent boundary vertices for adjacent faces.

3. **Face meshing:** BRepMesh_IncrementalMesh processes each face:
   - Maps the face boundary (discretized edges) to the surface's (u,v) parameter space.
   - Creates an initial 2D Constrained Delaunay Triangulation (CDT) of the boundary.
   - Refines by inserting interior points where deflection exceeds tolerance.
   - Uses Watson's incremental Delaunay algorithm for the triangulation.
   - BRepMesh_DelaunayDeflectionControlMeshAlgo controls the refinement.

4. **Output:** Per-face Poly_Triangulation objects containing vertices, normals,
   UV coordinates, and triangle indices. Stored as attributes on the TopoDS_Face.

5. **Visualization:** AIS_Shape reads the Poly_Triangulation data and builds
   OpenGL vertex buffers. Shaded mode sends triangles, wireframe mode sends edges.

**Key parameters:**

```cpp
BRepMesh_IncrementalMesh mesh(shape, linearDeflection, isRelative, angularDeflection);
// linearDeflection: max distance from surface to triangle (mm)
// angularDeflection: max angle between edge segments (radians)
// isRelative: if true, linearDeflection is relative to bounding box size
```

**Edge presentation:** OCCT extracts wireframe edges directly from B-REP topology
(not from the tessellation). Edges are discretized into polylines following the
exact curve geometry. This is why edges are pixel-perfect — they don't depend on
the mesh quality.

**Performance:** Meshing is often the bottleneck. OCCT supports parallel face
meshing (each face is independent). Complex models with many small fillets are
expensive to tessellate. The Express Mesh module is a faster alternative for
visualization-only tessellation.

**For GraniteX:** If we ever integrate OpenCascade (via opencascade-rs), we get
this entire pipeline for free. For our custom kernel, the key takeaway is: tessellate
per-face, use CDT, refine by deflection, store results per-face, extract edges from
topology not from mesh.

### 9. Half-Edge Data Structure

The half-edge (also called DCEL — Doubly Connected Edge List) is the standard mesh
connectivity data structure used by CAD kernels and mesh editors.

**Core idea:** Every edge in the mesh is split into two HALF-EDGES, one for each
direction. Each half-edge "belongs to" one face and points from one vertex to another.

**What each element stores:**

```
HalfEdge {
    twin:   HalfEdgeId,   // the opposite half-edge (same geometric edge, reverse direction)
    next:   HalfEdgeId,   // next half-edge around the same face (CCW)
    prev:   HalfEdgeId,   // previous half-edge around the same face (optional, can derive)
    vertex: VertexId,     // the vertex this half-edge ORIGINATES from
    face:   FaceId,       // the face this half-edge borders (to the left)
    edge:   EdgeId,       // the parent edge (optional, for edge attribute storage)
}

Vertex {
    position: Vec3,
    halfedge: HalfEdgeId,  // ANY outgoing half-edge from this vertex
}

Face {
    halfedge: HalfEdgeId,  // ANY half-edge on this face's boundary
}

Edge {
    halfedge: HalfEdgeId,  // one of the two half-edges
}
```

**Traversal operations (all O(1) per step):**

```
// Iterate edges around a face:
start = face.halfedge
he = start
do:
    process(he)
    he = he.next
while he != start

// Iterate edges around a vertex (vertex ring / umbrella):
start = vertex.halfedge
he = start
do:
    process(he)
    he = he.twin.next   // go to twin, then advance to next face
while he != start

// Get the other vertex of an edge:
other_vertex = he.twin.vertex

// Get both faces of an edge:
face_left  = he.face
face_right = he.twin.face

// Get all vertices of a face:
// follow next pointers, collect he.vertex
```

**Why half-edge is better than indexed meshes for CAD:**

| Operation           | Half-Edge     | Indexed Mesh    |
|---------------------|---------------|-----------------|
| Adjacent faces      | O(1)          | O(n) scan       |
| Vertex neighbors    | O(k) k=valence| O(n) scan       |
| Edge split          | O(1) pointer update | Rebuild indices |
| Face subdivision    | O(1) pointer update | Rebuild indices |
| Extrude face        | O(k) per face | Rebuild entire mesh |
| Fillet edge         | Local updates | Rebuild entire mesh |
| Euler operators     | Natural       | Not supported   |

**Euler operators** (the fundamental B-REP editing operations) map directly to
half-edge manipulations:

- **MEV** (Make Edge-Vertex): Split a vertex by inserting a new edge. Creates one
  new vertex and one new edge (two half-edges).
- **MEF** (Make Edge-Face): Split a face by inserting a new edge between two vertices
  on the face boundary. Creates one new face and one new edge.
- **KEMR** (Kill Edge, Make Ring): Remove an edge to create an inner loop (hole) in
  a face. Used for creating holes in faces.
- **MEVVLFS** (Make Edge, Vertex, Vertex, Loop, Face, Shell): The "seed" operator
  that creates the initial topology from nothing.

These operators maintain topological validity (Euler's formula: V - E + F = 2 for
a closed shell). Every modeling operation (extrude, revolve, boolean) can be
decomposed into sequences of Euler operators.

**Winged edge vs half-edge:**

Winged edge is the predecessor. It stores more info per edge (both faces, all four
neighboring edges) but is harder to traverse because you need to check orientation
at each step. Half-edge is simpler — direction is built-in — and is the modern
standard. CGAL, Blender (BMesh), OpenCascade all use half-edge variants.

**For GraniteX:** When we build the real B-REP kernel, half-edge is the data
structure to use. For now, our flat mesh with face metadata is sufficient for
rendering, but we should design our Face/Edge/Vertex types with half-edge
migration in mind.

### 10. Sketch Constraint Solvers

(Extends the earlier research in section "Sketch-on-Face Architecture Research")

**How commercial systems work:**

**D-Cubed (Siemens):** The industry-standard constraint solver, used by SolidWorks,
NX, Solid Edge, Inventor, and others. Proprietary. Uses a hybrid approach:
1. **Graph-based decomposition:** Decomposes the constraint graph into rigid clusters
   and solves them hierarchically. A "rigid cluster" is a set of entities whose
   relative positions are fully determined by constraints.
2. **Numerical solving within clusters:** Each cluster is solved using Newton-type
   iteration.
3. **Sequential cluster assembly:** Clusters are positioned relative to each other.

This decomposition is what gives D-Cubed its speed — it never solves the entire
system as one monolithic problem.

**FreeCAD planegcs solver:**

Open-source, well-studied. Implements four algorithms:
1. **DogLeg (default):** Trust-region method. Combines Gauss-Newton (fast near
   solution) with gradient descent (safe far from solution). The trust region
   radius adapts automatically.
2. **Levenberg-Marquardt:** Alternative trust-region. Similar to DogLeg but uses
   a damping parameter instead of explicit trust region.
3. **BFGS:** Quasi-Newton method. Approximates the Hessian matrix using gradient
   history. Good for smooth problems.
4. **SQP:** Sequential Quadratic Programming. Used automatically when temporary
   constraints exist (e.g., during dragging).

All four build the Jacobian matrix at each iteration. Each constraint type
implements `calcGrad()` for analytical partial derivatives.

**SolveSpace:**

Open-source, compact, well-documented.
- Uses modified Newton's method with rank detection.
- For underconstrained systems, solves in a least-squares sense with each
  equation designed as a useful penalty metric.
- The "least surprising" behavior for dragging: minimizes total displacement
  from the initial guess while satisfying constraints.
- Handles rank-deficient Jacobians by detecting which parameters are free
  and reporting them to the user.
- Available as a standalone library (libslvs).

**The Jacobian matrix in detail:**

For a system with N parameters and M constraint equations:

```
        p1   p2   p3   p4   ...  pN
    +                              +
c1  | dc1/dp1  dc1/dp2  ...       |
c2  | dc2/dp1  dc2/dp2  ...       |
... |                              |
cM  |                              |
    +                              +
```

Each row = one constraint equation.
Each column = one parameter (x or y of a point, radius, angle, etc.)
Most entries are zero (sparse) because each constraint only references a few params.

Example: "distance between points P1=(x1,y1) and P2=(x2,y2) equals d"

Constraint: sqrt((x2-x1)^2 + (y2-y1)^2) - d = 0

Jacobian row: [-(x2-x1)/dist, -(y2-y1)/dist, (x2-x1)/dist, (y2-y1)/dist]
for columns [x1, y1, x2, y2], zeros elsewhere.

**Dragging implementation:**

When the user drags a point, the solver needs to:
1. Fix the dragged point's (x,y) to the mouse position (add temporary FixPoint
   constraints).
2. Re-solve the system from the current positions.
3. The LM/DogLeg solver naturally moves other entities minimally to satisfy
   constraints while the dragged point moves to the target.

SolveSpace adds the drag target as a "soft" constraint with high weight, so the
solver tries hard to reach it but can compromise if constraints prevent it.

FreeCAD uses the SQP algorithm specifically for drag operations because it handles
equality constraints (the existing sketch constraints) better during optimization.

**For GraniteX:** Start with DogLeg or LM (argmin crate has both). The key insight
from FreeCAD: having multiple solver backends is valuable because different
algorithms handle different constraint configurations better. Implement DogLeg
first, add LM as fallback.

### Summary of Key Takeaways for GraniteX

| Topic | Current State | What We Should Do |
|-------|---------------|-------------------|
| B-REP | Mesh-based bodies | Design types for half-edge migration |
| Tessellation | Fixed segments | Move to deflection-based adaptive |
| Edge rendering | Depth bias overlay | Correct approach, add silhouette edges |
| Shading | Per-vertex normals | Correct approach, use analytic normals |
| Z-fighting | Basic depth bias | Implement reversed-Z depth buffer |
| Triangulation | Fan triangulation | Switch to earcut, later CDT |
| Cylinder normals | Radial normals | Correct approach, generalize to all surfaces |
| OCCT | Not integrated | Consider opencascade-rs for Phase 9+ |
| Half-edge | Not implemented | Plan for B-REP kernel Phase 9+ |
| Constraint solver | LM planned | Implement DogLeg, add LM fallback |

### Sources

- [Boundary Representation — Wikipedia](https://en.wikipedia.org/wiki/Boundary_representation)
- [What Is B-Rep — Spatial Corp](https://www.spatial.com/glossary/b-rep)
- [B-REP — Shapr3D](https://www.shapr3d.com/content-library/what-is-b-rep)
- [BRep vs Mesh — CAD Exchanger](https://cadexchanger.com/blog/crash-course-on-cad-data-part-3/)
- [NURBS Tessellation — ScienceDirect](https://www.sciencedirect.com/science/article/abs/pii/0010448595000437)
- [Adaptive Tessellation for Trimmed NURBS — Academia](https://www.academia.edu/802844/Adaptive_Tessellation_for_Trimmed_NURBS_Surface)
- [Robust NURBS Tessellation — ScienceDirect](https://www.sciencedirect.com/science/article/abs/pii/S0010448515000032)
- [SolidWorks Silhouette Edges — SolidWorks Help](https://help.solidworks.com/2024/English/WhatsNew/c_wn2024_fundamentals_accelerate_display_silhouette_edges.htm)
- [Edge Enhancement — Visualization Library](https://visualizationlibrary.org/docs/1.0/pag_guide_edge_rendering.html)
- [Half-Edge Data Structure — UC Berkeley CS184](https://cs184.eecs.berkeley.edu/sp19/article/15/the-half-edge-data-structure)
- [Half-Edge — Illinois CS418](https://cs418.cs.illinois.edu/website/text/halfedge.html)
- [Half-Edge — Jerry Yin](https://jerryyin.info/geometry-processing-algorithms/half-edge/)
- [DCEL — Wikipedia](https://en.wikipedia.org/wiki/Doubly_connected_edge_list)
- [CGAL Halfedge DS](https://doc.cgal.org/latest/HalfedgeDS/index.html)
- [OpenCascade Mesh User Guide](https://dev.opencascade.org/doc/occt-7.5.0/overview/html/occt_user_guides__mesh.html)
- [BRepMesh Algorithm — OCCT Forum](https://dev.opencascade.org/content/brepmeshincremental-mesh-algorithm)
- [BRepMesh Intro — Unlimited 3D](https://unlimited3d.wordpress.com/2024/03/17/brepmesh-intro/)
- [Phong Shading — Wikipedia](https://en.wikipedia.org/wiki/Phong_shading)
- [Smooth Phong Shading — LearnWebGL](http://learnwebgl.brown37.net/10_surface_properties/smooth_vertex_normals.html)
- [Flat vs Gouraud vs Phong — Baeldung](https://www.baeldung.com/cs/shading-flat-vs-gouraud-vs-phong)
- [Z-Fighting — Wikipedia](https://en.wikipedia.org/wiki/Z-fighting)
- [Polygon Offset — OpenGL FAQ](https://www.opengl.org/archives/resources/faq/technical/polygonoffset.htm)
- [Polygon Offset Basics — Khronos](https://www.khronos.org/opengl/wiki/Basics_Of_Polygon_Offset)
- [Reversed-Z Depth Precision — Nathan Reed](https://www.reedbeta.com/blog/depth-precision-visualized/)
- [Reversed-Z — AJ Weeks](https://ajweeks.com/blog/2019/04/06/ReverseZ/)
- [Depth Precision — NVIDIA](https://developer.nvidia.com/blog/visualizing-depth-precision/)
- [Fan Triangulation — Wikipedia](https://en.wikipedia.org/wiki/Fan_triangulation)
- [Polygon Triangulation — Wikipedia](https://en.wikipedia.org/wiki/Polygon_triangulation)
- [Constrained Delaunay Triangulation — Wikipedia](https://en.wikipedia.org/wiki/Constrained_Delaunay_triangulation)
- [Geometric Constraint Solving — Wikipedia](https://en.wikipedia.org/wiki/Geometric_constraint_solving)
- [Constraint Solving Introduction — Siemens PLM](https://blogs.sw.siemens.com/plm-components/geometric-constraint-solving-1-introduction/)
- [SolveSpace Technology](https://solvespace.com/tech.pl)
- [FreeCAD GCS Solver — DeepWiki](https://deepwiki.com/FreeCAD/FreeCAD/3.1.2-constraint-system-and-gcs-solver)
- [planegcs — GitHub](https://github.com/Salusoft89/planegcs)
- [Vertex Normals — Sketchfab](https://help.sketchfab.com/hc/en-us/articles/209143406-Vertex-Normals)
- [Crease Angle Smooth Normals — Khronos Forum](https://community.khronos.org/t/smooth-vertex-normals-with-crease-angle/59237)
- [AMCAX B-REP Representation](https://docs.amcax.net/v5_0_0/en_us/html/_brep1.html)
- [CAD Exchanger SDK B-Rep](https://docs.cadexchanger.com/sdk/sdk_data_model_geometry_topology.html)
- [Logarithmic Depth Buffer — Outerra](https://outerra.blogspot.com/2009/08/logarithmic-z-buffer.html)
- [Maximizing Depth Buffer Precision — Outerra](https://outerra.blogspot.com/2012/11/maximizing-depth-buffer-range-and.html)
- [Z-Buffer Precision Analysis — Zero Radiance](https://zero-radiance.github.io/post/z-buffer/)

---

## Rust CAD Kernel Crate Comparison (2026-03-27)

Research into available Rust crates for BREP/CAD kernel functionality, evaluating suitability
for GraniteX (a SolidWorks-inspired 3D CAD application using wgpu).

### 1. truck (ricosjp/truck)

**Repo:** https://github.com/ricosjp/truck
**Stars:** ~1,400 | **License:** Apache 2.0 | **Last commit:** September 2024 (6 months stale)
**Pure Rust:** Yes (no C++ dependencies)

#### What it provides
- **NURBS geometry** (B-spline curves/surfaces, knot vectors)
- **Full BREP topology** (vertex, edge, wire, face, shell, solid)
- **Tessellation** via `truck-meshalgo` (can triangulate BREP for rendering)
- **Boolean operations** via `truck-shapeops` (union, intersection, difference) — added relatively recently
- **STEP I/O** via `truck-stepio` (read/write)
- **wgpu rendering** via `truck-platform` and `truck-rendimpl` (uses wgpu natively!)
- **WebAssembly** support (compiles to WASM)
- Modular "Ship of Theseus" architecture — 10+ sub-crates, each independently versioned

#### What's missing or weak
- **No fillet/chamfer** — only "prototyping" of fillet surfaces exists. This is the biggest gap.
- **Boolean operations are fragile** — reported issues with edge cases; not battle-tested like OCCT
- **Development has slowed** — last commit was Sept 2024, no v1.0, maintainer is a Japanese research company (RICOS)
- **Limited documentation** — a tutorial exists but is sparse
- **No shell operations** (offset, thicken, hollow)
- NURBS surface intersection (the hard math problem) has known limitations

#### Real-world usage
- **CADmium** (1.6k stars) used truck as its kernel for a browser-based CAD app. The project was
  archived in Sept 2025, partly because truck lacked fillets and had fragile booleans.
- CADmium demonstrated that truck *can* do basic extrude + STEP export, but couldn't complete
  models requiring fillets or complex boolean chains.

#### Verdict
Truck is the most promising pure-Rust option, but it's not production-ready for SolidWorks-class
features. The missing fillet/chamfer is a dealbreaker for mechanical CAD. Boolean fragility is
a serious concern. Development stalling is worrying.

---

### 2. opencascade-rs (bschwind/opencascade-rs)

**Repo:** https://github.com/bschwind/opencascade-rs
**Stars:** ~228 | **License:** LGPL-2.1 | **Last commit:** January 2025
**Pure Rust:** No — wraps the C++ OpenCASCADE (OCCT) kernel via cxx.rs

#### What it provides
- **Full OpenCASCADE power** — the same kernel used by FreeCAD, KiCad 3D viewer
- **Extrude, revolve, loft, sweep, pipe** — all working
- **Boolean operations** (union, subtract, intersect) — battle-tested in OCCT for 30+ years
- **Fillet and chamfer** — fully working, exposed in the Rust API
- **STEP, STL, SVG, DXF import/export** — all working
- **Tessellation** — OCCT's built-in mesher, outputs triangles
- Two API layers: low-level (direct OCCT bindings) and high-level (ergonomic Rust wrappers)
- Uses glam for vector math (same as GraniteX)

#### What's problematic
- **C++ dependency is heavy** — OpenCASCADE is ~7M lines of C++. Building from source takes
  significant time and disk space (~17GB intermediate files on Windows reported)
- **Windows build issues** — CMake policy warnings, MSVC quirks, vcpkg complications. It *works*
  but requires CMake + C++ compiler + patience
- **LGPL-2.1 license** — requires dynamic linking or open-sourcing your code (GraniteX is
  open-source so this is fine, but worth noting)
- **Maintainer is a solo dev** working part-time — "major work in progress" status
- **API is "still in flux"** — the high-level Rust API may change significantly
- **OCCT is old C++** — memory management is manual (reference counting), error handling is
  exceptions-based. The Rust wrapper papers over this but leaky abstractions are inevitable.

#### Windows build requirements
- Rust toolchain
- CMake (3.20+)
- Visual Studio 2022 / MSVC C++ compiler with C++11 support
- ~17GB disk space during compilation
- Can build OCCT from source (default) or link to pre-installed OCCT

#### Verdict
This gives you the full power of a production CAD kernel. Fillet, chamfer, booleans — everything
works because OCCT has been doing this for 30 years. The cost is build complexity, C++ dependency
management, and a solo maintainer. If GraniteX needs real CAD operations *now*, this is the
pragmatic choice.

---

### 3. Fornjot (hannobraun/fornjot)

**Repo:** https://github.com/hannobraun/fornjot
**Stars:** ~2,500 | **License:** permissive | **Last commit:** active (19k+ commits)
**Pure Rust:** Yes

#### What it provides
- BREP kernel foundation
- Code-first CAD modeling approach

#### What's missing
- **Self-described as "early-stage and experimental"**
- **"Lacks the features for anything more advanced, making it unsuited for real-world use cases"**
  (direct quote from README)
- Development of mainline code **paused over a year ago** in favor of experiments
- No boolean operations, no fillets, no STEP I/O in a usable state
- Unclear when/if experimental code will replace the main branch

#### Verdict
Not viable. Despite high star count (attracted attention early), it's an ongoing research project,
not a usable kernel. The author is honest about this.

---

### 4. monstertruck (virtualritz/monstertruck)

**Repo:** https://github.com/virtualritz/monstertruck
**Stars:** ~5 | **License:** Apache 2.0 | **Last commit:** active (2,763 commits)
**Pure Rust:** Yes (fork of truck)

#### What it provides
- Fork of truck with improved naming conventions (idiomatic Rust + standard CAD terminology)
- Claims: boolean operations, shape healing, tessellation, STEP I/O, NURBS
- Standardized API naming over truck's "confusing translations" from Japanese

#### What's problematic
- Only 5 stars — essentially no community
- No published releases on crates.io
- Created because PRs weren't accepted upstream — suggests upstream disagreement
- Same fundamental limitations as truck (NURBS intersection, fillets)

#### Verdict
Interesting fork but too obscure. If truck is risky, monstertruck is riskier.

---

### 5. Other options surveyed

| Crate | Status | Notes |
|-------|--------|-------|
| `cadk` | Abandoned | 4 commits total, 0 stars, only STEP parsing scaffolding |
| `chijin` | Minimal | OCC 7.9 bindings, v0.4.1, alternative to opencascade-rs |
| `breprs` | Alpha | v0.6.1-alpha, unknown maturity |
| `hedron` | Unknown | v0.2.0, "all-in-one 3D modelling" — too new to evaluate |

None of these are viable alternatives.

---

### Feature Comparison Matrix

| Feature | truck | opencascade-rs | Fornjot |
|---------|-------|----------------|---------|
| BREP topology | Yes | Yes (OCCT) | Partial |
| NURBS surfaces | Yes | Yes (OCCT) | No |
| Extrude | Yes | Yes | Basic |
| Boolean ops | Fragile | Rock-solid | No |
| Fillet | No | Yes | No |
| Chamfer | No | Yes | No |
| STEP I/O | Basic | Full | No |
| Tessellation to tris | Yes | Yes | Partial |
| wgpu integration | Native | Via meshing | No |
| Windows 11 | Yes | Yes (heavy build) | Yes |
| Pure Rust | Yes | No (C++ dep) | Yes |
| API stability | Moderate | In flux | N/A |
| Active development | Stalled | Slow | Paused |
| Production users | CADmium (archived) | None known | None |

---

### Recommendation

**Short answer: opencascade-rs, with eyes open about the costs.**

#### Reasoning

For a SolidWorks-inspired application, you *need* fillets, chamfers, and robust booleans. These
are not nice-to-haves — they're core to the parametric modeling workflow. Without them, users
can't complete real parts.

- **truck** would require implementing fillets/chamfers from scratch. This is measured in
  "developer-careers" of effort on established kernels. The CADmium project tried and gave up.
- **opencascade-rs** gives you 30 years of battle-tested CAD operations immediately. The price
  is build complexity and a C++ dependency.
- **Fornjot** and others are research projects, not tools.

#### Suggested approach

1. **Start with opencascade-rs** for the geometry kernel. Accept the C++ build cost.
2. **Use OCCT for**: boolean operations, fillets, chamfers, STEP I/O, surface intersections
3. **Use your own code for**: rendering (wgpu), UI (egui), sketch system, feature tree
4. **Tessellate OCCT shapes** to triangle meshes and feed them to your existing wgpu renderer
5. **Keep the OCCT dependency isolated** behind a trait boundary so it could theoretically be
   swapped for a pure-Rust kernel in the future (if truck or another project matures)

#### Alternative approach: truck + custom fillet

If the C++ dependency is truly unacceptable:
1. Use truck for topology + NURBS + tessellation + STEP
2. Implement basic constant-radius fillets yourself (rolling-ball algorithm)
3. Accept that booleans will have edge cases
4. Plan to contribute upstream or fork

This is higher risk but keeps the stack pure Rust and WASM-compatible.

### References
- [truck GitHub](https://github.com/ricosjp/truck)
- [opencascade-rs GitHub](https://github.com/bschwind/opencascade-rs)
- [Fornjot](https://www.fornjot.app/)
- [CADmium (archived)](https://github.com/CADmium-Co/CADmium)
- [CADmium blog post — Matt Ferraro](https://mattferraro.dev/posts/cadmium)
- [monstertruck](https://github.com/virtualritz/monstertruck)
- [Hacker News: truck discussion](https://news.ycombinator.com/item?id=40616155)
- [Hacker News: truck discussion (2023)](https://news.ycombinator.com/item?id=35071317)
- [opencascade-rs docs.rs](https://docs.rs/opencascade/latest/opencascade/)
- [chijin crate](https://crates.io/crates/chijin)
