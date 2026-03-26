# GraniteX DSL Design: **GX** (GraniteX Script)

Last updated: 2026-03-26

## Language Name: **GX**

Short, memorable, greppable, file extension `.gx`. Pronounced "gee-ex."

---

## 1. Design Principles

| Principle | Rationale |
|---|---|
| Line-oriented | One operation per line makes parsing trivial and diffs clean |
| Minimal punctuation | No semicolons, minimal braces. Whitespace-significant indentation for sketch blocks only |
| Rust-flavored types | `f64` literals, snake_case names, `//` comments -- familiar to our Rust codebase |
| LLM-friendly | Predictable structure, no ambiguous grammar, easy to generate token-by-token |
| Deterministic | Same script always produces the same geometry. No implicit state beyond the feature tree |
| Error-locatable | Every operation maps 1:1 to a source line, so errors point to exact locations |

---

## 2. Core Syntax -- Five Examples of Increasing Complexity

### Example 1: A simple box
```gx
// A 50x30x10mm block
sketch s1 on XY
  rect center=(0,0) size=(50,30)
end

extrude s1 height=10
```

### Example 2: Box with a hole
```gx
sketch base on XY
  rect center=(0,0) size=(50,30)
end

extrude base height=10 -> body

sketch hole on body.top
  circle center=(0,0) r=5
end

cut hole through
```

### Example 3: Filleted bracket with bolt holes
```gx
let width = 80
let height = 40
let thickness = 5
let hole_d = 6.5

sketch profile on XY
  rect origin=(0,0) size=(width, height)
end

extrude profile height=thickness -> bracket

// Bolt holes
sketch holes on bracket.top
  circle center=(15, 10) r=hole_d/2
  circle center=(15, 30) r=hole_d/2
  circle center=(65, 10) r=hole_d/2
  circle center=(65, 30) r=hole_d/2
end

cut holes through

fillet bracket.edges(z_parallel) r=3
```

### Example 4: Revolved knob with patterned grip
```gx
sketch profile on XZ
  line (0,0) -> (15,0)
  line -> (15,5)
  arc -> (12,10) r=5
  line -> (12,25)
  arc -> (10,30) r=3
  line -> (5,30)
  line -> (5,28)
  line -> (0,28)
  close
end

revolve profile axis=Y angle=360 -> knob

// Grip grooves
sketch groove on knob.face(r_max)
  rect center=(0,0) size=(2,20)
end

cut groove depth=1 -> grip_cut
pattern grip_cut circular axis=Y count=24
```

### Example 5: Parametric enclosure with snap-fit lid
```gx
// --- Parameters ---
let L = 100       // length
let W = 60        // width
let H = 40        // height
let wall = 2.0    // wall thickness
let corner_r = 3  // corner radius
let tol = 0.15    // fit tolerance

// --- Bottom shell ---
sketch outer on XY
  rrect center=(0,0) size=(L,W) r=corner_r
end

extrude outer height=H -> shell

sketch inner on XY
  rrect center=(0,0) size=(L - 2*wall, W - 2*wall) r=corner_r - wall
end

cut inner depth=H-wall  // hollow out, leave bottom

fillet shell.edges(top) r=0.5

// --- Snap-fit lip on inside wall ---
sketch lip_profile on shell.face(inner_wall, x_max) at y=0
  line (0,0) -> (0.8, 0)
  line -> (0.8, -1.5)
  line -> (0, -1.5)
  close
end

extrude lip_profile symmetric=true along=face_normal depth=0.8 -> lip
pattern lip mirror plane=YZ   // mirror to opposite wall
pattern lip mirror plane=XZ   // mirror to other pair of walls

// --- Lid (separate body) ---
sketch lid_outer on XY at z=H+0.5
  rrect center=(0,0) size=(L,W) r=corner_r
end

extrude lid_outer height=wall -> lid

sketch lid_skirt on lid.bottom
  rrect center=(0,0) size=(L - 2*wall + 2*tol, W - 2*wall + 2*tol) r=corner_r - wall
end

extrude lid_skirt height=-5 -> skirt  // negative = downward

// Snap groove matching lip
sketch groove_profile on skirt.face(outer, x_max) at y=0
  line (0, -1) -> (1, -1)
  line -> (1, -2.5)
  line -> (0, -2.5)
  close
end

cut groove_profile depth=0.8+tol -> groove
pattern groove mirror plane=YZ
pattern groove mirror plane=XZ
```

---

## 3. Sketch Syntax

A sketch is a 2D drawing on a plane or face. It defines closed profiles that operations (extrude, cut, revolve) consume.

### Entering a sketch

