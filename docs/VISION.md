# GraniteX — Ultimate Vision

Last updated: 2026-03-26

## The North Star

GraniteX is not just a CAD tool. It is a **conversational CAD system** where a human and an AI agent collaborate to design 3D parts through natural dialogue.

The user speaks (or types). The agent understands, asks clarifying questions, highlights relevant geometry in the viewport, proposes operations, and executes them upon confirmation. The interaction is fluid, iterative, and intelligent.

## What It Looks Like

```
User:   "Hazme un agujero aquí" (pointing at a face)
Agent:  [highlights the face in the viewport]
        "¿Te refieres a esta cara? ¿Qué diámetro quieres para el agujero?"
User:   "Sí, esa. 5 milímetros."
Agent:  [shows a preview of the hole, highlighted in blue]
        "¿Así? ¿Pasante o con profundidad específica?"
User:   "Pasante."
Agent:  [executes the cut-extrude through the entire body]
        "Hecho. ¿Algo más en esta cara?"
User:   "Ponme otro igual a 20mm de distancia."
Agent:  [highlights a preview at 20mm offset]
        "¿Aquí?"
User:   "Perfecto."
```

## Key Capabilities

### 1. Natural Language Understanding
- Understands spatial references: "aquí", "esa cara", "el borde de arriba", "la esquina"
- Understands operations: "agujero", "extruir", "redondear", "cortar"
- Understands constraints: "5mm", "paralelo a esto", "centrado"
- Multilingual (at minimum Spanish + English)

### 2. Intelligent Visual Feedback
- The agent can **highlight** parts of the model (faces, edges, vertices) to confirm understanding
- The agent can **show previews** of operations before executing
- The agent can **redirect the camera** to show the relevant area
- Color-coded: blue for "I'm asking about this", green for "preview of what I'll do", red for "this will be removed"

### 3. Conversational Flow
- The agent asks clarifying questions when ambiguous
- The agent proposes reasonable defaults ("¿5mm de radio está bien?")
- The agent confirms before destructive operations
- The agent remembers context within a session ("hazme lo mismo en la otra cara")

### 4. Voice Integration
- Speech-to-text for user input (hands-free while looking at the model)
- Text-to-speech for agent responses (optional)
- Wake word or push-to-talk

### 5. Agent-Driven Exploration
- "Muéstrame la pieza desde arriba" → agent moves camera
- "¿Cuánto mide este borde?" → agent measures and reports
- "¿Hay algún problema con este diseño?" → agent runs basic checks (wall thickness, overhangs, etc.)

## Architecture Implications

This vision fundamentally shapes the architecture:

### The Operation Layer Must Be Programmatic
Every operation (extrude, cut, fillet, etc.) must be callable from code, not just from UI clicks. The agent needs an API to drive the CAD engine. This means:
- Every operation = a function with typed parameters
- Every operation is undoable
- Every operation can generate a preview before committing

### The Selection System Must Be Queryable
The agent needs to be able to:
- Query: "what face is at screen position (x, y)?"
- Query: "what faces are adjacent to this edge?"
- Query: "what is the nearest face to this point?"
- Highlight: "make face #17 glow blue"
- This means the geometry kernel must support topological queries, not just rendering.

### The Viewport Must Be Controllable
The agent needs to:
- Move the camera to show specific geometry
- Overlay highlights, annotations, dimension labels
- Show operation previews (ghost geometry)

### The Agent Protocol
A structured protocol between the LLM agent and the CAD engine:

```json
// Agent → Engine
{"action": "highlight", "target": {"type": "face", "id": 17}, "color": "blue"}
{"action": "camera_look_at", "target": {"type": "face", "id": 17}, "distance": 2.0}
{"action": "preview_operation", "op": "cut_extrude", "params": {"face": 17, "diameter": 5.0, "through": true}}
{"action": "execute_operation", "op": "cut_extrude", "params": {"face": 17, "diameter": 5.0, "through": true}}

// Engine → Agent
{"event": "highlight_done", "target": {"type": "face", "id": 17}}
{"event": "user_clicked", "target": {"type": "face", "id": 17}, "screen_pos": [640, 360]}
{"event": "operation_preview_ready", "preview_id": "abc123"}
{"event": "operation_complete", "op": "cut_extrude", "result": "success"}
```

### Voice Pipeline
```
Microphone → Whisper (STT) → LLM Agent → Operation Protocol → CAD Engine → Viewport
                                  ↑                                           ↓
                                  └─── User sees result, speaks again ←───────┘
```

## How This Changes the Roadmap

The existing roadmap (Phases 1-10) builds the CAD engine. The agent layer sits ON TOP of the engine. We need to ensure:

1. **Every phase builds APIs, not just UI** — the agent will call the same operations as the UI
2. **Phase 3 (Selection) is critical** — the agent's ability to point at things depends on robust selection/query
3. **Phase 6 (Undo/Redo) is critical** — the agent needs to undo mistakes gracefully
4. **A new Phase 11: Agent Integration** — wire up LLM, voice, and the operation protocol

The agent layer itself is relatively straightforward IF the CAD engine exposes clean programmatic APIs. The hard part is the engine. We're building it right.

## Competitive Context

- **Fusion 360** has basic AI features (generative design) but no conversational interface
- **SolidWorks** has no AI integration
- **Zoo.dev (KittyCAD)** is building an AI-first CAD tool — closest competitor to this vision
- **Plasticity** is beautiful but no AI
- **OpenSCAD / CadQuery** are code-first but not conversational

GraniteX's differentiator: **the conversation IS the interface**. The 3D viewport is the shared canvas between human and agent. The mouse/keyboard are secondary to voice and natural language.

## Open Questions

- Which LLM to use? Claude API is the obvious choice given the builder. Could also support local models (Llama) for offline use.
- How to handle spatial references? "Here" requires mapping screen coordinates to geometry. Raycasting + LLM context about what's visible.
- How to handle ambiguity? The agent should ask, not guess. But how many questions before it gets annoying?
- Voice latency: STT + LLM + TTS roundtrip needs to feel responsive. Maybe stream the LLM response?
- Multi-language: The LLM handles this natively, but dimension/unit parsing needs care (comma vs dot for decimals).
