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

## 16. Operation Decomposition — The Granularity Problem

**The problem**: AGENT_VISION.md treats "extrude" as a single atomic operation. But consider what the agent must actually do to extrude:

1. Identify the face → **requires user interaction or spatial query**
2. Determine direction → normal? custom vector? both sides?
3. Determine depth → specific value? through-all? to-next-face?
4. Show preview → **requires rendering pipeline support**
5. Get user confirmation → **requires protocol round-trip**
6. Execute → **actual geometry modification**
7. Report results → new face IDs, updated feature tree

That's 7 sub-steps for what we call "one operation." Now multiply by parameter complexity:

| Operation | Parameters needed | Minimum round-trips to collect them |
|-----------|------------------|-------------------------------------|
| Extrude | face + depth + direction | 1-2 |
| Fillet | edge_set + radius | 1-2 |
| Pattern linear | body + direction + count + spacing | 3-4 |
| Revolve | sketch + axis_edge + angle | 2-3 |
| Sweep | sketch + path_curve | 2 (but path selection is complex) |
| Loft | profile1 + profile2 + guides | 3+ |
| Boolean subtract | body_A + body_B | 1-2 |

Each round-trip = agent asks → user responds → LLM API call (2-5s) → agent processes. A 4-parameter operation takes **8-20 seconds of pure waiting**. That's unacceptable.

**The fix: batch parameter proposals.** The agent proposes ALL parameters at once with sensible defaults, and the user adjusts what's wrong:

```
Agent: "I'll fillet these 4 edges [highlighted in orange] with a 2mm radius.
        Adjust radius with the slider, or click edges to add/remove.
        Confirm when ready."
```

One confirmation instead of 4 questions. The UI provides a **parameter panel** for the operation in progress — the agent fills in defaults, the user tweaks. This is exactly how SolidWorks' PropertyManager works: everything filled in, user adjusts what's wrong.

**Implication for the protocol**: Operations aren't just "execute with params." They're:
1. `propose(operation, default_params)` → shows preview + parameter panel
2. User adjusts params via UI sliders/clicks
3. `confirm()` → executes with final params
4. OR `cancel()` → discard preview

---

## 17. The "Adjust" Problem — Hybrid Agent-Manual Interaction

**The problem**: The agent shows a preview. The user says "a bit more." What happens?

**Scenario**:
```
Agent: shows extrude preview, depth=5mm
User: "a bit more"
Agent: ??? — How much is "a bit more"?
```

"A bit more" is inherently imprecise. The LLM could guess (+20%? +1mm? +2mm?) but it's wasting round-trips on something the user could resolve in 0.5 seconds with a drag handle.

**The real solution: the agent can hand control of a single parameter to the user's mouse.** When the agent shows an extrude preview, the user should be able to DRAG the preview cap to adjust depth — exactly like our current manual extrude workflow. The agent initiated it, but the user fine-tunes it spatially.

**Protocol extension**:
```
Agent → Engine: propose_extrude(face=42, depth=5.0, allow_adjust=["depth"])
Engine → User: shows preview with draggable depth handle
User → Engine: drags to depth=7.3
Engine → Agent: params_adjusted(depth=7.3)
User → Engine: clicks "Confirm"  (or presses Enter)
Engine → Agent: confirmed(final_params={depth: 7.3})
Agent: proceeds with depth=7.3
```

**This is the killer UX insight**: the agent isn't just text-in/text-out. It can **delegate spatial adjustments to the user's mouse** while retaining control of the overall plan. The agent is the director, the user is the fine-tuner.

**Rendering implication**: every operation preview needs interactive handles (drag points, sliders, rotation rings) that work identically whether the agent or the user initiated the operation.

---

## 18. GPU/Rendering Bugs a Graphics Engineer Would Catch

### 18a. Highlight Bleeding at Triangle Seams

When we highlight face 42 by checking `face_id` in the fragment shader, adjacent triangles from different faces share pixels at their boundary (due to rasterization). At the seam between face 41 and face 42, some fragments will come from face 41's triangles and some from face 42's — within the same pixel. With MSAA, this means the pixel gets a blend of highlighted and non-highlighted color.

**Result**: A 1-2 pixel "halo" or "fringe" around highlighted faces. Barely visible on flat surfaces, very noticeable on curved surfaces where many small triangles share edges.