```gx
sketch <name> on <plane_or_face> [at <offset>]
  ...
end
```

- **Planes**: `XY`, `XZ`, `YZ`, or `XY at z=50`
- **Faces**: `body.top`, `body.face(3)`, `body.face(x_max)`
- Sketches are always 2D. The coordinate system is local to the plane/face.

### Geometry primitives

```gx
// Lines
line (x1,y1) -> (x2,y2)              // absolute endpoints
line -> (x2,y2)                        // chain from last point
line (x1,y1) -> dx=20 dy=0            // relative displacement

// Arcs
arc (x1,y1) -> (x2,y2) r=10          // arc by radius (shorter arc)
arc -> (x2,y2) r=10                    // chained
arc (x1,y1) -> (x2,y2) center=(cx,cy) // arc by center

// Circles
circle center=(x,y) r=5
circle center=(x,y) d=10              // diameter variant

// Rectangles (syntactic sugar -- expand to 4 lines)
rect origin=(x,y) size=(w,h)          // corner-anchored
rect center=(x,y) size=(w,h)          // center-anchored
rrect center=(x,y) size=(w,h) r=3     // rounded rectangle

// Slots, polygons
slot center=(0,0) length=20 r=5       // stadium/slot shape
polygon center=(0,0) r=10 sides=6     // regular polygon

// Close the profile
close                                  // line from last point to first point
```

### Constraints

Constraints restrict sketch geometry. They follow the primitives they constrain.

```gx
sketch s1 on XY
  line p1=(0,0) -> p2=(20,0)
  line -> p3=(20,15)
  line -> p4=(0,15)
  close

  // Dimensional constraints
  dim p1-p2 = 20             // horizontal distance
  dim p2-p3 = 15             // vertical distance
  angle p1-p2 to p2-p3 = 90  // angle between segments

  // Geometric constraints
  horizontal p1-p2            // force horizontal
  vertical p2-p3              // force vertical
  parallel p1-p2 to p3-p4    // parallel lines
  perpendicular p1-p2 to p2-p3
  coincident p4 with (0,15)  // point on point
  tangent arc1 to line1      // tangent continuity
  equal line1 line3           // equal length
  symmetric p1 p2 axis=Y     // mirror symmetry

  // Fix a point (anchor the sketch)
  fix p1 at (0,0)
end
```

**Point naming**: Points are named inline (e.g., `p1=`, `p2=`). If unnamed, they get auto-generated names (`_p1`, `_p2`, ...). Named points are preferred for constraint references.

### Construction geometry

```gx
sketch s1 on XY
  // Construction lines -- used for constraints, not part of profile
  @line (0,-50) -> (0,50)         // @ prefix = construction
  @circle center=(0,0) r=20

  // Real geometry constrained to construction
  line p1=(0,10) -> p2=(20,10)
  coincident p1 with @line1       // p1 sits on the construction line
end
```

---

## 4. Operation Syntax

Operations transform sketches into 3D geometry or modify existing bodies.

### Extrude

```gx
extrude <sketch> height=<val> [-> <name>]
extrude <sketch> depth=<val>                // alias for height
extrude <sketch> to=<face_or_distance>      // extrude up to a face
extrude <sketch> through                     // through all bodies
extrude <sketch> symmetric=true height=20    // 10mm each side
extrude <sketch> draft=5                     // draft angle in degrees
```

### Cut

```gx
cut <sketch> depth=<val>
cut <sketch> through
cut <sketch> to=<face>
```

### Revolve

```gx
revolve <sketch> axis=<axis_or_line> angle=<deg> [-> <name>]
revolve <sketch> axis=Y angle=360 -> knob
revolve <sketch> axis=sketch.line1 angle=180
```

### Fillet & Chamfer

```gx
fillet <edge_query> r=<val>
chamfer <edge_query> d=<val>
chamfer <edge_query> d1=<val> d2=<val>   // asymmetric
```

### Pattern

```gx
// Linear pattern
pattern <feature> linear axis=X count=5 spacing=10

// Circular pattern
pattern <feature> circular axis=Y count=12 [angle=360]

// Mirror
pattern <feature> mirror plane=YZ
pattern <feature> mirror plane=body.face(x_mid)
```

### Boolean

```gx
union body1 body2 [-> <name>]
subtract body1 body2 [-> <name>]
intersect body1 body2 [-> <name>]
```

### Sweep & Loft

```gx
// Sweep a profile along a path
sweep <sketch_profile> along=<sketch_path> [-> <name>]

// Loft between profiles
loft [profile1, profile2, profile3] [-> <name>]
loft [profile1, profile2] guide=<sketch_path>
```

