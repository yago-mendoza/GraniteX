# GraniteX — Known Problems & Technical Debt

Last updated: 2026-03-26

## Active Blockers

### PROB-001: Rust not installed
**Status:** BLOCKING — cannot proceed with any code until resolved
**Description:** Yago's machine doesn't have the Rust toolchain installed.
**Resolution:** Install via rustup (rustup.rs or `winget install Rustlang.Rustup`). Need to restart terminal after install.

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

(Empty — no code yet. Will track debt as it accumulates.)