**Fix**: Accept it (it's subtle) or use a stencil-based approach:
1. Render highlighted faces into the stencil buffer
2. Full-screen pass: where stencil is set, blend highlight color
This gives pixel-perfect highlight boundaries but costs an extra pass.

### 18b. Ghost Preview Depth Interaction

Our current extrude preview renders with alpha blending and standard depth testing. This works when the ghost is ABOVE the existing geometry. But:

**Cut preview**: The ghost represents REMOVED material. It should show the void inside the body. But the ghost geometry is inside the body — it fails depth test and is invisible.

**Fillet preview**: The fillet surface partially overlaps existing edges. Z-fighting between ghost and real geometry at the tangent lines.

**Boolean preview**: The intersection volume is INSIDE both bodies. Invisible with standard depth test.

**Fixes** (per operation type):
- **Cut**: Don't show the subtracted volume. Instead, show the RESULT: render the body with the cut applied as ghost, existing body dimmed. Expensive (requires actually computing the cut) but correct.
- **Fillet**: Use `depth_bias` to push ghost slightly toward camera. Accept minor visual artifacts at tangent lines.
- **Boolean**: Show intersection OUTLINE (curves on the surface) instead of intersection volume. Use thick lines, depth-tested against both bodies.

### 18c. MSAA + Alpha Blending Artifacts

We use MSAA 4x. When we render semi-transparent geometry (ghost previews, dimmed bodies), alpha blending interacts with multisampling:

1. MSAA resolves by averaging sub-samples
2. At the edge of a semi-transparent triangle, some sub-samples hit the triangle (alpha=0.3), others don't (alpha=0.0)
3. The resolved pixel gets alpha=0.075 (average of 0.3, 0.0, 0.0, 0.0 for a 1/4 coverage pixel)
4. The alpha-blended result is barely visible at edges — looks like the ghost is "melting away" at its silhouette

**Fix**: Either:
- Use `alpha_to_coverage` instead of alpha blending for the ghost pass (converts alpha to MSAA coverage mask — binary transparency, no blending artifacts)
- Resolve MSAA before the transparent pass (render opaque at 4x MSAA, resolve to texture, render transparent at 1x on top)
- Accept it (it's subtle, most CAD tools have this too)

### 18d. Depth Precision for Dimmed Geometry

"Dim everything except face 42" means rendering the rest of the body at reduced opacity. But semi-transparent geometry has depth-write disabled (otherwise transparent faces would occlude each other incorrectly). Without depth-write, the back faces of the dimmed body show through the front faces.

**Scenario**: Dimmed cube viewed from front. Front face at alpha=0.2, back face at alpha=0.2. Both render (no depth culling for transparent). Result: you see both faces, the cube looks like a wireframe-ish mess instead of a subtle ghost.

**Fixes**:
- **Two-pass dimming**: First pass renders dimmed geometry with depth-write ON, depth-test ON, alpha=1.0 but tinted. The depth buffer prevents back-face visibility. This isn't true transparency but looks correct for "dimming."
- **Depth peeling**: Proper order-independent transparency. Overkill for this use case.
- **Outline-only dimming**: Don't make geometry transparent. Instead, render it with desaturated/gray color and only show edge outlines. The model becomes a "wireframe sketch" while the highlighted face is full color. This is actually how many CAD tools do it (OnShape's "isolate" mode).

**Recommendation**: Outline-only dimming. It's the simplest to implement, looks professional, and avoids all transparency sorting issues. Highlighted faces: full color + shading. Everything else: gray + edge outlines only. No alpha blending needed.

### 18e. Floating Label Depth Ordering

3D labels (text floating near faces) need to be readable regardless of camera angle. But:

**Problem 1**: Label is behind geometry from certain angles. If depth-tested, it disappears. If not depth-tested, it renders on top of everything (even faces in front of it).

**Problem 2**: Multiple labels overlap in screen space. "Face A" and "Face B" labels render on top of each other, becoming unreadable.

**Problem 3**: Label anchor point (face centroid) moves as camera orbits. Labels "swim" across the screen, which is disorienting.

**Fixes**:
- **egui overlay approach** (recommended): Project face centroid to screen coordinates. Render labels as egui windows/text at those screen positions. Depth-test by comparing the centroid's depth against the depth buffer at that screen position. If occluded, either hide or show at reduced opacity with a "[behind]" indicator.
- **Label collision avoidance**: If two labels are within N pixels, offset them vertically with leader lines (thin lines from label to anchor point). Like chart data labels.
- **Sticky labels**: When camera is orbiting, don't update label positions every frame. Update them at ~10 Hz (every 6 frames at 60fps). This prevents swimming and reduces the visual noise of moving text.

---

## 19. UI State Machine — The Agent's Face

**The problem**: The vision document describes what the agent CAN do but not what the UI SHOWS at each moment. The user needs to understand the agent's state at a glance.

### The agent has 7 distinct states:

| State | UI indicator | Viewport behavior | Chat panel behavior |
|-------|-------------|-------------------|---------------------|
| **Idle** | Green dot in status bar | Normal interaction | Text input enabled, "Ask me anything" placeholder |
| **Thinking** | Pulsing blue dot + "Thinking..." | Normal interaction (user can still work) | Input disabled, typing animation |
| **Proposing** | Orange dot + "Review plan" | Preview geometry shown, parameter panel open | Shows plan steps, "Confirm / Adjust / Cancel" buttons |
| **Executing** | Blue dot + "Working... (3/7)" | Viewport animates (camera moves, geometry appears) | Progress list, each step checks off, "Stop" button |
| **Waiting for input** | Yellow dot + "Your turn" | Selection mode active (if agent asked for a click) | Input enabled, prompt text from agent, "Skip" button |
| **Error** | Red dot + "Problem" | Last good state shown, failed geometry highlighted | Error description, "Retry / Undo / Help" buttons |
| **Paused** | Gray dot + "Paused" | Normal interaction | "Resume" button, plan visible but dimmed |

**Critical transitions**:
- Thinking → Proposing: agent has a plan, shows it before executing
- Proposing → Executing: user confirmed the plan
- Executing → Waiting: agent needs user input (click, text, choice)
- Executing → Error: an operation failed
- ANY → Idle: user clicks "Cancel" or "Take over"

**The plan panel** (this is essential and was missing from the vision):
```
┌─── Agent Plan ──────────────────┐
│ ✅ 1. Base rectangle (80×40)    │
│ ✅ 2. Extrude 3mm               │
│ → 3. Place bolt holes (M6×4)    │ ← current step, highlighted
│ ○ 4. Cut through-all            │
│ ○ 5. Fillet edges (2mm)         │
│                                  │
│ [Pause] [Cancel] [Take over]    │
└──────────────────────────────────┘
```

The user can:
- Click any completed step to see that state (time travel in the feature tree)
- Click a future step to see a preview of what's coming
- Drag to reorder steps (risky but powerful)
- Delete a future step (agent replans without it)

---

## 20. Capability Discovery — What Can the Agent Do?

**The problem**: Neither the user nor the agent knows the full set of available operations at any given moment.

**User side**: The user asks "can you thread this hole?" The agent must honestly say no. But how does the user discover what IS available without trial and error?

**Three approaches (use all three)**:

1. **Capability hints in chat**: When the agent finishes an operation, it suggests related operations:
   ```
   Agent: "Extruded to 3mm. ✅"
   Agent: "I can also: fillet the edges, add holes, shell to make it hollow, or pattern this body."
   ```
   Context-sensitive suggestions, not a full list.

2. **Operation palette in UI**: A permanent sidebar panel showing all operations organized by category (Sketch, Feature, Modify, Assembly). Unavailable operations grayed out with "Coming soon" or "Requires BREP kernel." The agent can execute any non-grayed operation.

3. **Agent self-assessment**: The agent's system prompt includes a capabilities manifest:
   ```
   AVAILABLE_OPERATIONS:
   - extrude(face, depth, direction) ✅
   - cut(face, depth) ✅
   - fillet(edges, radius) ❌ requires BREP
   - revolve(sketch, axis, angle) ❌ not implemented
   ```
   This is updated automatically when new operations are implemented. The agent reads it and can honestly say "I can't do fillets yet, but I can chamfer edges as a workaround."

**Agent side**: The Claude API tool definitions ARE the capability manifest. Each tool = one operation. If the tool isn't in the list, the agent can't call it. When we implement revolve, we add a `revolve` tool. The agent automatically knows about it.

---

## 21. Errors the Vision Document Got Wrong

### 21a. The protocol is NOT "in-process JSON"
The vision says "this is NOT a REST API — it runs in-process as an async channel. But the message format is JSON-like for LLM compatibility."

**Wrong.** The LLM is external (Claude API over HTTPS). The actual architecture is:

```
┌────────────┐      in-process      ┌──────────────────┐      HTTPS       ┌───────────┐
│ GraniteX   │ ←──────────────────→ │ Agent Controller │ ←───────────────→ │ Claude    │
│ Engine     │   Rust function       │ (Rust module)    │   API calls       │ API       │
│            │   calls               │                  │                   │           │
└────────────┘                      └──────────────────┘                   └───────────┘
```

The Agent Controller is a Rust module that:
- Builds context from engine state (model, selection, camera)
- Serializes context + conversation history for the Claude API
- Calls Claude API with tool definitions
- Parses tool_use responses into operation calls
- Dispatches operations to the engine
- Collects results and feeds them back to the next API call

The "JSON messages" in the vision document are **Claude tool definitions and tool results**, not a custom protocol. We should use Claude's native tool_use format, not invent our own.

### 21b. GX script vs tool_use — pick one
The vision says the agent generates GX script. The critique says it should use Claude tool_use. These are contradictory.

**Resolution**: Use **Claude tool_use** (native). Each CAD operation is a tool:

```json
{
  "name": "extrude",
  "description": "Extrude a face along its normal to create a solid protrusion",
  "input_schema": {
    "type": "object",
    "properties": {
      "face_id": { "type": "integer", "description": "ID of the face to extrude" },
      "depth": { "type": "number", "description": "Extrusion depth in mm" },
      "direction": { "enum": ["normal", "reverse", "both"], "default": "normal" }
    },
    "required": ["face_id", "depth"]
  }
}
```

The agent calls tools directly. No parsing needed. Claude handles the structured output natively. Error handling comes for free (tool results can be error messages).

**GX script's role changes**: GX is for **human authoring, macro recording, and file serialization** — not for agent output. The agent speaks tool_use. GX scripts can be replayed by converting each line to the equivalent tool call.

### 21c. Tool definition token overhead
With 30+ operations as tools, each tool definition is ~100-150 tokens. That's 3,000-4,500 tokens of overhead on EVERY API call, before any conversation content.

**Fix**:
- **Dynamic tool loading**: Only send tool definitions for operations that are relevant to the current context. If the user is in sketch mode, only send sketch tools. If they're working with solids, send solid tools. Saves ~60% of tool tokens.
- **Tool groups**: "sketch_tools" (8 tools), "feature_tools" (6 tools), "query_tools" (5 tools). Send groups based on the current mode.

---

## 22. Geometric Validation Layer — The Safety Net

**The problem I underestimated**: The agent will generate operations that are geometrically invalid in ways neither the LLM nor a simple parameter check would catch.

**Examples of geometrically invalid operations that pass type checking**:

1. **Self-intersecting extrude**: Extrude a concave face along its normal. The resulting side walls cross each other. Mesh is non-manifold. The BREP kernel might reject this, but our current mesh layer won't — it'll create garbage geometry.

2. **Fillet larger than face**: Fillet with radius 10mm on a face that's 8mm wide. The fillet consumes the entire face, leaving zero-area or negative-area geometry.

3. **Sketch circle overlapping face boundary**: Circle of radius 5mm placed 3mm from the edge. Part of the circle extends beyond the face. The contour is open (clipped by face boundary), extrude fails.

4. **Degenerate thin walls**: After a cut, the remaining wall thickness is 0.01mm. Numerically valid but physically meaningless, and causes rendering artifacts (z-fighting between close faces).

5. **Coincident faces after boolean**: Two faces at exactly the same position (the result of a boolean where faces are coplanar). The renderer draws both, z-fighting.

**The fix: a validation layer between the agent and the kernel.**

```rust
pub fn validate_operation(engine: &Engine, op: &Operation) -> ValidationResult {
    match op {
        Operation::Extrude { face_id, depth, .. } => {
            let face = engine.get_face(*face_id)?;
            // Check face exists
            // Check depth > minimum threshold (0.01mm)
            // Check depth < maximum (10000mm)
            // Check extrude won't create self-intersection (convexity test)
            // Check result won't create thin walls (minimum thickness)
        }
        Operation::Fillet { edge_ids, radius, .. } => {
            for edge in edge_ids {
                let adj_faces = engine.adjacent_faces(*edge)?;
                let min_face_width = adj_faces.iter().map(|f| f.min_width()).min();
                if *radius > min_face_width * 0.5 {
                    return Err(ValidationError::FilletTooLarge { edge, radius, max: min_face_width * 0.5 });
                }
            }
        }
        // ... etc for each operation
    }
}
```

The validation layer:
- Runs BEFORE execution (not after)
- Returns descriptive errors the agent can understand and explain to the user
- Suggests fixes ("radius too large, maximum is 3.5mm for this edge")
- Is operation-specific (each operation has its own validation rules)
- Is fast (geometric queries, not full kernel operations)

**The agent gets validation errors as tool results** and can either auto-fix (reduce radius) or ask the user.

---

## 23. The Viewport Screenshot Approach — A Simpler Alternative

**A radical simplification I didn't consider**: Instead of complex highlight/label/camera protocols, what if the agent could simply **see a screenshot of the viewport**?

Modern multimodal LLMs (Claude) can process images. If the engine takes a screenshot and sends it to the agent as part of the context:

```
Agent Controller → Claude API:
  [image: current viewport screenshot, 800x600]
  "The user asked to add bolt holes. Here's the current model state."
```

**Advantages**:
- Agent can SEE what the user sees — no need to serialize face/edge data
- Agent can point at things: "the flat surface on top" (from the screenshot, not from IDs)
- Dramatically reduces context token count for model state
- Works with ANY geometry, including meshes without a feature tree

**Disadvantages**:
- The agent can't get precise coordinates from a screenshot
- The agent can't reference specific face IDs (it sees pixels, not topology)
- Resolution-dependent: small faces might be invisible
- Screenshot must be taken from a good angle (who picks the angle?)
- Adds image tokens to every API call (~1000 tokens for a 800x600 image)

**Hybrid approach (best of both worlds)**:
- Send BOTH: a compact text description (feature tree + active body summary) AND a viewport screenshot
- The text gives the agent precise data for tool calls (face IDs, dimensions)
- The screenshot gives the agent spatial understanding (where things are visually)
- For "show the user" steps, the agent controls the camera, and the ENGINE takes the screenshot to verify the view looks right before asking the user

**This is worth prototyping early.** It might make the whole "context building" problem dramatically simpler.

---

## 24. The Conversation Memory Problem

**The problem**: LLMs are stateless. Every API call includes the full conversation. But multi-turn CAD sessions can go 50+ turns.

**At 50 turns:**
- User messages: ~5,000 tokens
- Agent messages: ~10,000 tokens
- Tool calls + results: ~15,000 tokens (30 tool calls × 500 tokens average)
- Model state per turn: ~2,000 tokens
- System prompt + tools: ~5,000 tokens

**Total: ~37,000 tokens per API call** — and growing with every turn.

**Mitigation strategies:**

1. **Conversation pruning**: After each major milestone (feature completed), summarize the conversation so far and replace the history with the summary. "Turns 1-15: created base bracket, 80x40x3mm, with 4 M6 holes" replaces 15 turns of back-and-forth.

2. **Tool result pruning**: Old tool results are mostly irrelevant. After the agent has used a query result, compress it: `{faces: [12,15,18,21], ...}` → `"queried 4 faces (details in turn 7)"`. The agent can re-query if needed.

3. **Checkpoint system**: At natural breakpoints (feature complete, user says "looks good"), save a checkpoint: full model state + feature tree + conversation summary. If the conversation gets too long, restart from the latest checkpoint.

4. **Split long tasks**: If the user asks for a complex part (20+ features), the agent should break it into "sessions": "Let me first create the base body. Once that's confirmed, we'll add the mounting features in a new conversation."

---

## Summary of New Issues (16-24)

| # | Issue | Severity | When It Hits |
|---|-------|----------|-------------|
| 16 | Operation parameter collection is too slow | High | Day 1 of agent usage |
| 17 | Hybrid agent-manual interaction for fine-tuning | High | Every "adjust" command |
| 18a | Highlight bleeding at triangle seams | Low | Visible but cosmetic |
| 18b | Ghost preview depth for cuts/booleans | Medium | Cut and boolean previews |
| 18c | MSAA + alpha blending fringing | Low | Cosmetic, most CAD tools have it |
| 18d | Dimmed geometry back-face visibility | Medium | Every "focus on this" command |
| 18e | Floating label depth + overlap | Medium | Multi-entity labeling |
| 19 | No UI state machine for agent states | High | Day 1 — user won't know what's happening |
| 20 | No capability discovery mechanism | Medium | User asks for unsupported feature |
| 21a | Protocol architecture was wrong | High | Implementation time — must be right |
| 21b | GX vs tool_use conflict | High | Implementation time |
| 21c | Tool definition token overhead | Medium | Every API call costs more |
| 22 | No geometric validation before execution | High | First invalid operation |
| 23 | Viewport screenshot as context (opportunity) | — | Could simplify everything |
| 24 | Conversation memory blowup | Medium | After ~20 turns |

---

---

# PART III: FILLING THE CRACKS — The Lived Experience

*Everything above analyzes the architecture. Everything below analyzes what actually happens when a human sits down and uses this.*

---

## 25. The Dead Air Problem — The 2-5 Second UX Death Zone

**This is the single most important UX problem not addressed in any document so far.**

Every time the user sends a message, the Claude API takes 2-5 seconds to respond. During those seconds, the user stares at a "Thinking..." indicator. They can't do anything productive. The 3D viewport sits there, static.

**Why this kills the experience**: CAD is spatial and tactile. Users expect instant feedback. When they click "Extrude" in SolidWorks, the preview appears in <50ms. When they type "extrude this 5mm" to our agent, they wait **100x longer** just for the agent to ACKNOWLEDGE the command, then another 2-5 seconds for each follow-up.

**A 5-step operation with 3 confirmations:**
```
0:00  User: "Make a bracket with holes"
0:00  [Thinking...]           ← user stares at nothing for 3 seconds
0:03  Agent: "I'll create an 80x40mm rectangle. OK?"
0:03  User: "Yes"
0:03  [Thinking...]           ← another 3 seconds
0:06  Agent: [executes sketch + extrude, camera moves]
0:07  Agent: "Base done. Where should I put the holes?"
0:07  User: "8mm from edges"
0:07  [Thinking...]           ← another 3 seconds
0:10  Agent: [places circles, shows preview]
0:10  Agent: "Like this?"
0:10  User: "Yes"
0:10  [Thinking...]           ← another 3 seconds
0:13  Agent: [cut-extrude]
0:13  "Done!"

Total: 13 seconds. User was WAITING for 12 of them.
```

In SolidWorks, a skilled user does the same thing in 15-20 seconds — but they're ACTIVE the whole time. Click-drag-type-click-drag. Our agent makes the user a SPECTATOR for 90% of the time.

**Mitigations (must use ALL of these):**

**a) Speculative execution**: While waiting for user confirmation, start preparing the NEXT step. Pre-compute the query results, pre-tessellate the preview geometry. When the user says "yes", the visual update appears instantly because it was ready.

