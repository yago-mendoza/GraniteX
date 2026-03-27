# GraniteX — Learnings & Discoveries

Last updated: 2026-03-27

## L-001: Triangle soup is a dead end for CAD (2026-03-27)

Every professional CAD kernel (Parasolid, ACIS, OpenCASCADE) uses BREP where faces are parametric surfaces and topology is explicit. Operating directly on triangles means:
- Loss of geometric information at every operation (a circle becomes a polygon)
- No way to compute surface-surface intersections (needed for fillets, booleans)
- Adjacency queries are O(F×V) instead of O(1)
- No STEP export possible (STEP requires parametric geometry)

The triangle mesh should ONLY be a display artifact (tessellation for the GPU), never the source of truth.

## L-002: Gamma correction matters more than you'd think (2026-03-27)

Doing lighting math in sRGB space (the default if you don't think about it) makes:
- Ambient too dark or too light (nonlinear scaling)
- Specular highlights too harsh (pow() in sRGB = double gamma curve)
- Color mixing looks wrong (adding sRGB values != physically correct)

Fix: convert to linear before math, convert back to sRGB at the end. Just `pow(color, 2.2)` and `pow(color, 1/2.2)`.

## L-003: Backface culling is wrong for CAD (2026-03-27)

`cull_mode: None` seems wasteful but is correct. Cut pockets create inward-facing walls that the user needs to see from inside. SolidWorks also renders both sides. The shader uses `abs(dot(normal, light))` for two-sided lighting.

## L-004: Edge depth bias is inherently fragile (2026-03-27)

wgpu's `DepthBiasState` (constant + slope_scale + clamp) is device-dependent — the constant unit size varies between Vulkan/DX12/Metal. No single set of values works perfectly everywhere. The "correct" solution is rendering edges in a separate pass with slightly modified projection (polygon offset in OpenGL terms), or using a stencil-based approach.

## L-005: CADmium validated our concerns about truck (2026-03-27)

CADmium was a Rust CAD project that tried to build on the `truck` crate. It was archived partly because truck lacks fillets and has fragile boolean operations. This confirms that for SolidWorks-level features, wrapping a mature C++ kernel (OpenCASCADE) is more pragmatic than waiting for a pure Rust kernel to mature.

## L-006: Winding order is easy to get wrong in cube definitions (2026-03-27)

When defining cube indices manually, it's easy to accidentally flip the winding on some faces. The cross product `(v1-v0) × (v2-v0)` should give the outward normal for every face. In our cube, the top and bottom faces had inverted winding — the cross product pointed inward. This was masked by `cull_mode: None` and `abs()` lighting, but would break any operation that relies on correct normals.

## L-007: JSON is fine for CAD project files at this stage (2026-03-27)

The .gnx save format uses JSON (via serde_json) rather than a binary format. For the current mesh complexity this is perfectly adequate — file sizes are small and human-readable for debugging. Binary formats (bincode, messagepack) can be swapped in later behind the same serde traits if file size becomes an issue with complex models. The key decision was making the format self-describing and versionable from day one.

## L-008: Edge picking needs screen-space distance, not raycast (2026-03-27)

Face picking uses ray-triangle intersection (world space), but edge picking requires a different approach: project edge endpoints to screen space, then compute the 2D distance from the mouse cursor to the line segment. This is because edges have zero area — a ray will almost never intersect one. The screen-space approach also naturally handles the "close enough" threshold in pixels, which feels consistent regardless of zoom level.

## L-009: Selection mode state machines get complex fast (2026-03-27)

Adding edge selection alongside face selection introduced a mode toggle (Tab key) with different behaviors per mode: face mode does face raycast + highlight, edge mode does edge proximity + length display. Multi-select (Shift+click) adds another axis. The interaction matrix grows as modes × modifiers × actions. Keeping this organized requires clear state enums and routing input through them early, before it becomes spaghetti.
