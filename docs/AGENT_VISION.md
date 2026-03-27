# GraniteX -- AI Agent Vision

**The North Star of the Project**

Last updated: 2026-03-27 (Session 12)

---

## Executive Summary

GraniteX is not building a CAD tool with an AI sidebar. GraniteX is building a **conversational CAD environment** where an AI agent is the primary operator of the modeling engine, and the human is the decision-maker.

The central insight is this: **the agent does not need the user to manually create planes, place points, draw sketches, or sequence features.** The agent can *reason* through the entire CAD workflow itself -- procedurally decomposing a high-level intent ("make a bracket with two mounting holes") into the exact sequence of operations a CAD engineer would perform (select plane, sketch rectangle, extrude, select top face, sketch circles, cut-extrude). It then executes those operations programmatically through an Operation API, using the 3D viewport as a shared visual canvas to communicate with the human at every step.

The human's role shifts from *operator* to *director*. They describe what they want in natural language, watch the agent build it step by step, and intervene only to confirm, adjust, or redirect. Every construction plane, every sketch entity, every feature parameter -- the agent handles it all. The user never needs to know which plane to sketch on or what order to apply features. The agent knows.

This is achievable because:
1. **LLMs can reason procedurally.** Given knowledge of CAD operations and their sequencing rules, an LLM can decompose "make a bracket" into a valid feature sequence -- the same way it can decompose "build a web app" into file creation, routing, database setup, etc.
2. **CAD operations are deterministic.** Each operation is a typed function with known inputs and outputs. There is no ambiguity in execution -- only in intent, which the human resolves.
3. **The 3D viewport is a communication medium.** Camera control, geometry highlighting, and ghost previews give the agent a visual language for pointing at things in 3D space, eliminating the need for the user to interpret textual descriptions of geometry.
4. **Human-in-the-loop eliminates the hallucination problem.** The agent never commits geometry the user hasn't seen and approved. Every step is visible, every dimension is adjustable, every decision is reversible.

---

## 1. The Core Thesis: Agent as CAD Engineer

### 1.1 What Changes

In traditional CAD, the user must learn and execute a complex procedural workflow:

```
User's mental model: "I want a bracket"
User must know:     Which plane to start on
                    How to enter sketch mode
                    What entities to draw (lines, arcs, dimensions)
                    How to constrain the sketch
                    When to exit sketch mode
                    Which feature to apply (extrude, cut, revolve)
                    What parameters to use
                    What order to apply features in
                    How to recover when something fails
```

This is the CAD skill gap -- the distance between *intent* ("I want a bracket") and *execution* (40+ discrete mouse/keyboard actions in the correct sequence). Professional CAD users spend years closing this gap.

The agent eliminates it:

```
User's mental model: "I want a bracket"
User says:          "Make a mounting bracket, 80x40mm, 3mm thick, two bolt holes"
Agent reasons:      base plate = sketch rect on XY -> extrude 3mm
                    holes = sketch circles on top face -> cut through-all
                    rib = sketch triangle on side face -> extrude
Agent executes:     Step by step, showing each result, asking for confirmation
User validates:     "Yes" / "Make it wider" / "Move the holes closer to the edge"
```

The agent handles every mechanical step -- plane selection, sketch creation, entity placement, feature application, parameter calculation. The user handles every *design decision* -- shape, dimensions, placement, aesthetics.

### 1.2 Why Procedural Reasoning is the Key

The agent does not generate a finished 3D model from a text description. That approach (text-to-mesh, text-to-STL) fails for CAD because:

- **Ambiguity**: "a bracket" has infinite valid interpretations. One-shot generation forces the AI to guess.
- **Precision**: CAD requires exact dimensions. LLMs cannot reliably produce spatially correct geometry without iterative feedback.
- **Trust**: Engineers must understand and validate each feature, not just the final shape.
- **Editability**: A generated mesh is a dead end. A parametric feature tree is editable forever.

Instead, the agent **reasons procedurally** -- it plans a sequence of operations the way a CAD engineer would think through the problem:

```
Agent's internal reasoning:
"To make a mounting bracket with two bolt holes, I need:
 1. A base plate: sketch a rectangle on XY, extrude to 3mm
 2. Two mounting holes: sketch two circles on the top face, cut through-all
    - Hole diameter: M6 clearance = 6.4mm
    - Hole placement: centered vertically, 15mm from each end
 3. Optional: fillet the outer edges for a cleaner look

Let me start with step 1 and show the user before proceeding."
```

Each step in this plan maps to a concrete API call. The agent generates the plan, executes it one step at a time, and uses visual feedback to keep the user informed.

### 1.3 The Human-in-the-Loop Contract

The agent operates under a strict contract with the user:

1. **Show before committing.** Every operation is previewed as ghost geometry before it becomes permanent. The user sees what will happen.
2. **Confirm at decision points.** After each meaningful step (not every micro-operation), the agent pauses for user approval. "Does this base shape look right?"
3. **Accept corrections gracefully.** "Make it wider" does not restart from scratch. The agent adjusts the relevant parameter and re-shows.
4. **Never surprise.** Camera moves, highlights, and labels ensure the user always knows what the agent is referring to and what it is about to do.
5. **Always reversible.** Every agent action goes on the undo stack. "Undo" takes back the last step, not the whole plan.

This is not AI generating things autonomously. This is AI and human collaborating through a shared visual workspace, with the AI handling mechanical complexity and the human handling design intent.

---

## 2. Architecture: Agent <-> Engine Protocol

### 2.1 The Two-Client Model

The fundamental architectural principle: **the UI and the agent are both clients of the same Operation API.** They call the same functions. They produce the same results. The engine does not know or care which client initiated an operation.

```
                    +--------------------------------------------------+
                    |             Operation API Layer                    |
                    |  Typed Rust functions: extrude, cut, fillet,       |
                    |  sketch_rect, camera_look_at, highlight ...        |
                    |  Every function: params -> Result<Output, Error>   |
                    +----------+--------------------------+-------------+
                               |                          |
                               v                          v
                    +------------------+       +-----------------------+
                    |   UI Client      |       |   Agent Client        |
                    |                  |       |                       |
                    |  Mouse/keyboard  |       |  LLM -> operation     |
                    |  -> API calls    |       |  calls via protocol   |
                    |                  |       |                       |
                    |  Direct, instant |       |  Reasoned, step-by-   |
                    |                  |       |  step with previews   |
                    +------------------+       +-----------------------+
```

This architecture has a critical consequence: **every feature we build for manual use is automatically available to the agent.** Sketch mode, extrude preview, face highlighting, camera animation -- all of these serve double duty. The manual UI uses them for direct manipulation; the agent uses them for visual communication.

### 2.2 The Operation API

Every modeling operation is a standalone, typed, serializable function:

```rust
pub struct ExtrudeParams {
    pub face_id: FaceId,
    pub direction: ExtrudeDirection,  // Normal, Custom(Vec3), BothSides
    pub depth: f64,
    pub draft_angle: Option<f64>,
}

pub struct SketchRectParams {
    pub plane: PlaneRef,              // FaceId, DefaultPlane, or CustomPlane
    pub origin: Point2D,              // sketch-plane coordinates
    pub width: f64,
    pub height: f64,
    pub centered: bool,
}

pub struct FilletParams {
    pub edge_ids: Vec<EdgeId>,
    pub radius: f64,
}

pub struct CameraCommand {
    pub target: CameraTarget,         // FitAll, LookAtFace(FaceId), LookAtPoint(Vec3)
    pub orientation: Option<ViewOrientation>,
    pub transition: TransitionType,   // Instant, Smooth(duration)
}

pub struct HighlightCommand {
    pub entities: Vec<EntityRef>,     // faces, edges, vertices
    pub style: HighlightStyle,        // Select, Hover, AgentFocus, Dim
    pub label: Option<String>,        // floating text label near entity
}
```

Key properties of every operation:
- **Serializable** (serde) -- can be sent as JSON, stored in undo history, saved to macro files, or generated by an LLM.
- **Returns typed results** -- success with metadata (new entity IDs, bounding boxes) or error with context.
- **Composable** -- the agent chains them into plans; the UI triggers them individually.
- **Undoable** -- every operation pushes to the undo stack.

### 2.3 Agent <-> Engine Message Protocol

The agent communicates with the engine through structured messages. This runs in-process as an async channel, but the message format is JSON-compatible for LLM tool-use integration.

**Agent -> Engine messages:**

```jsonc
// Execute a CAD operation (with optional preview)
{
  "type": "execute",
  "operation": "extrude",
  "params": { "face_id": 42, "depth": 10.0, "direction": "normal" },
  "preview": true    // show ghost geometry before committing
}

// Control the viewport
{
  "type": "camera",
  "action": "look_at_face",
  "face_id": 42,
  "orientation": "isometric",
  "transition": "smooth"
}

// Highlight geometry for user communication
{
  "type": "highlight",
  "entities": [{"face": 42}, {"edge": 17}],
  "style": "agent_focus",
  "label": "This face will be extruded"
}

// Query model state
{
  "type": "query",
  "query": "faces_of_body",
  "body_id": 1,
  "filter": { "normal_direction": [0, 0, 1] }
}

// Ask the user a question
{
  "type": "ask_user",
  "message": "Should I round these edges with a 2mm fillet?",
  "options": ["Yes, 2mm", "Yes, but 1mm", "No, leave them sharp"],
  "allow_freeform": true
}

// Request the user to click/select geometry
{
  "type": "request_selection",
  "message": "Click the face where you want the holes placed",
  "selection_type": "face",
  "count": 1,
  "highlight_candidates": [12, 15, 18, 21]
}
```

