# GraniteX — Known Problems & Technical Debt

Last updated: 2026-03-27 (reviewed Session 12 — no new problems)

## Architectural Limitation

### The Triangle Soup Problem
**Status:** FUNDAMENTAL — shapes all future development
**Description:** GraniteX operates on a triangle mesh (`Vec<Vertex>` + `Vec<u32>` indices) with face_id tags. SolidWorks/Fusion360 use a BREP kernel where faces are parametric surfaces and topology is explicit. Our approach requires fragile heuristics (vertex deduplication, angle-sorting, stored_boundaries HashMap) that break on complex geometry.
**Impact:** Every mesh operation (extrude, cut, inset) has edge cases. The `face_boundary_corners` function fails on concave polygons. Boolean face splitting is approximate. Undo is snapshot-based (full mesh clone).
**Long-term fix:** Migrate to `opencascade-rs` (Rust bindings for OpenCASCADE). Research (2026-03-27) evaluated all Rust BREP options: `truck` (stalled since Sept 2024, no fillets), Fornjot (paused, experimental), opencascade-rs (only viable option with fillets + robust booleans + STEP export). CADmium project was archived partly because truck lacked these capabilities.
**Architecture:** Kernel behind a trait boundary so it can be swapped in the future. Current triangle mesh becomes display-only tessellation. See RESEARCH.md for full evaluation.
**Short-term mitigation:** `stored_boundaries` HashMap tracks face ordering. `triangulate_3d_polygon` uses earcutr. `split_parent_face` uses geo boolean operations. These work for 90% of cases.

## Fixed Issues (2026-03-27 Robustness Hardening)

### PROB-022: contour_closed fires before entity validation
**Status:** FIXED (2026-03-27)
**Description:** Rect/Circle tools set `contour_closed=true` BEFORE `add_rect`/`add_circle` validated the entity. User saw "ready to extrude" toast but nothing was created. `pending_start` was consumed by `.take()`, stranding the user.
**Resolution:** `add_rect`/`add_circle` return bool. `contour_closed` only set on success. `pending_start` restored on failure with error toast.

### PROB-023: add_rect never called mark_dirty
**Status:** FIXED (2026-03-27)
**Description:** `add_rect` pushed 4 Line entities but never called `region_solver.mark_dirty()`. Region computation stale for rects unless caller happened to mark dirty.
**Resolution:** `add_rect` now calls `mark_dirty()` unconditionally.

### PROB-024: Failed extrude/cut pushes undo snapshot
**Status:** FIXED (2026-03-27)
**Description:** `save_state` was called before validating the target. Failed operations pushed a useless snapshot and nuked the redo stack. Also, sketch-based extrude created a base face that was never cleaned up on failure.
**Resolution:** `save_state` moved after target validation. Failed ops roll back via `history.undo()`. Failure toast added.

### PROB-025: operation_dragging leaks across tool switches
**Status:** FIXED (2026-03-27)
**Description:** Pressing tool shortcut keys during drag-to-extrude/cut didn't cancel the drag. Mouse release on wrong tool could fire stale operation.
**Resolution:** Tool switch clears drag state. Keyboard blocked during drag (except Esc). Right-click cancels drag.

### PROB-026: Sketch not invalidated on mesh undo
**Status:** FIXED (2026-03-27)
**Description:** Mesh undo could remove the face the sketch was bound to. Sketch held stale `face_id` and `face_boundary_2d`.
**Resolution:** After mesh undo/redo, check if sketch's `face_id` still exists. Destroy sketch only if face is gone (not blanket invalidation).

## Fixed Issues (2026-03-27 Deep Audit)

### PROB-015: Sketch undo desyncs chain state
**Status:** FIXED (2026-03-27)
**Description:** After Ctrl+Z removed a sketch entity, `pending_start` still pointed to the removed line's endpoint. Next click created a disconnected line floating in space.
**Resolution:** `undo_last()` now restores `pending_start` to the previous entity's endpoint, or resets to idle if no entities remain.

### PROB-016: H/V inference breaks Rect and Circle tools
**Status:** FIXED (2026-03-27)
**Description:** `resolve_cursor()` applied horizontal/vertical inference to ALL tools when `pending_start` existed. For Rect, this could snap corner2 to be collinear with corner1 → zero-height rect → silently rejected. For Circle, it warped the radius.
**Resolution:** Added `line_mode` parameter to `resolve_cursor()`. H/V inference only activates for the Line tool.

### PROB-017: Circles have no edge snap points
**Status:** FIXED (2026-03-27)
**Description:** `Circle.endpoints()` only returned center. Impossible to snap lines to circle edges — fundamental connectivity break.
**Resolution:** `endpoints()` now returns center + 4 quadrant points. Added circumference snap (nearest point on circle) as lower-priority snap target. New SnapType variants: Quadrant, Circumference.

### PROB-018: Cut drag V-shaped behavior
**Status:** FIXED (2026-03-27)
**Description:** Cut used `.abs()` on drag distance, so dragging below start point INCREASED depth instead of reducing to 0. User couldn't reduce cut depth by dragging back down.
**Resolution:** Changed to `.max(0.0)` — drag up = deeper, drag down = shallower, below start = 0.

### PROB-019: Coplanar-merged faces corrupt future extrudes
**Status:** FIXED (2026-03-27)
**Description:** After extrude, side walls merged into adjacent faces via `find_coplanar_adjacent_face`. The merged face kept a stale `stored_boundary` from before the merge. Future extrudes on these faces used wrong boundary (convex hull of L-shape instead of actual shape).
**Resolution:** `create_side_walls` now invalidates `stored_boundaries` for merged faces, forcing `face_boundary_corners()` to recompute via angle-sort (correct for convex merged faces).

