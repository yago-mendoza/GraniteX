# GraniteX — Master TODO

Last updated: 2026-03-26 (Session 4)

## Legend
- [ ] Not started
- [~] In progress
- [x] Done
- **P0** = Blocker, **P1** = Must have, **P2** = Should have, **P3** = Nice to have

---

## Phase 0: Project Bootstrap (Target: 2026-03-27) — COMPLETE

- [x] **P0** Create GitHub repo (yago-mendoza/GraniteX)
- [x] **P0** Initialize project structure (README, docs/)
- [x] **P0** Set up CLAUDE.md with project instructions
- [x] **P0** Set up docs/ as persistent brain
- [x] **P0** Install Rust toolchain on Yago's machine
- [x] **P0** Create Cargo.toml with initial dependencies
- [x] **P0** Create module skeleton (src/main.rs, src/app.rs, src/renderer/, etc.)
- [x] **P1** Set up .gitignore
- [x] **P1** Initial commit and push to GitHub
- [ ] **P2** Set up cargo-watch for dev loop
- [ ] **P2** Set up cargo-nextest
- [ ] **P3** GitHub Actions CI (clippy + fmt + test)

## Phase 1: Window + 3D Viewport (Target: 2026-04-03) — COMPLETE

- [x] **P0** wgpu device/surface initialization
- [x] **P0** winit event loop with resize handling
- [x] **P0** Basic render pipeline (vertex + fragment shaders)
- [x] **P0** Render a colored cube (hello world of 3D)
- [x] **P1** Orbit camera (arcball rotation)
- [x] **P1** Camera pan (middle mouse + Ctrl)
- [x] **P1** Camera zoom (scroll wheel)
- [x] **P1** Infinite grid on XZ plane
- [x] **P2** Axis indicator (RGB XYZ gizmo in corner)
- [x] **P2** Wireframe overlay toggle (POLYGON_MODE_LINE, auto-detected)
- [x] **P3** Anti-aliasing (MSAA 4x)
- [ ] **P3** Depth-based grid fade

## Phase 2: Mesh Import & Display (Target: 2026-04-15) — IN PROGRESS

- [x] **P0** STL file loading (binary + ASCII)
- [x] **P0** OBJ file loading (via tobj)
- [x] **P0** Indexed mesh data structure (positions + normals + u32 indices)
- [x] **P0** Flat shading with normals
- [ ] **P1** Smooth shading (vertex normals)
- [x] **P1** Bounding box calculation + auto-fit camera
- [x] **P1** Drag-and-drop file import
- [x] **P2** Mesh info panel (vertex count, face count, dimensions — in status bar)
- [ ] **P2** Multiple meshes in scene
- [ ] **P3** PLY file support
- [ ] **P3** glTF import

## Phase 3: Selection & Interaction (Target: 2026-05-01) — PARTIAL

- [x] **P0** Ray casting from mouse into scene
- [x] **P0** Face selection (highlight selected face)
- [ ] **P0** Edge selection
- [ ] **P0** Vertex selection
- [ ] **P1** Multi-select (Shift+click)
- [ ] **P1** Selection mode toggle (face/edge/vertex)
- [ ] **P1** Selection highlighting (color overlay)
- [ ] **P2** Marquee/box selection
- [ ] **P2** Selection info in inspector

## Phase 4: UI Shell (Target: 2026-05-15)

- [ ] **P0** egui integration with wgpu viewport
- [ ] **P0** Top toolbar (tool selection)
- [ ] **P0** Right panel — inspector (object properties)
- [ ] **P0** Left panel — scene tree / feature tree
- [ ] **P1** Status bar (cursor position, selection info)
- [ ] **P1** Keyboard shortcuts system
- [ ] **P2** Dark/light theme
- [ ] **P2** Dockable panels
- [ ] **P3** Command palette (Ctrl+Shift+P)

## Phase 5: Basic Mesh Operations (Target: 2026-06-01)

- [ ] **P0** Extrude face
- [ ] **P0** Move/translate selection
- [ ] **P0** Delete face/edge/vertex
- [ ] **P1** Transform gizmo (move/rotate/scale)
- [ ] **P1** Inset face
- [ ] **P1** Loop cut
- [ ] **P2** Merge vertices
- [ ] **P2** Subdivide face
- [ ] **P3** Knife tool

