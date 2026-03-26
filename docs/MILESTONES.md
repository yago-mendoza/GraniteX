# GraniteX — Milestones

## M0: Project Bootstrap
**Target:** 2026-03-27
**Status:** IN PROGRESS
**Deliverable:** Cargo project compiles, module skeleton exists, pushed to GitHub.
**Success criteria:** `cargo run` opens an empty window.

## M1: 3D Viewport
**Target:** 2026-04-03
**Status:** NOT STARTED
**Deliverable:** Window with a rendered cube, orbit/pan/zoom camera, grid.
**Success criteria:** Can rotate around a cube with mouse. Grid visible. Axis gizmo in corner.

## M2: Mesh Import
**Target:** 2026-04-15
**Status:** NOT STARTED
**Deliverable:** Load STL/OBJ files and display them.
**Success criteria:** Drag an STL file onto the window, it appears with proper shading. Camera auto-fits.

## M3: Selection & Interaction
**Target:** 2026-05-01
**Status:** NOT STARTED
**Deliverable:** Click to select faces/edges/vertices. Visual feedback.
**Success criteria:** Click a face on an imported mesh, it highlights. Mode toggle between face/edge/vertex.

## M4: UI Shell
**Target:** 2026-05-15
**Status:** NOT STARTED
**Deliverable:** egui panels: toolbar, inspector, scene tree.
**Success criteria:** Full application layout with functional panels. Keyboard shortcuts work.

## M5: Mesh Editing
**Target:** 2026-06-01
**Status:** NOT STARTED
**Deliverable:** Extrude, move, delete. Transform gizmo.
**Success criteria:** Can extrude a face on a cube, move it, undo it.

## M6: Persistence
**Target:** 2026-06-15
**Status:** NOT STARTED
**Deliverable:** Undo/redo, save/load project files.
**Success criteria:** Full undo history. Save project, close app, reopen, everything restored.

## M7: Sketch Mode
**Target:** 2026-07-15
**Status:** NOT STARTED
**Deliverable:** 2D sketch on planes with constraints.
**Success criteria:** Draw a rectangle on a face, add dimensions, constraints solve correctly.

## M8: Parametric Modeling
**Target:** 2026-09-01
**Status:** NOT STARTED
**Deliverable:** Feature tree, extrude/cut/revolve from sketches.
**Success criteria:** Create a part with multiple features. Edit an early feature, model regenerates.

## M9: BREP Kernel
**Target:** 2026-11-01
**Status:** NOT STARTED
**Deliverable:** Solid body representation with boolean operations.
**Success criteria:** Create two solids, subtract one from the other, fillet an edge.