**Engine -> Agent messages:**

```jsonc
// Operation result
{
  "type": "operation_result",
  "status": "success",
  "created_entities": { "faces": [43, 44, 45, 46, 47], "edges": [80, 81, 82] },
  "body_id": 1
}

// Query result
{
  "type": "query_result",
  "faces": [
    { "id": 12, "normal": [0, 0, 1], "center": [40, 0, 1.5], "area": 6400.0 },
    { "id": 15, "normal": [0, 0, -1], "center": [40, 0, 0], "area": 6400.0 }
  ]
}

// User response (to ask_user or request_selection)
{
  "type": "user_response",
  "message": "Yes, 2mm",
  "selected_option": 0
}
```

---

## 3. The Interaction Model: Human-in-the-Loop

### 3.1 The Conversation Loop

```
+-----------+     natural language     +----------------+
|           | -----------------------> |                |
|   User    |                          |  Agent (LLM)   |
|           | <----------------------- |                |
| sees 3D   |  text + visual feedback  | reasons about  |
| viewport  |  (camera, highlights,    | CAD procedures |
|           |   ghost previews)        |                |
+-----+-----+                         +-------+--------+
      |                                        |
      | approves / rejects / adjusts           | API calls
      |                                        |
      v                                        v
+------------------------------------------------------------+
|                    GraniteX Engine                           |
|  +--------+  +----------+  +--------+  +-----------------+  |
|  | BREP   |  | Renderer |  | Camera |  | Selection /     |  |
|  | Kernel |  | (wgpu)   |  | System |  | Highlighting    |  |
|  +--------+  +----------+  +--------+  +-----------------+  |
+------------------------------------------------------------+
```

A typical interaction turn:

1. **User speaks.** "Make a mounting bracket, 80 by 40, 3mm thick, with two bolt holes."
2. **Agent receives context.** The message plus current model state (body count, feature tree, active selection, camera position).
3. **Agent plans internally.** Decomposes the request into a feature sequence. Does not share the full plan with the user -- just starts executing.
4. **Agent executes step 1.** Creates a sketch on XY, draws an 80x40 rectangle, extrudes to 3mm. Shows the result.
5. **Agent shows.** Moves camera to isometric view. Highlights the new body. Displays dimensions. Asks: "Here's the base plate, 80x40x3mm. Does this look right?"
6. **User confirms.** "Yes" or "Make it 100mm wide" or "Actually, start on the front plane instead."
7. **Agent adjusts or proceeds.** If the user requested changes, the agent modifies (undo + redo with new params). If confirmed, the agent moves to step 2.
8. **Agent executes step 2.** Selects the top face, creates a sketch, draws two circles, shows preview of the cut operation.
9. **Agent shows.** Zooms to top-down view. Highlights the two circles on the face. Shows red cut preview. Asks: "Two M6 holes, 15mm from each end. Good?"
10. **User adjusts.** "Move them closer together."
11. **Agent adjusts.** Recalculates positions, re-shows preview.
12. **User confirms.** Agent commits the cut-extrude.
13. **Done.** Agent offers: "Would you like me to add fillets, a reinforcement rib, or anything else?"

### 3.2 What the User Never Has to Do

Under this model, the user **never** needs to:

- Select a sketch plane manually
- Enter or exit sketch mode
- Draw individual lines, arcs, or circles by clicking
- Calculate dimensions or positions
- Choose which feature to apply (extrude vs. cut vs. revolve)
- Sequence features in the correct order
- Debug a failed operation (the agent handles retries)
- Navigate to a specific face to work on (the agent moves the camera)

The user **only** needs to:

- Describe what they want (natural language)
- Look at the viewport (the agent shows everything)
- Confirm, adjust, or reject (simple yes/no/correction)
- Optionally click geometry when the agent asks ("click where you want the hole")

### 3.3 Spatial Reference Resolution

When the user says "put a hole here" or "on that face" or "the top edge," the agent must resolve these spatial references to concrete geometry. This is handled through a multi-strategy approach:

**Strategy 1: Geometric reasoning.** The agent queries the model -- "find all faces with normal [0,0,1]" -- and resolves "the top face" from the query results.

**Strategy 2: Highlight and confirm.** When multiple candidates match, the agent highlights them with labels ("A", "B", "C"), moves the camera to show all candidates, and asks the user to pick. "Which of these faces do you mean?"