**b) Streaming partial actions**: Claude supports streaming. As the response streams in, start executing AS SOON as the first tool call is complete (don't wait for the full response). If the agent calls `query_faces` then `extrude`, start the query as soon as it appears in the stream.

**c) Fill the dead air with visual activity**: During "Thinking...", don't show a static viewport. Show the agent "examining" the model — a subtle scanning highlight that moves across faces (like a laser scan), or the camera slowly orbiting to a better angle. This gives the impression of activity even though the GPU is idle.

**d) Batch simple confirmations**: Instead of "OK?" after every step, the agent should execute a sequence of obvious steps without asking, and only pause at DECISION POINTS. "I created the base and extruded it. Now, where do you want the holes?" — that's one pause instead of three.

**e) Local inference for simple decisions**: Use a tiny local model (or even hardcoded heuristics) for simple decisions: "user said yes" → confirm. "user said a number" → use as parameter. Only call Claude for actual reasoning. This eliminates ~40% of API round-trips.

---

## 26. The "Relative To What" Problem — Spatial Reference Ambiguity

Every spatial word the user says is ambiguous in 3D:

| User says | Could mean |
|-----------|-----------|
| "the top" | +Y face? +Z face? The face currently visible at the top of the screen? The face with the highest vertex? |
| "the left side" | -X face? The face on the left of the screen? Left relative to the front face? |
| "the back" | -Z face? The face facing away from the camera? The face opposite to "front"? |
| "the other side" | Opposite face of what? The body? The last selected face? The sketch plane? |
| "here" | Where the mouse is? Where the agent last highlighted? The origin? |
| "bigger" | Scale up? Extrude more? Increase a dimension? Which dimension? |
| "move it up" | +Y? +Z? Screen up? Along face normal? |
| "rotate it" | Around which axis? By how much? Relative to what? |

