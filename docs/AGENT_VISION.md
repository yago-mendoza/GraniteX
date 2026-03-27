# GraniteX — AI Agent Vision: Architecture & Design Document

Last updated: 2026-03-27

---

## 1. The Big Idea

GraniteX's endgame is not just a traditional CAD tool — it's a **conversational CAD environment** where an AI agent can drive the modeling engine step-by-step, with the human in the loop as a decision-maker and validator.

The user says: *"Make me a mounting bracket with four M6 bolt holes, 3mm thick, 80mm wide."*

The agent:
1. Reasons about the procedure: sketch rectangle → extrude → sketch circles → cut-extrude
2. Creates a sketch plane, draws the base rectangle
3. **Shows the user**: camera moves to isometric view, sketch is highlighted
4. Asks: *"Does this base shape look right? Adjust width/height?"*
5. User confirms → agent extrudes to 3mm
6. **Shows the user**: the solid appears, camera orbits to reveal thickness
7. Agent places four circles on the top face, equidistant from edges
8. **Shows the user**: circles highlighted on the face, dimensions visible
9. User says *"move the holes 2mm closer to the edges"*
10. Agent adjusts, shows again → user confirms → cut-extrude through-all
11. Done. Every step was visible, every decision was the user's.

This is not a "generate and hope" approach. It's **procedural reasoning with visual checkpoints** — the agent thinks like a CAD engineer, executes operations one at a time, and uses the 3D viewport as a shared canvas for communication.

---

## 2. Why This Matters

### 2.1 The CAD Skill Gap Problem
Traditional CAD has a brutal learning curve. You need to know:
- Which plane to sketch on
- What sketch entities to use
- How constraints work
- The correct sequence of features (extrude before fillet, etc.)
- How to recover when a feature fails

An AI agent that can **reason about the procedure** and **show each step** collapses this learning curve to near-zero. The user only needs to describe intent and validate results.

### 2.2 Why Not Just Generate the Whole Model?
One-shot generation (user describes → AI produces finished model) fails for CAD because:
- **Ambiguity**: "a bracket" has infinite valid interpretations
- **Precision**: CAD requires exact dimensions — LLMs aren't good at spatial reasoning without iterative feedback
- **Trust**: engineers need to understand and validate each feature, not just the final shape
- **Editability**: a one-shot generated mesh can't be parametrically edited later

Step-by-step with checkpoints solves all of these. Each step is:
- **Unambiguous**: one operation, clearly visible
- **Precise**: agent proposes dimensions, user can adjust
- **Trustworthy**: user sees exactly what changed
- **Editable**: each step is a feature in the parametric tree, modifiable later

### 2.3 Why the Visual Loop is Essential
The agent can't just print text descriptions of geometry — humans think spatially. The 3D viewport **is the communication medium**:
- Agent highlights a face → "I mean this face"
- Agent moves the camera → "look at this from this angle"
- Agent shows a preview ghost → "this is what I'm about to do"
- Agent dims irrelevant geometry → "focus on this part"

This is why **camera control, highlighting, previews, and visual feedback** are not cosmetic features — they are the agent's language for pointing at things in 3D space.

---

## 3. Architecture

### 3.1 The Two-Client Model

```
┌──────────────────────────────────────────────────────────────────┐
│                      Operation API Layer                         │
│  (typed Rust functions: extrude, cut, fillet, select, move...)   │
│  Every operation returns a Result<OperationOutput, CadError>     │
└─────────┬───────────────────────────────────────┬────────────────┘
          │                                       │
          ▼                                       ▼
┌──────────────────┐                   ┌──────────────────────────┐
│    UI Client      │                   │    Agent Client           │
│  (mouse, keyboard │                   │  (LLM → operation calls)  │
│   egui panels)    │                   │                           │
│                   │                   │  Parses natural language   │
│  User clicks →    │                   │  into operation calls.     │
│  calls API        │                   │  Uses visual feedback      │
│                   │                   │  to confirm with user.     │
└──────────────────┘                   └──────────────────────────┘
```