**Strategy 3: Click delegation.** For truly ambiguous references ("here"), the agent enters a selection mode: "Click the spot where you want the hole." The user clicks, the agent receives the face ID and coordinates.

**Strategy 4: Context from camera.** If the user is looking at a specific area of the model, the agent can infer context from the camera orientation. "The face you're looking at right now" is a valid reference.

### 3.4 Handling Ambiguity

The agent has specific strategies for different types of ambiguity:

| Ambiguity Type | Example | Agent Strategy |
|---|---|---|
| **Geometric** | "Extrude this face" (which face?) | Query candidates, highlight with labels, ask user to pick |
| **Dimensional** | "Make it bigger" | Propose specific increase (+20%), show preview, ask if correct |
| **Procedural** | "Add a hole" (where? how big? how deep?) | Use sensible defaults (M6, through-all), show preview, let user adjust |
| **Intent** | "Fix this" | Analyze model for issues, propose fixes, ask which to apply |
| **Referential** | "Do the same on the other side" | Remember prior operations, mirror the procedure, show result |

### 3.5 Error Recovery

When an operation fails, the agent does not simply report the error. It reasons about recovery:

```
Agent: [tries to extrude sketch]
Engine: Error -- sketch contour is not closed (gap at vertex 23)

Agent's internal reasoning:
  - The sketch has an open gap
  - I can query the gap location
  - Options: (a) close it automatically, (b) ask the user

Agent: [zooms camera to the gap, highlights it]
       "The sketch has a small gap here. Should I close it automatically,
        or would you like to adjust the sketch?"
```

The agent always prefers to show the problem visually rather than describe it textually.

---

## 4. Why Camera, Highlighting, and Preview are Critical Infrastructure

These three systems are not cosmetic features. They are the agent's **communication primitives** -- the equivalent of a human pointing at something, circling it with a marker, and holding up a sketch to show what they plan to do.

### 4.1 Camera Control: "Let Me Show You"

The agent controls the viewport to direct the user's attention. Without this, the agent would have to describe geometry in text ("the face on the positive Z side, approximately 80x40mm, at height 3mm"), which is slow, ambiguous, and cognitively demanding.

With camera control:
- **After creating geometry:** Agent orbits to the best angle to show the result. "Here's the base plate" + [camera smoothly rotates to isometric view].
- **Before an operation:** Agent zooms to the relevant face. "I'm going to put holes here" + [camera zooms to top-down view of the face].
- **When asking for input:** Agent positions the camera so the user can see all relevant geometry clearly. "Which of these edges should I fillet?" + [camera shows all candidate edges].
- **When showing a problem:** Agent zooms to the issue. "There's a thin wall here that might be too fragile" + [camera zooms to the problem area].

Camera control requires:
- Programmatic smooth transitions (not instant jumps -- those disorient the user)
- View planning: choosing an angle that shows the relevant geometry without occlusion
- Distance calculation: close enough to see detail, far enough to maintain context

### 4.2 Geometry Highlighting: "I Mean This"

The agent highlights faces, edges, and vertices to unambiguously refer to specific geometry. Without this, every spatial reference would require a round-trip: "Do you mean the top face?" "Which top face?" "The large one." "They're both large."

With highlighting:
- **Referencing geometry:** Agent highlights a face in blue glow. "I'll extrude this face" is unambiguous because the user can SEE which face.
- **Showing candidates:** Agent highlights 3 faces with labels A/B/C. "Which face should I sketch on?" is a simple choice, not a puzzle.
- **Showing relationships:** Agent highlights two edges that will be filleted. The user sees exactly which edges, with no room for misinterpretation.
- **Dimming context:** Agent fades out irrelevant geometry, making the relevant parts visually dominant.

Highlighting requires:
- Per-entity highlight with multiple simultaneous styles (agent highlight vs. user selection vs. hover)
- Label rendering (floating text near highlighted entities)
- Dimming/transparency for de-emphasized geometry

### 4.3 Ghost Previews: "This is What I'm About to Do"

The agent shows a transparent preview of the operation result before committing it. This is the visual equivalent of "here's my plan -- do you approve?"

With preview:
- **Extrude preview:** Blue ghost geometry shows where material will be added.
- **Cut preview:** Red ghost geometry shows where material will be removed.
- **Fillet preview:** Curved ghost geometry shows how edges will be rounded.
- **The user sees the future.** They can say "yes," "less," "more," or "no" based on what they see -- not based on imagining what "extrude 10mm along normal" means.

Preview requires:
- Operation-specific ghost geometry generation (each operation type needs its own preview logic)
- Distinct visual style (transparent, colored, outlined) to distinguish preview from committed geometry
- Depth testing against real geometry so previews look spatially correct