**This is not solvable by the LLM.** The LLM doesn't know what "top" means without knowing the coordinate convention, the camera angle, and the user's mental model.

**The fix: a spatial reference resolver in the engine.**

```rust
pub enum SpatialRef {
    WorldDirection(Direction),   // +X, -Y, +Z, etc.
    CameraRelative(ScreenDir),  // screen-up, screen-left, toward-camera
    FaceRelative(FaceId, RelDir), // normal, tangent, opposite
    BodyRelative(BodyId, RelDir), // top_face, bottom_face, largest_face
    LastSelection,               // whatever was last selected
    MousePosition,               // current cursor world position
}

pub fn resolve_spatial_ref(
    engine: &Engine,
    camera: &Camera,
    description: &str,  // "the top", "left side", etc.
) -> Vec<(SpatialRef, f32)> {  // candidates with confidence
    // Returns ranked candidates, not a single answer
}
```

**The agent's workflow**:
1. User says "extrude the top face"
2. Agent calls `resolve_spatial("the top face")` → gets ranked candidates
3. If top candidate has high confidence (>0.8): use it, mention which face in response
4. If ambiguous: highlight top 2-3 candidates with labels, ask user to pick
5. Agent ALWAYS states which entity it chose: "I'll extrude the top face [this one, highlighted in orange]"

