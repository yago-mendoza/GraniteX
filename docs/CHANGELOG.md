# GraniteX — Changelog

## 2026-03-27 — Deep Audit: 7 Critical Bugs Fixed

Traced every user workflow path end-to-end. Found and fixed 7 logic bugs, ranging from geometry corruption to silent input rejection.

- **FIX**: Sketch undo (Ctrl+Z) now restores chain state — previously created disconnected lines
- **FIX**: H/V inference restricted to Line tool only — was breaking Rect (zero-height rejection) and Circle (radius warp)
- **FIX**: Circles now expose quadrant snap points + circumference snap — was impossible to connect lines to circles
- **FIX**: Cut drag now uses `max(0)` instead of `abs()` — dragging below start no longer increases depth
- **FIX**: Coplanar face merge invalidates stale stored_boundaries — prevents corrupt extrudes on merged faces
- **FIX**: Pre-selected construction plane takes priority over mesh face picking — user intent respected
- **FIX**: Regions outside parent face boundary are now rejected — prevents floating geometry creation

## 2026-03-27 — Construction Geometry Foundation

- **NEW**: Construction geometry system (`src/construction.rs`) — data model for reference planes and axes
- **NEW**: Construction renderer (`src/renderer/construction_renderer.rs`) — semi-transparent plane quads + axis lines in viewport
- **NEW**: Default origin planes (XY/XZ/YZ) and axes (X/Y/Z) visible and selectable
- **NEW**: Interactive feature tree — planes/axes clickable with colored indicators, visibility toggles
- **NEW**: Construction geometry picking — ray-plane and ray-line intersection for clicking planes/axes
- **NEW**: Sketch on reference plane — can now start sketches on origin planes (not just mesh faces)
- **NEW**: Construction lines in sketches (CLine tool) — orange reference lines that don't form regions
- **CHANGED**: `Sketch.face_id` is now `Option<u32>` — `None` for reference plane sketches
- **CHANGED**: Extrude/cut from sketch guards `split_parent_face` on `face_id.is_some()`
- This is the **gateway prerequisite** for Phase 8 (revolve, sweep, mirror, patterns)

## 2026-03-26 — Project Inception

- Created GitHub repository (yago-mendoza/GraniteX)
- Set up project structure: README.md, CLAUDE.md, docs/
- Created docs brain: TODO, ARCHITECTURE, TOOLING, DECISIONS, IDEAS, PROBLEMS, LEARNINGS, CHANGELOG, MILESTONES, RESEARCH
- Defined core stack: wgpu + winit + egui + glam
- Mapped out 10-phase development plan
- Waiting on Rust installation to begin coding

## 2026-03-26 — Session 1: Foundation

- Installed Rust 1.94.0
- Created Cargo project with wgpu + winit + egui + glam stack
- Implemented: wgpu init, colored cube, orbit/pan/zoom camera, MSAA 4x, infinite grid
- Documented ultimate vision: conversational AI agent driving CAD through natural language chat
- User feedback: grid lines too jagged, wants minimal UI like SolidWorks, wants DSL with live compiler
- Next: subtle grid, egui UI shell, then work toward operations layer

## 2026-03-26 — Session 2: DSL Design

- Designed GX (GraniteX Script) -- a domain-specific language for parametric CAD
- Full spec in docs/DSL_DESIGN.md covering: syntax, sketches, operations, references, variables, errors
- 5 examples from simple box to parametric enclosure with snap-fit lid
- Hybrid reference system: semantic names (body.top) + geometric queries (body.face(x_max))
- Constraint syntax for 2D sketches (dimensions, parallel, perpendicular, coincident, etc.)
- LLM-friendly design: line-oriented, minimal keywords (~30), flat structure, no nesting
- Parser strategy: logos (lexer) + chumsky (parser) for best error recovery
- Integration plan: GX scripts, incremental editing, and live REPL mode for the AI agent

## 2026-03-26 — Session 3: Sketch-on-Face Research

- Deep architectural research on SolidWorks-style "sketch on face" workflow
- Documented: sketch plane construction, LM constraint solver algorithm, closed contour detection, rendering approach, under/over-constrained handling, data structures
- Added to RESEARCH.md: 9 sections covering the full sketch system architecture
- Key decisions: LM via argmin or hand-rolled, slotmap for entity storage, hybrid 3D+egui rendering, separate points with coincident constraints (not merged points)