### 4.4 Together: The Agent's Visual Language

These three systems compose into a visual language:

| Agent wants to say | Camera | Highlight | Preview |
|---|---|---|---|
| "Look at this" | Move to face | Highlight face | -- |
| "I'll change this" | Move to face | Highlight face | Show ghost |
| "Which one?" | Show all candidates | Label A/B/C | -- |
| "Here's the result" | Orbit to best angle | Highlight new geometry | -- |
| "There's a problem" | Zoom to issue | Highlight problem area | -- |
| "This is my plan" | Show overview | Highlight affected areas | Show full preview |

This is why these systems must be built as **programmable APIs**, not just mouse-event handlers. The agent needs to call `camera_look_at(face_42, smooth_transition)` and `highlight(edges, agent_focus_style, label="fillet here")` as programmatic functions.

---

## 5. Operation API Requirements

### 5.1 Model State Queries (Read)

The agent must be able to introspect the model:

| Query | Returns | Agent Use Case |
|---|---|---|
| `list_bodies` | All bodies with bounding boxes | "There are 2 bodies" |
| `faces_of_body(id)` | Faces with normals, areas, centers | Find "the top face" |
| `edges_of_face(id)` | Edges with endpoints, lengths | Find edges to fillet |
| `feature_tree` | Ordered feature list with params | Understand how the model was built |
| `bounding_box(id)` | Min/max corners | Understand overall size |
| `distance(e1, e2)` | Distance between entities | Measurement |
| `active_selection` | Currently selected entities | Context for commands |
| `camera_state` | Position, target, FOV | Know what user is looking at |

### 5.2 Modeling Operations (Write)

| Operation | Key Parameters | Returns |
|---|---|---|
| `create_sketch(plane)` | plane reference | sketch ID |
| `sketch_rect(origin, w, h)` | position, dimensions | entity IDs |
| `sketch_circle(center, r)` | position, radius | entity ID |
| `sketch_line(p1, p2)` | endpoints | entity ID |
| `close_sketch` | -- | validation result |
| `extrude(sketch, depth)` | sketch ref, distance | body ID, face IDs |
| `cut(sketch, depth)` | sketch ref, distance | modified body |
| `fillet(edges, radius)` | edge refs, radius | modified edges |
| `chamfer(edges, dist)` | edge refs, distance | modified edges |
| `revolve(sketch, axis, angle)` | refs, axis, angle | body ID |
| `mirror(body, plane)` | refs | new body ID |
| `pattern_linear(body, dir, n, spacing)` | params | new body IDs |
| `boolean_union(b1, b2)` | body refs | merged body |
| `boolean_subtract(b1, b2)` | body refs | modified body |

### 5.3 Visual Control (Show)

| Action | Effect |
|---|---|
| `camera_look_at(entity)` | Smooth camera transition to focus on entity |
| `camera_orientation(preset)` | Snap or animate to standard view (front/top/iso) |
| `camera_zoom_fit(entity?)` | Fit all or specific entity in view |
| `highlight(entities, style)` | Glow/color entities to draw attention |
| `dim_except(entities)` | Fade out everything except specified entities |
| `show_preview(op, params)` | Ghost preview of proposed operation |
| `show_label(entity, text)` | Floating text label near entity |
| `show_dimensions(entities)` | Display measurement annotations |
| `clear_visual_feedback` | Remove all agent-driven highlights/labels/previews |

### 5.4 User Interaction (Ask)

| Action | Effect |
|---|---|
| `ask_text(prompt)` | Show question in chat, wait for text response |
| `ask_choice(prompt, options)` | Show buttons in chat |
| `ask_confirm(prompt)` | Yes/no dialog |
| `ask_select(prompt, type, count)` | Enter selection mode, wait for clicks |
| `ask_point(prompt, plane?)` | Enter point-pick mode on a face/plane |
| `show_message(text)` | Non-blocking info message in chat |

---

## 6. Example Workflow: "Make a Bracket with Two Holes"

This section traces a complete interaction from the user's first message to the finished part, showing exactly what the agent reasons, what it executes, and what the user sees.

### Turn 1: User Request

**User:** "Make me a mounting bracket, 80mm wide, 40mm tall, 3mm thick, with two bolt holes for M6 screws."

### Turn 2: Agent Plans and Begins

**Agent's internal reasoning (not shown to user):**
```
Decomposition:
  1. Base plate: sketch rectangle (80x40) on XY plane, extrude 3mm
  2. Bolt holes: sketch 2 circles (M6 clearance = 6.4mm dia) on top face,
     cut-extrude through-all

Placement: holes centered vertically (y=20), spaced at x=15 and x=65
(15mm inset from each end -- standard for M6 in 80mm bracket)

Plan:
  Step 1: Sketch + extrude base
  CHECKPOINT: show result
  Step 2: Sketch holes + cut
  CHECKPOINT: show result
  Step 3: Offer refinements (fillets, ribs)
```