## Phase 6: Undo/Redo & Project Files (Target: 2026-06-15)

- [ ] **P0** Command pattern for all operations
- [ ] **P0** Undo stack (Ctrl+Z)
- [ ] **P0** Redo stack (Ctrl+Y)
- [ ] **P0** Save project (.gnx custom format)
- [ ] **P0** Load project
- [ ] **P1** Auto-save
- [ ] **P1** Export to STL/OBJ
- [ ] **P2** Recent files list

## Phase 7: 2D Sketch Mode (Target: 2026-07-15)

- [ ] **P0** Enter sketch mode on a plane/face
- [ ] **P0** Draw lines
- [ ] **P0** Draw arcs/circles
- [ ] **P0** Draw rectangles
- [ ] **P1** Dimension constraints
- [ ] **P1** Coincident/parallel/perpendicular constraints
- [ ] **P1** Constraint solver (geometric)
- [ ] **P2** Trim/extend
- [ ] **P2** Fillet/chamfer on sketch
- [ ] **P3** Spline curves

## Phase 8: Parametric Features (Target: 2026-09-01)

- [ ] **P0** Feature tree (ordered list of operations)
- [ ] **P0** Extrude sketch → solid
- [ ] **P0** Cut extrude (boolean subtract)
- [ ] **P1** Revolve
- [ ] **P1** Fillet edges
- [ ] **P1** Chamfer edges
- [ ] **P1** Edit earlier feature → regenerate
- [ ] **P2** Pattern (linear, circular)
- [ ] **P2** Mirror
- [ ] **P3** Loft
- [ ] **P3** Sweep

## Phase 9: BREP Kernel (Target: 2026-11-01)

- [ ] **P0** Evaluate whether truck crate is sufficient or custom kernel needed
- [ ] **P0** Solid body representation (shells, faces, edges, vertices)
- [ ] **P0** Boolean operations (union, subtract, intersect)
- [ ] **P1** Filleting (constant radius)
- [ ] **P1** STEP file export
- [ ] **P2** STEP file import
- [ ] **P3** NURBS surface support

## Phase 10: Polish & Advanced (Target: 2027+)

- [ ] Assembly mode (multiple parts, mates/constraints)
- [ ] Drawing/2D documentation from 3D model
- [ ] Rendering mode (PBR materials, environment lighting)
- [ ] Plugin system
- [ ] GX Script Language (custom DSL -- see docs/DSL_DESIGN.md)
- [ ] Performance: LOD, frustum culling, instancing
- [ ] Collaboration features

## Phase 11: AI Agent — THE VISION (Target: 2027+)

- [ ] **P0** Operation API layer — every CAD operation callable programmatically with typed params
- [ ] **P0** Agent ↔ Engine protocol (JSON messages: highlight, preview, execute, query)
- [ ] **P0** LLM integration (Claude API) — natural language → operation mapping
- [ ] **P0** Intelligent highlighting — agent can highlight faces/edges/vertices to confirm understanding
- [ ] **P0** Operation previews — agent shows ghost geometry before committing
- [ ] **P1** Camera control by agent — "let me show you this part" → viewport moves
- [ ] **P1** Spatial reference resolution — "this face", "the top edge", "here" → geometry query
- [ ] **P1** Conversational memory — "do the same on the other side"
- [ ] **P1** Voice input (Whisper STT) — hands-free interaction
- [ ] **P2** Voice output (TTS) — agent speaks responses
- [ ] **P2** Streaming responses — LLM response streams while operating
- [ ] **P2** Multi-language support (Spanish + English minimum)
- [ ] **P3** Local LLM fallback (Llama) for offline use
- [ ] **P3** Agent learns user preferences over sessions

---

## Backlog (Unprioritized)

- Measurement tool (distance, angle, area)
- Section view / clipping plane
- Mesh repair (close holes, fix normals)
- Point cloud import
- Annotations / markup
- VR/AR viewport
- WASM build for web viewer
- Theming / customizable UI