### PROB-020: Construction plane sketch priority
**Status:** FIXED (2026-03-27)
**Description:** When a construction plane was selected in UI, clicking a drawing tool still prioritized mesh face picking. If the cube overlapped the plane, the user could never start a sketch on the plane.
**Resolution:** If a construction plane is pre-selected in UI, it now takes priority over mesh face picking. User intent (selecting the plane) is respected.

### PROB-021: Regions outside face boundary selectable
**Status:** FIXED (2026-03-27)
**Description:** If a sketch contour extended beyond the face boundary, `compute_regions` created an "uncovered" region floating outside the face. User could select it → extrude created geometry disconnected from the mesh.
**Resolution:** `select_region_at()` now checks that the region centroid is inside the parent face boundary. Regions outside the face are rejected.

## Active Issues (2026-03-27 Audit)

### PROB-008: Shader lacks specular + gamma correction
**Status:** FIXED (2026-03-27)
**Description:** Lighting math was in sRGB space (should be linear). No specular highlights made surfaces look flat/matte.
**Resolution:** Added Blinn-Phong specular, sRGB↔linear conversion, 3-light setup.

### PROB-009: Cube top/bottom winding inverted
**Status:** FIXED (2026-03-27)
**Description:** Top face (8,9,10) and bottom face (12,14,13) had cross products producing inward normals. Doesn't affect rendering (abs() two-sided lighting) but would break backface culling.
**Resolution:** Swapped index order for top and bottom faces.

### PROB-010: Preview z-fighting with underlying mesh
**Status:** FIXED (2026-03-27)
**Description:** Extrude/cut preview cap faces sat exactly on the mesh surface → z-fighting.
**Resolution:** Added small normal offset (±0.001) to preview base positions.

### PROB-011: Edge depth bias magic numbers
**Status:** IMPROVED (2026-03-27)
**Description:** Edge rendering depth bias `constant:-4, slope_scale:-2.0, clamp:-0.01` was device-dependent and too aggressive at shallow angles.
**Resolution:** Adjusted to `constant:-8, slope_scale:-1.5, clamp:-0.0001` for better cross-angle behavior.

### PROB-015: Face splitting fragility
**Status:** HARDENED (2026-03-27)
**Description:** `split_parent_face` used geo::difference which can: (a) panic on degenerate inputs, (b) produce tiny sliver polygons, (c) delete the parent face without creating remainder when region covers 100% of parent.
**Resolution:** Added 3 robustness guards:
- `catch_unwind` around geo::difference (recovers from panics)
- Area ratio check: skip split if region covers >95% of parent
- Degenerate sliver filter: skip faces with area < 1e-6
- Remainder area validation: skip if remainder < 1% of parent

### PROB-016: Cylindrical detection heuristic
**Status:** FIXED (2026-03-27)
**Description:** `is_cylindrical = n > 4` wrongly treated pentagons, hexagons, and L-shapes as cylinders → smooth normals instead of flat faces → no hard edges on polygon extrusions.
**Resolution:** Changed to `n > 16 AND all_corners_equidistant_from_center(5%)`. Only actual tessellated circles get smooth normals now.

### PROB-012: No real topology (half-edge)
**Status:** OPEN — blocked on kernel migration
**Description:** Face adjacency requires O(F×V) position comparison. Edge detection scans all triangles. No shared vertices between faces.

### PROB-013: No parametric surfaces
**Status:** OPEN — blocked on kernel migration
**Description:** Everything is flat triangles. Cylinders are faceted. No fillets/chamfers possible. No STEP export.

### PROB-014: No feature tree
**Status:** OPEN — blocked on kernel migration
**Description:** Operations are destructive on triangle soup. Cannot edit parameters after the fact. Undo is full mesh snapshot.

## Resolved

### PROB-001: Rust not installed
**Status:** RESOLVED (2026-03-26)
**Resolution:** Installed Rust 1.94.0 via rustup.

### PROB-005: Ghost face after extrude
**Status:** RESOLVED (2026-03-27)
**Description:** `extrude_face` mutated vertices in-place without removing old triangles → duplicate geometry at cap position.
**Resolution:** Save triangle topology, remove old face indices, mutate vertices to cap, re-add cap triangles. Same fix for `cut_face`.

### PROB-006: No hole support in polygon faces
**Status:** RESOLVED (2026-03-27)
**Description:** Fan triangulation couldn't represent faces with holes (washer/frame shapes).
**Resolution:** `add_polygon_face_with_holes_flush` uses earcutr with hole_indices. Regions from `geo` boolean ops now extract interior rings as holes.

### PROB-007: Face boundary fails on concave shapes
**Status:** MITIGATED (2026-03-27)
**Description:** `face_boundary_corners` angle-sorting fails for concave polygons.
**Resolution:** `stored_boundaries` HashMap preserves original boundary ordering. All face-creating paths now populate it.

## Technical Debt Register

### DEBT-001: Per-frame O(n) face_count()
**Severity:** P2
**Fix:** Cache face count; invalidate on mesh mutation.

### DEBT-002: Full mesh snapshot on every undo checkpoint
**Severity:** P2
**Fix:** Delta-based undo or copy-on-write.

### DEBT-003: Linear scan in face_normal()
**Severity:** P2
**Fix:** HashMap<face_id, FaceInfo> cache.

### DEBT-004: No BVH for picking
**Severity:** P3 (fine for now, P1 at 10k+ faces)
**Fix:** AABB BVH tree.

### DEBT-005: Edge rendering portability
**Severity:** P2
**Description:** Edge overlay now uses LineList topology with boundary-only edges (no POLYGON_MODE_LINE needed). The old PolygonMode::Line wireframe toggle still requires the GPU feature.
**Status:** Mostly resolved. Wireframe toggle hidden if feature unavailable.
