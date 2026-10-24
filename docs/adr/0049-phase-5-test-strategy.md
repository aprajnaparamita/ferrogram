# 0049: Phase 5 test strategy

- Status: accepted
- Date: 2026-10-18
- Task: P5-01

## Context

The Phase 5 exit criterion is a user workflow: "a user can build a
10-shape flowchart with connected shapes across two pages, save it,
reopen it and keep editing." KICKOFF (planning brief decision 12) asks
for headless command tests through the registry, undo property tests,
connector glue tests, `egui_kittest` interaction tests, the scripted
flowchart test, coverage targets per crate, and the parity commands
table (the P4-15 deferred item). ADR 0027 and ADR 0037 cover the Phase
3 and Phase 4 strategies; this ADR extends them.

## Decision

### 1. Headless command tests for every editing command

`crates/commands/tests/editing.rs` (P5-04) and per-module tests run every
registered editing command through `Registry::execute` with
`Source::Test`: `Ok` with an `Edit` for the happy path, `NotEnabled` when
the predicate says so (with its reason), `NeedsInput` for missing args,
`InvalidArgs` for bad values. The test walks `Registry::ids` and fails
when a Phase 5 id in the per-module `TABLE`s has no test entry, so a
registered command cannot ship untested.

### 2. Undo is the inverse (property tests)

`proptest` (ADR 0040 rule 4): random command sequences over a seeded
document; after `undo` of every step the document equals the seed, and
after `undo` + `redo` it equals the post-command state. Comparisons
ignore `changes`, the undo stack and warnings. Each editing command also
gets a hand-written pair test (apply/revert) so a failure names the
command.

### 3. Connector glue tests

`crates/model/tests/mutate.rs` and `crates/commands/tests/connector.rs`
(P5-02, P5-08): glue to a connection point and to an outline, replace an
endpoint's glue, unglue, drop glue when the target is detached, and
re-route after move/resize/undo. The routing math (`larrygram_geom::
routing`) gets unit tests for both styles including degenerate endpoints.

### 4. UI interaction tests (`egui_kittest`)

`crates/ui/tests/phase5/` (P5-11): drag a shape (one undo step), resize
and rotate with handles, marquee selection, snapping (grid and
connection point, Alt to disable), the in-place text editor, and a
drawing tool. Each drives `ToolMachine` events plus a `Shell` frame and
asserts on the registry state and the overlay.

### 5. The scripted exit-criterion flowchart

`apps/larrygram/tests/flowchart.rs` (P5-10), `#[ignore = "needs corpus"]`
style guard not needed (it builds its own document): dispatch
`insert.shape` ten times (process/decision/terminator mix), glue them
into a flow with `connector.*`, add a second page with more shapes,
`file.save` to a temp path, `file.open` it again, assert model equality
of the saved and reopened documents (ADR 0048), then move a shape and
assert the change counter and a successful re-save. The test runs in CI
(only standard temp files) and is the phase's proof.

### 6. Coverage targets

The Phase 2/4 thresholds stay (from `coverage.toml`): geom, model, vsdx,
shapesheet, commands, ui-spec ≥ 85%; render, text ≥ 75%; ui, apps ≥ 50%.
Phase 5 stub code is excluded from nothing: the implementing tasks
replace the P5-01 stub tests with behaviour tests, and the master's
ratchet update happens at the phase end (AGENTS.md section 7). The
audit task (P5-13) names any critical uncovered path (undo failure
paths, glue error paths).

### 7. The parity commands table

`cargo xtask parity` (owner P5-10) gains a second table in
`docs/parity/controls.md`: `| command | label | status |` listing every
registry id, including command-only ids with no Fluent control (the
P4-15 deferred item). The gate's `parity` step checks freshness of both
tables. Ids in the per-module `TABLE`s therefore appear automatically
when they land.

### 8. Housekeeping owned by the test pass (P5-11)

- `crates/ui` gains a `proptest` dev-dependency (the P4-10 note).
- Fuzz seed corpora move to per-target directories (`corpus/<target>/`)
  for `render`, `vsdx` and `ui` (the P4-17 note), and `crates/ui-spec/fuzz`
  gets its missing `!corpus/` rule (P4-10 note).
- Each new fuzz target gets a seed directory with at least one seed
  (AGENTS.md section 7 lesson).

## Consequences

- The exit criterion is enforced by one fast, headless test; the phase
  gate proves it every run.
- Undo correctness is a property, not a hope; regressions name the
  command.
- The parity matrix stops hiding command-only ids, so Phase 8's control
  channel coverage is visible.

## Alternatives considered

- **Golden snapshots of edited documents.** Rejected: model equality is
  exact and diffable; XML goldens would churn on writer formatting.
- **A GUI-scripted flowchart test (egui_kittest driving every gesture).**
  Rejected as *the* proof: it is slow and brittle; the command-level test
  proves the workflow, the UI tests prove the gestures.
- **Coverage thresholds raised in Phase 5.** Rejected: the phase adds
  surface faster than tests; the ratchet and the audit keep the line
  honest.
- **Hand-maintained parity command list.** Rejected by ADR 0037: derived
  or it lies.
