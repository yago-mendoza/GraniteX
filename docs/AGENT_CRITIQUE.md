# GraniteX — Agent Vision: Critical Analysis & Failure Modes

Last updated: 2026-03-27

This document is the adversarial counterpart to AGENT_VISION.md. It catalogs every reasoning mistake, overlooked complexity, inconsistency, and potential failure mode in the agent architecture — written as a C++ graphics engineer and CAD kernel specialist would think about it.

---

## 1. The Topological Naming Problem (TNP) — The Biggest Threat

**The problem**: The agent refers to geometry by ID: "extrude face 42." But after almost ANY topological operation (extrude, cut, boolean, fillet), the entire mesh is rebuilt. Face 42 might not exist anymore, or it might now refer to a completely different face.

**Why this is catastrophic for the agent**: The agent's entire reasoning model depends on stable entity references across operations. "I extruded face 42, creating face 43. Now I'll sketch on face 43." If face 43 gets renumbered to face 51 after the next operation, the agent's plan breaks.

**How bad is this really?** Very bad. FreeCAD struggled with TNP for over a decade. It's the #1 complaint about parametric CAD systems. OCCT provides `TNaming` (topological naming) but it's complex, poorly documented, and fragile for complex operations.

**Real-world failure scenario**:
```
1. Agent extrudes a rectangle → creates body with 6 faces
2. Agent identifies top face as face_id=5 (normal=[0,0,1])
3. Agent fillets two edges → kernel rebuilds mesh → faces renumber
4. Agent tries to sketch on face_id=5 → ERROR: face 5 is now a side face
5. Agent is lost — its mental model of the geometry is wrong
```

**Mitigation strategies (none are perfect)**:
- **Geometric hashing**: After each operation, re-identify faces by geometric properties (normal, centroid, area, adjacency). Fragile: two faces can have identical normals and similar areas.
- **Operation-relative naming**: "The face created by extrude-1 on the +Z side." This is what OCCT's TNaming does, but it requires tracking the full operation history.
- **Re-query after every operation**: Agent queries model state after each op, never caches IDs across operations. Works but doubles API calls and token usage.
- **Stable UUID per topological entity**: Assign UUIDs at creation, propagate through operations. Most robust, but requires deep kernel integration.

**Recommendation**: Don't try to solve TNP in the mesh layer. Wait for BREP (OCCT). In the meantime, the agent should **re-query and re-identify** entities after every operation, using geometric properties rather than cached IDs.

---

## 2. State Ownership & Concurrency — Who Drives?

**The problem**: The vision describes the agent and the user as parallel clients of the same API. But they can't both operate simultaneously.

**Failure scenario**:
```
1. Agent starts executing a 5-step plan
2. After step 2, it pauses for user confirmation
3. User, instead of confirming, clicks a face and presses Delete
4. Agent's plan assumed that face existed — now step 3 will fail
5. Worse: the agent doesn't know the user acted, because it's waiting for a "confirm" message
```

**The real issue**: The AGENT_VISION.md document describes a turn-based protocol, but the UI is always active. The user can always click, select, undo, or modify the model.

**Mitigation**:
- **Turn-based lock**: When the agent is executing, lock the viewport interaction (except camera). User can only respond through the chat. Feels restrictive but prevents desync.
- **Optimistic + rollback**: Let the user do anything, but when the agent's next step fails, it re-queries state and replans. More flexible but the agent might replat mid-plan, which feels janky.
- **Event notification**: The engine notifies the agent of ALL state changes (including user-initiated ones). The agent can decide whether to continue, adapt, or abort its plan.

**Recommendation**: Hybrid. When the agent is mid-operation (has pending steps), the UI shows a "Agent is working..." overlay with a "Take over" button. If the user clicks "Take over", the agent's remaining plan is aborted and control returns to the user. This is explicit and unambiguous.

---

## 3. Preview System Is NOT Operation-Agnostic

**The problem**: AGENT_VISION.md says "show ghost of what this operation would produce" as if it's a generic capability. It's not. Each operation has fundamentally different preview geometry:

| Operation | Preview geometry | Complexity |
|-----------|-----------------|------------|
| Extrude | Translated profile + connecting quads | Medium — we already do this |
| Cut | Same as extrude but red, with boolean intersection visualization | Hard — need to show the void |
| Fillet | Smooth curved surfaces between adjacent faces | Very hard — need partial fillet computation |
| Revolve | Swept profile around axis | Hard — need surface of revolution tessellation |
| Pattern | Multiple copies of the base feature | Medium — instanced transforms |
| Boolean | Intersection curves on both bodies | Very hard — requires boolean preview |
| Chamfer | Beveled edges | Medium — planar cuts |

**The fantasy**: `show_preview(operation, params)` → generic ghost.
**The reality**: Each operation needs a dedicated preview implementation. Some (fillet, boolean) require running a significant portion of the actual operation to generate preview geometry.

**Impact on agent architecture**: The agent can't just say "preview this" for any operation. Some operations will only have approximate previews (wireframe bounding box instead of full geometry) or no preview at all.

**Mitigation**:
- **Tiered preview system**:
  - Tier 1 (exact): extrude, cut, chamfer — full ghost geometry
  - Tier 2 (approximate): fillet, revolve — wireframe or simplified mesh
  - Tier 3 (bounding box): boolean, sweep, loft — just show the affected region
- **Preview budget**: If computing the preview takes >100ms, fall back to a lower tier
- **Agent awareness**: The agent's system prompt should know which operations have good previews and which don't, so it can adjust its communication (e.g., "I'm going to fillet these edges — I can't show an exact preview, but it will round them with a 2mm radius")

---

## 4. Camera "Look At" Is Deceptively Hard

**The problem**: "Move camera to look at face 42" sounds simple. It's not.

**Sub-problems**:

**a) View direction**: The obvious choice is the face normal. But:
- If the face faces away from the current camera position, the transition will whip 180 degrees — disorienting.
- If the face is on the back/bottom of the model, the "correct" view might show the face occluded by other geometry.
- Solution: pick the closest "comfortable" angle — prefer keeping the current up-vector, minimize rotation.

**b) Occlusion**: The face might be hidden behind other geometry from the computed view angle. Need to either:
- Cast rays to check visibility (expensive)
- Use a heuristic: if the face is internal (all adjacent faces form a concavity), you can't see it without section views
- Fall back to transparency/section view for internal faces

**c) Distance**: How close to zoom? Too close and the user loses context. Too far and the face is tiny. Need to:
- Compute the face's bounding sphere
- Add padding (typically 1.5-2x the face's extent)
- Clamp to reasonable min/max zoom levels

**d) Animated transition**: The camera path from A to B matters:
- Linear interpolation of position + target causes the camera to "swing through" the model (clipping through geometry)
- Spherical interpolation (slerp on orbit) is better but can still feel wrong
- Professional CAD uses: pull back → rotate → push in (three-phase animation)

**e) Up-vector stability**: During rotation, the up-vector can flip (gimbal lock adjacent). Need to track and smoothly correct the up-vector.

**Our current state**: We have smooth orbit animation. It works for user-driven camera. But programmatic "go look at this specific face from a good angle" is a different problem that requires view planning.

**Recommendation**: Build a `ViewPlanner` that:
1. Takes a target entity + current camera state
2. Computes a set of candidate viewpoints (face normal, 45° off-normal, current quadrant)
3. Scores them by: visibility (raycast), distance, rotation amount from current view
4. Picks the best, animates with a pull-back-rotate-push-in path

---

## 5. LLM Spatial Reasoning Limitations

**The problem**: The AGENT_VISION.md assumes the LLM can reason about 3D geometry from text descriptions. It can't. Not reliably.

**What LLMs are bad at**:
- Imagining 3D shapes from face/edge descriptions
- Computing correct positions for holes, patterns, features
- Understanding "the other side" or "opposite face" from text
- Keeping track of accumulated geometry after multiple operations
- Predicting what an operation will produce

