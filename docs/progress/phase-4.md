# Phase 4 report: App shell and switchable command bar

Phase 4 builds the desktop and web app shell around one command registry. The command bar is generated from the committed Visio 2016 and Microsoft 365 layouts and switches without a restart. The canvas shows the Phase 3 raster, and there are read-only panes, original stencils, keyboard shortcuts, and file open, save and export. Started 2026-10-16, closed 2026-10-18, tag `phase-4`.

## What was built

| Area | Content | Tasks |
|---|---|---|
| `crates/commands` (L4) | One command registry (ADR 0028): `CommandId`, `Intent`, `Source`, `Invocation`, `Outcome`, `UiRequest`, `CommandError` including `NeedsInput`. `AppCtx` and `DocumentSession` with a session `id` (ADR 0029). An undo stack with a capped capacity. 24 builtin ids for file, view, page, window and edit actions, and a stub for every other `commands.yaml` row. The Options, Quit and `view.units` commands. | P4-02, P4-15 |
| `apps/larrygram-cli` | Runs on the registry (programmatic source, no dialogs). | P4-02 |
| `crates/ui` shell (L6) | `Shell::show_in`, menus, status bar and dispatch. Every command error goes to the status bar as one line. Dialogs, downloads and quit run only for interactive sources. Edits go onto the undo stack of the session the command ran against. Settings are persisted as a JSON slot (`TextStore`), and Options dispatches registry commands (ADR 0032 amended). Native and wasm file pickers sit behind a testable seam. | P4-03, P4-16 |
| Command bar (L6) | Generated from `ui-spec` layouts (ADR 0031) in both layouts. Switching uses `view.ui_layout.*` on the next frame. Backstage and QAT are included. | P4-04 |
| Canvas (L6) | Phase 3 PNG raster as a texture (ADR 0033), zoom and pan, grid and rulers in the chosen units. The raster cache is keyed by snapshot, page and zoom, and a failed key is not retried every frame. | P4-05, P4-16 |
| Panes (L6) | Read-only Shape Data, Size & Position, and page panes (ADR 0035). | P4-07 |
| Stencils (L6) | 12 + 9 original masters in the Shapes window (ADR 0034), with previews rendered from our own scene. | P4-06 |
| Shortcuts (L6) | A chord-sequence trie from `ui-spec` shortcuts (ADR 0036). Bindings are scope-aware (Text > Selection > Page > CommandBar > Window > Global), and egui keeps its own text-editing chords. A mismatching chord starts a new sequence. The table has 74 bindings. | P4-08, P4-14, P4-16 |
| Apps (L7) | `apps/larrygram` (eframe desktop, `--ui-layout 2016\|365`) and `apps/larrygram-web` (wasm). | P4-03 |
| Parity | `cargo xtask parity` derives `docs/parity/controls.md` from the layouts and the registry (ADR 0037), and the gate checks that it is fresh (11 gate steps). Current counts: 7 implemented, 685 stub, 14 out-of-scope. | P4-09 |
| Test harness | Phase 4 integration flows, `egui_kittest` interaction tests, a corpus open test, insta snapshots, and proptest for settings. A `crates/ui/fuzz` crate (`settings_json`, `shortcuts_map`) with a seed corpus. Gate-runner and parity-error tests in `xtask`. | P4-09, P4-10, P4-16, P4-17 |

## Exit criterion

**Both layouts load and switch without a restart, and opening corpus files shows them on the canvas. Met, with 5 known blank renders.**

- `switching_layouts_from_the_options_dialog_needs_no_restart` (`crates/ui/tests/phase4/interactions.rs`) switches 2016 to 365 from the Options dialog through the registry. The command bar changes on the next frame. The menu and command-bar paths have their own flow tests.
- `LARRYGRAM_REQUIRE_CORPUS=1 cargo test --workspace --all-features` passes (1904 tests, 2 ignored). `apps/larrygram/tests/open_corpus.rs` opens all 15 corpus files through `file.open` and reads the canvas raster.
- 10 of the 15 files paint visible content. 5 files are blank by known cause (`KNOWN_BLANK`/`SKIP_RASTER`):
  - their shape or page cells have no cached values, and the builder substitutes 0;
  - `recursion-cycle.vsdx` has no page part.
  The Phase 6 value pipeline closes this.

## Gate results

```
cargo xtask gate (full, 2026-10-18, main after the P4-16 merge)
  PASS  codegen
  PASS  parity
  PASS  fmt
  PASS  clippy
  PASS  test      (1904 tests; 2 ignored: the fixture regenerator and the oracle smoke test)
  PASS  doc
  PASS  coverage
  PASS  deny
  PASS  layers
  PASS  wasm
  PASS  fuzz      (crates/ui-spec/fuzz, crates/vsdx/fuzz, crates/render/fuzz, crates/ui/fuzz)
gate: GREEN

LARRYGRAM_REQUIRE_CORPUS=1 cargo test --workspace --all-features: OK
```

## Coverage

