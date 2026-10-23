# 0034: Original stencils and the Shapes window

- Status: accepted
- Date: 2026-10-16
- Task: P4-01

## Context

KICKOFF Phase 4: "Shapes window: with original 'Basic Shapes' and 'Basic
Flowchart' stencils." AGENTS.md section 4: every stencil master we ship is
original work; every committed asset has an `ATTRIBUTION.md` row. The
model already has masters, geometry rows (ADR 0019) and a stencil document
kind (ADR 0016), and the writer can produce `.vssx` (ADR 0018). The open
questions are the source format of the masters, where they live, and what
the Shapes window shows before Phase 5 adds drag-and-drop.

## Decision

### Source format: hand-authored Rust data, built into model masters

The two stencils are **hand-authored data in Rust const tables** in
`crates/model/src/stencils/` (L1), with builders that produce
`model::Master` values (geometry rows, line and fill defaults, Shape Data
rows, connection points). Rationale:

- Compile-checked geometry (typed `GeometryRow` values, no stringly YAML
  to validate), and the data is small and stable (about 20 masters).
- `crates/commands` (L4) can build an `insert.shape` command over the
  same definitions headlessly in Phase 5; nothing about the stencils is
  UI-specific.
- One test per stencil writes the built document to `.vssx` through
  `larrygram_vsdx::write_document` and reads it back (the writer is the
  "export to .vssx" path KICKOFF's repo layout implies); the shipped
  product needs no binary asset at all.

```rust
// crates/model/src/stencils/mod.rs (P4-06 writes this; interface fixed here)
/// A hand-authored stencil: named masters of our own design.
pub struct StencilDef {
    /// Display name (e.g. "Basic Shapes").
    pub name: &'static str,
    /// Masters in Shapes window order.
    pub masters: Vec<MasterDef>,
}

pub struct MasterDef {
    /// Unique name (`NameU`), e.g. "Rectangle".
    pub name_u: &'static str,
    /// Display name (`Name`).
    pub name: &'static str,
    /// Master size in inches.
    pub width: f64,
    pub height: f64,
    /// Geometry rows in local coordinates (unit box scaled by width/height).
    pub geometry: Vec<GeometryRow>,
    /// Default line and fill (our own visual style defaults, ADR 0023).
    pub line: LineDefaults,
    pub fill: FillDefaults,
    /// Shape Data rows shown in the Shape Data pane (ADR 0035).
    pub shape_data: Vec<(&'static str, &'static str)>,
    /// Connection points in local coordinates (Phase 5 glues to these).
    pub connection_points: Vec<(f64, f64)>,
}

/// The "Basic Shapes" stencil (original work).
pub fn basic_shapes() -> StencilDef;
/// The "Basic Flowchart" stencil (original work).
pub fn basic_flowchart() -> StencilDef;
/// Every stencil the Shapes window lists.
pub fn catalog() -> Vec<StencilDef>;
/// Builds the stencils as one stencil-kind `Document` (ADR 0016), so the
/// writer can save it as `.vssx`.
pub fn stencil_document() -> Document;
/// Builds one master's shapes onto a page as a new instance (Phase 5).
pub fn instantiate(target: &mut Document, master: &MasterDef) -> ShapeId;
```

### The masters

**Basic Shapes** (about 12): rectangle, rounded rectangle, ellipse,
triangle, right triangle, diamond, pentagon, hexagon, octagon, five-point
star, arc, line. **Basic Flowchart** (about 8): process, alternate
process, decision, terminator, data, document, manual operation,
preparation, connector. Geometry is simple polygons, ellipses and rounded
rectangles we define ourselves in inches; no shape is traced, copied or
derived from a Microsoft stencil (AGENTS.md section 4). Provenance rows
are added to `ATTRIBUTION.md` in the committing task: "original work,
authored for Larrygram, no third-party source".

### Shapes window (Phase 4 scope)

`crates/ui::shapes_window` lists the catalog: stencil names as collapsible
sections, master names as list entries with a small preview image. The
preview is a raster of the master rendered through
`render_raster` (ADR 0033) on a one-shape scene — same pixels as the PNG
backends, no new renderer. Clicking an entry selects it (the status bar
names it); **dragging onto the page is Phase 5** (`insert.shape` with
`stencil` and `master` args, `instantiate` above). Phase 4 shows the
window, the list and the previews, and proves the stencils are real model
data.

## Consequences

- Original work only: `ATTRIBUTION.md` gains two rows, both "original".
- The model gains a `stencils` module (P4-06 owns it) and a test that
  round-trips `.vssx`; no new dependency.
- Phase 5 gets `instantiate` and connection points already in the data.
- If a later phase wants stencil files on disk (`.vssx` shipped), the same
  data is written by the test path; nothing else changes.

## Alternatives considered

- **Author as `.vssx` XML in the repo.** Rejected: raw XML is not
  reviewable geometry, and hand-writing the package parts invites
  round-trip bugs; the writer already produces `.vssx` from model data.
- **Author as YAML/JSON loaded at runtime.** Rejected: a second parser,
  runtime validation and error paths for data that never changes; Rust
  consts are checked at compile time. Revisit if users can add stencils.
- **A separate `crates/stencils`.** Rejected: masters are model data
  (L1); a new crate would sit in the model's layer for no gain.
- **Ship an open-licensed stencil set.** Rejected: AGENTS.md requires our
  own masters; "every stencil master we ship is original work".
- **Build the Shapes window preview as egui vector art.** Rejected: two
  renderers again; the raster path is tested and shared (ADR 0033).