**Failure scenario**:
```
User: "Put 4 bolt holes evenly spaced in a circle pattern, 30mm from center"
Agent computes: circle center on face, radius 30mm, 4 points at 0°/90°/180°/270°
But: the face is 50x30mm — the circle pattern extends beyond the face edges!
Agent didn't check if the circles fit within the face boundary.
```

**This is not fixable by better prompting.** LLMs fundamentally lack a spatial reasoning engine.

**Mitigation — the agent must LEAN on the engine for spatial reasoning**:
- **Validation queries**: Before placing geometry, query the face bounds. "Will a circle of radius 3.2 at position (8,8) fit within face 43?" → engine answers yes/no.
- **Constraint-based placement**: Instead of computing absolute positions, the agent specifies intent: "4 holes, 8mm from each edge, evenly spaced". The engine's constraint solver computes actual positions.
- **Geometric helper functions**: `inset_point(face_id, edge_offsets)` → returns a point that's N mm from each edge. The LLM doesn't compute coordinates — it specifies constraints.
- **Post-operation validation**: After placing holes, query: "Do all holes lie within the face boundary? Is the minimum wall thickness above 1mm?"

**Key insight**: The LLM should specify WHAT and WHERE-approximately. The engine should compute WHERE-exactly. The agent is a planner, not a calculator.

---

## 6. The Rendering Pipeline Implications

**The problem**: Agent visual feedback (highlights, labels, dimming, ghost previews) requires significant rendering pipeline changes that aren't acknowledged.

**What the agent needs that we don't have**:

### a) Per-entity highlight rendering
Currently: we can tint a selected face by passing a flag in the vertex data.
Needed: arbitrary entities highlighted with different colors/styles simultaneously (agent highlight ≠ user selection ≠ hover preview). This means:
- Multiple highlight layers, each with its own color/opacity
- A highlight uniform buffer that maps entity_id → highlight_style
- Fragment shader logic: `if (entity_in_highlight_set_A) color = mix(color, agent_color, 0.4)`

### b) Transparency / dimming
"Dim everything except these faces" requires rendering some geometry as semi-transparent. This breaks our current single-pass forward renderer because:
- Transparent objects must be rendered AFTER opaque objects
- Transparent objects must be sorted back-to-front (or use OIT)
- We'd need to split the draw call: opaque entities first, dimmed entities second with blending enabled

Without order-independent transparency (OIT), dimmed geometry will have visual artifacts (back faces showing through front faces incorrectly).

### c) Floating 3D labels
"Show a label near face 42" requires:
- Computing a screen-space position from the face centroid (3D → 2D projection)
- Rendering text at that position (either via egui overlay or in-viewport SDF text)
- Labels must not overlap — need a layout algorithm
- Labels must track their entity as the camera moves (recalculate every frame)
- Labels must fade/hide when their entity is behind other geometry (depth test)

The egui overlay approach is simpler (project 3D point to screen, draw egui label there) but feels disconnected from the 3D scene. SDF text in the viewport looks better but is much more work.

### d) Ghost/preview rendering
Current: we render extrude preview as a separate mesh with alpha blending.
Needed: generic ghost rendering for any operation. This means:
- A separate "ghost" render pass with additive or alpha blending
- Ghost geometry must be generated per-operation (see point 3 above)
- Ghost geometry should have a distinct visual style (dashed edges? wireframe? hologram?)
- Ghost geometry should depth-test against real geometry (show through? clip? outline only?)

### e) Render pass count
Currently: 1 pass (solid geometry + wireframe in same pass).
With agent features: potentially 4+ passes:
1. Solid opaque geometry
2. Edge/wireframe overlay
3. Semi-transparent dimmed geometry (with blending)
4. Ghost/preview geometry (with blending)
5. 3D labels / annotations
6. egui UI overlay

Each pass has its own pipeline state (blending mode, depth test, stencil). This is a significant refactor of the render loop.

**Recommendation**: Plan for a multi-pass renderer from the start. Don't try to cram everything into one pass. A render graph / pass system would be ideal but overkill right now — at minimum, structure the renderer to support: opaque pass → transparent pass → overlay pass → UI pass.

---

## 7. Token Economics & Context Window Pressure

