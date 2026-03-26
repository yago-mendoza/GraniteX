# GraniteX — Changelog

## 2026-03-26 — Project Inception

- Created GitHub repository (yago-mendoza/GraniteX)
- Set up project structure: README.md, CLAUDE.md, docs/
- Created docs brain: TODO, ARCHITECTURE, TOOLING, DECISIONS, IDEAS, PROBLEMS, LEARNINGS, CHANGELOG, MILESTONES, RESEARCH
- Defined core stack: wgpu + winit + egui + glam
- Mapped out 10-phase development plan
- Waiting on Rust installation to begin coding

## 2026-03-26 — Session 1: Foundation

- Installed Rust 1.94.0
- Created Cargo project with wgpu + winit + egui + glam stack
- Implemented: wgpu init, colored cube, orbit/pan/zoom camera, MSAA 4x, infinite grid
- Documented ultimate vision: conversational AI agent driving CAD through natural language chat
- User feedback: grid lines too jagged, wants minimal UI like SolidWorks, wants DSL with live compiler
- Next: subtle grid, egui UI shell, then work toward operations layer

## 2026-03-26 — Session 2: DSL Design

- Designed GX (GraniteX Script) -- a domain-specific language for parametric CAD
- Full spec in docs/DSL_DESIGN.md covering: syntax, sketches, operations, references, variables, errors
- 5 examples from simple box to parametric enclosure with snap-fit lid
- Hybrid reference system: semantic names (body.top) + geometric queries (body.face(x_max))
- Constraint syntax for 2D sketches (dimensions, parallel, perpendicular, coincident, etc.)
- LLM-friendly design: line-oriented, minimal keywords (~30), flat structure, no nesting
- Parser strategy: logos (lexer) + chumsky (parser) for best error recovery
- Integration plan: GX scripts, incremental editing, and live REPL mode for the AI agent

## 2026-03-26 — Session 3: Sketch-on-Face Research

- Deep architectural research on SolidWorks-style "sketch on face" workflow
- Documented: sketch plane construction, LM constraint solver algorithm, closed contour detection, rendering approach, under/over-constrained handling, data structures
- Added to RESEARCH.md: 9 sections covering the full sketch system architecture
- Key decisions: LM via argmin or hand-rolled, slotmap for entity storage, hybrid 3D+egui rendering, separate points with coincident constraints (not merged points)