## 2026-03-26 — Session 4: Modular Foundation + Mesh Import

### Architecture
- Split app.rs (471 lines) into module directory: app/mod.rs (240), app/input.rs (132), app/sketch_ops.rs (159)
- Upgraded indices from u16 to u32 — supports meshes with >65k vertices (essential for STL/OBJ import)
- Created src/import/ module with error types via thiserror

### New Features
- **STL import**: Binary + ASCII parser (hand-rolled, zero deps). Reads header, triangle count, normals, positions.
- **OBJ import**: Via tobj crate. Merges multiple models into single mesh with flat shading.
- **File dialog**: rfd crate. Import button in toolbar opens native OS file picker with STL/OBJ filters.
- **Drag-and-drop**: Drop STL/OBJ files onto window to import. Handled via winit DroppedFile event.
- **Auto-fit camera**: Camera.fit_to_bounds() frames imported mesh. Calculates bounding box, sets distance to fill ~60% viewport.
- **Wireframe toggle**: POLYGON_MODE_LINE pipeline (auto-detected from GPU features). "Wire" checkbox in toolbar.
- **Cut operation**: Cut into a face along its negative normal (pocket). Cut preview (red ghost). UI controls in left panel.

### Infrastructure
- Added deps: tobj (OBJ parsing), rfd (file dialogs), thiserror (error types)
- GPU feature detection: POLYGON_MODE_LINE requested if available, wireframe UI hidden if not
- Mesh::from_triangles() — generic constructor for imported meshes (each triangle gets unique face_id)
- Mesh::bounding_box() — AABB computation for camera fitting

## 2026-03-26 — Session 5: Code Health + Mesh Operations

### Code Health
- **Removed `#![allow(dead_code)]`** — global dead code suppression was hiding 6 warnings
- Fixed all warnings: removed unused methods (undo_count, camera_eye, has_preview), wired all ViewPreset variants into toolbar (Back, Bottom, Left), targeted allows for future-ready API (hit_point, empty, set_view_instant)
- Fixed borrow checker issue in apply_ui_state (import request must be handled before borrowing renderer)
- Fixed stale `indices_u16()` call in picking.rs (leftover from u16→u32 migration)
- Fixed missing `color` field in PreviewUniform constructor
- Fixed missing `cached_eye` field in MeshPipeline constructor

### New Features
- **Delete face**: Select a face, press Delete key to remove it. Vertex compaction + index remapping.  Full undo support via Ctrl+Z.
- **Inset face**: New Inset tool (toolbar button + "i" shortcut). Shrinks face boundary toward center by configurable amount, creates inner face + connecting quad strips. Selects the inner face after operation. Full undo support.
- **All 7 view presets wired**: Front, Back, Top, Bottom, Right, Left, Isometric now all accessible from toolbar.

### Architecture
- Created app/mesh_ops.rs — mesh operations module (delete face)
- Keyboard shortcuts expanded: Delete key → delete face, "i" → Inset tool

## 2026-03-26 — Session 6: SolidWorks UX Polish + Audit

### Code Health
- Zero clippy warnings (was 4). Fixed: redundant `use egui`, needless_range_loop, targeted allow for future-ready boundary_edges()
- Split mesh.rs (576 lines) → mesh/mod.rs (275, core) + mesh/ops.rs (272, operations)
- Removed duplicate `create_edge_buffer` method in pipeline.rs

### SolidWorks UX Features
- **Rich status bar**: shows active tool name, mesh stats (F/V/T), selected face info (ID, normal vector, area), 3D cursor world coordinates (XZ ground plane projection). Matches SolidWorks information density.
- **Right-click context menu**: Right-click on any face → popup with Extrude, Cut, Inset, Delete, Zoom to Face. Auto-selects face under cursor. Only appears outside sketch mode.
- **Toast notification system**: Bottom-right toasts with fade-out animation. Shows operation feedback ("Extruded 0.50m", "Imported mesh.stl (5420 triangles)", "Face deleted"). Auto-dismiss after 3 seconds.
- **Face area computation**: Mesh::face_area() sums triangle areas per face_id.

### Performance
- Hover picking skipped when cursor hasn't moved (was running O(n) raycasting every frame)
- Real frame timing for camera animation (was hardcoded 1/60s, now uses std::time::Instant)
- Camera animation dt capped at 100ms to prevent jumps on frame drops