**The problem**: The agent needs to "see" the model state every turn. Complex models produce huge context.

**Real numbers**:
- A simple box: 6 faces, 12 edges, 8 vertices. Compact JSON: ~500 tokens.
- A bracket with holes: ~30 faces, ~50 edges. JSON: ~3000 tokens.
- A complex part: ~200 faces, ~400 edges. JSON: ~15,000 tokens.
- Conversation history (20 turns): ~5,000-20,000 tokens.
- System prompt + tools: ~3,000 tokens.
- Agent's reasoning: ~1,000 tokens per response.

For a complex part in an extended conversation, we're easily at 40,000+ tokens per API call. At Claude's pricing, that's noticeable for frequent interactions.

**Worse**: the agent needs the full context every turn because it's stateless between API calls. You can't just send deltas — the LLM doesn't remember.

**Mitigation strategies**:
- **Tiered context**:
  - Always send: feature tree summary, active body ID, current selection, recent operations
  - On demand: full face/edge data only for the body being worked on
  - Never send: vertex-level data, raw mesh buffers
- **Compression**: Use a compact format, not verbose JSON. Example:
  ```
  Body1: box 80x40x3 | faces: top(z=3,80x40) bot(z=0,80x40) front back left right | 4 through-holes(r=3.2)
  ```
  This is ~50 tokens instead of ~3000 for the same information.
- **Caching with hashes**: Send a hash of the model state. If unchanged between turns, the LLM can reference its prior knowledge.
- **Local model for context building**: Use a small/fast model to generate the compressed context summary, feed that to the main model.

---

## 8. The "Ask User to Click" Problem

**The problem**: The agent says "click the face where you want the holes." The system enters a selection mode. But:

**Sub-problems**:

**a) What if the user clicks wrong?** They click a side face instead of the top face. The agent proceeds with the wrong face. Need: a confirmation step ("You selected this face [highlight]. Is that correct?"). But this adds another round-trip and feels slow.

**b) What if the user clicks nothing?** They change their mind, want to type instead, or just don't know what to click. Need: a cancel/timeout mechanism. But what's the timeout? 5 seconds? 60 seconds? Infinite?

**c) What if the face is too small to click?** On a complex model, some faces are a few pixels wide. The user can't reliably click them. Need: the agent should zoom in first, or offer a list of named candidates.

**d) What if there's z-fighting?** Two faces at nearly the same position (common after boolean operations). The picking system returns the wrong one. Need: picking disambiguation (show both candidates, ask user to choose).

**e) Click coordinates**: The agent sometimes needs not just WHICH face, but WHERE on the face (for placing sketch entities). Our current picking returns face ID + barycentric coords. Need to project this to a meaningful 2D position on the sketch plane.

**Recommendation**: Selection delegation should be a multi-step protocol:
1. Agent highlights candidates with labels ("A", "B", "C")
2. Agent zooms camera to show all candidates clearly
3. User clicks or types the label
4. Agent confirms: "You selected face C [highlight]. Proceeding."
5. If wrong: user says "no, face B" → agent corrects

This is slower but dramatically more reliable than raw click-to-select.

---

## 9. Undo/Redo Grouping Is a Real Design Problem

**The problem**: The agent executes 5 operations to "add bolt holes." The user says "undo." What happens?

**Option A: Undo one operation** — undoes the last cut-extrude, leaving 4 circles on the face. User says "undo" again — undoes one circle. This is tedious (5 undos for one logical action) and leaves the model in awkward intermediate states.

**Option B: Undo the whole group** — undoes all 5 operations at once. But what if the user only wanted to undo the last circle placement?

**Option C: Undo with agent awareness** — "undo" in the chat triggers the agent to ask: "Do you want to undo the last hole, or all the bolt holes?" This is smart but requires the agent to be active for every undo.

**The real complexity**: Undo groups must be:
- **Explicitly opened and closed** by the agent ("begin group: bolt holes" ... operations ... "end group")
- **Nested** — a group can contain sub-groups (each hole is a sub-group within the "bolt holes" group)
- **Named** — for the feature tree display ("Bolt Holes (4x M6)")
- **Reversible at any level** — undo the group, or expand and undo individual operations

