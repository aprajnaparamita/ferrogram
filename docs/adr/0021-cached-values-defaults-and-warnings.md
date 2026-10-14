# 0021: Cached values, defaults and warnings

- Status: accepted
- Date: 2026-10-14
- Task: P3-01

## Context

Phase 3 renders from **cached values only**: no formula evaluation until
Phase 6. Files cache values at save time, so this covers everything Visio
last computed. But a cached value can be missing (the cell has only a
formula), an error (`E='#VALUE!'`), of the wrong kind, or the inherit
marker `F='Inh'` whose `V` is only a snapshot (ADR 0013 amendment). The
renderer must do something predictable in each case, tell the caller, and
never fail a render because one cell is bad.

## Decision

### One facade: `ferrogram_model::CellReader`

Every cell the renderer reads goes through `CellReader` (L1, in `model`,
because it needs the lookup of ADR 0013):

```rust
pub enum SheetRef { Shape(ShapeId), Page(PageId) }

pub enum WarningKind {
    Missing,                    // nothing defines the value
    ErrorValue(String),         // cached error, e.g. "#VALUE!"
    KindMismatch,               // cached value has another kind
}

pub struct CellWarning {
    pub sheet: SheetRef,
    pub cell: CellName,
    pub kind: WarningKind,
    pub substituted: CellValue, // what was used instead
}

pub struct CellReader<'d> { /* document + collected warnings */ }
impl<'d> CellReader<'d> {
    pub fn new(doc: &'d Document) -> Self;
    pub fn number(&mut self, sheet: SheetRef, cell: &CellName) -> Result<f64, Error>;
    pub fn angle(&mut self, sheet: SheetRef, cell: &CellName) -> Result<Angle, Error>;
    pub fn boolean(&mut self, sheet: SheetRef, cell: &CellName) -> Result<bool, Error>;
    pub fn text(&mut self, sheet: SheetRef, cell: &CellName) -> Result<String, Error>;
    pub fn color(&mut self, sheet: SheetRef, cell: &CellName) -> Result<Color, Error>;
    pub fn list(&mut self, sheet: SheetRef, cell: &CellName) -> Result<Vec<f64>, Error>;
    pub fn row_number(&mut self, sheet: SheetRef, section: &str, row: usize,
                      cell: &CellName) -> Result<f64, Error>;
    pub fn warnings(&self) -> &[CellWarning];
    pub fn take_warnings(&mut self) -> Vec<CellWarning>;
}
```

### The policy

For each requested value, `CellReader`:

1. Resolves the cell through the lookup (ADR 0013 and its ADR 0019
   extensions). A resolving error (unknown id) is an `Err`, because it is a
   caller bug, not file data.
2. Uses the cached value when it is present, not an error and of the
   requested kind.
3. Otherwise substitutes the **hard default** of the kind (number `0`,
   angle `0`, bool `false`, text `""`, color black, list empty) and records
   a `CellWarning`. The `Inh` snapshot is **not** used (ADR 0013
   amendment): the lookup already continued to the master, style and
   defaults, and a snapshot of those would hide a real resolution bug.

A warning never fails the build: `build_scene`, `layout` and the renderers
return `Ok` with warnings collected in `Scene::warnings` (text warnings in
`Layout::warnings`).

### How warnings reach the caller and the CLI

- `Scene::warnings` and `Layout::warnings` carry them to the caller.
- The CLI prints one line per warning to **stderr** (`warning: shape 12
  LineWeight: no cached value, using 0`), keeps rendering, and exits 0.
- With `--strict`, warnings are errors: the CLI still writes the output
  unless rendering failed, and exits with the `warnings` exit code (4,
  ADR 0026).
- `inspect` reports the warning count in its JSON output.

## Consequences

- Rendering a file with one bad cell still produces an image; the exit
  criterion "every corpus file renders without panics" is achievable.
- Provenance is one step away (`resolve` is still public) for debugging.
- Once Phase 6 evaluates formulas, the same facade can prefer an evaluated
  value over the cached one without changing a single call site.
- Warnings are per read; a caller that reads the same cell twice warns
  twice. Fine for Phase 3; a per-shape cache can come with Phase 6.

## Alternatives considered

- **Fail the render on a missing cached value.** Rejected: KICKOFF's exit
  criterion is rendering without panics on real files, and real files have
  gaps (the corpus has `EllipticalArcTo` rows without `C`/`D`).
- **Use the `Inh` snapshot as a fallback.** Rejected for now (ADR 0013
  amendment keeps it out): snapshots are stale data that masks inheritance
  bugs. If corpus evidence shows worse output without it, amend ADR 0013.
- **Return `Result<Option<f64>>` per cell and let every caller decide.**
  Rejected: 420 cell kinds times N callers, each with its own default
  policy; the policy belongs in one place.
- **Warnings as `log` output.** Rejected: the library must not depend on a
  logging facade, and the CLI needs structured warnings for `--strict` and
  `inspect` counts.
