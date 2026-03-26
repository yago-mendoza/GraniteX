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
- **stl_io** / **nom-stl** — STL import/export.
- **obj-rs** — OBJ import.
- **truck** (optional) — Rust BREP kernel with STEP support (experimental).

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
│   ├── app.rs               # Application state & main loop
│   ├── renderer/
│   │   ├── mod.rs            # Renderer orchestration
│   │   ├── pipeline.rs       # Render pipelines (mesh, wireframe, grid)
│   │   ├── camera.rs         # Camera (orbit, pan, zoom)
│   │   ├── gpu_state.rs      # wgpu device/surface/queue
│   │   └── vertex.rs         # Vertex formats
│   ├── scene/
│   │   ├── mod.rs            # Scene graph
│   │   ├── mesh.rs           # Mesh data structure (half-edge or indexed)
│   │   └── transform.rs      # Spatial transforms
│   ├── ui/
│   │   ├── mod.rs            # UI orchestration
│   │   ├── viewport.rs       # 3D viewport panel
│   │   ├── toolbar.rs        # Tool selection
│   │   └── inspector.rs      # Object properties
│   ├── io/
│   │   ├── mod.rs
│   │   ├── stl.rs            # STL import/export
│   │   └── obj.rs            # OBJ import
│   └── tools/
│       ├── mod.rs            # Tool trait & registry
│       ├── select.rs         # Selection tool
│       └── transform.rs      # Move/rotate/scale gizmo
├── docs/
├── assets/                   # Test meshes, shaders
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
