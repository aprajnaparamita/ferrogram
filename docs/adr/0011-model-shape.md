# 0011: Model shape — arenas, identifiers, cells and mutations

- Status: accepted
- Date: 2026-10-13
- Task: P2-01

## Context

Phase 2 builds `crates/model` (L1): documents, pages, masters, shapes
(including groups and sub-shapes), stylesheets, ShapeSheet sections, rows and
cells. The model must serve three masters at once:

- `.vsdx` read/write with lossless preservation (ADR 0012),
- lookup-based inheritance, never flattened on load (ADR 0013),
- Phase 4 editing and undo, which needs stable references to entities across
  mutations.

Visio's own identifiers (`ID`, `NameU`, `Name`) are file-level, optional and
sometimes duplicated across pages, so they cannot serve as in-memory keys.

## Decision

### Entities and arenas

`Document` owns one **shape arena** (`Vec<Shape>`) plus the lists of pages,
masters and stylesheet rows. Everything references shapes by `ShapeId`, a
dense `u32` index newtype. Group shapes list their sub-shapes as
`Vec<ShapeId>` (`children`), and every shape knows its `parent: Option<ShapeId>`.
Sub-shapes are ordinary arena shapes, so a lookup can walk the chain and a
command can move one without re-parenting data.

Pages (`PageId`), masters (`MasterId`) and styles (`StyleId`) are index
newtypes into their lists. Ids are stable for the document's lifetime:
entries are not removed or moved while any id is live. If Phase 4 needs
removal, it retires entries and the arena grows a free list; that change does
not affect any signature here.

Each entity keeps Visio's values alongside the id: `visio_id: i32` (the `ID`
attribute), `name: String` (`Name`) and `name_u: String` (`NameU`), kept
verbatim, never normalized on load.

### Sheets, sections, rows, cells

A shape, page sheet or stylesheet row holds a `Sheet { cells, sections }`:

- `cells` are the direct cells (the "object" sections: Shape Transform,
  Character, Paragraph, Shape Data, ...), which `.vsdx` writes as direct
  `Cell` children;
- `sections` are the repeated-row sections (Geometry, Connection Points, ...),
  each `Section { name, rows }`, each `Row { name, row_type, ix, cells }`.

A `Cell` keeps three things separately (ADR 0008, KICKOFF):

```rust
pub struct Cell {
    pub name: CellName,
    pub formula: Option<String>,   // source text, without the leading '='
    pub value: Option<CellValue>,  // cached value, canonical units
    pub unit: Unit,                // the unit the file wrote
    pub preserved: Preserved,
}
```

**Canonical units** (used for `CellValue::Number`): lengths in **inches**,
angles in **degrees**, percentages as a fraction (`0.5` = 50%), and
`Unitsless` values as plain numbers. The `unit` field records the unit the
file wrote (`Unit::Feet`, `Unit::Radians`, ...), so a write can restore the
file's own spelling. P2-03 implements the conversions in `ferrogram-geom`;
P2-06 checks the degree assumption against the corpus and the spec, and
amends this ADR if the cached angle unit turns out to be radians.

`CellValue` is `Number(f64) | Str(String) | Bool(bool) | Color(Color) |
List(String) | Error(String)`. List and error cached values stay raw strings:
their structure is not needed before the formula engine.

### Formula text, not parsed formulas

Phase 2 stores a formula as its source `String`. Parsing belongs to
`ferrogram-shapesheet` (ADR 0014) and no Phase 2 test needs an AST.

### Mutations and undo

The model is plain data with public fields; mutation safety is not the
model's job. What the model provides for Phase 4 is the **shape of
mutations**: every setter that an editing command will need returns the
previous value (`Cell::set_formula` -> `Option<String>`,
`Cell::set_value` -> `Option<CellValue>`), so a command can record its own
inverse. The undo stack itself lives in `ferrogram-commands` (L4), not here.
Command design (transaction grouping, redo) is Phase 4 work; nothing in this
ADR blocks it.

## Consequences

- `Document::shape(id)` is an O(1) lookup; group walks are O(children).
- File-level identity (`ID`, `NameU`) is data, not key. Two shapes on
  different pages may carry the same `visio_id` without conflict.
- Public fields make the model easy to test and to map, but nothing prevents
  inconsistent edits; `ferrogram-commands` is the only writer in the app.
- Retiring arena entries needs a free list one day; do it inside `Document`
  without changing `ShapeId`.

## Alternatives considered

- **A tree of owned `Shape` values.** Natural for groups, but every command
  and the renderer would hold paths instead of ids; moving a sub-shape
  between groups is a deep move; undo inverses get expensive. Rejected.
- **`slotmap` or `generational-arena` crates.** Better stale-id safety, but
  an extra dependency for what a Vec plus a never-reused index does here.
  Rejected for Phase 2; revisit if ids start going stale in Phase 4.
- **Use `NameU` as the key.** Rejected: optional in files, only unique per
  document in theory, and absent on sub-shapes.
- **Parsed formula ASTs in cells.** Rejected for Phase 2: parsing 420 cell
  kinds is out of scope, and raw text round-trips losslessly.
- **Flatten `Sheet` into shape/page/style structs.** Rejected: three copies
  of the same fields; `Sheet` is also what the ShapeSheet table (ADR 0008)
  describes.
