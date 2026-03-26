# Face/Edge/Vertex Selection — Research Notes (2026-03-26)

## Recommended: Hybrid Approach (GPU pick + CPU refinement)

### GPU Pick Pass
- Render scene to offscreen texture with unique u32 face IDs as fragment output
- Use `Rgba32Uint` format, `RENDER_ATTACHMENT | COPY_SRC`
- On click: `copy_texture_to_buffer` for 1x1 pixel at cursor → read face ID
- Readback is async (1 frame latency) — acceptable for click, fine for hover with double-buffered staging

### CPU Refinement (for edge/vertex from known face)
- After GPU returns face ID, project face's 3 edges to screen space
- Compute 2D distance from cursor to each screen-space line segment
- If within threshold (~5px): edge selection. Otherwise: face selection.
- Vertex: nearest vertex of the hit face to cursor in screen space.
- This is 3 distance computations — trivial.

### Why Hybrid Over Pure CPU Raycasting
- No BVH to build/maintain (rebuilds needed on every geometry edit in CAD)
- GPU pick scales to millions of triangles with no per-click cost
- CPU refinement for sub-face picks (edge/vertex) avoids extra GPU geometry

### Highlighting
- **Overlay pass**: Re-render selected face with alpha blend + `DepthCompare::LessEqual`
- **Stencil outline** (gold standard): Render selected to stencil, post-process for outline
- For GraniteX: start with overlay pass, upgrade to stencil outline later

### ID Encoding
| Mode | ID assignment |
|---|---|
| Face | Each triangle/quad gets unique u32 |
| Edge | Half-edge pairs share u32, rendered as thickened lines |
| Vertex | Each vertex gets u32, rendered as billboard quads |