**Convention we must establish and show in the UI**:
- Our world is Y-up (like SolidWorks). "Top" = +Y. "Front" = +Z toward camera. "Right" = +X.
- Show XYZ axis colors persistently in the viewport corner (we have this: the gizmo).
- When the agent references a direction, it should say "+Z (front)" not just "front."

---

## 27. Context Drift — The Silent Killer of Long Sessions

**The problem**: Over a 30-turn conversation, the agent's understanding of the model slowly diverges from reality. Not catastrophically — subtly. And subtle drift is worse than a crash because nobody notices until it's too late.

**How drift happens**:

1. **Rounding in descriptions**: The agent says "the box is 80x40x3mm." Internally it's 79.998×40.002×3.001mm (floating point). After 5 operations, these tiny errors accumulate. The agent says "I'll place a hole at x=72" but the actual face edge is at x=79.998, not x=80. The hole ends up 0.002mm from the edge instead of 8mm.

2. **Forgotten undo**: User presses Ctrl+Z during a pause. The agent's conversation history still says "extruded to 3mm" but the extrusion was undone. The model is back to a flat sketch, but the agent thinks there's a solid.

3. **Stale face IDs**: Agent cached "top face is ID 43" from turn 12. By turn 20, after 3 more operations, face 43 is now a side face. Agent references it without re-querying.

4. **Pruned conversation**: The conversation was pruned at turn 15 to save tokens. The summary says "4 bolt holes added." But the user modified one hole's position at turn 14. The pruned summary lost this detail.

5. **Screenshot lag**: The screenshot sent to the agent was taken before the last operation rendered. The agent sees stale geometry.

**Mitigations**:

**a) Canonical state snapshot every turn**: Before every API call, the Agent Controller generates a fresh, canonical model state summary. This is NOT from conversation history — it's from the ACTUAL engine state. The agent always sees ground truth, not its own memory.

```
[CURRENT MODEL STATE — this overrides any previous description]
Bodies: 1
Body1: 6 faces, bbox=(0,0,0)→(80,40,3)
  top: face_id=43, z=3, 80×40mm
  bot: face_id=44, z=0, 80×40mm
  ...
Selection: face 43
Camera: looking at (40,20,1.5) from iso-front
Feature tree: Sketch1 → Extrude1(3mm)
```

This costs ~200-500 tokens but prevents ALL drift. The agent never relies on what it "remembers" — it always reads current state.

**b) State hash verification**: Compute a hash of the model state. Include it in every turn. If the hash changes unexpectedly (user did something), the Agent Controller adds a note: "WARNING: Model state changed since last turn. User may have performed manual operations. Re-query before proceeding."

**c) Explicit undo tracking**: The Agent Controller tracks the undo stack depth. If it decreases between turns, inject: "NOTE: User pressed undo N times. The last N operations were reverted. Current state: [fresh snapshot]."

---

## 28. The First 5 Minutes — Onboarding or Death

**Scenario**: New user opens GraniteX. Empty viewport. Chat panel on the right. What happens?

**Bad first 5 minutes** (what NOT to do):
```
[Empty viewport, blank chat panel]
User: "hello"
[3 seconds of thinking...]
Agent: "Hello! I'm GraniteX's AI modeling assistant. I can help you create
        3D parts by describing what you want. What would you like to build?"
User: "a box"
[3 seconds of thinking...]
Agent: "I'd be happy to help! What dimensions would you like for the box?
        Please specify width, height, and depth."
User: "uh... 50mm?"
[3 seconds of thinking...]
Agent: "Is that the width? What about the height and depth?"
```

**15 seconds in, user has seen NOTHING in the viewport.** They're playing 20 questions with a chatbot. This is death.

**Good first 5 minutes** (what TO do):
```
[Empty viewport with a subtle welcome overlay:
 "Type what you want to build, or click a template below"
 [Box] [Cylinder] [Bracket] [Import file...]]

User: "a box"
[1 second — local heuristic recognizes "box", no API call needed]
[A 50×50×50mm box appears in the viewport with a glowing outline]
Agent: "Here's a 50mm cube. Drag the handles to resize, or tell me
        the dimensions you want."
[The box has 3 drag handles (XYZ) visible immediately]
User drags the X handle to ~80mm
Agent: (no API call — engine tracks the drag locally)
[Box updates in real-time to 80×50×50mm]
User: "make it thinner, like 3mm"
[2 second API call — but the viewport already shows the box, user isn't staring at nothing]
Agent: [sets Z dimension to 3mm, camera adjusts to show the thin plate]
        "80×50×3mm plate. What's next?"
```

**Key differences**:
1. Immediate visual feedback — even before the first API call
2. Templates for common starting shapes — no cold start from empty viewport
3. Drag handles for obvious parameters — no need to type dimensions
4. Local processing for simple commands — "a box" doesn't need an LLM
5. The viewport has content within 1 second

**Implementation requirements**:
- A set of template shapes (box, cylinder, sphere, plate, bracket) with parameterized defaults
- Simple keyword matching for common shapes (runs locally, no API)
- Interactive resize handles on all template shapes
- The agent "takes over" only when the task requires reasoning, not for simple parameter setting

---

## 29. The Disagreement Problem — Replanning Under Conflict

**Scenario**:
```
Agent: [has a 7-step plan, just finished step 3]
        "I'll add 4 bolt holes next, M6 size."
User: "No, I don't want round holes. I want slots."
```

The agent has to:
1. Understand that "slots" replaces "bolt holes" in the plan
2. Abort steps 4-7 (bolt holes)
3. Generate new steps 4'-7' (slots)
4. BUT: the engine state after step 3 is correct — don't undo steps 1-3
5. The feature tree should show: Sketch1 → Extrude1 → Extrude2 → [Slots] not [Holes]

**The hard part**: The agent's ENTIRE plan was generated in one API call. Steps 4-7 were "already decided." There's no way to partially modify a past LLM response. The agent must make a NEW API call with updated context:

```
[System: User changed requirements. Steps 1-3 complete.
 Original plan steps 4-7 (bolt holes) CANCELLED.
 User wants: slots instead of round holes.
 Current model state: [fresh snapshot]
 Generate new steps from here.]
```

**But what if the disagreement is about a COMPLETED step?**

```
Agent: [finished step 3: extruded to 3mm]
User: "That's too thick, make it 1.5mm"
```

Now the agent must:
1. Undo step 3
2. Re-execute step 3 with new parameter (1.5mm)
3. Continue the plan from there

But if steps 4-7 were dependent on step 3's face IDs (they will be — the top face ID changes after re-extrude), the entire remaining plan must be regenerated.