## 2026-03-26 — Session 6: SolidWorks Polish

### SolidWorks-Style Improvements
- **Hover pre-highlight**: Faces now subtly highlight on mouse-over BEFORE clicking (warm light tint). Uses the existing raycast picker running per-frame when not dragging. New `hovered_face` field in shader uniform.
- **Inset preview**: Teal transparent ghost shows the inset result in real-time as you adjust the amount slider. Same architecture as extrude/cut previews.
- **Smooth camera transitions**: View presets (F/T/R/Iso) now animate with 0.25s ease-out cubic. Camera has `target_yaw/pitch` animation state, updated per frame.
- **Edge rendering**: Dark line overlay on face boundaries (SolidWorks-style). Separate pipeline with depth bias to render on top of filled faces.

### Rendering Bug Fixes
- **Sketch contours no longer create double faces**: Drawing a contour on a face now DELETES the parent face and creates the contour FLUSH (no z-offset). Previously, the contour sat 0.002 units above the parent → z-fighting, shading artifacts, visible seam.
- **Circle segments increased**: 48 → 64 for finer tessellation on circular sketches.
- **Cylinder extrude uses smooth radial normals**: When extruding a face with >6 boundary corners (detected as cylindrical), side wall vertices get per-vertex radial normals instead of flat per-face normals. All wall segments share one face_id → no internal edges drawn → smooth cylinder appearance.
- **Edge rendering uses LineList topology**: Boundary edges rendered as actual line segments (position-only vertex buffer) instead of PolygonMode::Line on triangles. Only edges between different face_ids are drawn → cube internal diagonals never appear.
- **Zoom-to-fit**: Home key frames the entire model in the viewport.
- **Preview colors**: Extrude = blue, Cut = red, Inset = teal. Color passed via uniform to the preview shader.

### Technical
- Preview shader now receives color via uniform (was hardcoded blue). Shader reads `preview.color` instead of fixed vec4.
- MeshPipeline SceneUniform expanded: `hovered_face: i32` field added alongside `selected_face`.
- Renderer tracks `hovered_face: Option<u32>`, only updates uniform when hover changes (avoids unnecessary GPU writes).
- All 3 operation tools (Extrude, Cut, Inset) now have consistent preview → apply → undo workflow.

## 2026-03-26 — Session 7: SolidWorks Rendering Parity

### Edge Rendering Overhaul
- **Mesh boundary edges now drawn**: Open surfaces (after face deletion) show their boundary edges. Previously all single-face edges were suppressed.
- **Coplanarity filter**: Edges between coplanar faces (same normal, <1.8° angle) are suppressed — matches SolidWorks behavior. Inset connecting quads on a flat plane don't show internal edge lines.
- **Removed dead XOR-hash boundary_edges()**: Old method in mesh/ops.rs used collision-prone XOR hash. Replaced by tuple-based `PosKey` in pipeline.rs (collision-free).

### Sketch-to-Face Workflow Fix
- **Parent face preserved**: Drawing a contour no longer deletes the parent face. Contour sits on top with z-offset (0.003). This prevents creating holes in the mesh.
- **Auto-selection**: After closing a sketch contour, the new face is automatically selected (blue highlight). User can immediately extrude without clicking.
- **Z-offset increased**: 0.0003 → 0.003 for reliable depth separation and picking.

### Cylinder Smoothing
- **Lowered cylindrical threshold**: n>6 → n>4. Pentagons and hexagons now get smooth radial normals instead of flat per-face normals.

### UX Polish
- **Context menu wired**: Extrude/Cut/Inset from right-click context menu now switches to the corresponding tool (was silently ignored).
- **Fillet button disabled**: Shown grayed-out with "Coming soon" tooltip instead of doing nothing.
- **Ctrl+Shift+Z = Redo**: Alternative redo shortcut (SolidWorks convention).
- **Ctrl+O = Import**: Keyboard shortcut for file import dialog.

## 2026-03-27 — Session 9: Smooth Shading + Hole-Aware Extrusion