### Shell

```gx
shell <body> thickness=<val> [remove=<face_query>]
shell box thickness=2 remove=box.top
```

---

## 5. Reference System

How to refer to faces, edges, and vertices. This is one of the hardest parts of a parametric CAD language. GX uses a hybrid approach: **named references from operations** + **geometric queries**.

### Named references (stable)

Operations produce named bodies. Bodies expose topological names:

```gx
extrude base height=10 -> block

// Face references (by semantic position)
block.top          // the face created at the extrusion cap
block.bottom       // the face on the sketch plane
block.sides        // all lateral faces (list)

// After a cut:
cut holes through -> hole_cut
hole_cut.walls     // cylindrical faces created by the cut
```

### Geometric queries (flexible)

When semantic names are insufficient, query by geometric property:

```gx
body.face(x_max)        // face with highest X centroid
body.face(x_min)        // face with lowest X centroid
body.face(y_max)        // etc.
body.face(z_max)
body.face(normal=(0,0,1)) // face whose normal is closest to +Z
body.face(area_max)      // largest face
body.face(3)             // face by index (fragile -- avoid in parametric scripts)

body.edge(z_max)         // highest edge
body.edge(longest)       // longest edge
body.edges(z_parallel)   // all edges parallel to Z
body.edges(top)          // all edges on the top face
body.edges(fillet_candidates) // edges eligible for fillet

body.vertex(x_max, y_max) // corner vertex by position query
```

### Face from face (chained queries)

```gx
body.face(z_max).edges        // all edges of the top face
body.face(z_max).edge(longest)
body.face(x_max).adjacent(z_max)  // face adjacent to x_max face, closest to z_max
```

### Query lists and filtering

```gx
body.edges(z_parallel and length > 10)   // compound filter
body.faces(area > 100)                    // all faces with area > 100mm^2
```

### Why this approach over index-only or name-only

| Approach | Pros | Cons |
|---|---|---|
| Index-only (like STEP entity IDs) | Simple | Fragile, changes when model is edited |
| Name-only (OpenSCAD) | Stable | Can't name everything, verbose |
| **GX hybrid** | Semantic names are stable, queries handle the rest | Slightly more complex parser |

The geometric query system is designed so that an LLM can refer to geometry naturally ("the top face", "the longest edge") without needing to know internal IDs.

---

## 6. Variables, Parameters, and Expressions

### Variables

```gx
let width = 50
let height = width * 0.6       // expressions allowed
let hole_r = 3.25
let count = 4                   // integers
let name = "bracket_v2"         // strings (for metadata only)
let through_hole = true         // booleans
```

### Expressions

GX supports standard arithmetic and a small set of math functions:

```gx
// Arithmetic
let a = 10 + 5 * 2          // = 20 (standard precedence)
let b = (10 + 5) * 2        // = 30
let c = width / 2 - wall

// Math functions
let r = sqrt(x*x + y*y)
let angle = atan2(y, x)
let h = sin(30) * radius     // trig functions take degrees
let n = ceil(length / pitch)
let m = min(a, b)
let p = max(a, b)
let q = abs(a - b)

// Constants
let circ = 2 * PI * r
```

### Conditional geometry

```gx
if hole_count > 0
  sketch holes on body.top
    circle center=(0,0) r=hole_r
  end
  cut holes through
end
```

### Loops (for parametric patterns beyond `pattern`)

```gx
for i in 0..hole_count
  let x = margin + i * spacing
  sketch h on body.top
    circle center=(x, 0) r=hole_r
  end
  cut h through
end
```

### Functions (reusable geometry blocks)

```gx
fn mounting_hole(face, x, y, d)
  sketch _h on face
    circle center=(x,y) r=d/2
  end
  cut _h through
end

// Use it
mounting_hole(bracket.top, 10, 10, 6.5)
mounting_hole(bracket.top, 70, 10, 6.5)
```

### Modules (multi-file)

```gx
import "fasteners.gx" as fasteners

fasteners.m5_through_hole(body.top, 10, 10)
```

---

## 7. Error Handling

### Error categories

| Category | Example | Severity |
|---|---|---|
| **Syntax** | `extrude height=` (missing value) | Fatal -- cannot parse |
| **Reference** | `body.face(top)` when `body` is not defined | Fatal -- unresolved name |
| **Sketch** | Open profile used in extrude | Fatal -- operation requires closed profile |
| **Constraint** | Over-constrained or under-constrained sketch | Warning or Fatal |
| **Geometry** | Zero-thickness wall, self-intersecting extrude | Fatal -- invalid geometry |
| **Topology** | Fillet radius too large for edge | Recoverable -- skip and warn |
| **Type** | `extrude s1 height="ten"` | Fatal -- type mismatch |