**Agent executes:**
```jsonc
{"type": "execute", "operation": "create_sketch", "params": {"plane": "xy"}}
{"type": "execute", "operation": "sketch_rect", "params": {"origin": [0,0], "w": 80, "h": 40, "centered": true}}
{"type": "execute", "operation": "close_sketch"}
{"type": "execute", "operation": "extrude", "params": {"depth": 3.0}, "preview": true}
```

**User sees:** Camera smoothly moves to isometric view. A blue ghost rectangle appears on XY plane, then extrudes upward to 3mm. Dimensions are labeled: "80mm x 40mm x 3mm."

**Agent says:** "Here's the base plate -- 80 by 40, 3mm thick. Look good?"

### Turn 3: User Confirms

**User:** "Yes."

**Agent executes:** Commits the extrude (ghost becomes solid geometry).

### Turn 4: Agent Continues to Holes

**Agent executes:**
```jsonc
{"type": "query", "query": "faces_of_body", "filter": {"normal": [0,0,1]}}
// Engine returns: face_id=5, normal=[0,0,1], center=[0,0,3], area=3200
{"type": "camera", "action": "look_at_face", "face_id": 5, "orientation": "top"}
{"type": "execute", "operation": "create_sketch", "params": {"plane": {"face": 5}}}
{"type": "execute", "operation": "sketch_circle", "params": {"center": [-25, 0], "r": 3.2}}
{"type": "execute", "operation": "sketch_circle", "params": {"center": [25, 0], "r": 3.2}}
{"type": "execute", "operation": "close_sketch"}
{"type": "execute", "operation": "cut", "params": {"depth": "through_all"}, "preview": true}
```

**User sees:** Camera smoothly transitions to top-down view. The top face is highlighted. Two red circles appear on the face with a red cut-preview extending through the body. Labels show: "M6 clearance (6.4mm dia), 25mm from center."

**Agent says:** "Two M6 bolt holes, evenly spaced. Here's how they'll look. Good?"

### Turn 5: User Adjusts

**User:** "Move them a bit closer to the ends."

**Agent reasons:** "Closer to ends = increase x offset from center. Currently 25mm. Try 30mm (10mm from each end instead of 15mm)."

**Agent executes:**
```jsonc
{"type": "execute", "operation": "undo"}  // undo preview
{"type": "execute", "operation": "sketch_circle", "params": {"center": [-30, 0], "r": 3.2}}
{"type": "execute", "operation": "sketch_circle", "params": {"center": [30, 0], "r": 3.2}}
{"type": "execute", "operation": "cut", "params": {"depth": "through_all"}, "preview": true}
```

**User sees:** Circles move outward. New labels: "10mm from each end."

**Agent says:** "Moved them to 10mm from each end. Better?"

### Turn 6: User Confirms

**User:** "Perfect."

**Agent executes:** Commits the cut.

**Agent says:** "Done. The bracket has a base plate (80x40x3mm) with two M6 through-holes. Want me to add fillets on the edges or a reinforcement rib?"

### End State

The user has a parametric bracket with a clean feature tree:
```
Feature Tree:
  1. Sketch1 (XY plane) -- rectangle 80x40
  2. Extrude1 -- 3mm
  3. Sketch2 (Top face) -- 2 circles r=3.2
  4. Cut1 -- through-all
```

Every feature is editable. The user can go back and change any dimension. Total user effort: 3 natural language messages and 2 confirmations. Zero knowledge of CAD procedures required.

---

## 7. The GX Script Connection

The GX scripting language (see `docs/DSL_DESIGN.md`) is the bridge between the agent, the engine, and the user.

**The agent generates GX script as its execution plan:**

```gx
sketch on xy_plane {
    rect center=(0,0) w=80 h=40
}
extrude depth=3

sketch on top_face {
    circle center=(-30, 0) r=3.2
    circle center=(30, 0) r=3.2
}
cut through_all
```

Why GX script instead of raw JSON API calls:
- **LLMs generate code-like syntax better than structured JSON.** Fewer hallucinated fields, more natural output.
- **Readable by humans.** The user can inspect, understand, and edit the agent's plan.
- **IS the macro/replay file.** Undo/redo, save/load, and agent generation all use the same format.
- **Smaller token count.** Faster generation, cheaper API calls.
- **Editable after generation.** User can tweak the script directly for fine control.

The agent can execute GX step-by-step, pausing between blocks for visual feedback and user confirmation.

---