### Hole-Aware Sketch Extrusion (SolidWorks-Style)
- **Parent face boundary included in region computation**: Drawing a shape on a face now creates 2+ regions — the inner shape(s) AND the outer area (face minus shapes). Previously only inner shapes were selectable.
- **Outer region extrusion punches holes**: Extruding the outer region creates inner side walls for hole boundaries (reversed winding). The hole becomes a true opening in the extruded geometry.
- **`stored_holes` on Mesh**: Faces created with `add_polygon_face_with_holes_flush` now store their hole boundaries. `extrude_face` and `cut_face` read these to create inner walls.
- **`split_parent_face` handles holes**: When splitting the parent face after extrusion, the full region polygon (outer + holes) is subtracted, producing correct remainder geometry.
- **Undo system updated**: `MeshSnapshot` now includes `stored_holes` for correct undo/redo of hole-aware operations.
- **Sketch stores parent face boundary**: `Sketch::new()` receives the parent face boundary in 2D. The `RegionSolver` uses it as the outermost contour when computing regions.

### The Big One: Crease-Angle Smooth Shading
- **New `apply_smooth_shading(30°)` system** in `mesh/smooth.rs`. Three-phase pipeline:
  1. **Face merging**: Union-find over edge adjacency. Adjacent triangles with normals within 30° get the same `face_id`. A cylinder barrel → 1 face. A box → 6 faces. A sphere → 1 face.
  2. **Normal averaging**: For each vertex, averages geometric normals of all adjacent triangles within the crease angle. Cylinders get smooth Gouraud-interpolated normals. Sharp edges (>30°) stay sharp automatically.
  3. **Vertex welding**: Merges duplicate vertices by (position, face_id, normal). ~6x vertex reduction for imported meshes.
- **Called automatically on import**: `from_triangles()` applies smooth shading before returning. Every STL/OBJ import benefits.
- **Edge rendering improved for free**: Face merging means fewer face boundaries → edge lines only appear at real geometric creases, not at every triangle boundary.
- **Face selection improved for free**: Clicking a surface selects the entire logical face (e.g., whole cylinder barrel), not one triangle.
- **Primitive operations untouched**: Cube, extrude, cut, inset keep their manually-set normals and face_ids.

### Why This Matters
Before: imported cylinder = faceted octagon. Sphere = disco ball. Every curved surface screamed "wrong."
After: imported cylinder = smooth cylinder. Sphere = smooth sphere. Looks like a real CAD tool.

## 2026-03-27 — Session 11: AI Agent Vision Documentation

### Documentation
- Created **AGENT_VISION.md** — comprehensive architecture document for the conversational AI agent
  - Two-client model (UI + Agent both call Operation API)
  - Agent ↔ Engine message protocol (execute, camera, highlight, query, ask_user)
  - Agent reasoning model (procedural planning, ambiguity handling, error recovery)
  - GX script as agent output format
  - Implementation phases (A through F)
  - Comparison with existing AI+CAD approaches
- Created **AGENT_CRITIQUE.md** — adversarial analysis of the vision
  - 15 failure modes and mitigation strategies
  - Top 5 risks: TNP, LLM spatial reasoning, state sync, rendering complexity, token budget
  - Deep analysis: camera view planning, preview system limits, undo grouping, coordinate systems
  - Security analysis of GX parser as trust boundary
- Updated IDEAS.md with expanded AI agent section

