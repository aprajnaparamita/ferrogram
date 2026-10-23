# 0056: Edits with formulas (supersedes 0038)

- Status: accepted
- Date: 2026-10-21
- Task: P6-01

## Context

ADR 0038 ("editing without a formula engine") defined what a move, a
resize or a cell edit writes while nothing recomputed anything, and named
this phase as its replacement. Now the engine exists (ADRs 0051–0055):
a resize must recompute geometry instead of scaling it literally (P5-02's
`scale_geometry`), a cell edit must fill in the cached value, a
`GUARD`-ed formula must survive the edits it protects against, and undo
must put back exactly what the pass changed. Everything still runs
through one seam (`AppCtx::push_edit`, ADR 0040) and a failed mutation
still changes nothing (ADR 0039).

## Decision

### What each edit writes

- **Move** writes `PinX`/`PinY` as literal values (as ADR 0038 rule 1),
  then recalc updates the cells whose formulas read them.
- **Resize** writes `Width`/`Height` as literal values and lets recalc
  compute `LocPinX`/`LocPinY`, `Txt*` and every geometry row from their
  formulas. `Document::scale_geometry` is **retired** (task P6-07 removes
  its callers; the model function stays until a later cleanup task).
- **Rotate** writes `Angle` (a delta, P5-04's semantics) and recalc.
- **A cell edit** writes the user's formula (or literal) exactly as
  typed; recalc fills the cached value the same pass.
- **Structural edits** (insert, delete, paste, group) dirty the cells
  they add or remove and let recalc settle dependents.

### Recalc runs inside the undo step

`AppCtx::push_edit` is still the only push path (ADR 0040). Its Phase 6
extension: after `Edit::apply` (and after `Edit::revert`) the dispatcher
recalculates the cells the step reports ([`Edit::dirty_cells`]) and their
dependents (ADR 0055), then pushes **one** step: the original edit plus a
`RecalcEdit` holding every `(cell, before, after)` the pass changed. Undo
runs both halves backwards and restores the old values **by snapshot, not
by recomputation** — recomputing on revert would not restore what the
user actually had (an unknown function's cached value, a cycle's kept
values) and is not deterministic against them.

A failed recalc keeps the ADR 0039 rule: if the edit itself fails,
nothing is written and nothing is recalculated; if the pass truncates at
its budget (ADR 0055) the edit stands and the outcome says which cells
are stale.

### What recalc writes: values, and materialised formulas for inherited cells

A cell that has its own formula gets only its cached `V` updated. A cell
whose formula is `Inh` (or that has none) gets the evaluated formula and
value written on the shape itself — **materialised** — because an `Inh`
cell does not define anything for the lookup (ADR 0013) and a value
written on it would be invisible to the renderer. Materialisation is the
honest state (the file contains what it draws), it is what
`scale_geometry` already did, and it happens **only to cells the pass
recomputed**: an untouched instance keeps its `Inh` rows and its own
snapshot value (ADR 0054). The writer emits the materialised formula and
value like any other cell (ADR 0057).

### GUARD and SETF

- **`GUARD(x)`** protects a formula from being *replaced by another
  action*: a move, resize, style apply or any programmatic write that
  would overwrite a `GUARD`-ed cell is skipped for that cell (the write
  records a `Guarded` warning and the cell keeps its formula). A direct
  user edit of that cell's formula replaces it, `GUARD` and all.
  `GUARD` does not stop recalculation: the guarded formula still computes
  and its value is still written.
- **`SETF(target, value)`** (basic form) writes `value` into `target`
  when the cell carrying it is evaluated. The write is an ordinary cell
  write in the same pass: it dirties `target`, and the pass continues
  over the newly dirty dependents within the same budget. `SETF` of a
  cell that is itself being evaluated in the same cycle is cut by the
  cycle rule (ADR 0055). The `SETATREF` family is not evaluated (ADR
  0053) and keeps its cached value.

### One write path for `SETF` (P6-17 amendment)

The evaluator never writes the document. Every `SETF` write — the
formula's own or one inside a **referenced** cell's formula that a read
re-evaluated inline — is **recorded** during evaluation and applied by
the recalc pass through the model's mutation API, next to the pass's own
value writes. One write path owns the document: every write gets its
`CellUpdate`, dirties its dependents within the same budget and sits
inside the snapshot `RecalcEdit` takes, so undo restores the target.

Two rules complete the picture:

- **A standing-back evaluation writes nothing.** When an evaluation
  stands back (an unknown function, a formula the parser refuses), the
  cell keeps its cached value and every `SETF` write recorded during
  that evaluation is **dropped**: a cell that keeps its value has no
  side effects.
- **The budget counts nested work.** A read of a cell that caches
  nothing re-evaluates its formula inline (ADR 0057's fill-in case);
  each such nested evaluation counts against
  `RecalcOptions::max_evaluations`, so the budget bounds the pass's
  total work, not just its top-level calls.

### Connector re-routing versus formula geometry (ADR 0045)

Re-routing owns a connector whose path is **literal** (the Phase 5
router writes literal geometry rows, ADR 0045). A connector whose
geometry rows carry formulas (a master-driven dynamic connector) is
owned by recalc: `reroute_after_edit` skips it and recalc computes the
rows. The two never write the same row. Which corpus connectors are
which is a black-box question; until it is
answered, the rule is: formula present, recalc owns it; no formula, the
router owns it.

## Consequences

- A resized master instance draws at the new size with geometry that came
  from the master's formulas (the phase exit criterion), and the file
  says so.
- One user action is still one undo step, now including the recalculated
  cells; the undo property test of ADR 0040 keeps working unchanged
  because revert restores snapshots.
- `scale_geometry` and its non-uniform-resize distortions (P5-02) go
  away; only the documented geometry rows it dropped (`NurbsTo` E,
  `PolylineTo` A) remain a gap until the evaluator covers those rows'
  formulas.
- Files grow slightly on edit (materialised rows), the same trade-off
  ADR 0038 rule 3 already accepted.

## Alternatives considered

- **Keep `Inh` and write only the snapshot `V`.** Rejected: the lookup
  ignores an `Inh` cell's value for provenance and the writer's
  round-trip would keep a stale formula beside a fresh value. (ADR 0054
  uses snapshots for *reading*; writing them back without materialising
  would be a second, invisible state.)
- **Recompute on revert instead of snapshotting.** Rejected: not
  deterministic against cached values of unknown functions and cycles
  (see above).
- **A separate recalc undo step per pass, outside the command's step.**
  Rejected: two undo entries per user action, and the second one's label
  is meaningless to the user.
- **Keep `scale_geometry` as a fallback for unimplemented rows.**
  Rejected: two resize paths that disagree are worse than one honest gap;
  the gap is listed in the task file's acceptance criteria.
- **Evaluate `SETATREF`/`GETREF` now.** Rejected (ADR 0053): they are
  interactive edit plumbing, and a wrong evaluation would corrupt cells
  during ordinary edits.

## Amendments to earlier ADRs

- **ADR 0038 is superseded** by this one. Its rule 1 (write formula and
  value) stays in force for edits recalc does not cover; rule 2
  (dependents stay stale) and rule 3 (`scale_geometry`) are replaced.
- **ADR 0013 is amended** by ADR 0054 (an `Inh` cell's snapshot is the
  preferred value; provenance still comes from the formula rung).
- **ADR 0045 is clarified**: formula-driven connector geometry belongs to
  recalc, literal geometry to the router.
