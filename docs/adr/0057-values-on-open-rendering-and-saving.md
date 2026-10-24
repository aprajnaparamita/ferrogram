# 0057: Values on open, in rendering and on save

- Status: accepted
- Date: 2026-10-21
- Task: P6-01

## Context

Two questions the planning brief asks: does the app trust the values a
file caches or re-evaluate them on open, and what does the writer emit
for a cell the engine recomputed. Both touch the Phase 2/3 rules: the
renderer reads cached values through `CellReader` (ADR 0021), the writer
patches the source trees and must stay a pure function of the model (ADR
0048), and five corpus files render **blank** today because their cells
have formulas but no cached values at all (P4-09: `FillPattern`,
`LinePattern`, `LineWeight`, `Width`, `Height`, `PinX`, `PinY` missing on
`dwg`, `fdo86664`, `testfile5`, `tab-short-prefix`).

## Decision

### On open: trust the cache, fill only what is missing

A file's cached values are Visio's own last computation and are what the
cross-check measures against (ADR 0059), so a full recalculation on open
is wrong twice over: it would rewrite every file on load and it would
erase the disagreement the phase report is supposed to measure.

Instead, after a document is read and before its session is created, one
**fill-in pass** runs with `fill_missing_only`: it evaluates only cells
that have a formula and **no** cached value, and writes the values it
computes. That is exactly the five blank corpus files (P4-09 closes),
it changes nothing that had a value, and it runs at load, so it takes no
undo step and does not bump the change counter.

### In rendering: the local snapshot wins for inherited cells

`CellReader`'s value policy (ADR 0021) is amended with ADR 0054's rule:
for a cell whose own entry has a cached value (including an `Inh`
snapshot), that value is what the reader returns; the fallback chain
runs as before (defining rung's cached value, documented default, hard
default) and provenance still comes from `Document::resolve`. This is the
change that makes a resized instance in a corpus file render at its own
size (P3-15's "inherited geometry on resized instances") and it needs no
engine at render time: the scene builder and `render` are unchanged.

The warning policy of ADR 0021 is unchanged: a missing value still
substitutes and warns; `Inh` snapshots no longer warn (they are values
now).

### On save: the writer writes what the model holds, and nothing else

- `F` is written exactly as stored (a materialised formula of ADR 0056
  included); the writer never rewrites formula text.
- `V` is written from the model's cached value, `U` from the cell's unit
  spelling. The typed mapping is `Value::to_cached` (ADR 0052, P2-14
  closed): canonical magnitude plus the unit the file used.
- **The writer never recalculates.** Recalc is an editing act with undo
  (ADR 0056); making `write_document` a pure function of the model is
  what keeps the ADR 0048 round-trip identity (`read(write(read(write(d))))`
  equals `read(write(d))`) intact — a recalc between write and read would
  move `V` and break it.
- The two P5-10 reader artefacts (`U="BOOL"`/`U="STR"` kept in
  `Preserved`, the `<Rel>` child kept in `Preserved`) may be removed by
  the task that fixes the writer's kind spelling, **with** the round-trip
  tests that prove the removal is safe; until then they stay.

## Consequences

- The five blank corpus files render after a plain `file.open`, with no
  special cases in the renderer and no `SKIP_RASTER`/`KNOWN_BLANK` in the
  exit harness (P4-09's markers go when P6-08 lands).
- An untouched file saves byte-identical (the writer is deterministic),
  and a file the engine touched saves the values it computed — which is
  what Visio does too.
- The phase report's agreement rate stays meaningful: opening a file does
  not silently overwrite the numbers being compared.

## Alternatives considered

- **Full recalc on open.** Rejected (see above): it hides disagreements,
  rewrites files and costs O(document) per open.
- **Recalculate at render time, lazily, per cell.** Rejected: render cost
  becomes unpredictable, warnings get duplicated per frame, and a cycle
  in the dirty path would land inside a draw call.
- **Write the recalculated value but not the materialised formula.**
  Rejected in ADR 0056: the lookup would ignore it and the file would be
  self-inconsistent.
- **Recalculate in the writer so a saved file is always fresh.** Rejected:
  it breaks the ADR 0048 identity and turns a save into an edit.
- **Keep the `SKIP_RASTER`/`KNOWN_BLANK` markers and fill lazily in the
  app.** Rejected: the fill-in pass fixes the files for every consumer
  (CLI, tests, panes) at one point.

## Amendments

### Amended 2026-10-22, P6-15: the blank files were a style-inheritance gap

The first Consequences bullet (and the Decision's claim that the fill-in
pass is "exactly the five blank corpus files") assumed the missing
cached values were what made the files render blank. Wrong premise: P6-08
scanned every corpus cell and found **no** cell with a formula and no
cached value, so the fill-in pass is a no-op on the current corpus. What
the blank files lacked was style inheritance: their master instances
took no `FillStyle`/`LineStyle`/`TextStyle` from their master, so
`FillPattern`/`LinePattern` fell through to the hard default 0 (P6-08
open issues 1 and 6). The lookup now inherits the master's style links
(ADR 0013 amendment P6-15); `dwg`, `fdo86664` and `testfile5` render
non-blank and left the harness's `KNOWN_BLANK`. The two files that stay
marked (`tab-short-prefix`, `recursion-cycle`) have nothing to draw by
construction and their markers say so. The fill-in pass stays as
written: it is still the right rule for a file whose cells do lack
cached values, and it still changes nothing that had one.