**The fundamental principle**: the UI and the agent are both *clients* of the same Operation API. They call the same functions. This is already captured in ADR-006.

### 3.2 The Operation API

Every modeling operation is a standalone function with typed parameters:

```rust
// Example operations (future API surface)

pub struct ExtrudeParams {
    pub face_id: FaceId,
    pub direction: ExtrudeDirection, // Normal, Custom(Vec3), BothSides
    pub depth: f64,
    pub draft_angle: Option<f64>,
}

pub struct SketchRectParams {
    pub plane: PlaneRef,          // FaceId, DefaultPlane, or CustomPlane
    pub origin: Point2D,           // in sketch-plane coordinates
    pub width: f64,
    pub height: f64,
    pub centered: bool,
}

pub struct FilletParams {
    pub edge_ids: Vec<EdgeId>,
    pub radius: f64,
    pub variable: Option<Vec<(f64, f64)>>,  // position, radius pairs
}

pub struct CameraCommand {
    pub target: CameraTarget,       // FitAll, LookAtFace(FaceId), LookAtPoint(Vec3)
    pub orientation: Option<ViewOrientation>, // Front, Back, Iso, Custom(Quat)
    pub transition: TransitionType, // Instant, Smooth(duration)
}

pub struct HighlightCommand {
    pub entities: Vec<EntityRef>,   // face IDs, edge IDs, vertex IDs
    pub style: HighlightStyle,      // Select, Hover, AgentFocus, Dim
    pub label: Option<String>,      // optional text label shown near entity
}
```

Key properties:
- **Every operation is serializable** (serde) — can be sent over JSON, stored in undo history, saved to a macro file, or generated by an LLM
- **Every operation returns a typed result** — success with metadata (new face IDs, dimensions) or error with context
- **Operations are composable** — the agent chains them, the UI triggers them one-by-one

### 3.3 The Agent Protocol

The agent communicates with the engine through a structured message protocol. This is NOT a REST API — it runs in-process as an async channel. But the message format is JSON-like for LLM compatibility.

#### Agent → Engine messages:

```jsonc
// Execute a CAD operation
{
  "type": "execute",
  "operation": "extrude",
  "params": { "face_id": 42, "depth": 10.0, "direction": "normal" },
  "preview": true  // show ghost before committing
}

// Control the viewport
{
  "type": "camera",
  "action": "look_at_face",
  "face_id": 42,
  "orientation": "isometric",
  "transition": "smooth"
}

// Highlight geometry to communicate with user
{
  "type": "highlight",
  "entities": [{"face": 42}, {"edge": 17}],
  "style": "agent_focus",
  "label": "This face will be extruded"
}

// Query the model state
{
  "type": "query",
  "query": "faces_of_body",
  "body_id": 1,
  "filter": { "normal_direction": [0, 0, 1], "tolerance": 0.1 }
}

// Ask the user a question (shown in the chat panel)
{
  "type": "ask_user",
  "message": "Should I round these edges with a 2mm fillet?",
  "options": ["Yes, 2mm", "Yes, but 1mm", "No, leave them sharp"],
  "allow_freeform": true
}

// Ask the user to click/select geometry
{
  "type": "request_selection",
  "message": "Click the face where you want the holes placed",
  "selection_type": "face",
  "count": 1,
  "highlight_candidates": [12, 15, 18, 21]  // optional: pre-highlight valid choices
}
```

#### Engine → Agent messages:

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

// User response
{
  "type": "user_response",
  "message": "Yes, 2mm",
  "selected_option": 0
}

