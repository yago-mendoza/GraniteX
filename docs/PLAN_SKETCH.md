# Sketch System Implementation Plan

## Architecture

```
User clicks face → enters Sketch Mode → draws on 2D plane → exits → contour available for extrude/cut
```

### Data Flow
1. SketchPlane: derives 2D coordinate system from selected face (origin + U + V + normal)
2. Mouse cursor → unproject to ray → intersect with sketch plane → 2D coordinates
3. Drawing tools create SketchEntity objects (lines, rects, circles, arcs)
4. SketchRenderer draws entities as 3D geometry on the face plane
5. ContourDetector finds closed loops for operations

### Key Structs
- `SketchPlane { origin, u_axis, v_axis, normal }` — 2D/3D mapping
- `SketchEntity` — enum: Line, Rect, Circle, Arc
- `Sketch { plane, entities, is_active }` — the sketch state
- `SketchRenderer` — GPU pipeline for drawing lines/curves on faces

## Implementation Steps (ordered)

1. [x] SketchPlane — derive from face, 2D↔3D transforms
2. [x] Sketch struct — entity storage
3. [x] Mouse-to-plane projection — cursor → 2D sketch coords
4. [x] Line drawing — click start, click end, line appears
5. [x] SketchRenderer — draw lines as 3D geometry with depth bias
6. [x] UI mode switch — Sketch tool shows sketch controls
7. [ ] Rectangle tool — 2 clicks → 4 lines
8. [ ] Circle tool — center + radius
9. [ ] Snapping — grid, endpoints, midpoints
10. [ ] Closed contour detection
11. [ ] Visual feedback — valid contours green, open red
12. [ ] Extrude from contour
