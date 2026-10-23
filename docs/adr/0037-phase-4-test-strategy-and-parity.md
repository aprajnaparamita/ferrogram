# 0037: Phase 4 test strategy and the parity matrix

- Status: accepted
- Date: 2026-10-16
- Task: P4-01

## Context

Phase 4's exit criterion: "both layouts load and switch without a restart,
and opening corpus files shows them on the canvas." KICKOFF asks for
`egui_kittest` tests of command bar generation, layout switching,
enabled/disabled state and shortcut dispatch. AGENTS.md section 7 wants the
`ui` and app crates at 50% coverage, and KICKOFF wants
`docs/parity/controls.md` regenerated every phase. ADR 0027 covered the
Phase 3 render tests; the UI adds a new kind of test.

## Decision

### UI tests: `egui_kittest` (L6/L7 only)

All UI tests live in `crates/ui/tests/` and `apps/larrygram/tests/`
(P4-09 writes them) and drive the real widgets through
`egui_kittest::Harness`:

1. **Command bar generation:** for each layout, a test walks
   `ui_spec::Layout` and asserts the rendered command bar contains one widget
   per placement (tabs, groups, controls, backstage tabs, context menus,
   QAT), by accessible name. Counts are pinned per layout (snapshot via
   `insta`), so an extractor change that adds a control shows up as a
   snapshot diff, not a silent gap.
2. **Layout switching:** start with 2016, dispatch
   `view.ui_layout.panel_365`, assert the same frame shows 365 tabs and
   the harness never restarted (same `Context`, same `AppCtx`).
3. **Enabled/disabled:** a stub control renders disabled with the
   "Not yet implemented" tooltip; an implemented, enabled command renders
   enabled; `Enabled::no(reason)` greys out with the reason as tooltip.
4. **Shortcut dispatch:** synthesize key events for a sample of
   `shortcuts.yaml` entries (single-chord and one multi-chord sequence)
   and assert the registry receives the right `Intent`; assert egui text
   editing keeps Ctrl+C inside a focused text edit.

Widget tests that need no rendering use plain `egui::Context` (allowed,
lighter).

### Headless command tests

`crates/commands/tests/` (P4-02, P4-09): every registered command runs
through `Registry::execute` with `Source::Test`: stub commands return
`Unsupported`, file commands round-trip a temp file, view commands mutate
`ViewState`, `NeedsInput` comes back when an argument is missing. Undo
properties arrive with Phase 5.

### The exit-criterion test: corpus files on the canvas

`apps/larrygram/tests/open_corpus.rs` (P4-09): for each corpus `.vsdx`
(15 files, `corpus/PROVENANCE.md`), run `file.open` through the registry in
a `Shell`, render one frame with `egui_kittest`, and assert:

- the command returns `Ok` (or a documented per-file `skip_reason`);
- the canvas widget rasterized a non-blank texture (the `RasterImage` has
  at least one non-background pixel), and the raster matches
  `render::render_raster` of the same page scene within the ADR 0027
  metric.

The test reads `corpus/` through the worktree symlink and is
`#[ignore = "needs corpus"]` unless `LARRYGRAM_REQUIRE_CORPUS=1`
(the Phase 3 convention).

### Coverage

`crates/ui` and the app crates reach 50% through the harness tests
(coverage counts `src/`; the harness exercises `shell`, `tab_bar`,
`canvas`, `panes`, `shortcuts` together). `crates/commands` keeps its 85%
threshold with headless tests. No threshold change is proposed in this
phase; the ratchet is updated at the phase end by the master.

### The parity matrix

`docs/parity/controls.md` status becomes **derived, not hand-maintained**:

- `implemented` when the control's `CommandId` (its `command` link, else
  the id's `commands.yaml` entry) resolves to a registry command with
  `CommandStatus::Implemented`;
- `out-of-scope` when the Fluent id is listed in `ui-spec/out-of-scope.txt`;
- `stub` otherwise.

When a control is both listed out of scope and backed by an implemented
command, `out-of-scope` wins: the list records a product decision, and
an implemented command does not reverse it (added at the P4-09 merge).

The generator stays in `xtask/src/extract/parity.rs` but gains the
implemented set from `larrygram_commands::builtins::registry()` (xtask is
L9 and may depend on it). A new gate step **`parity`** runs
`cargo xtask parity --check`, which regenerates to a buffer and fails when
`docs/parity/controls.md` differs (same freshness contract as
`codegen --check`). Unlike `cargo xtask extract`, it needs no
`reference/`, so the gate can enforce it everywhere including CI.

## Consequences

- The gate proves the exit criterion on every run (the corpus test is
  `LARRYGRAM_REQUIRE_CORPUS`-guarded for CI without the corpus; the
  master runs it with the corpus at each phase end).
- The parity matrix is honest by construction: it cannot claim
  `implemented` for a stub, and it flips to `implemented` in the same
  commit that registers the handler.
- `xtask` gains a dependency on `larrygram-commands` (fine at L9) and one
  gate step line.

## Alternatives considered

- **Snapshot-only command bar tests (no AccessKit walk).** Rejected: widget
  names and enabled state are exactly what the harness exposes cheaply.
- **A pixel-diff of the whole app window.** Rejected: no fidelity goal in
  this run; the ADR 0027 metric on the canvas raster is the meaningful
  comparison.
- **Parity status hand-edited in `controls.md`.** Rejected: it drifts from
  the code within one phase and lies in the alpha report.
- **Parity check inside `cargo xtask extract` only.** Rejected: extract
  needs `reference/` and runs rarely; a stale matrix would survive whole
  phases.
- **GUI tests in a real window (winit test harness).** Rejected:
  `egui_kittest` is headless and CI-safe; a window test needs a display.
