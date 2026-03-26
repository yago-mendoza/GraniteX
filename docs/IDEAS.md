# GraniteX — Ideas & Exploration Notes

Last updated: 2026-03-26

## Feature Ideas

### Near-term (worth thinking about now)
- **Dark mode by default** — every CAD tool is dark. Light mode as option.
- **"Focus mode"** — double-click an object to isolate it, hiding everything else. Like SolidWorks' "isolate" but smoother.
- **Smart snapping** — snap to grid, to edges, to midpoints, to intersections. This is what makes a CAD tool *feel* professional.
- **Mini command line** — like Blender's operator search or VS Code's command palette. Type operations instead of hunting through menus.
- **Viewport presets** — Front/Back/Left/Right/Top/Bottom buttons. One-click to snap to orthographic view.

### Mid-term (Phase 4-6)
- **Node-based material editor** — for when we add PBR rendering. Blackjack's node system is a good reference.
- **Macro recording** — record a sequence of operations, replay them. Useful for repetitive tasks.
- **Ruler/measurement always visible** — show dimensions on hover without entering a special mode.
- **Ghost mode for previous state** — when editing, show a transparent ghost of the mesh before the operation.

### Long-term / Moonshot
- **AI-assisted modeling** — "create a bracket with two holes" → generates a parametric model. Use an LLM to generate sketch constraints + features.
- **Collaborative editing** — CRDTs for the feature tree? Wild idea, but imagine Google Docs for CAD.
- **Generative design** — given constraints (load points, material, weight limit), optimize the shape. Topology optimization.
- **Integrated FEA** — basic finite element analysis. Show stress distribution on the model.
- **Version control for CAD** — Git-like branching for design iterations. Diff between model versions.
- **WASM web viewer** — export a model as a self-contained HTML file that anyone can view in a browser. Zero install sharing.

## Technical Exploration

### Things to research
- How does Blender's BMesh work internally? Their mesh editing is best-in-class.
- SolveSpace's constraint solver architecture — it's open source and well-documented.
- How does Fusion 360 handle the sketch→feature→solid pipeline?
- PGA (Projective Geometric Algebra) for geometric operations — might be cleaner than traditional linear algebra for CAD.
- Exact arithmetic vs floating point for geometry — the "robust predicates" problem.

### Crates to evaluate later
- `truck` — when we need BREP, evaluate if it's mature enough
- `opencascade-rs` — fallback if truck isn't sufficient
- `rapier3d` — physics simulation (for assembly mates?)
- `rhai` — embedded scripting language for macro/plugin system
- `serde` — serialization for project files (already planning to use)
- `notify` — file system watcher for auto-reload of imported meshes

## UX References

### Apps to study
- **SolidWorks** — the gold standard for parametric CAD UX
- **Fusion 360** — modern cloud CAD, good sketch workflow
- **Blender** — best-in-class mesh editing, keyboard-driven workflow
- **OnShape** — browser-based CAD, interesting constraint system
- **FreeCAD** — open source reference, learn from their mistakes too
- **Shapr3D** — iPad CAD, interesting for touch/pen input ideas
- **Plasticity** — Indie CAD with gorgeous UI, built on Parasolid

### UX principles for GraniteX
1. Mouse-centric by default (SolidWorks style), keyboard shortcuts for power users
2. Context menus everywhere — right-click should always show relevant operations
3. Preview before commit — show what an operation will do before confirming
4. Undo is sacred — every operation must be undoable, no exceptions
5. Progressive disclosure — simple tools visible, advanced tools in menus/submenus