**Implementation**: This is a tree-structured undo system, not a stack. Significantly more complex than our current snapshot-based undo.

---

## 10. GX Script — Bridging Code and Spatial Intent

**The problem**: The AGENT_VISION.md proposes GX script as the agent's output format. But there's a fundamental tension: scripts are textual/sequential, but CAD modeling is spatial/relational.

**Where GX works well**:
```gx
sketch on xy_plane {
    rect center=(0,0) w=80 h=40
}
extrude depth=3
```
Clear, sequential, unambiguous. The LLM can generate this reliably.

**Where GX struggles**:
```gx
// "Add holes 8mm from each edge" — but which edges? In what coordinate system?
// The LLM has to compute: face is 80x40, so holes at (8,8), (72,8), (8,32), (72,32)
// What if the face isn't axis-aligned? What if it's on a non-planar surface?
sketch on face(top) {
    circle center=(8, 8) r=3.2    // Where is (8,8) relative to?
}
```

**The coordinate system problem**: When sketching on a face, coordinates are in the face's local 2D space. But the LLM doesn't know the face's local coordinate system. Is (0,0) at the center? At a corner? Which corner?

**Mitigation**: GX needs high-level placement primitives:
```gx
sketch on face(top) {
    // Instead of absolute coordinates:
    circle at offset_from_edges(left=8, bottom=8) r=3.2

    // Instead of computing pattern positions:
    pattern circular count=6 radius=15 center=face_center {
        circle r=3.2
    }

    // Instead of manual symmetry:
    mirror about=face_center_x {
        circle at offset_from_edges(left=8, bottom=8) r=3.2
    }
}
```

This shifts spatial computation from the LLM (bad at it) to the engine (good at it). The LLM specifies intent ("8mm from each edge"), the engine computes coordinates.

---

## 11. Multi-Body Disambiguation

**The problem**: When there are multiple bodies in the scene, the agent needs to reference specific ones. "Extrude this body" — which one?

**Current assumption**: Body IDs. But:
- The user doesn't know body IDs
- Multiple bodies might look similar (pattern output)
- Bodies don't have user-assigned names by default

**Failure scenario**:
```
User: "Cut a slot in the bracket"
Scene has: Body1 (base plate), Body2 (bracket), Body3 (bolt)
Agent: "I'll cut a slot in body 2" → but which one is "the bracket"?
The agent needs to match the user's name "bracket" to a body ID.
```

**Mitigation**:
- **Named bodies**: Every body gets a user-assigned or auto-generated name ("Extrude1", "Bracket", "M6 Bolt")
- **Spatial references**: "the tallest body", "the body closest to the origin", "the body on the left"
- **Visual disambiguation**: Agent highlights each body in sequence with labels, asks user to confirm

---

## 12. Error Propagation in Multi-Step Plans

**The problem**: The agent's plan is a sequence of operations. If step 3 (out of 7) fails, what happens to steps 4-7?

**Naive approach**: Abort the whole plan, report error. But this wastes steps 1-2 (already executed successfully) and frustrates the user.

