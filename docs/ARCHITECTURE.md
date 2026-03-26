# GraniteX Architecture

## Overview

GraniteX is a 3D CAD application built in Rust, targeting mesh editing and eventually parametric modeling.

## Core Stack

### Rendering
- **wgpu** — Cross-platform GPU abstraction (Vulkan/DX12/Metal). The backbone of all rendering.
- **winit** — Window creation and input handling.

### UI
- **egui** — Immediate-mode GUI. Fast to iterate, great for toolbars/panels/inspectors.
- **egui-wgpu** — Integration layer between egui and wgpu.

### Math
- **glam** — Fast, SIMD-optimized math (vectors, matrices, quaternions). Used in hot paths.
- **nalgebra** — More feature-rich linear algebra. Used for constraint solving / geometric algorithms.

### Geometry
- **Custom BREP kernel** (long-term) — Boundary representation for solid modeling.
- **meshopt** — Mesh optimization (simplification, vertex cache).
- **earcut** — Triangulation of polygons.

### File I/O
- **Hand-rolled STL parser** — Binary + ASCII, zero external deps.
- **tobj** — OBJ import (well-maintained, handles materials).
- **rfd** — Native file dialogs (cross-platform).
- **truck** (optional, future) — Rust BREP kernel with STEP support.

### Testing & Quality
- **criterion** — Benchmarking (critical for geometry/rendering perf).
- **proptest** — Property-based testing for geometry invariants.
- **insta** — Snapshot testing for serialized geometry / UI state.
- **tracing** + **tracing-subscriber** — Structured logging & profiling.

### Dev Experience
- **cargo-watch** — Auto-rebuild on save.
- **cargo-nextest** — Faster test runner.
- **cargo-flamegraph** — CPU profiling.
- **RenderDoc** — GPU debugging (external tool, works with wgpu).

## Module Structure

```
granitex/
├── src/
│   ├── main.rs              # Entry point
│   ├── app/
│   │   ├── mod.rs            # App struct, UI sync, event loop, file import
│   │   ├── input.rs          # Mouse/keyboard input handling
│   │   └── sketch_ops.rs     # Sketch drawing & contour→mesh conversion
│   ├── commands.rs           # Undo/redo (mesh snapshot-based)
│   ├── import/
│   │   ├── mod.rs            # Load dispatcher (by file extension)
│   │   ├── stl.rs            # STL parser (binary + ASCII, hand-rolled)
│   │   └── obj.rs            # OBJ loader (via tobj)
│   ├── renderer/
│   │   ├── mod.rs            # Renderer orchestration, mesh loading
│   │   ├── pipeline.rs       # Mesh render + wireframe pipelines
│   │   ├── camera.rs         # Orbit/pan/zoom camera + fit_to_bounds
│   │   ├── gpu_state.rs      # wgpu device/surface/queue + feature detection
│   │   ├── mesh.rs           # Mesh data (vertices, u32 indices, face ops)
│   │   ├── grid.rs           # Infinite XZ ground grid
│   │   ├── gizmo.rs          # RGB XYZ axis indicator
│   │   ├── preview.rs        # Extrude/cut ghost preview
│   │   ├── sketch_renderer.rs # Sketch line rendering
│   │   ├── picking.rs        # CPU raycasting for face selection
│   │   ├── vertex.rs         # Vertex format (pos + normal + face_id)
│   │   └── *.wgsl            # GPU shaders
│   ├── sketch/
│   │   ├── mod.rs            # Sketch state, 2D↔3D projection
│   │   ├── entities.rs       # Line, rect, circle primitives
│   │   └── plane.rs          # Sketch plane (normal, origin, axes)
│   └── ui.rs                 # egui panels (toolbar, feature tree, chat, status)
├── docs/                     # Claude's persistent brain
└── Cargo.toml
```

## Milestones

1. **Window + 3D Viewport** — wgpu context, camera, grid, render a cube
2. **Mesh Import** — Load STL/OBJ, display in viewport
3. **Camera Controls** — Orbit, pan, zoom (like SolidWorks)
4. **Selection** — Click to select faces/edges/vertices
5. **Basic Operations** — Extrude, move, delete faces
6. **UI Panels** — Toolbar, inspector, scene tree
7. **Undo/Redo** — Command pattern
8. **File Save/Load** — Custom project format
9. **Constraint Solver** — 2D sketch constraints
10. **Parametric Features** — Feature tree, history-based modeling
