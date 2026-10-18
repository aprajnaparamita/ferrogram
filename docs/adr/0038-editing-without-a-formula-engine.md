# 0038: Editing without a formula engine

- Status: accepted
- Date: 2026-10-18
- Task: P5-01

## Context

Phase 5 edits documents; Phase 6 brings the ShapeSheet formula engine.
Until then nothing recomputes `Width*0.5` or `LocPinX`. Every edit must
still produce a file Visio and our own reader see the same way, must not
paint Phase 6 into a corner, and must not need a data migration when the
engine lands (the Phase 5 planning brief, decision 1).

Two model facts shape the answer. Cells store a formula, a cached value
and a unit separately (ADR 0011), and the reader resolves through the
inheritance lookup (ADR 0013/0019), where `Inh` does not define a cell.
The renderer reads cached values through `CellReader` (ADR 0021).

## Decision

### 1. An edit writes the formula *and* the cached value

Every cell write carries both (`model::CellWrite { formula, value }`).
The formula is a literal (`"2.5"`) or the documented simple formula of the
cell (`LocPinX = Width*0.5`, `TxtWidth = Width*1`, from the ADR 0021
documented-defaults table); the cached value is what the edit computed.
Writing only `V` would make our file re-read correctly and Visio's recalc
disagree; writing only `F` would leave our own reader with nothing to
render until Phase 6.

`Inh` is expressed by *not* writing the cell at all, as today.

### 2. Dependent cells are not recomputed; documented defaults are

An edit updates exactly the cells the user changed **plus** the cells the
documented-default table derives from them by one multiplication (the
existing `related_number` rule of ADR 0021: `LocPinX = Width*0.5` is
`Width * 0.5` of the cached value). Any other dependent cell keeps its
current cached value and is left stale; the stale set is not written into
the file, and the document change counter (ADR 0040) is what tells the UI
to re-render. Phase 6's recalc replaces the stale values in place: it
reads the formulas we wrote and recomputes `V`. Nothing is migrated.

### 3. Resizing scales the shape's own geometry rows literally

Without formulas, a resized instance whose geometry rows are inherited
would keep the master's size (the ADR 0019 open issue). Phase 5 therefore
resizes by **scaling literal geometry**: `Document::scale_geometry`
materialises the row union into the shape's own `Geometry` section and
scales absolute row coordinates per axis (`Rel*` rows keep their fraction
values, per ADR 0019). This is honest: the file contains the geometry it
draws, and the writer round-trips it. A master whose rows use formulas
like `Width*0.5` still degrades to master coordinates inside an instance
whose rows are all `Inh`; that case stays the documented Phase 6 gap.

### 4. What Phase 6 changes, without migration

Phase 6 evaluates formulas and fills `V`; `Cell` gains nothing. Edits may
then write formula-only and let recalc fill the cache, and
`scale_geometry` can be replaced by recalc-driven geometry. Both are
behaviour changes with no data change, so no document version is needed.

## Consequences

- Every saved file is self-consistent for our reader and for Visio, with
  or without the engine.
- The edit surface is one write API (`write_cell`, `write_row_cell`) with
  a `CellWrite` value; commands never touch `Sheet` internals.
- Stale dependent cells are a known, documented state until Phase 6; the
  formula cross-check test (Phase 6) measures the disagreement.
- Resizing a master instance changes the instance's own rows, which makes
  the file bigger than Visio's formula-based one. Accepted for the alpha.

## Alternatives considered

- **Cached value only.** Rejected: Visio recalculates on open and the
  drawing silently changes; the file would not re-open "identically".
- **Formula only, no cache.** Rejected: our own renderer and the Phase 2
  round-trip read the cache; a file with no `V` renders blank today.
- **Recompute dependents with a mini-evaluator now.** Rejected: that is
  Phase 6, and a partial evaluator that is later replaced invites
  divergent results.
- **Flatten master geometry into every instance on edit.** Rejected: it
  breaks the master link semantics and the Phase 6 cross-check target
  (resized master-based shapes recompute geometry).