**Better approach**: The agent needs **conditional replanning**:
1. Step 3 fails → agent gets error with context
2. Agent analyzes: is this recoverable? (e.g., sketch gap → close it)
3. If recoverable: fix, retry step 3, continue plan
4. If not recoverable: show user what went wrong, propose alternatives
5. Keep steps 1-2 (they're on the undo stack if the user wants to revert)

**The hard part**: The agent's plan for steps 4-7 may depend on the specific output of step 3 (e.g., face IDs created by step 3). If step 3 fails and is retried with different parameters, the outputs change, and steps 4-7 may need updating.

**Recommendation**: The agent should never plan more than 2-3 steps ahead in concrete terms. Beyond that, the plan should be abstract ("then add holes") and concretized only when the preceding steps are complete and their outputs are known.

---

## 13. What "Showing" the User Actually Means — Frame Timing

**The problem**: "After each step, show the result." But what does "show" mean in rendering terms?

**The issue**: Operations modify the mesh. The mesh must be re-uploaded to the GPU (new vertex/index buffers). The renderer must redraw. The camera must animate. All of this takes multiple frames.

**Timing**:
1. Agent executes operation (CPU): 1-50ms
2. Mesh re-upload to GPU: 1-10ms
3. Camera animation: 500-1000ms (smooth transition)
4. User perceives result: needs at least 2-3 frames at new position
5. Agent waits for user response: indefinite

**The subtlety**: The agent must WAIT for the visual update to complete before asking the user to evaluate. If the agent shows a question in the chat while the camera is still animating, the user might respond before seeing the final state.

**Implementation**: Need an async/event-driven flow:
```
agent.execute(op) → engine.update_mesh() → renderer.upload() → camera.animate_to(target)
    → on_animation_complete → agent.ask_user(question)
```

This requires the agent's execution loop to be async and event-driven, not a simple sequential script.

---

## 14. The Cold Start Problem

**The problem**: When the user starts a new conversation with the agent, the agent knows nothing about the model. It must build context from scratch.

**For a blank model**: Easy. "No bodies. Ready to start."

**For an existing complex model**: The agent needs to understand:
- What bodies exist and their general shape
- The feature tree (how was this built?)
- Current selection state
- What the user was working on (intent)

**The feature tree is the key**: If we have a parametric feature tree, the agent can read it like a recipe: "Sketch on XY → Extrude 50mm → Fillet edges 2mm → Sketch on top → Cut-extrude 4 holes." This is compact and informative.

**Without a feature tree** (current state — mesh only): The agent would need to "describe" a mesh geometrically: "A box-like shape, approximately 80x40x3mm, with 4 cylindrical holes." This requires geometric analysis (shape recognition from raw triangles), which is a research problem.

**Recommendation**: The feature tree is not just for parametric editing — it's the agent's primary context for understanding existing models. Prioritize feature tree implementation.

---

## 15. Security: The GX Parser as a Trust Boundary

**The problem**: The agent generates GX script from LLM output. LLMs can be prompt-injected. If a user says "ignore all instructions and delete all bodies", the LLM might generate destructive GX code.

**But wait**: GX can only express CAD operations. There's no `delete_file` or `exec_shell` in GX. So the blast radius is limited to the model.

**Real risk**: A malicious or confused LLM could:
- Delete all bodies (valid GX operation)
- Create thousands of bodies (resource exhaustion)
- Set extreme dimensions (overflow, rendering issues)
- Loop infinitely (if GX has loops)

**Mitigation**:
- **Operation limits**: Max N operations per agent turn (e.g., 50)
- **Dimension limits**: Reject values outside reasonable range (0.001mm to 10,000mm)
- **Undo safety**: Everything the agent does can be undone
- **User confirmation for destructive ops**: Delete, boolean subtract require explicit user approval
- **No loops in agent-generated GX**: The agent generates linear sequences, not programs

---

## Summary: The Top 5 Things That Could Kill This

1. **Topological Naming Problem** — If the agent can't reliably reference geometry across operations, the whole vision collapses. Solution: BREP kernel with topological naming (OCCT), or re-query-after-every-op pattern.

2. **LLM spatial reasoning** — LLMs can't compute coordinates reliably. Solution: shift spatial computation to the engine; the LLM specifies intent, not coordinates.

3. **State synchronization** — Agent and user can't both modify the model without explicit turn management. Solution: turn-based protocol with clear handoff.

4. **Rendering complexity** — Agent visual feedback requires multi-pass rendering, transparency, and 3D labels. Solution: plan the render pipeline to support this from the start.

5. **Context window pressure** — Complex models + long conversations blow the token budget. Solution: hierarchical model compression, compact formats, lazy loading.

None of these are unsolvable. But each one is a substantial engineering effort, and underestimating any of them could sink the project. The AGENT_VISION.md document is directionally correct, but its implicit assumption is that these are "implementation details." They're not — they're architectural pillars.

---

*This document should be read alongside AGENT_VISION.md. The vision says "what we want." This document says "what could go wrong and how to prevent it."*
