# GraniteX — Known Problems & Technical Debt

Last updated: 2026-03-26

## Active Blockers

### PROB-001: Rust not installed
**Status:** RESOLVED (2026-03-26)
**Description:** Yago's machine doesn't have the Rust toolchain installed.
**Resolution:** Installed Rust 1.94.0 via rustup.

## Anticipated Problems

### PROB-002: wgpu backend selection on Windows
**Risk:** Medium
**Description:** wgpu on Windows defaults to DX12, which has some edge cases with older drivers. May need to force Vulkan backend for RenderDoc compatibility.
**Mitigation:** Set `WGPU_BACKEND=vulkan` env var if DX12 causes issues. Test both backends early.

### PROB-003: egui + 3D viewport input conflict
**Risk:** Medium
**Description:** egui captures mouse/keyboard events. When the cursor is over the 3D viewport, we need to pass input to the camera/tools instead of egui. Need to check `egui::Context::wants_pointer_input()` and `wants_keyboard_input()`.
**Mitigation:** Well-known pattern, documented in egui examples. Handle early in Phase 1.

### PROB-004: Floating point precision in geometry
**Risk:** High (long-term)
**Description:** Floating point arithmetic causes accumulation errors in geometric operations. Boolean operations, intersection tests, and constraint solving all suffer from this.
**Mitigation:** Use epsilon comparisons, robust predicates crate, and consider exact arithmetic for critical operations. This is a Phase 9 concern.

## Technical Debt Register

### DEBT-001: Per-frame O(n) face_count()
**Severity:** P2
**Description:** `face_count()` collects all face_ids, sorts, and deduplicates every frame (called from UI stats). On large imported meshes this is wasteful.
**Fix:** Cache face count; invalidate on mesh mutation.

### DEBT-002: Full mesh snapshot on every undo checkpoint
**Severity:** P2
**Description:** `CommandHistory::save_state()` clones the entire vertex/index vectors. For large meshes (100k+ tris from STL import), each undo point is >1MB.
**Fix:** Implement delta-based undo (store only changed faces) or copy-on-write with `Arc<Vec<...>>`.

### DEBT-003: Linear scan in face_normal()
**Severity:** P2
**Description:** `face_normal()` does `.find()` on the full vertex list. Called frequently during operations and picking.
**Fix:** Build `HashMap<face_id, FaceInfo>` cache.

### DEBT-004: No BVH for picking
**Severity:** P3 (fine for now, P1 at 10k+ faces)
**Description:** `pick_face()` tests every triangle via Moller-Trumbore. O(n) per click/hover.
**Fix:** Build AABB BVH tree, cull branches before triangle tests.

### DEBT-005: Edge rendering requires POLYGON_MODE_LINE
**Severity:** P2
**Description:** Edge overlay uses `wgpu::PolygonMode::Line` which requires a device feature not available on all GPUs or WebGPU.
**Fix:** Generate explicit edge line geometry from mesh topology (extract unique edges, render as LineList).
