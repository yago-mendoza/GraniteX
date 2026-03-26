# GraniteX — Architecture Decision Records

## ADR-001: Use wgpu over raw Vulkan/DX12
**Date:** 2026-03-26
**Status:** Accepted

**Context:** Need a GPU API for rendering. Options: raw Vulkan (via ash), wgpu, or a higher-level engine like Bevy.

**Decision:** Use wgpu directly.

**Reasoning:**
- Raw Vulkan is too verbose for a CAD app (not a AAA game engine)
- Bevy adds game engine overhead and opinions we don't need
- wgpu gives us cross-platform (Vulkan/DX12/Metal/WebGPU) with a clean Rust API
- wgpu is the most actively maintained Rust GPU crate
- Future option: could compile to WASM/WebGPU for a web viewer

**Consequences:**
- We write our own render pipeline (more work, more control)
- We need to manage our own scene graph and draw calls
- Performance ceiling is very high (wgpu is thin over native APIs)

---

## ADR-006: API-First Architecture — Every Operation Must Be Programmatic
**Date:** 2026-03-26
**Status:** Accepted

**Context:** The ultimate vision for GraniteX is a conversational AI agent that drives the CAD engine through natural language. The agent needs to call the same operations as the UI.

**Decision:** Every CAD operation (extrude, cut, fillet, select, highlight, camera move) must be implemented as a function with typed parameters, separate from the UI layer. The UI and the AI agent are both *clients* of the same operation API.

**Reasoning:**
- If operations are only triggered by mouse clicks, the agent can't use them
- A clean operation API also enables: undo/redo (command pattern), macros, scripting, testing
- This is how professional CAD kernels work (OCCT, Parasolid) — the GUI is a thin layer

**Consequences:**
- More upfront work per operation (API + UI binding, not just UI)
- Forces clean separation of concerns from day one
- Every operation is automatically testable, scriptable, and agent-callable

---

## ADR-002: Use egui for UI, not iced or custom
**Date:** 2026-03-26
**Status:** Accepted

**Context:** Need a UI framework for toolbars, panels, inspectors.

**Decision:** Use egui (immediate mode).

**Reasoning:**
- Immediate mode = no UI state sync bugs (critical for a complex tool)
- egui-wgpu integration is mature and well-documented
- Fast iteration — change UI code, see results instantly
- Huge widget library out of the box
- Used by many Rust 3D tools (proven pattern)

**Trade-offs:**
- Immediate mode can be less efficient for static UIs (not a real issue for us)
- egui's look is "dev tool"-ish by default (can be themed later)
- Layout system is less powerful than retained mode (acceptable)

---

## ADR-003: glam over nalgebra for primary math
**Date:** 2026-03-26
**Status:** Accepted

**Context:** Need a linear algebra library for vectors, matrices, quaternions.

**Decision:** Use glam as primary, nalgebra as secondary for constraint solving.

**Reasoning:**
- glam is SIMD-optimized and designed for real-time graphics
- Simpler API than nalgebra for common 3D operations
- Bevy, wgpu examples, and most Rust 3D ecosystem uses glam
- nalgebra is better for generic linear algebra (constraint solver will need it)

---

## ADR-004: Start with indexed mesh, not half-edge
**Date:** 2026-03-26
**Status:** Accepted

**Context:** Need a mesh data structure. Options: indexed (positions + indices), half-edge, winged-edge.

**Decision:** Start with simple indexed mesh. Migrate to half-edge when we need adjacency queries (Phase 3-5).

**Reasoning:**
- Indexed mesh is trivial to implement and render
- Half-edge is complex and we don't need adjacency queries until selection/editing
- Premature optimization of data structures will slow us down
- We can wrap indexed mesh in a trait and swap implementation later

**Migration plan:**
- Phase 1-2: Indexed mesh (Vec<Vertex>, Vec<u32>)
- Phase 3+: Introduce half-edge alongside, migrate operations gradually

---

## ADR-007: GX Script Language Over Embedded Scripting (Rhai/Lua)
**Date:** 2026-03-26
**Status:** Accepted

**Context:** Need a way for users and the AI agent to define geometry textually. Options: embed an existing language (Rhai, Lua, Python), use an existing CAD language (OpenSCAD), or design a custom DSL.

**Decision:** Design a custom DSL called GX (GraniteX Script). Full spec in docs/DSL_DESIGN.md.

**Reasoning:**
- Existing scripting languages (Rhai, Lua) are general-purpose -- too much syntax surface for LLMs to hallucinate on, no built-in CAD concepts
- OpenSCAD uses CSG-only approach (no sketch-on-face, no feature tree, deeply nested)
- CadQuery depends on Python runtime (slow, heavy, runtime type errors)
- A custom DSL gives us: smallest possible grammar (~30 keywords), purpose-built for parametric CAD, line-oriented for LLM streaming, error messages that reference CAD concepts
- The AST maps directly to our Operation API, so the parser is also the compiler
- File extension `.gx` is short and unique

**Trade-offs:**
- We must build and maintain a parser (mitigated by logos+chumsky, well-understood problem)
- Users must learn a new language (mitigated by it being very small and intuitive)
- No existing ecosystem/libraries (acceptable -- it's a domain language, not a general language)

**Consequences:**
- Parser implementation needed before scripting features land
- Documentation and examples needed for users
- Rhai remains an option for non-geometry plugin scripting if needed later

---

## ADR-005: Defer BREP kernel to Phase 9
**Date:** 2026-03-26
**Status:** Accepted

**Context:** SolidWorks-like parametric modeling requires a BREP kernel. But building one is a multi-year effort.

**Decision:** Build mesh-based editing first (Phases 1-6), then sketch+extrude (Phase 7-8), then evaluate truck crate vs custom BREP (Phase 9).

**Reasoning:**
- A BREP kernel is the hardest component. Starting here means months with nothing visible.
- Mesh editing gives us a useful tool quickly and teaches us the rendering/interaction layers.
- By Phase 9, the truck crate may be more mature, or we'll know exactly what we need from a kernel.
- Many successful CAD tools started as mesh editors and added parametric later.