### Key Architectural Insights
- LLM should specify INTENT, engine should compute COORDINATES (LLMs can't do spatial math)
- GX script needs high-level placement primitives (offset_from_edges, face_center) not raw coords
- Agent requires multi-pass renderer (opaque → transparent → ghost → labels → UI)
- Turn-based state ownership needed (agent working → UI locked except camera)
- Feature tree is essential for agent context (not just parametric editing)

### Deep Analysis: 41 Failure Modes Documented
- Part I (1-15): Architecture failures (TNP, state sync, previews, camera, rendering pipeline)
- Part II (16-24): Operation-level failures (granularity, hybrid interaction, GPU bugs, validation)
- Part III (25-41): Lived experience failures (latency, spatial ambiguity, drift, onboarding, units, anaphora, performance, cost)
- Revised threat ranking: #1 is Dead Air (API latency), not TNP — latency drives users away before architecture bugs surface
- Key new insights: propose/adjust/confirm pattern, outline-only dimming, "show wrong thing > ask right question", screenshot timing race, canonical state snapshot every turn, spatial reference resolver, performance budgets

## 2026-03-27 — Session 10: Deep CAD Audit + Kernel Decision

### Full Codebase Audit vs. SolidWorks
Systematic audit of every renderer and geometry file, comparing our approach to how professional CAD systems (SolidWorks/Parasolid, ACIS, OpenCASCADE) work internally.

### Rendering Fixes
- **Blinn-Phong specular highlights**: Surfaces now have metallic/plastic shine (was flat matte)
- **Linear lighting (gamma correction)**: All lighting math now in linear color space with sRGB↔linear conversion. Colors are physically correct.
- **Three-light setup**: Key + fill + rim lights for industrial CAD look
- **Cube winding fix**: Top/bottom face indices had inverted normals (cross product pointed inward)
- **Preview z-fighting fix**: Base faces offset ±0.001 along normal to prevent z-fighting with underlying mesh
- **Edge depth bias tuned**: Less aggressive slope_scale, tighter clamp to prevent edge pop-in at shallow angles

### SolidWorks UX Parity
- **Snap points**: Cursor snaps to face corners (orange square), edge midpoints (cyan triangle), nearest edge point (magenta diamond), existing endpoints (yellow circle). Visual indicators show BEFORE clicking.
- **Grid snapping**: Sketch points snap to 0.05m grid in the sketch plane.
- **Dimension labels**: Floating distance value shown during extrude/cut/inset preview (blue/red/teal).
- **Sketch dimensions**: Line length and angle shown during drawing (green label).
- **H/V inference**: Lines auto-constrain to horizontal/vertical when close to axis.
- **Construction lines**: `CLine` tool for reference geometry (not included in regions).
- **Measure tool**: Click two points to show distance.
- **Entity selection/deletion**: Click to select sketch entities, Delete key to remove.
- **Edge picking**: Screen-space distance to mesh boundary edges (in picking.rs).
- **Export**: STL/OBJ export stubs (coming soon).

### CAD Kernel Decision
- Evaluated all Rust BREP options: truck (stalled, no fillets), Fornjot (paused), opencascade-rs (viable)
- **Decision: opencascade-rs** — only option with fillets + robust booleans + STEP I/O
- CADmium (Rust CAD project) archived partly due to truck limitations — validates our evaluation
- Architecture: kernel behind trait boundary, current triangle mesh becomes display-only tessellation
- Updated Phase 9 roadmap with concrete integration plan

### Identified Fundamental Issues
- PROB-012: No topology (half-edge) — requires O(F×V) for adjacency
- PROB-013: No parametric surfaces — cylinders always faceted, no fillets possible
- PROB-014: No feature tree — operations are destructive, no parameter editing

## 2026-03-27 — Session 12: File I/O, Selection Modes, Measurement & UX

### File I/O
- **New Scene (Ctrl+N)**: Resets the entire scene to a fresh state.
- **Save/Load project (.gnx format)**: JSON-based custom format. Ctrl+S saves, Ctrl+O opens. Preserves full mesh state.
- **Export STL/OBJ**: Binary STL export and OBJ export with normals. Available from toolbar/menu.
- **serde/serde_json dependencies added** for serialization.

### Selection & Interaction
- **Multi-select (Shift+click)**: Toggles face selection — Shift+click adds/removes faces from the selection set.
- **Edge selection mode**: Tab key toggles between face and edge selection modes. Click to select edges, selected edge shows length in status bar.
- **Sketch entity selection & deletion**: Click to select sketch entities (lines, circles, etc.), Delete key removes them.

### Tools
- **Measurement tool**: M key activates. Two-click point-to-point distance measurement. Displays total distance plus dX/dY/dZ components.
- **Construction lines (CLine tool)**: Dashed rendering for reference geometry. Not included in region computation.

### Shortcuts
- **Numpad view shortcuts**: 1 (Front), 3 (Right), 7 (Top), 0 (Isometric), with Ctrl variants for opposite views (Back, Left, Bottom).

## 2026-03-26 — Session 8: CAD Kernel Research

### Research
- Deep technical research on how professional CAD systems work internally
- Documented in RESEARCH.md: BREP architecture, half-edge data structure, Parasolid internals, surface representation, adaptive tessellation, boolean operations, Euler operations, edge rendering algorithms, mesh vs BREP trade-offs, smooth shading
- Evaluated Rust BREP kernel options: truck-rs (recommended), opencascade-rs, Fornjot
- Defined progressive migration strategy: short-term mesh fixes → hybrid BREP+mesh → full BREP
- Key immediate recommendations: smooth normals with crease angles, explicit feature edge storage, polygon offset for edge lines, coplanar face merging
