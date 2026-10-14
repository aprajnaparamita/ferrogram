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
angles in **radians**, percentages as a fraction (`0.5` = 50%), and
`Unitsless` values as plain numbers. The `unit` field records the unit the
file wrote (`Unit::Feet`, `Unit::Degrees`, ...), so a write can restore the
file's own spelling. P2-03 implements the conversions in `ferrogram-geom`.

**Amended by P2-06 (2026-10-13): the canonical angle unit is radians, not
degrees.** A `.vsdx` file caches values in canonical units whatever `U`
declares, and the corpus shows angles among them: `Angle
V='1.5707963267949' U='DEG'` is a right angle written in radians,
`PageWidth V='11.69291338582677' U='MM'` is a 297 mm page width written in
inches, and `Value V='0.8' U='PER'` is 80% written as a fraction. [MS-VSDX]
2.2.5.3.3.1 reads the same way: the cell default value of a cell whose `U`
is `DEG`, `DA`, `AD` or `RAD` is "0.00 radians" and of a `MM`, `PT`, `IN_F`
cell is "0.00 inches". The reader therefore uses the cached value as
written; neither read nor write converts. `ferrogram_geom::UnitKind::Angle`
still counts degrees as the canonical unit of *conversions*; that mismatch
touches only callers that convert (the ShapeSheet table's `default_value`
parsing in `ferrogram-model`), not the model itself. P2-06b closes that
mismatch too (see Amendments).

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

## Amendments

### Amended 2026-10-14, P2-06b: radians everywhere, and the slots the files need

- **The canonical angle unit is radians in `ferrogram-geom` too.**
  `Angle` stores radians and `UnitKind::Angle` converts to radians
  (`Unit::Radians` is the base unit, `Unit::Degrees` a factor of `pi/180`).
  The amendment of P2-06 already made the *model* radians; the mismatch it
  left in `geom` would have made `lookup::parse_default_value` produce
  degrees for angle defaults. Everything now agrees with the file.
- **`Section` has `cells`.** `.vsdx` writes `Geometry`'s `NoFill`, `NoLine`,
  `NoShow`, `NoSnap` and `NoQuickDrag` as `Cell` children of the `Section`
  element, outside its rows. They were kept verbatim (lossless but
  invisible to the lookup); `Section::cells` holds them and the lookup
  matches a section's own cells before its row cells.
- **`Master` has `shapes: Vec<ShapeId>`, not `shape: Option<ShapeId>`.** A
  master part may hold several top-level shapes ([MS-VSDX] allows it and the
  corpus has masters with up to 33); the singular field kept the first and
  lost the rest from reach. The list is in document order.
- **Style references are Visio style IDs.** `Shape::line_style`,
  `fill_style`, `text_style` and the same three fields on `Style` are
  `Option<u32>`, the `ID` of a stylesheet row (`.vsdx` writes
  `LineStyle="7"`); `Style` also keeps its own `visio_id`. The old fields
  held the file text of the reference and `Style::base_style` held a `Style`
  cell the reader consumed; neither matched what `.vsdx` writes (a style's
  chain is its `LineStyle`/`FillStyle`/`TextStyle` attributes), so the
  style rung of the lookup never matched a file-loaded shape. `base_style`
  is gone; a `Style` cell is an ordinary cell again.
- **`Connect` has `preserved`.** A `Connect` element's unmapped attributes
  and children stay on the record instead of falling back to the page's
  `Preserved`, so the page's bag no longer mixes in per-record content.

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
