# 0054: Reference resolution and inheritance

- Status: accepted
- Date: 2026-10-21
- Task: P6-01

## Context

A formula's references must find the cell they name, on this sheet or
another one, through a master, a style or a default (ADR 0013), and an
**inherited** formula must evaluate against the right sheet: the geometry
rows a master defines evaluate against each instance's `Width` and
`Height`, which is exactly what the phase exit criterion needs ("resizing
master-based shapes recomputes geometry correctly").

Two more facts force the design. The corpus writes `F='Inh'` on cells
whose value is inherited (P2-06b), and the value it caches next to it is
**not** the master's own value: `corpus/vsdx/testfile1.vsdx` has masters
of width 0.984251968503937 in with geometry rows at 0.984251968503937 in,
while its page instances carry `F='Inh'` geometry rows with values like
2.165354330708662 in, matching their own 2.165354330708662 in `Width`.
The snapshot is the value **for this sheet**, computed by Visio at save
time. ADR 0013's "the snapshot is not used" rule therefore hides the one
value that is already instance-correct (the P3-15 open item "master-sized
geometry on resized instances").

Sources (`docs/sources.md` rows 4 and 19): `about-cell-references.md`,
`about-formulas.md` (inherited/local formulas), plus ADR 0013 and its
amendments.

## Decision

### Resolution runs through ADR 0013's rungs, and splits formula from value

For a target sheet and a cell name, resolution returns a
`ResolvedCell { formula, value, source }`:

- **`source` and `formula` come from the first rung that defines them**
  (local, master, style chain, section default, hard default; ADR 0013
  and its amendments). `F='Inh'` defines **nothing**: it means "the
  formula is the next rung's formula" (the ADR 0013 master note, reused
  for the formula side).
- **`value` prefers the local cell's own cached value** when the cell has
  one — including the snapshot of an `Inh` cell, which is this sheet's
  value at save time (corpus evidence above). When the local cell has no
  cached value, the value comes from the defining rung's cached value,
  then from the documented default (ADR 0021), then the hard default.

This **amends ADR 0013** ("the cached snapshot is not used as a
fallback"): it is now the *preferred* value for an inherited cell, and the
fallback is the master's cached value. Provenance is still decided by the
formula rung, so the snapshot cannot mask an inheritance bug: a test
asserts that a shape whose master defines `PinX` still resolves
`source = Master(..)` while using its own snapshot.

### An inherited formula evaluates in the target sheet's context

`DocumentResolver` resolves a reference in two steps: find the cell, then
find the formula that defines it. When a master's formula is evaluated for
an instance, **local names bind to the instance's sheet**: `Width` in a
master geometry row is the instance's `Width` (resolved through the same
rungs), `Sheet.8!PinX` is that shape on the instance's page, and
`ThePage!` is the page the instance sits on. This is the rule that makes
resize recompute geometry (ADR 0056) and it is the reason the evaluator
takes a `SheetRef` per call rather than reading a "current sheet" from
the document.

### Reference forms

Every form of `about-cell-references.md` (ADR 0051 keeps the parts as
written), resolved here:

| Written | Resolves to |
|---|---|
| `Width` | the cell on the target sheet |
| `Wheel!Angle` | the named shape's cell on the same page |
| `Frame.2!Height` | the shape named `Frame` with `ID` 2 |
| `Char.Size[3]` | `Character` section, `Size` column, row `IX`/position 3 |
| `Scratch.B3` | `Scratch` section, `B` column, row 3 |
| `Prop.Cost`, `User.Name` | the **named row's default cell** (`Value`) of `Prop`/`User` |
| `Geometry1.X2` | `Geometry` section 1, row 2, `X` cell |
| `Sheet.12!FillForegnd` | the shape with `ID` 12 on the target's page |
| `Masters[Bearing]!Hub!Geometry1.X1` | a master's named sub-shape's cell |
| `Pages[Sheet-3]!Sheet.4!BeginX` | a page's shape's cell |
| `Styles!Ledger!LineColor` | a stylesheet row's cell |
| `ThePage!User.Anchor_Point`, `TheDoc!DocScale` | the page sheet / document sheet |
| `'2-C'!LineColor` | a nonstandard name; `''` is the quote escape |

The column-versus-row split of a bare name (`Char.Size` is a column,
`Prop.Cost` is a row) is decided **against the section**: a name matches
a column of the section's row shape first (with trailing digits or `[n]`
as the row index, default row 0), and a named row of a named-row section
second. Names match **case-insensitively** (flagged for the black-box
black-box check); the file's own spelling is what gets written back.

**One rule, one home (P6-17 amendment).** The split and the name
splitters sit in one module (`crates/shapesheet/src/names.rs`) that both
the resolver and the dependency graph (ADR 0055) ask: a name cannot mean
one thing to the resolver and another to the graph (the P6-12 review's
F2). The graph resolves a reference through the same rungs — a formula
that reads a master's cell is edged to that cell — so an edit of a
master cell re-evaluates its instances' formulas.

A name that resolves to nothing is `#REF!` with one `MissingReference`
warning (`about-error-values.md`), never an error and never a dropped
document.

## Consequences

- Resizing an instance recomputes its geometry from the master's formulas
  at the instance's size, without materialising anything to be correct
  (though ADR 0056 does materialise what it recomputes, for the file).
- The P3-15 rendering gap closes with a one-line policy change in
  `CellReader` (ADR 0057): prefer the local snapshot of an `Inh` cell.
- The resolver is the only place that knows row shapes, so the parser
  (ADR 0051) and the evaluator stay document-free.
- The corpus snapshot rule makes cached values authoritative again after
  an open, so an untouched file renders and saves byte-stable (ADR 0057).

## Alternatives considered

- **Keep ADR 0013's "snapshot is not used".** Rejected: the snapshot is
  the only instance-correct value in the file, and the phase exit
  criterion is about instance-sized geometry. The original fear (a stale
  snapshot masking a resolution bug) is answered by keeping provenance on
  the formula side and testing it.
- **Flatten inherited formulas into every instance on open.** Rejected:
  it rewrites files on open (ADR 0048 identity), doubles file size and
  breaks the master link (ADR 0038's rejected alternative).
- **Evaluate the master's formula in the master's context and scale the
  result.** Rejected: masters' formulas are not all linear in size
  (`MIN`, `IF`, arcs), and scaling is what `scale_geometry` does badly
  (P5-02).
- **Resolve `Prop.Cost` at parse time.** Rejected in ADR 0051: the split
  needs the section's row shape.
- **Match names case-sensitively.** Rejected: the ShapeSheet window
  treats names case-insensitively (a black-box check for the observation
  track), and an exact-only match would report a missing reference for a
  mere case difference. The rule is corpus-tested and flagged for the
  black-box check.
