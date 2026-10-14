# 0013: Inheritance lookup API

- Status: accepted
- Date: 2026-10-13
- Task: P2-01

## Context

KICKOFF: "Resolve master/instance inheritance and stylesheet inheritance
through a lookup API. Do not flatten them on load." A Visio shape's effective
cell value can come from the shape itself, from the master it is an instance
of, from its styles (each of which has its own style chain), or from
defaults. Phase 3 rendering, Phase 4 property panels and the formula engine
all need this value, and each needs to know **where it came from**.

## Decision

### Resolution order

`Document::resolve(shape, cell)` walks these sources in order and returns the
first that defines the cell:

1. **Local** — the cell on the shape itself.
2. **Master** — the same cell on the master's content shape, when the shape
   is an instance of a master. Sub-shape rule: a sub-shape resolves against
   the master's sub-shape at the same child position; if the master's
   structure does not match, the chain continues to the master's own sheet.
3. **Style chain** — the shape's `LineStyle`/`FillStyle`/`TextStyle`-selected
   style for the cell's section (a Geometry cell never comes from a style;
   Character/Paragraph/Fill/Line cells do), then that style's own `Style`
   cell's chain, and so on.
4. **Section default** — the `default_value` of the cell in the ShapeSheet
   table (`ui-spec/shapesheet.toml`, ADR 0008).
5. **Hard default** — the default of the cell's type (0, empty string, false,
   black), used when the table has no default.

The result is a `Resolved`:

```rust
pub struct Resolved<'a> {
    pub value: Option<&'a CellValue>,
    pub source: ValueSource,          // Local | Master(id) | Style(id)
                                      // | SectionDefault | HardDefault
    pub formula: Option<&'a str>,     // formula of the defining source
    pub via: Vec<ValueSource>,        // every source consulted, in order
}
```

`via` is the provenance chain a UI can show ("PinX: from master *Box*, style
*Normal*"). `source` is the last entry of `via`.

### Cycles

A style chain can cycle (`Style` cells can point at each other) and a master
chain can too (`BaseID`). The lookup keeps a visited set per chain:

- On revisiting a source, the lookup **stops the chain** at the repeat,
  records the repeated `ValueSource` once more at the end of `via`, and
  resolves down to section default / hard default.
- The lookup never loops, never recurses unboundedly (recursion is bounded by
  the visited-set size) and never errors on a cycle: a cyclic file is
  malformed but readable, and rendering it must not hang.

### Not flattened on load

The reader (`ferrogram-vsdx`) stores each cell exactly as written on the
entity it is written on. It does not copy master values into instances or
style values into shapes. `Resolved` borrows from the document; nothing in
the document changes when a lookup runs.

## Amendments

### Amended 2026-10-14, P2-06b: the master position rule and the style-id chain

- **The master rung matches by position when a master has several
  top-level shapes.** A master holds a list of top-level shapes
  (`Master::shapes`, ADR 0011 amendment). An instance matches the master's
  only shape when the list has exactly one, and the shape at its own
  position otherwise: the instance root's position among its siblings (a
  parent group's children, or the page's top-level shapes), reused at every
  rung of a chained master reference. A position the master does not have
  degrades to the master's first shape. Why: the singular "master's content
  shape" of the original text does not exist in the file ([MS-VSDX] allows
  several, the corpus has them), and picking the first shape for every
  instance would resolve every instance of a multi-shape master to one
  shape.
- **The style chain follows Visio style IDs, per cell kind.** The
  `LineStyle`/`FillStyle`/`TextStyle` values are the `ID` of a stylesheet
  row (`Document::style_by_visio_id`), not a style name, and a style's own
  chain is its own three attributes, not a `Style` cell (which `.vsdx` does
  not write for this). A `Line Format` cell follows the `line_style` link,
  a `Fill Format` cell the `fill_style` link, a `Character`/`Paragraph`
  cell the `text_style` link, at every step of the chain. `style_by_name`
  stays for callers that hold a name. Why: the name-based rung never
  matched a file-loaded shape, so inheritance was invisible on real files.
- **A cell name looked up in a section matches the section's own cells
  before its row cells** (`Section::cells`, ADR 0011 amendment), after the
  sheet's direct cells. Why: `Geometry` writes `NoFill` and friends at
  section level; the lookup must see them where the file puts them.
- **A cell defines its source only when it is not the inherit marker.**
  The corpus writes `F='Inh'` on cells that resolve through master, style
  or default inheritance (e.g. `blue-box.vsdx`'s `LineWeight`). The lookup
  still treats any formula or cached value as defining (unchanged here);
  treating `Inh` as "continue the chain" is a Phase 3 question, recorded as
  an open issue of P2-06b.

### Amended 2026-10-14, master (after P2-06b): `Inh` does not define

`.vsdx` writes `F="Inh"` on a cell whose value comes from the master, a
style or a default. Its `V` is a snapshot of the inherited value at save
time. A cell whose formula is exactly `Inh` therefore does **not** define
the cell for the lookup: resolution continues with the next source. The
cell stays in the model unchanged, so the writer emits it as it was read.
The cached snapshot is not used as a fallback. If Phase 3 finds cells
that resolve worse than their snapshot, it may add that fallback as a
further amendment.

## Consequences

- Rendering and property panels call `resolve` per cell; caching is a later
  concern (Phase 3 measures first). A cache must key on (shape, cell) and be
  invalidated by edits; it can live in `ferrogram-commands` without changing
  this API.
- Editing a master cell changes its instances only through the lookup, which
  is exactly Visio's behavior.
- Tests can assert provenance: a corpus shape's `PinX` should resolve to
  `Master(..)` for a master instance.

## Alternatives considered

- **Flatten on load (copy effective values down).** Rejected by KICKOFF:
  destroys provenance, doubles memory, and a later edit of the master would
  not propagate.
- **Inheritance as a lazily-evaluated formula engine.** Rejected for Phase 2:
  that is the Phase 3+ formula engine's job; a plain ordered lookup is enough
  and testable now.
- **Return the whole chain and let callers pick.** Half-adopted: `Resolved`
  returns both the first hit (`value`, `source`) and the chain (`via`), so
  the common case is one call and provenance is still visible.
- **Error on cycles.** Rejected: a cycle must not turn a readable file into a
  load failure; the file may still render most shapes correctly.