| Crate | Lines | Coverage | Threshold | Phase 3 ratchet | New ratchet |
|---|---|---|---|---|---|
| larrygram-geom | 1677 | 100.0% | 85 | 100.0 | 100.0 |
| larrygram-model | 7700 | 99.7% | 85 | 99.7 | 99.6 |
| larrygram-ui-spec | 1884 | 99.4% | 85 | 99.3 | 99.3 |
| larrygram-text | 1768 | 98.9% | 75 | 98.8 | 98.8 |
| larrygram-render | 6601 | 98.5% | 75 | 98.4 | 98.4 |
| larrygram-commands | 2155 | 98.2% | 85 | — | 98.1 |
| larrygram-vsdx | 7233 | 98.0% | 85 | 98.0 | 97.9 |
| larrygram-cli | 1266 | 96.8% | 50 | 96.8 | 96.7 |
| larrygram-ui | 6065 | 96.5% | 50 | — | 96.4 |
| larrygram-shapesheet | 45 | 100.0% | 85 | 100.0 | 100.0 |
| xtask | 10245 | 93.3% | — | 91.6 | 93.3 |
| larrygram | 250 | 80.0% | 50 | 66.6 | 80.0 |
| larrygram-web | 140 | 77.9% | — | 0.0 | 77.8 |

`cargo xtask coverage --update-ratchet` rounds down to one decimal place. For this reason the model, vsdx and cli ratchets are 0.1 below their displayed values. The percentages include inline `#[cfg(test)]` code as instrumented lines (see the caveats in [the coverage audit](../reviews/phase-4-coverage.md)). `larrygram-control` and `larrygram-vsd-binary` are still empty placeholders.

## Reviewer findings and resolution

| Report | Verdict | Blocker / major / minor | Resolution |
|---|---|---|---|
| [Test pass](../reviews/phase-4-tests.md) (P4-10) | — | one product bug | `CommandId::intern` leaked every rejected id; fixed in P4-10. The dropped mismatch chord was fixed in P4-16. |
| [Code review](../reviews/phase-4-review.md) (P4-11) | approve with fixes | 0 / 4 / 12 | P4-15 (commands half) and P4-16 (shell half). Minor 9 was resolved by keeping the context menus parked. |
| [Coverage](../reviews/phase-4-coverage.md) (P4-12) | pass | 0 / 6 / 10 | P4-16 (majors 1–4) and P4-17 (majors 5–6, minor 9). |
| [Clean-room](../reviews/phase-4-cleanroom.md) (P4-13) | pass | 0 / 0 / 1 | `ATTRIBUTION.md` rows for the ui snapshots and the proptest regression files, fixed on main. |

## Fixes made on main by the master

- P4-14 let bound `Ctrl` chords dispatch inside text edits, which broke copy, paste and undo in dialog fields. egui's own editing chords now defer to egui again (ADR 0036 amended).
- The P4-07 shell frame test was tautological. It now asserts that a real frame raises no widget stub error.
- zip 9 upgrade for RUSTSEC-2026-0367..0369. zip 8.6 remains only through calamine in xtask, which never calls `extract()`, and is ignored in `deny.toml` (ADR 0014).
- RUSTSEC-2026-0370..0372 (zbus 5 family, unmaintained, no fix) come in under eframe's Linux accessibility bridge. They are ignored in `deny.toml` with that reason.
- A P4-17 gate test printed a fake `gate: GREEN` into the real gate output. The test now writes to a buffer (`gate::run_to`).
- `ATTRIBUTION.md` rows were added for the P4-13 finding.
- The ADR 0036 sketch was updated to the shipped `resolve` signature (`&ActiveScopes`, `Result<Vec<Intent>, UiError>`).

## Decisions made during the phase

- **One registry for every source:** the menus, command bar, shortcuts, CLI and Options dialog all dispatch command ids. Programmatic sources never open a dialog, download or quit. The shell acts on `UiRequest` only for interactive sources.
- **No shell intercepts:** quitting and Options are registry commands (`app.quit`/`file.exit` → `UiRequest::Quit`, `app.office.center.options` → `UiRequest::ShowOptions`).
- **Settings:** Options returns intents. Grid and rulers in `Settings` are the defaults for new sessions, and `ViewState` is seeded from them.
- **Shortcut scopes:** the conflict key is sequence + scope. The most specific active scope wins, and an exact repeat is dropped silently (ADR 0036 amended).
- **Parity is derived, not hand-written:** out-of-scope beats implemented (ADR 0037 amended).
- **Original stencils only:** every master is our own drawing (ADR 0034).

## ADRs added

- 0028 command registry and invocation
- 0029 application state
- 0030 GUI stack (amended: dependency table)
- 0031 command bar from ui-spec
- 0032 layout switching and settings (amended: Options dispatches commands)
- 0033 canvas raster backend
- 0034 original stencils and the Shapes window
- 0035 Shape Data and Size & Position panes
- 0036 shortcuts and file I/O (amended: scope-aware conflicts, egui text chords, `resolve` signature)
- 0037 Phase 4 test strategy and parity (amended: out-of-scope beats implemented)

## Known issues

- **The app is a viewer, not an editor.** 685 of 706 command bar controls are stubs. Selection, shape editing and drop-to-page are Phase 5 work.
- **CI has still never run** (no remote).
- **Rendering:** 5 of 15 corpus files are blank because of missing cached values (Phase 6). The canvas compares a document snapshot every frame because `DocumentSession` has no change counter.
- **Command bar data:** the extractor emits nine `TabSave` placements. The 365 backstage has 205 placements for 127 unique ids.
- **Shell:**
  - pane warnings appear as a "Warning" row, not in the status bar;
  - focus scope follows pointer presses only;
  - the context menus are parked;
  - the rfd dialog calls are untested (desktop only).
- **Commands:**
  - the parity matrix has no table for command-only ids;
  - the `with_undo` placeholder stack has zero capacity;
  - `file.open` does not update the recent files;
  - a wasm `file.save` with a `path` argument fails.
- **Tests:** the 4096-id global intern pool is a hazard in tests. The `render` and `vsdx` fuzz seed corpora are flat, not stored per target.
- The rest of the deferred items are in `docs/STATUS.md`.
