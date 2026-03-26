# GraniteX — Changelog

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

## 2026-03-26 — Session 6: SolidWorks Polish

### SolidWorks-Style Improvements
- **Hover pre-highlight**: Faces now subtly highlight on mouse-over BEFORE clicking (warm light tint). Uses the existing raycast picker running per-frame when not dragging. New `hovered_face` field in shader uniform.
- **Inset preview**: Teal transparent ghost shows the inset result in real-time as you adjust the amount slider. Same architecture as extrude/cut previews.
- **Smooth camera transitions**: View presets (F/T/R/Iso) now animate with 0.25s ease-out cubic. Camera has `target_yaw/pitch` animation state, updated per frame.
- **Edge rendering**: Dark line overlay on face boundaries (SolidWorks-style). Separate pipeline with depth bias to render on top of filled faces.
- **Zoom-to-fit**: Home key frames the entire model in the viewport.
- **Preview colors**: Extrude = blue, Cut = red, Inset = teal. Color passed via uniform to the preview shader.

### Technical
- Preview shader now receives color via uniform (was hardcoded blue). Shader reads `preview.color` instead of fixed vec4.
- MeshPipeline SceneUniform expanded: `hovered_face: i32` field added alongside `selected_face`.
- Renderer tracks `hovered_face: Option<u32>`, only updates uniform when hover changes (avoids unnecessary GPU writes).
- All 3 operation tools (Extrude, Cut, Inset) now have consistent preview → apply → undo workflow.