## 8. Technical Prerequisites: What We Need to Build

### 8.1 Already Built (Foundation)

These exist today and will serve the agent directly:

- [x] Camera control system with smooth animated transitions
- [x] Face/edge selection with hover highlighting
- [x] Extrude / cut with visual ghost preview
- [x] Sketch system (planes, lines, rectangles, circles)
- [x] egui chat panel (agent interface placeholder)
- [x] Undo/redo (snapshot-based command pattern)
- [x] Keyboard shortcuts and command system
- [x] Measurement tool

### 8.2 Must Build (Agent Dependencies)

These are required before the agent can function:

| Prerequisite | Why the Agent Needs It | Phase |
|---|---|---|
| **Operation API refactor** | Agent calls typed functions, not UI button handlers | Phase B |
| **Entity ID stability** | Agent references faces/edges across operations | Phase 9 (BREP) |
| **Camera animation API** | Agent positions viewport programmatically | Phase B |
| **Programmatic highlighting** | Agent highlights entities by ID with custom styles | Phase C |
| **Label rendering** | Agent labels geometry with text ("A", "B", "this face") | Phase C |
| **Ghost preview API** | Agent shows proposed operations before committing | Phase C |
| **BREP kernel (OCCT)** | Robust operations, topological naming, face/edge queries | Phase 9 |
| **Feature tree** | Agent reads/understands model construction history | Phase 8-9 |
| **Claude API integration** | Natural language processing via tool-use | Phase D |
| **Chat panel upgrade** | Structured messages: buttons, selection prompts, progress | Phase D |
| **GX parser** | Agent output parsed and executed step-by-step | Phase E |

### 8.3 Infrastructure That Serves Double Duty

Key insight: **construction geometry, measurement tools, and visualization features help manual users AND the agent.** They are not separate feature sets.

| Feature | Manual User Benefit | Agent Benefit |
|---|---|---|
| Sketch planes | User places sketches on faces | Agent chooses planes programmatically |
| Construction lines | User aligns geometry visually | Agent communicates placement rationale |
| Dimension display | User reads measurements | Agent labels proposed dimensions for user validation |
| Face highlighting | User selects faces by clicking | Agent points at faces it's referring to |
| Camera presets | User switches views quickly | Agent positions camera for optimal viewing angle |
| Ghost preview | User sees operation result before committing | Agent shows plan before executing |
| Feature tree | User edits parametric history | Agent understands model construction, plans modifications |

This is why the agent vision does not require a separate "agent-only" feature set. Almost everything the agent needs is something the manual workflow also benefits from.

---

## 9. Why This is Achievable

### 9.1 LLMs Can Reason Procedurally

The core capability the agent requires -- decomposing high-level intent into a sequence of typed operations -- is something modern LLMs already do well. Claude Code decomposes "build a web API" into file creation, routing, database setup, error handling, tests. The same reasoning applies to CAD:

- "Build a bracket" -> sketch, extrude, sketch holes, cut
- "Add a reinforcement rib" -> select face, sketch triangle, extrude
- "Round the edges" -> identify edges, apply fillet with radius

The operations are typed and well-defined. The sequencing rules are learnable from the system prompt. This is tool-use, which is a mature LLM capability.

### 9.2 The Tool-Use Pattern Maps Perfectly

Anthropic's Claude tool-use API is structurally identical to what we need:

- Each CAD operation = a tool with typed parameters
- The LLM receives model state as context (like a codebase) and tools as capabilities
- The LLM generates tool calls, receives results, reasons about next steps
- Multi-step workflows with intermediate results and branching logic

This is not a novel integration pattern. It is the same pattern used by Claude Code, MCP servers, and every agentic LLM application.

### 9.3 The Visual Loop Compensates for LLM Weaknesses

LLMs are weak at spatial reasoning. They cannot reliably compute coordinates, predict 3D geometry, or reason about occlusion. The visual loop sidesteps this:

- The LLM specifies *intent* ("hole 8mm from the edge"), the *engine* computes coordinates.
- The LLM proposes an operation, the *user* validates the visual result.
- The LLM does not need to "imagine" the geometry -- it can *query* the engine and *show* the user.

The human-in-the-loop is not a limitation; it is the architecture that makes the system reliable despite LLM imperfections.

### 9.4 The Comparison to Existing Approaches

| Approach | How It Works | Why GraniteX Is Different |
|---|---|---|
| **Text-to-mesh** (Meshy, Point-E) | LLM generates mesh directly | Not parametric, not editable, not precise |
| **Text-to-code** (ChatGPT + OpenSCAD) | LLM generates complete script, user runs it | No step-by-step feedback, no visual communication |
| **AI assistant in CAD** (Fusion 360 AI) | Sidebar helper that suggests features | Doesn't *drive* the engine -- suggests, doesn't execute |
| **GraniteX agent** | LLM reasons procedurally, executes step-by-step, communicates visually, human validates | Full agentic loop with visual shared workspace |