**The rule**: **After ANY retroactive change, discard all future plan steps and regenerate.** Don't try to patch the plan. The LLM is cheap; stale plans are expensive.

**UI for disagreement**:
- The plan panel should have an "edit" affordance on each step
- Clicking a completed step should offer: "Modify this step (will redo all later steps)"
- This is exactly the SolidWorks behavior: edit Feature 3 → Features 4-10 rebuild automatically

---

## 30. Network Failure Mid-Plan

**Scenario**: Agent is on step 4 of 7. The Claude API call for step 5 times out (network error, rate limit, server overload).

**State**: Steps 1-4 are committed to the model. The agent's conversation history exists in the Agent Controller's memory. But the API call failed — we have no response.

**What happens now?**

**Bad**: "Error: API call failed. All progress lost." ← the user will uninstall GraniteX

**Good**:
1. The model state (steps 1-4) is SAFE — it's in the engine and on the undo stack
2. The Agent Controller saves: plan snapshot + conversation history + step progress
3. UI shows: "Connection lost. Your model is safe. [Retry] [Continue manually] [Save and quit]"
4. On retry: reconstruct the conversation, include current model state, tell the agent "you were on step 4, continue from step 5"
5. On manual: user takes over, model is intact, agent becomes idle

**The critical invariant**: **The model state must NEVER depend on the API connection.** All geometry lives in the engine. The API is for intelligence, not for state. If the API goes away, the model is still there and fully functional.

**Related failures**:
- **API slow (10+ seconds)**: Show a "Still thinking... (8s)" timer. Offer a "Cancel" button that doesn't undo any geometry — just stops waiting for the agent's next instruction.
- **Rate limited**: Queue the request. Show "Waiting for API availability (queued)." Don't spam retries.
- **API response is garbage** (malformed JSON, hallucinated tool names): Log the raw response for debugging. Tell the agent: "Your last response was unparseable. Try again. Here's the current model state." One retry; if it fails again, drop to manual mode.

---

## 31. Units — The Most Boring Bug That Will Bite Hardest

**Our engine works in meters internally.** The user thinks in millimeters. The agent speaks millimeters. The tool definitions say millimeters. Somewhere, someone will forget to convert.

**The bug that WILL happen**:
```
User: "extrude 5mm"
Agent: calls extrude(depth=5)     ← forgot to specify units
Engine: extrudes 5 METERS          ← interprets as internal units
Result: the model is now 5000mm tall. Viewport shows... nothing
        (camera auto-fit zooms so far out the original body is a pixel)
```

**Or the inverse**:
```
User: "extrude 5mm"
Agent: calls extrude(depth=0.005)  ← correctly converted
Engine: extrudes 0.005 meters (5mm)
Agent reports: "Extruded 0.005m"   ← confusing to user
User: "That's not what I said! I said 5!"
```

**The fix: the API layer must have EXPLICIT units, and they must be in the user's preferred unit system.**

```rust
pub struct ExtrudeParams {
    pub face_id: FaceId,
    pub depth: Length,  // NOT f64. Length is unit-aware.
    pub direction: Direction,
}

pub struct Length {
    pub value: f64,
    pub unit: LengthUnit,  // mm, cm, m, in, ft
}
```

The tool definition:
```json
{
  "depth": {
    "type": "number",
    "description": "Extrusion depth in millimeters (mm)"
  }
}
```

The Agent Controller converts to internal units:
```rust
let internal_depth = params.depth.to_meters(); // 5mm → 0.005m
```

**Rules**:
1. The LLM ALWAYS speaks in mm (matches the tool definition)
2. The Agent Controller ALWAYS converts to meters before calling the engine
3. The engine ALWAYS works in meters internally
4. The UI ALWAYS displays in the user's preferred unit (mm by default)
5. No function in the entire codebase accepts a bare `f64` for a dimension — always `Length`

---

## 32. The "This" Problem — Anaphora Resolution in 3D

NLP anaphora resolution ("this", "that", "it", "the one") is hard in text. In 3D, it's a nightmare.

**Examples**:
```
User: "Extrude this"
→ "this" = the selected face? the hovered face? the sketch?

User: "Make it bigger"
→ "it" = the last created body? the selected face? the whole model?

User: "Do the same thing on the other side"
→ "the same thing" = the last operation? the last N operations?
→ "the other side" = opposite face? mirror across which plane?

User: "Not that one, the one behind it"
→ "that one" = the highlighted face? the one the agent just mentioned?
→ "behind it" = behind in screen space? in world Z? relative to the face normal?
```

**The resolution strategy (ordered by priority)**:

1. **Explicit selection wins**: If the user has a face selected (blue highlight), "this" = the selected face. Always.
2. **Agent's last reference**: If the agent just highlighted face 43, "this" probably means face 43.
3. **Mouse position**: If nothing is selected but the cursor is over a face, "this" = the hovered face.
4. **Recency in conversation**: "Make it bigger" → "it" is the most recently mentioned entity.
5. **Disambiguate**: If none of the above are clear, ask. But ask SMART: highlight the top 2 candidates, not "what do you mean?"

**The engine needs a "reference stack"**: A short list of recently referenced entities, maintained by both the agent and the UI:
```rust
pub struct ReferenceContext {
    pub selected: Vec<EntityRef>,      // user's current selection
    pub agent_highlighted: Vec<EntityRef>, // agent's last highlight
    pub hovered: Option<EntityRef>,    // face under cursor
    pub recently_created: Vec<EntityRef>,  // entities from last operation
    pub recently_mentioned: Vec<EntityRef>, // entities the agent talked about
}
```

This context is sent to the LLM every turn. The agent can then resolve "this" without a round-trip.

---

## 33. Performance Cliffs — When the Agent Creates Unrenderable Geometry

**Scenario**: Agent performs a boolean subtract between two complex bodies. The result has 200k triangles (dense intersection curves produce many small triangles). Framerate drops from 60fps to 8fps. The viewport becomes a slideshow.

**Why this happens with the agent specifically**: A human user feels the performance degradation incrementally — they'd stop adding detail before it gets this bad. The agent doesn't feel frame rate. It'll happily add a 1000-segment circular pattern because the LLM doesn't know that 1000 circles × 64 segments = 64,000 triangles per circle = 64 million triangles total.

**Failure cascade**:
1. Agent creates complex geometry
2. Framerate drops to <15fps
3. Camera animation stutters (the "show the user" step looks broken)
4. Picking becomes unreliable (raycast O(n) at 64M triangles = seconds per pick)
5. The user's experience degrades not just for this step but for ALL subsequent interaction
6. Even Ctrl+Z is slow (restoring the previous mesh from snapshot takes time)

**Mitigations**:

**a) Triangle budget**: Every operation must report the estimated triangle count BEFORE executing. The validation layer (see #22) checks: "will this operation exceed the triangle budget?"
```rust
pub struct PerformanceBudget {
    pub max_total_triangles: usize,    // e.g., 500_000
    pub max_per_operation: usize,      // e.g., 50_000
    pub warning_threshold: usize,      // e.g., 200_000 — agent gets a warning
}
```

**b) LOD for agent previews**: During the agent's "show" step, use a simplified mesh (LOD) if the full mesh is too complex. The full mesh is used for the final render after confirmation.

**c) Agent awareness of performance**: Include frame time in the model state:
```
[Performance: 16ms/frame (60fps), 487k triangles]
```
If frame time exceeds 33ms (30fps), the agent gets a warning: "Performance is degraded. Consider simplifying before adding more geometry."

**d) Operation complexity estimation in tool definitions**:
```json
{
  "name": "pattern_circular",
  "description": "...",
  "performance_note": "Creates N copies of the body. Each copy adds ~same triangle count as original. Use count<20 to maintain interactive framerate."
}
```

---

## 34. The Agent Watching the User — The Creepy Observer Problem

When the agent is idle and the user is working manually, what does the agent know?

**Option A: Agent is blind to manual work.** User works manually, agent's context gets stale. When user re-engages the agent, it doesn't know what happened. User has to explain: "I added a shelf on the right side."

**Option B: Agent sees everything.** Every mouse click, every operation, every undo — all logged and sent to the agent on the next turn. This means the agent always has full context. But:
- If the user did 50 manual operations between agent turns, that's 50 operations to summarize (token cost)
- It feels like the tool is watching you. "I noticed you struggled with that fillet for 7 attempts." — creepy and unwelcome

**Option C (correct): Agent sees the RESULT, not the PROCESS.** When the user re-engages the agent, the Agent Controller:
1. Takes a fresh model state snapshot (current, not historical)
2. Diffs it against the last snapshot the agent saw
3. Summarizes the diff: "Since we last talked: 2 new bodies added, 1 fillet applied, camera moved to top view"
4. The agent knows WHAT changed but not HOW (not every intermediate step)

This preserves context without surveillance. The agent says: "I see you added some features. Should I continue from here?" — helpful, not creepy.

---

## 35. The Partial Undo Catastrophe

**Scenario**: Agent executed a 5-step plan. User says "actually, undo step 2 but keep steps 3-5."

**Why this is impossible (and we must handle it gracefully)**:

Steps 3-5 were built ON TOP of step 2. Step 3 selected a face that step 2 created. Step 4 extruded that face. Step 5 filleted an edge from step 4. If you remove step 2, the face from step 3 doesn't exist. Steps 3-5 are all invalid.

**This is not an agent problem — it's a parametric modeling problem.** SolidWorks has it too: delete Feature 2, and Features 3-10 that depend on it show "rebuild errors."

**What the agent should do**:
1. Identify dependencies: "Steps 3-5 all depend on step 2. I can't undo just step 2."
2. Offer alternatives:
   - "I can undo steps 2-5 (back to step 1) and redo 3-5 differently"
   - "I can modify step 2's parameters instead of undoing it"
   - "I can undo everything and start fresh"
3. If the user insists: undo steps 5, 4, 3, 2 (in reverse order), then replay steps 3-5 on the new base geometry. Steps 3-5 will have different face IDs and may fail — the agent must handle this.

**Implementation**: The undo stack needs dependency metadata:
```rust
pub struct UndoEntry {
    pub operation: Operation,
    pub created_entities: Vec<EntityId>,
    pub consumed_entities: Vec<EntityId>,  // entities from previous operations that this one uses
    pub depends_on: Vec<usize>,  // indices of undo entries this depends on
}
```

This lets us compute: "if I undo entry 2, entries 3-5 are also invalidated."

---

## 36. When to Ask vs When to Assume — The Autonomy Calibration

**This determines whether the agent feels helpful or annoying.**

**Too many questions** (annoying):
```
User: "Make a bracket"
Agent: "What width?"
User: "80mm"
Agent: "What height?"
User: "40mm"
Agent: "What thickness?"
User: "3mm"
Agent: "What material?"
User: "I DON'T CARE JUST MAKE IT"
```

**Too few questions** (wrong results):
```
User: "Make a bracket"
Agent: [silently creates a 100x100x10mm L-bracket]
User: "That's not what I wanted at all"
```

**The calibration framework**:

1. **Assume when**: The parameter has a reasonable default (thickness=3mm for a bracket), the user didn't mention it, and getting it wrong is cheap to fix (just re-extrude).

2. **Ask when**: The parameter fundamentally changes the design (L-bracket vs flat bracket vs angle bracket), there's no reasonable default, or getting it wrong means undoing multiple steps.

3. **Ask ONCE with defaults, not sequentially**: "I'll create an 80×40×3mm flat bracket. Change anything?" — one question, one answer, all defaults shown.

4. **Learn from corrections**: If the user says "no, always 2mm thick", the agent remembers (within session) and uses 2mm as default for subsequent operations.

5. **Escalate, don't interrogate**: Instead of asking 5 separate questions, show a parameter panel with ALL defaults filled in. User adjusts what they want. Zero questions if defaults are OK.

**Concrete rules for the agent's system prompt**:
```
AUTONOMY RULES:
- If the user gives exact numbers, use them exactly
- If the user gives vague intent ("a bracket"), assume reasonable defaults and SHOW the result
- Never ask more than 1 question per turn
- When in doubt, DO something (with preview) rather than ASK
- After showing a result, say what you assumed: "I used 3mm thickness. Change?"
- If the user says "just do it" or "I don't care", maximize autonomy for the rest of this task
```

---

## 37. Screenshot Timing — The Subtle Rendering Race Condition

**The problem**: The Agent Controller takes a screenshot to send to the LLM. But:

1. Agent executes `extrude(face=42, depth=5)`
2. Engine computes new mesh (CPU): 5ms
3. New mesh uploaded to GPU: 2ms
4. Renderer draws frame with new mesh: 16ms
5. **Screenshot taken**: needs to happen AFTER step 4

If the screenshot is taken between steps 2 and 4, the LLM sees the OLD geometry — before the extrude. The agent thinks it succeeded but the screenshot contradicts the tool result.

**Worse scenario with camera animation**:
1. Agent calls `camera_look_at(face=43)`
2. Camera starts animating (500ms transition)
3. Screenshot taken at frame 3 of 30 → camera is mid-animation
4. LLM sees a blurry, mid-rotation view → thinks the model looks wrong

**The fix**: The Agent Controller must wait for a **stable frame** before capturing:

```rust
pub async fn capture_stable_screenshot(&self) -> Screenshot {
    // Wait for all pending mesh uploads
    self.renderer.wait_for_uploads().await;
    // Wait for camera animation to complete
    self.camera.wait_for_idle().await;
    // Wait for next frame to render
    self.renderer.wait_for_frame().await;
    // Capture
    self.renderer.screenshot()
}
```

