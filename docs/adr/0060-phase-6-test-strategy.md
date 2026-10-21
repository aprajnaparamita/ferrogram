# 0060: Phase 6 test strategy

- Status: accepted
- Date: 2026-10-21
- Task: P6-01

## Context

Phases 1–5 each ended in a test-strategy ADR (0017, 0027, 0037, 0049).
Phase 6's tests are named by KICKOFF (parser property tests and a fuzz
target, function table tests, recalc order tests, the cross-check) plus
the exit criterion: resizing master-based shapes from the corpus
recomputes geometry correctly for supported functions, with the agreement
rate in the phase report (ADR 0059).

## Decision

### Parser (`P6-02`)

- **Property tests** (`proptest`): parse-print-parse is the identity on
  generated trees (spans aside, ADR 0051), and printing a parsed corpus
  formula and reparsing it gives an equal tree.
- **Fuzz target**: `crates/shapesheet/fuzz/` with one target `parse`,
  seeds in `crates/shapesheet/fuzz/corpus/parse/`, and its own
  `.gitignore` holding `!corpus/` (the P4-17 rule). The contract: any
  input, any bytes, never a panic and never more than the limits
  (ADR 0051). The directory is created by P6-02, not by the planning
  task, so the gate's fuzz step starts with that task's hand-off.
- Table-driven cases for every reference form, unit suffix and operator
  precedence combination of ADR 0051, and the three limits.

### Values and units (`P6-03`)

Round-trip tests for `Value::to_cached`/`from_cached` (P2-14), unit
conversion tests per family (canonical in and out), coercion tests
including the documented defaults (`5 * "sheep"` is 0) and the non-finite
rule (a computed `inf` is `#NUM!`, P3-11/P3-16). Property tests: a value
round-trips through the cached form for every kind.

### Function table and evaluator (`P6-04`)

- Table tests: every name is spelled as the ShapeSheet reference spells
  it, arity bounds are sane, and `is_supported` agrees with the table.
- One test per evaluated function family: arithmetic on mixed units,
  comparison coercions, `IF` laziness (only the taken branch evaluates),
  `GUARD` and `SETF` behaviour, `INDEX`/`LOOKUP` bounds (a bad index is
  `#VALUE!`, never a panic).
- The unknown-function contract: a formula with an unknown call keeps the
  cell's cached value and records one warning (ADR 0053).

### Reference resolution (`P6-05`)

Per-form resolution tests of the ADR 0054 table over a small in-memory
document, the inherited-formula rule (a master formula evaluates against
the instance), the `Inh` snapshot rule (ADR 0054 amendment, with the
provenance assertion), the missing-reference rule (`#REF!` + one
warning), and the case-insensitive lookup.

### Dependency graph and recalc (`P6-06`)

Order tests (a diamond must evaluate the shared dependency once, in
order), incremental tests (dirtying one cell recomputes exactly its
dependents), cycle tests (`recursion-cycle.vsdx` produces one `Cycle`
warning and keeps cached values), budget tests (a forced tiny budget
truncates and reports), and idempotence (running recalc twice changes
nothing the second time).

### Edits and undo (`P6-07`)

The ADR 0040 undo property test extended with recalculated cells (run a
command, undo, compare the document including recalculated values),
`GUARD` tests (an indirect write skips the guarded cell), `SETF` tests
(a `SETF` write lands and dirties), and a resize test that no longer
calls `scale_geometry`.

### Open, render, save (`P6-08`)

The fill-in pass test (a document with a formula and no cached value
gains one, with no undo entry), the `Inh` snapshot rendering test (a
resized corpus instance draws at its own size), and round-trip tests for
the writer's kind spelling (P2-14, the P5-10 artefacts).

### Panels (`P6-09`)

`egui_kittest` frames for the Shape Data and Size & Position panels:
typed display, unit conversion on entry, and the read-only rule for
formula cells (ADR 0058).

### Cross-check and exit criterion (`P6-10`)

- The cross-check harness of ADR 0059, in
  `crates/shapesheet/tests/crosscheck.rs`.
- **The exit-criterion test** (`crates/shapesheet/tests/exit_resize.rs`):
  read `corpus/vsdx/blue-box.vsdx` (and `qs-box.vsdx`), pick a master
  instance, write `Width`/`Height` through the model mutation API, run
  recalc, and assert the geometry rows equal the master's formulas
  evaluated at the new size — hand-computed expectations, no cached
  value from the file involved. This is the criterion in code.

### Coverage

`ferrogram-shapesheet` joins the 85 % group with real code (it is already
listed in `coverage.toml`). One request for the master: the crate's
ratchet is 100.0 from Phase 5 (an all-stub crate); Phase 6 gives it real
code, so the ratchet should be reset to the 85 % threshold before the
implementation group starts, then re-raised at the phase end with
`cargo xtask coverage --update-ratchet`. Every interface stub keeps a
test of its "not implemented" contract, so coverage holds while tasks
land (the P6-01 stubs are at 100 %).

### The deferred items Phase 6 touches

| Item | Owner or reason to wait |
|---|---|
| P2-02 table gaps (10 cells, one section, 8 rows with no index) | P6-05 fills them only if a corpus reference needs one; otherwise wait for a source |
| P2-04 lazy defaults / name aliases | P6-05 uses `model::aliases` as-is; moving defaults into codegen waits |
| P2-06b `Inh` cached snapshot | closed by ADR 0054 |
| P2-14 typed values with units | closed by ADR 0052 (`Value::to_cached`), P6-03 and P6-08 |
| P3-04 bundled fonts compiled into every binary | not a formula problem; revisit in the web-build task |
| P3-11 / P3-16 non-finite values and coercions | closed by ADR 0052 (`#NUM!` at the source) |
| P3-15 inherited geometry on resized instances | closed by ADR 0054/0057, tested by P6-08 |
| P4-06 Shape Data values as strings | closed by ADR 0058 (P6-09) |
| P4-09 five blank corpus files | closed by ADR 0057 (fill-in pass, P6-08) |
| P5-02 `scale_geometry` | replaced by recalc (ADR 0056, P6-07) |
| P5-16 connection points on inherited sizes | P6-07: `Document::connection_points` reads values the recalc has filled; a scaling rewrite can wait |
| P5-10 reader artefacts (`U="BOOL"`/`U="STR"`, `Rel` child) | P6-08 may remove them with tests |
| P2-07 deferred writer gaps (deleted part cleanup etc.) | out of scope; the writer changes only its value spelling |
| P2-14 relationships-writer namespace | out of scope for Phase 6 |

## Consequences

- Every task has tests named against real behaviour, and the coverage
  ratchets stay reachable (the P5-13 lesson about unreachable arms).
- The exit criterion has a test that fails when resize recomputes wrong
  geometry, not only a report number.
- The fuzz step starts with P6-02's hand-off: the planning task adds no
  fuzz directory, so its gate run needs no `cargo +nightly fuzz build`.

## Alternatives considered

- **Snapshot tests for the cross-check output.** Rejected: the corpus is
  not committed (ADR 0005) and insta snapshots of numbers invite
  rubber-stamping; the printed block is the report.
- **One big integration test per crate instead of the split above.**
  Rejected: Phase 5 showed that a shared test file makes parallel tasks
  collide.
- **UI snapshot tests of the panels.** Rejected for this phase: the
  panels' contract is typed display and edit dispatch, which egui_kittest
  asserts directly (the existing snapshot files belong to the ribbon).