// User selection
{
  "type": "user_selection",
  "entities": [{"face": 12, "click_point": [25.3, 10.1, 1.5]}]
}
```

### 3.4 The Agent's Brain: Procedural Reasoning

The agent doesn't just translate "make a bracket" into a single API call. It **reasons about the procedure** like a CAD engineer:

```
User: "Make a mounting bracket, 80x40mm, 3mm thick, with 4 bolt holes for M6 screws"

Agent's internal plan:
  1. Create sketch on XY plane (or top face if body exists)
  2. Draw rectangle: 80mm x 40mm, centered on origin
  3. Extrude sketch: 3mm upward
  → CHECKPOINT: show result, confirm dimensions
  4. Select top face of extruded body
  5. Create sketch on that face
  6. Place 4 circles: M6 clearance = 6.4mm diameter
     - Position: 8mm inset from each corner
  7. Show circle placement to user
  → CHECKPOINT: confirm hole positions
  8. Cut-extrude circles: through-all
  → CHECKPOINT: show final result
  9. Optional: fillet outer edges (ask user)
```

This plan is generated by the LLM, but each step is **executed as a typed API call** and **validated by the engine**. If step 3 fails (e.g., sketch isn't closed), the agent gets an error and can reason about how to fix it.

### 3.5 The Conversation Loop

```
┌─────────────┐     natural language      ┌─────────────────┐
│             │ ──────────────────────────▶│                 │
│    User     │                           │   Agent (LLM)   │
│             │ ◀──────────────────────────│                 │
│  sees 3D    │   text + visual feedback   │  reasons about  │
│  viewport   │                           │  procedures     │
└──────┬──────┘                           └────────┬────────┘
       │                                          │
       │  approves/rejects/adjusts                │  API calls
       │                                          │
       ▼                                          ▼