**This introduces latency**: 16ms (one frame) minimum, 500ms+ if camera is animating. The Agent Controller must factor this into its timing — don't capture immediately after an operation.

---

## 38. History Linking — Feature Tree ↔ Conversation

**The problem**: Two independent records of what happened:
1. **Feature tree**: Sketch1 → Extrude1(5mm) → Sketch2 → Cut1(through)
2. **Conversation**: "Make a bracket" → "Extruded 5mm" → "Added holes" → "Cut through-all"

These must stay linked. If the user clicks "Extrude1" in the feature tree, the chat should scroll to the turn where it was created. If the user asks "why is this 5mm?", the agent should find the conversation context where 5mm was decided.

**Why this matters**: Without linking, the user can't understand WHY a feature has certain parameters. The feature tree says "5mm" but the conversation says the user originally asked for "about 5" and the agent rounded. That context is lost without linking.

**Implementation**:
```rust
pub struct FeatureRecord {
    pub feature_id: FeatureId,
    pub operation: Operation,
    pub conversation_turn: usize,   // which turn created this
    pub user_intent: String,        // "bracket base" (from agent's understanding)
    pub agent_assumptions: Vec<String>, // "assumed 3mm thickness", "used rectangular shape"
}
```

The chat panel and feature tree share these records. Click a feature → chat highlights the relevant turn. Click a chat message → feature tree highlights the feature it created.

---

## 39. The Empty Canvas Problem — Conjuring From Nothing

**When no geometry exists and the user says "make a bracket":**

The agent must:
1. Decide on a bracket TYPE (L-bracket? flat with holes? angle bracket? U-channel?)
2. Decide on DIMENSIONS (based on what? There's no reference geometry)
3. Decide on ORIENTATION (which plane? which direction is "up"?)
4. Decide on ORIGIN (centered on origin? corner at origin?)

**The LLM's problem**: Without any existing geometry to reference, the agent is designing from pure imagination. It has to make 20+ decisions with zero input. Most will be wrong.

**The fix: progressive disclosure from generic to specific.**

1. Agent starts with the MOST GENERIC interpretation: "I'll start with a flat plate. Here:" [shows a default 80×40×3mm plate]
2. This gives the user something CONCRETE to react to: "No, more like an L-shape" or "Yes but thicker"
3. Each correction narrows the design space
4. Within 2-3 turns, the agent has a clear picture

**The key insight**: **Showing a wrong thing is better than asking the right question.** The user can always say "no, more like THIS" (pointing at the screen), but they can't easily answer "what kind of bracket do you want?" in the abstract.

**Templates help enormously here**:
```
Agent: "What kind of bracket? I can start with one of these:"
[Shows 4 template thumbnails: flat plate, L-bracket, U-channel, angle bracket]
"Or describe something custom."
```

Visual templates convert an open-ended question into a multiple-choice question — dramatically easier for the user.

---

## 40. Dimensional Tolerance and Floating Point Display

**The engine works in f64.** After multiple operations, a dimension that started as exactly 80mm might be 79.99999999999997mm internally.

**Problems this causes**:

1. **Inconsistent display**: Agent says "80mm" but the measurement tool says "79.9999mm". User panics.
2. **Comparison failures**: `if face_width == 80.0` → false. Constraints break.
3. **Accumulation through chains**: Offset 40mm from a face that was offset 40mm from origin. Should be at 80mm. Actually at 79.99999999998mm. Over 10 operations, this can drift to 79.998mm — visible in measurements.

**The fix: display rounding + internal tolerance.**

```rust
pub fn display_dimension(value_meters: f64, unit: LengthUnit) -> String {
    let mm = value_meters * 1000.0;
    if (mm - mm.round()).abs() < 1e-6 {
        format!("{:.0}mm", mm.round())  // 79.9999997 → "80mm"
    } else if (mm * 10.0 - (mm * 10.0).round()).abs() < 1e-5 {
        format!("{:.1}mm", mm)  // 79.95 → "80.0mm"
    } else {
        format!("{:.2}mm", mm)  // 79.87 → "79.87mm"
    }
}
```

And for comparison:
```rust
pub fn dimensions_equal(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9  // 1 nanometer tolerance
}
```

The agent should also SNAP dimensions to clean values when generating operations: if the agent computes a position as 7.99998mm, snap it to 8.0mm before creating the sketch entity. The LLM naturally generates clean numbers; the engine should preserve them.

---

## 41. API Cost Awareness

**Every agent turn costs money.** A complex part might require 30+ turns. At ~$0.01-0.05 per turn (input + output tokens), that's $0.30-1.50 per part. Not expensive, but users are sensitive to AI costs.

**The user should see:**
1. A running cost counter in the status bar (subtle, not alarming): "~$0.12 this session"
2. Per-operation cost: the agent shouldn't make 5 API calls for a simple "yes" confirmation
3. Estimated cost BEFORE a complex plan: "This will take about 8 steps (~$0.15). Proceed?"

**The agent should be cost-efficient:**
- Batch obvious steps (don't ask for permission after every line in a sketch)
- Use the minimum context per call (pruned history, compact model state)
- Cache tool definitions (don't re-describe every tool when they haven't changed — Claude API supports this with caching)
- Use streaming to avoid redundant round-trips
- For simple operations ("undo", "change color", "zoom in"), handle locally without an API call

---

## Revised Top 10 — The Real Threats (Ordered by Likelihood × Impact)

| Rank | Issue | Likelihood | Impact | Ref |
|------|-------|-----------|--------|-----|
| 1 | Dead air / latency kills UX | Certain | Fatal | #25 |
| 2 | Spatial reference ambiguity ("the top") | Certain | High | #26 |
| 3 | Context drift over long sessions | Very likely | High | #27 |
| 4 | Topological naming (face IDs break) | Certain | High | #1 |
| 5 | LLM can't compute coordinates | Certain | High | #5 |
| 6 | Unit conversion bugs | Very likely | Medium | #31 |
| 7 | Onboarding failure (empty canvas) | Likely | High | #28, #39 |
| 8 | Asking too much / too little | Likely | Medium | #36 |
| 9 | Network failure mid-plan | Likely | Medium | #30 |
| 10 | Performance cliff from complex geometry | Likely | Medium | #33 |

The original "Top 5" was architecture-focused. This revised Top 10 is experience-focused. Issue #1 changed from Topological Naming to **Dead Air** — because TNP can be worked around (re-query), but dead air drives users away before they ever hit a TNP bug.

---

*Updated 2026-03-27. Three layers of analysis: AGENT_VISION.md (what we want), AGENT_CRITIQUE Part I-II (architecture failures), Part III (lived experience failures). Read all three.*