---

## 10. Implementation Phases

### Phase A: Foundation (In Progress Now)

Everything being built today with an eye toward agent compatibility. Key principle: separate operations from UI, keep APIs programmatic.

### Phase B: Operation API Layer

Refactor all existing operations into standalone typed functions. The UI becomes a thin wrapper that calls the API. This is good engineering regardless of the agent -- it enables undo/redo, scripting, testing, and macros.

### Phase C: Visual Feedback API

Build the agent's visual vocabulary: programmatic highlighting, programmatic camera, generic preview system, label rendering, geometry dimming.

### Phase D: Agent Integration

Connect to Claude API. Implement the message protocol. Build context serialization (model state -> compact text for LLM). Upgrade the chat panel for structured messages.

### Phase E: GX Script Pipeline

GX parser, AST-to-API mapping, step-by-step execution with pauses for confirmation. Agent generates GX, engine executes it.

### Phase F: Polish and Intelligence

Context window optimization, operation chaining (skip confirmation for trivial steps), learning from corrections, voice input (Whisper), multi-model routing (fast model for simple tasks, powerful model for complex reasoning).

---

## 11. Architectural Decisions to Make Now

Even though the agent is future work, these decisions today determine whether it is possible later:

1. **Keep operations separate from UI.** Every button click should call a standalone function. Never inline mesh manipulation in a UI callback. This is the single most important architectural decision for agent compatibility.

2. **Entity IDs must be first-class.** Every face, edge, and vertex needs a stable, queryable ID -- not just an index into a Vec. The BREP kernel (OCCT) provides this; our current mesh layer approximates it.

3. **Camera must be programmable.** Animated transitions must be callable functions, not just mouse-event handlers. `camera.animate_to(target, orientation, duration)`.

4. **The chat panel is the agent interface.** The egui chat panel is not cosmetic. It must support structured messages (buttons, selection prompts, progress indicators), not just text.

5. **Preview system must be operation-agnostic.** Ghost geometry for any operation, not just extrude. "Show what this operation would produce before committing."

6. **Feature tree is the agent's memory.** The parametric feature tree tells the agent how the model was built and what can be modified. Prioritize it.

7. **GX script is the universal bridge.** Human editing, agent generation, macro recording, and file serialization -- all the same format.

---

## 12. Inspiration and References

- **Cursor / Claude Code** -- AI agents that operate through tool APIs (LSP, file ops, terminal), executing step-by-step with results visible to the user. GraniteX agent is the same concept applied to CAD instead of code.
- **SolidWorks API** -- Every operation is an API call. The UI is a client. Exactly the two-client architecture we need.
- **Fusion 360 API** -- Python scripting that drives the same engine as the UI. Good reference for operation API design.
- **Anthropic tool_use** -- Claude's tool-calling capability maps perfectly to our Operation API. Each operation is a tool.
- **Blender Python API** -- Every operation scriptable. `bpy.ops.mesh.extrude_region()` is the pattern we follow.
- **OpenSCAD** -- Declarative CAD scripting. GX is inspired by this but imperative (step-by-step, not declarative).

---

## Appendix: Failure Modes and Mitigations

A comprehensive adversarial analysis of everything that can go wrong with this architecture is maintained in `docs/AGENT_CRITIQUE.md`. The top 5 risks:

1. **Topological Naming Problem** -- Entity IDs renumber after operations. Mitigated by BREP kernel (OCCT) with topological naming, and re-query-after-every-op pattern.
2. **LLM spatial reasoning limits** -- LLMs cannot compute coordinates reliably. Mitigated by shifting spatial computation to the engine; the LLM specifies intent, not coordinates.
3. **State synchronization** -- Agent and user cannot both modify the model simultaneously. Mitigated by turn-based protocol with explicit handoff.
4. **Rendering complexity** -- Agent visual feedback requires multi-pass rendering. Mitigated by planning the render pipeline to support opaque + transparent + overlay passes.
5. **Token budget** -- Complex models produce huge context. Mitigated by hierarchical compression, compact formats, and lazy loading.

None are unsolvable. Each is a substantial engineering effort. See `AGENT_CRITIQUE.md` for detailed analysis.

---

*This document is the north star of the GraniteX project. Every feature built -- every sketch plane, every highlight, every camera animation, every preview ghost -- serves double duty: it helps the manual user today, and it becomes part of the agent's visual language tomorrow. The destination is a CAD environment where the AI reasons, the engine executes, and the human decides.*