┌──────────────────────────────────────────────────────────────┐
│                     GraniteX Engine                          │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌─────────────┐ │
│  │ BREP     │  │ Renderer │  │ Camera   │  │ Selection / │ │
│  │ Kernel   │  │ (wgpu)   │  │ System   │  │ Highlighting│ │
│  └──────────┘  └──────────┘  └──────────┘  └─────────────┘ │
└──────────────────────────────────────────────────────────────┘
```

A typical turn:
1. User types natural language in the chat panel
2. Agent receives the message + current model state (body count, feature tree, active selection)
3. Agent generates a plan (internally) and starts executing step by step
4. After each meaningful step: agent shows the result (camera move, highlight, preview)
5. Agent asks for confirmation or input
6. User responds (text, click, or approval)
7. Loop continues until the task is complete or the user interrupts

---

## 4. What the Agent Can See and Do

### 4.1 Model State Queries (Read)

The agent can query the engine to understand the current state:

| Query | Returns | Example Use |
|-------|---------|-------------|
| `list_bodies` | All solid bodies with bounding boxes | "There are 2 bodies in the model" |
| `faces_of_body(id)` | All faces with normals, areas, centers | Find the top face |
| `edges_of_face(id)` | All edges with endpoints, lengths | Find edges to fillet |
| `vertices_of_edge(id)` | Start/end points | Precise positioning |
| `feature_tree` | Ordered list of features with params | "This body was made by extrude → fillet → cut" |
| `sketch_entities(id)` | Lines, arcs, circles in a sketch | Understand an existing sketch |
| `bounding_box(id)` | Min/max corners | Understand overall size |
| `distance(e1, e2)` | Distance between two entities | Measurement |
| `active_selection` | Currently selected entities | Context for commands |
| `camera_state` | Position, target, up, FOV | Know what user is looking at |

### 4.2 Modeling Operations (Write)

| Operation | Parameters | Result |
|-----------|-----------|--------|
| `create_sketch(plane)` | plane ref | sketch ID |
| `sketch_line(p1, p2)` | 2D points | entity ID |
| `sketch_rect(origin, w, h)` | origin, dims | entity IDs |
| `sketch_circle(center, r)` | center, radius | entity ID |
| `close_sketch` | — | validates closure |
| `extrude(sketch, depth)` | sketch ref, distance | body/face IDs |
| `cut(sketch, depth)` | sketch ref, distance | modified body |
| `fillet(edges, radius)` | edge refs, radius | modified edges |
| `chamfer(edges, dist)` | edge refs, distance | modified edges |
| `revolve(sketch, axis, angle)` | refs | body ID |
| `mirror(body, plane)` | refs | new body ID |
| `pattern_linear(body, dir, count, spacing)` | params | new body IDs |
| `pattern_circular(body, axis, count)` | params | new body IDs |
| `boolean_union(b1, b2)` | body refs | merged body |
| `boolean_subtract(b1, b2)` | body refs | modified body |
| `boolean_intersect(b1, b2)` | body refs | intersection body |

### 4.3 Visual Control (Show)

| Action | Parameters | Effect |
|--------|-----------|--------|
| `camera_look_at(entity)` | face/edge/vertex/body | smooth camera transition to focus on entity |
| `camera_orientation(preset)` | front/top/iso/custom | snap or animate to standard view |
| `camera_zoom_fit(entity?)` | optional entity | fit everything or specific entity in view |
| `highlight(entities, style)` | refs + style | glow/color entities to draw attention |
| `dim_everything_except(entities)` | refs | fade out irrelevant geometry |
| `show_preview(operation, params)` | operation + params | ghost preview (like our current extrude preview) |
| `show_dimensions(entities)` | refs | display measurement annotations |
| `show_label(entity, text)` | ref + text | floating text label near entity |
| `clear_visual_feedback` | — | remove all highlights, labels, previews |

### 4.4 User Interaction (Ask)

| Action | Parameters | Effect |
|--------|-----------|--------|
| `ask_text(prompt)` | question text | show in chat, wait for text response |
| `ask_choice(prompt, options)` | question + options | show buttons in chat |
| `ask_confirm(prompt)` | question | yes/no dialog |
| `ask_select(prompt, type, count)` | what to select | enter selection mode, wait for clicks |
| `ask_point(prompt, plane?)` | where to click | enter point-pick mode on plane/face |
| `show_message(text)` | info text | non-blocking message in chat |
| `show_progress(text, pct)` | status | progress indicator |

---

## 5. The Agent's Reasoning Model

### 5.1 How the LLM Thinks About CAD

The agent's system prompt will teach it to think like a CAD engineer:

```
You are GraniteX's AI modeling assistant. You create 3D parts by executing
CAD operations step-by-step through the GraniteX API.

THINK PROCEDURALLY:
- Every solid starts as a 2D sketch on a plane
- Sketches become 3D through features: extrude, cut, revolve, sweep
- Features modify existing geometry: fillet rounds edges, chamfer bevels them
- Order matters: you can't fillet an edge that doesn't exist yet
- Each step should produce visible, verifiable geometry

PLANNING STRATEGY:
1. Understand what the user wants (ask if ambiguous)
2. Decompose into features: base shape → modifications → details
3. For each feature: choose plane/face, sketch, apply operation
4. After each major step: show result, get confirmation
5. At the end: review the feature tree, offer refinements

SPATIAL REASONING:
- Use queries to understand existing geometry before modifying it
- Reference faces by their properties (normal direction, area, position)
- When multiple faces could match, highlight candidates and ask the user
- Always verify face IDs after operations (they may renumber)