### Error format

Errors include the source location and a human-readable message:

```
error[GX-E012]: open profile cannot be extruded
 --> bracket.gx:14
   |
14 | extrude profile height=10
   |         ^^^^^^^ profile 'profile' is not closed
   |
   = help: add `close` at the end of sketch 'profile'
   = note: extrude requires a closed 2D profile

error[GX-E031]: unresolved reference
 --> bracket.gx:22
   |
22 | fillet block.edges(top) r=3
   |        ^^^^^ 'block' is not defined
   |
   = help: did you mean 'bracket'? (defined at line 8)
```

### Error numbering scheme

- `GX-E0xx`: Syntax errors (parse failures)
- `GX-E1xx`: Reference errors (names, imports)
- `GX-E2xx`: Sketch errors (geometry, constraints)
- `GX-E3xx`: Operation errors (extrude/cut/fillet failures)
- `GX-E4xx`: Type errors
- `GX-W0xx`: Warnings (non-fatal)

### Sketch constraint diagnostics

The constraint solver reports its state:

```
warning[GX-W001]: under-constrained sketch
 --> bracket.gx:5-10
   |
   sketch 'base' has 2 degrees of freedom remaining
   |
   = help: consider adding: fix p1 at (0,0), dim p1-p2 = <value>
   = note: under-constrained sketches use current positions as defaults

error[GX-E201]: over-constrained sketch
 --> bracket.gx:5-12
   |
11 | dim p1-p2 = 20
   | dim p1-p2 = 25   // conflicts with line 11
   |
   = help: remove one of the conflicting constraints
```

---

## 8. Grammar Summary (Pseudo-EBNF)

```ebnf
program       = statement*
statement     = let_stmt | sketch_block | operation | if_stmt | for_stmt | fn_def | import_stmt | comment
let_stmt      = "let" IDENT "=" expr
sketch_block  = "sketch" IDENT "on" face_ref ["at" offset] NEWLINE sketch_body "end"
sketch_body   = (sketch_elem | constraint | comment)*
sketch_elem   = line_stmt | arc_stmt | circle_stmt | rect_stmt | rrect_stmt | slot_stmt | polygon_stmt | "close"
line_stmt     = ["@"] "line" [point_def] "->" (point_def | delta_def)
arc_stmt      = ["@"] "arc" [point_def] "->" point_def ("r=" expr | "center=" point)
circle_stmt   = ["@"] "circle" "center=" point "r=" expr
rect_stmt     = "rect" ("origin=" | "center=") point "size=" point
constraint    = dim_c | angle_c | geo_c | fix_c
dim_c         = "dim" ref "-" ref "=" expr
geo_c         = ("horizontal"|"vertical"|"parallel"|"perpendicular"|"coincident"|"tangent"|"equal"|"symmetric") ref+
fix_c         = "fix" ref "at" point
operation     = extrude | cut | revolve | fillet | chamfer | pattern | boolean | sweep | loft | shell
extrude       = "extrude" IDENT (height_spec) ["->" IDENT]
cut           = "cut" IDENT (depth_spec | "through")
face_ref      = plane | IDENT "." face_query
face_query    = "top" | "bottom" | "face" "(" query_args ")"
edge_query    = IDENT "." "edge" "(" query_args ")" | IDENT "." "edges" "(" query_args ")"
point         = "(" expr "," expr ")"
expr          = literal | IDENT | expr binop expr | unop expr | func_call | "(" expr ")"
```

This is not the full formal grammar (that will be written as a `pest` or `nom` parser in Rust) but captures the structure.

---

## 9. Why GX Over Alternatives

### vs. OpenSCAD

OpenSCAD uses CSG (Constructive Solid Geometry) exclusively -- you build shapes by combining primitives with `union/difference/intersection`. This is elegant for simple parts but becomes unwieldy for anything with fillets, chamfers, or sketched profiles. OpenSCAD has no concept of "sketch on face" which is fundamental to parametric CAD.

GX uses a **feature-based modeling** approach (sketch -> extrude -> modify) which mirrors how engineers actually think about parts. The feature tree is the natural way to describe manufacturable geometry.

OpenSCAD syntax: deeply nested function calls with no named intermediate results. GX: linear sequence of named operations.

### vs. CadQuery (Python)

CadQuery embeds CAD in Python, which gives you Python's full power but also Python's problems: slow execution, runtime type errors, and the entire Python syntax surface for an LLM to get wrong. CadQuery also uses a method-chaining API (`result = cq.Workplane("XY").rect(10,20).extrude(5)`) which is hard to parse back into a feature tree.

