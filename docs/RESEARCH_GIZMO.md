# Axis Gizmo — Research Notes (2026-03-26)

## Architecture Decision
Render gizmo in the SAME render pass using `set_viewport` to a small corner rect (120x120px, bottom-left).

## Key Design
- Gizmo uses main camera's **rotation only** (no translation) + orthographic projection
- Each axis arrow = cylinder (shaft) + cone (head), generated procedurally at init
- 3 positive + 3 negative axis arrows, colored RGB
- Per-arrow model matrix via push constants (or small uniform buffer)
- Total geometry: ~400 verts, ~1200 indices — negligible

## Click Detection
- Check if mouse is within gizmo screen rect
- Convert to gizmo NDC, build inverse view_proj
- Ray-capsule intersection against each axis (generous radius ~0.15 for comfort)
- Hover: run hit test every frame, highlight hovered axis via uniform

## Camera Snap Animation
- On click: start slerp from current rotation to target
- Ease-out cubic: `t_eased = 1.0 - (1.0 - t)^3`
- Duration: ~0.3s for snappy feel
- Target rotations: standard orthographic views (front/back/top/bottom/left/right)

## Depth Handling
- `set_viewport` resets depth mapping for gizmo rect
- Use `CompareFunction::Always` + back-to-front sort to prevent self-occlusion