COMMUNICATION:
- Show, don't tell. Move the camera, highlight geometry, show previews.
- Ask for input at decision points, not at every step.
- Use the viewport as your shared canvas with the user.
- Keep chat messages concise — the 3D view does the heavy lifting.
```

### 5.2 Handling Ambiguity

The agent has specific strategies for when things are unclear:

**Geometric ambiguity** — "extrude this face": which face?
→ Agent uses `faces_of_body` to find candidates, highlights them with labels ("A", "B", "C"), asks user to pick.

**Dimensional ambiguity** — "make it bigger"
→ Agent proposes a specific increase (e.g., +20%), shows preview, asks if that's right.

**Procedural ambiguity** — "add a hole": on which face? what size? through-all or blind?
→ Agent asks the minimum necessary questions, using sensible defaults and showing what it assumed.

**Intent ambiguity** — "fix this"
→ Agent examines the model, identifies potential issues (non-manifold edges, thin walls, unconstrained sketches), proposes fixes.

### 5.3 Error Recovery

When an operation fails, the agent doesn't just report the error — it reasons about alternatives:

```
Agent: [tries to extrude sketch]
Engine: Error — sketch is not a closed contour (gap at vertex 23)

Agent's reasoning:
- The sketch has an open gap
- I can query the gap location and show it to the user
- Options: (a) I close the gap automatically, (b) I ask the user to fix it

Agent: "The sketch has a small gap here [highlights gap, zooms camera].
        Should I close it automatically, or would you like to adjust the sketch?"
```

---

## 6. The GX Script Connection

The GX scripting language (see DSL_DESIGN.md) is closely related to the agent. In fact:

**The agent generates GX script as its "execution plan."**

Instead of the LLM generating raw JSON API calls, it generates GX code:

```gx
// Agent-generated GX script for a mounting bracket
sketch on xy_plane {
    rect center=(0,0) w=80 h=40
}
extrude depth=3

// Bolt holes
sketch on top_face {
    circle center=(8, 8) r=3.2    // M6 clearance
    circle center=(72, 8) r=3.2
    circle center=(8, 32) r=3.2
    circle center=(72, 32) r=3.2
}
cut through_all
```

This is better than raw JSON because:
- LLMs are better at generating code-like syntax than structured JSON
- The script is readable by humans (shared understanding)
- The script IS the macro/replay file (undo/redo for free)
- The script can be edited by the user after generation
- Smaller token count = faster generation = cheaper API calls

The agent can execute this script step-by-step, pausing between blocks for visual feedback and user confirmation.

---

## 7. Implementation Phases

### Phase A: Foundation (Prerequisites — being built NOW)

These are things we're already building that the agent will need:

- [x] Camera control system (orbit, pan, zoom, animated transitions)
- [x] Face selection + hover highlighting
- [x] Extrude / cut with visual preview
- [x] Sketch system (planes, lines, rectangles, circles)
- [x] egui chat panel (already exists in ui.rs)
- [ ] **Operation API refactor** — extract operations from UI code into standalone functions
- [ ] **Entity ID stability** — operations must return new entity IDs reliably
- [ ] **Camera animation API** — programmatic smooth camera transitions
- [ ] **BREP kernel** — for robust operations and face/edge queries

### Phase B: Operation API Layer

Refactor all existing operations to be API-callable:

```rust
// Before (tangled with UI):
if ui.button("Extrude").clicked() {
    let depth = self.extrude_depth;
    // ... 50 lines of mesh manipulation ...
}

// After (clean API):
pub fn extrude(engine: &mut Engine, params: ExtrudeParams) -> Result<ExtrudeResult> {
    // ... mesh manipulation ...
    Ok(ExtrudeResult { new_faces, new_edges })
}