GX is purpose-built: smaller grammar means faster parsing, better error messages, and more reliable LLM generation. A GX file IS the feature tree.

### vs. G-code

G-code describes tool paths (how to cut), not geometry (what to make). They solve completely different problems. G-code is the output of a CAM system; GX is the input to a CAD system.

### vs. SVG paths

SVG describes 2D vector graphics, not 3D parametric models. GX's sketch syntax borrows some ideas from SVG's path data (chained segments, arcs by radius) but adds constraints, dimensions, and the third dimension.

### vs. STEP / IGES

These are exchange formats for finished geometry, not design-intent languages. A STEP file contains the final BREP -- you cannot edit it parametrically. A GX file contains the recipe to reproduce the geometry, which means parameters can be changed and the model regenerates.

### vs. JSON/YAML-based protocols

The VISION.md describes a JSON protocol for agent-to-engine communication. GX is not a replacement for that protocol -- it is a higher-level language that compiles DOWN to those operations. The relationship:

```
User writes GX  -->  Parser  -->  Feature tree  -->  Operation API calls  -->  Geometry
LLM generates GX -->  same pipeline
JSON protocol    -->  directly to Operation API calls (lower level, for real-time agent interaction)
```

GX is for **scripts and saved designs**. The JSON protocol is for **live agent interaction**. Both call the same Operation API underneath.

### Key advantages of GX for LLM generation

1. **Line-per-operation** -- streaming-friendly, the LLM can emit one line at a time and each line is independently valid
2. **No nesting hell** -- unlike OpenSCAD's `difference() { union() { ... } }`, GX is flat
3. **Named references** -- the LLM can refer back to `body.top` instead of tracking CSG tree positions
4. **Constrained vocabulary** -- ~30 keywords total, reducing hallucination surface
5. **Looks like pseudocode** -- LLMs are trained on code that looks exactly like this

---

## 10. Parsing Strategy

The parser will be implemented in Rust using one of:

| Option | Pros | Cons |
|---|---|---|
| `nom` | Already in ecosystem (nom-stl), zero-copy, fast | Combinator style can be verbose |
| `pest` | PEG grammar in separate file, very readable | Slightly slower, extra build step |
| `logos` + hand-written | Maximum control, fastest possible | More code to write |
| `chumsky` | Best error recovery of any Rust parser lib | Newer, less battle-tested |

**Recommendation**: `logos` for lexing + `chumsky` for parsing. This gives us:
- The fastest possible lexer (logos compiles to a state machine)
- The best error messages (chumsky's error recovery is designed for exactly this)
- Both are pure Rust, no FFI, no code generation

The parser produces an AST that maps directly to the Operation API. Each AST node carries its source span for error reporting.

---

## 11. Integration with the Agent System

The AI agent has two modes of interacting with GX:

### Mode 1: Script generation
The agent generates a complete `.gx` file from a natural language description. This is for "create a bracket" type requests.

### Mode 2: Incremental editing
The agent modifies an existing `.gx` file -- adding lines, changing parameters, deleting features. This is for "make the hole bigger" type requests.

### Mode 3: Live REPL
The agent emits GX statements one at a time into a live session. Each statement is parsed and executed immediately, with visual feedback in the viewport. This is the conversational mode described in VISION.md.

```
User: "Add a hole in the center of the top face"
Agent generates: sketch h1 on body.top
                   circle center=(0,0) r=5
                 end
                 cut h1 through
Engine: executes, shows result
User: "Make it bigger"
Agent generates: // modifies parameter
                 // re-executes from that point
```

---

## 12. File Format

```
Extension:  .gx
Encoding:   UTF-8
Line ending: LF (normalize CRLF on Windows)
Comments:   // line comments only (no block comments -- keeps parsing simple)
```

### Metadata header (optional)

```gx
//! name: "Mounting Bracket v3"
//! author: "Yago Mendoza"
//! units: mm
//! created: 2026-03-26
//! description: "L-bracket with 4 mounting holes"
```

The `//!` prefix denotes doc-comments / metadata, parsed separately from geometry.

---

## 13. Reserved for Future

These keywords are reserved but not yet implemented:

- `assembly` -- multi-body assembly with mates
- `mate` -- constrain two bodies together (coincident, concentric, etc.)
- `export` -- export a body to STL/STEP from within the script
- `assert` -- geometry assertions for testing (`assert body.volume > 1000`)
- `animate` -- parameter animation for visualization
- `material` -- assign material properties (density, color)
- `tolerance` -- specify manufacturing tolerances on dimensions