// UI becomes a thin wrapper:
if ui.button("Extrude").clicked() {
    let result = extrude(&mut self.engine, ExtrudeParams { ... });
}
```

This is good engineering regardless of the agent — it enables undo/redo, scripting, testing, and macros.

### Phase C: Visual Feedback API

Build the agent's "visual vocabulary":

1. **Programmatic highlighting** — highlight arbitrary entities by ID with different styles
2. **Programmatic camera** — move camera to look at specific entities
3. **Preview system** — show ghost geometry for proposed operations
4. **Label system** — floating text labels attached to entities
5. **Dimming** — fade out irrelevant geometry to focus attention

### Phase D: Agent Integration

1. **Chat panel** — extend the existing egui chat panel to handle agent messages
2. **Claude API integration** — connect to Claude API for natural language processing
3. **Message protocol** — implement the Agent ↔ Engine protocol described above
4. **Context building** — teach the agent about the current model state each turn
5. **Selection delegation** — agent can request the user to click geometry

### Phase E: GX Script Pipeline

1. **GX parser** — parse GX script into AST
2. **GX → API mapping** — AST nodes map to Operation API calls
3. **Agent generates GX** — LLM output is GX script, parsed and executed
4. **Step-by-step execution** — pause between blocks, show results, await confirmation
5. **Script editing** — user can modify agent-generated scripts before execution

### Phase F: Polish & Intelligence

1. **Context window optimization** — minimize token usage for model state
2. **Operation chaining** — agent executes multiple simple steps without pausing
3. **Learning from corrections** — agent remembers user preferences within a session
4. **Voice input** (Whisper) — hands-free operation
5. **Multi-model** — use fast/cheap model for simple ops, powerful model for complex reasoning

---

## 8. What Makes This Different From Existing AI+CAD

### 8.1 Versus "AI generates STL/mesh"
(e.g., Meshy, Luma, Point-E)
- Those generate meshes — not editable, not parametric, not precise
- GraniteX agent generates **feature trees** — every dimension is editable
- Our approach is deterministic: same script always produces same geometry

### 8.2 Versus "AI generates code" (e.g., ChatGPT + OpenSCAD)
- Those generate a complete script, user runs it, sees result, iterate
- GraniteX agent executes **step-by-step with visual feedback**
- The user is in the loop at every step, not just at the end
- Camera control and highlighting make the 3D viewport a communication medium

### 8.3 Versus "AI assistant in CAD" (e.g., Fusion 360 AI experiments)
- Existing tools add AI as a sidebar helper (suggest features, answer questions)
- GraniteX agent **drives the engine** — it's not an assistant, it's a co-driver
- The agent can DO things, not just SUGGEST things

### 8.4 Versus "Generative design" (e.g., Fusion 360, nTopology)
- Generative design optimizes geometry under constraints (load, material, weight)
- That's topology optimization — fundamentally different from conversational modeling
- GraniteX agent handles the **creative/design** workflow, generative design handles **optimization**
- They're complementary — the agent could set up the design, then hand off to generative solver

---

## 9. Technical Challenges & Open Questions

### 9.1 Token Budget for Model State
The agent needs to "see" the current model state to reason about it. But a complex model with hundreds of faces generates huge context. Strategies:
- **Hierarchical summaries**: "Body 1: box 80x40x3mm, 6 faces, 4 through-holes" (not every face listed)
- **Relevance filtering**: only include faces/edges near the area of interest
- **Spatial indexing**: "top face (z=3, 80x40)", not full vertex data
- **Lazy loading**: agent queries specific details only when needed

### 9.2 Entity ID Stability
After operations, face/edge/vertex IDs may renumber. The agent needs to track what happened:
- "I extruded face 12, which created faces 43-47. The top face is 43."
- BREP kernels (OCCT) provide topological naming — a face retains identity through operations
- Without BREP, we need a naming convention (e.g., "the face created by extrude-1 on the +Z side")

### 9.3 Undo/Redo Integration
When the agent executes operations, they must go on the undo stack:
- User says "undo" → undoes agent's last operation (not the whole plan)
- Agent needs to know when the user has undone something (model state changed)
- Agent-generated operations should be grouped (undo "add bolt holes" = undo 5 operations)

### 9.4 Latency
LLM API calls take 1-5 seconds. The interaction must feel responsive:
- Show a thinking indicator while the agent reasons
- Stream the agent's plan as it's being generated
- Execute operations as soon as they're parsed (don't wait for full response)
- Use cached/local models for simple operations (classification, entity resolution)

### 9.5 Failure Modes
- LLM hallucinates an entity ID that doesn't exist → engine returns error, agent retries with query
- LLM generates invalid GX script → parser error, agent sees error and corrects
- User's request is physically impossible → agent explains why, proposes alternatives
- Operation produces unexpected geometry → agent checks result, asks user if it looks right

### 9.6 Multi-turn Context
The agent needs to remember conversation history:
- "Make the same thing on the other side" → needs to remember what "the same thing" was
- "That's too thick, try 2mm instead" → needs to undo and redo with new parameter
- "Actually, go back to the version before the holes" → needs feature tree understanding

### 9.7 Security
The agent runs user-provided natural language through an LLM to generate operations:
- Operations are typed and validated — the agent can't execute arbitrary code
- The GX parser is the security boundary — only valid GX syntax is accepted
- No file system access through the agent — only CAD operations
- Rate limiting on API calls to prevent abuse

---

## 10. Comparison: Manual Path vs Agent Path

| Aspect | Manual (Mouse/Keyboard) | Agent (Conversational) |
|--------|------------------------|----------------------|
| **Input** | Click, drag, type dimensions | Natural language + clicks when asked |
| **Precision** | User places every point | Agent calculates, user validates |
| **Speed** | Depends on user skill | Fast for complex procedures |
| **Learning curve** | Steep (must learn CAD) | Near-zero (describe what you want) |
| **Control** | Total (every vertex) | High (approve/reject/adjust each step) |
| **Creativity** | User-driven exploration | Agent proposes, user directs |
| **Repeatability** | Macros (manual recording) | GX scripts (auto-generated) |
| **Complex parts** | Tedious but possible | Easier (agent handles procedure) |
| **Simple edits** | Fast (just click) | Slower (describe, wait for LLM) |

**Both paths coexist.** The user can switch between manual and agent-driven at any time. Mid-operation handoff: the agent starts a procedure, the user takes over to fine-tune, then hands back to the agent.

---

## 11. What We Should Keep in Mind NOW

Even though the agent is far in the future, these architectural decisions **today** affect whether it's possible:

1. **Keep operations separate from UI** — every `if button.clicked()` should call a standalone function, not inline the logic. This is the single most important thing.

2. **Entity IDs must be first-class** — every face, edge, vertex needs a stable, queryable ID. Not just an index into a Vec.

3. **Camera must be programmable** — we already have animated transitions. Keep them as callable functions, not just mouse-event handlers.

4. **The chat panel is not just for show** — the egui chat panel we built is the future agent interface. It should support structured messages (buttons, selection prompts), not just text.

5. **Preview system must be operation-agnostic** — our extrude/cut preview system should work for any operation. "Show ghost of what this operation would produce."

6. **Feature tree is the agent's history** — the parametric feature tree is exactly what the agent needs to understand what it has done and what can be undone.

7. **GX script is the bridge** — the scripting language bridges human editing, agent generation, macro recording, and file serialization. It's all the same thing.

---

## 12. Inspiration & References

- **Cursor / Claude Code** — AI agents that operate through a tool API (LSP, file operations), showing results step by step. GraniteX agent is the same concept but for CAD instead of code.
- **SolidWorks API (SOLIDWORKS.Interop.sldworks)** — every operation is an API call. The UI is a client. This is exactly the architecture we need.
- **Fusion 360 API** — Python scripting that drives the same engine as the UI. Well-documented, good reference for API design.
- **OpenSCAD** — declarative CAD scripting. GX is inspired by this but imperative (step-by-step, not declarative).
- **Anthropic tool_use** — Claude's tool-calling capability maps perfectly to our Operation API. Each operation is a tool.
- **Blender Python API** — every operation in Blender is scriptable. `bpy.ops.mesh.extrude_region()` is exactly what we want.

---

*This document is a living vision. Updated as the architecture evolves and as we learn more about what the agent needs.*
