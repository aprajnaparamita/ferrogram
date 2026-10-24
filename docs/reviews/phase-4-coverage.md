# Phase 4 coverage audit (P4-12)

Date: 2026-10-17. Role: coverage-auditor (read-only). No code changed.

Command: `cargo xtask coverage` (workspace, all features) on branch
`agent/P4-12`, worktree at `/Volumes/Dara/larrygram/worktrees/P4-12`.
Line detail from `cargo llvm-cov report --lcov`.

Caveats (same as the [Phase 3 audit](phase-3-coverage.md)):

- The percentages count inline `#[cfg(test)]` code as instrumented lines, so
  a few uncovered lines listed below are test panic arms, not product code.
  They are marked where relevant.
- The gate compares with `EPSILON = 0.05` (`xtask/src/coverage.rs`), so a
  ratchet value written with one decimal still passes when the measured
  value rounds up to it.

## Per-crate summary

All crates are at or above their threshold and at or above their ratchet.
No blocker.

| Crate | Lines | Covered | Coverage | Threshold | Ratchet (Phase 3) | Delta | Verdict |
|---|---|---|---|---|---|---|---|
| larrygram (app) | 250 | 200 | 80.0% | 50 | 66.6 | +13.4 | pass |
| larrygram-cli | 1266 | 1225 | 96.8% | 50 | 96.8 | 0.0 | pass |
| larrygram-commands | 1873 | 1821 | 97.2% | 85 | — | — | pass |
| larrygram-control | 0 | 0 | n/a | 50 | — | — | pass (placeholder) |
| larrygram-geom | 1677 | 1677 | 100.0% | 85 | 100.0 | 0.0 | pass |
| larrygram-model | 7700 | 7676 | 99.7% | 85 | 99.7 | 0.0 | pass |
| larrygram-render | 6601 | 6501 | 98.5% | 75 | 98.4 | +0.1 | pass |
| larrygram-shapesheet | 45 | 45 | 100.0% | 85 | 100.0 | 0.0 | pass |
| larrygram-text | 1768 | 1748 | 98.9% | 75 | 98.8 | +0.1 | pass |
| larrygram-ui | 5434 | 5149 | 94.8% | 50 | — | — | pass |
| larrygram-ui-spec | 1884 | 1872 | 99.4% | 85 | 99.3 | +0.1 | pass |
| larrygram-vsd-binary | 0 | 0 | n/a | 85 | — | — | pass (placeholder) |
| larrygram-vsdx | 7233 | 7085 | 98.0% | 85 | 98.0 | 0.0 | pass |
| larrygram-web | 140 | 109 | 77.9% | — | 0.0 | +77.9 | pass |
| xtask | 9744 | 8956 | 91.9% | — | 91.6 | +0.3 | pass |

`larrygram-control` and `larrygram-vsd-binary` have zero instrumented lines
(an empty placeholder passes by the `coverage.toml` rule). Their thresholds
become meaningful when their phases land (control: Phase 5+, vsd-binary:
Phase 7).

## Critical uncovered paths

### 1. Registry dispatch (`crates/commands`)

- `crates/commands/src/lib.rs:92-95` — `CommandId::intern`, the poisoned
  pool-lock branch. The same for `lib.rs:102-105`, the intern-pool-full
  rejection (`INTERN_CAP`). Both are hostile-input-adjacent error paths.
- `crates/commands/src/lib.rs:155, 170` — two `validate_id` reject arms
  (id-shape validation) never fire in tests.
- `crates/commands/src/builtins/mod.rs:183-185` — `meta_for` when the
  builtin id does not parse (the `RegistryError::BadId` mapping).
- `crates/commands/src/builtins/mod.rs:238-240` — `no_document`, the
  "no open document" error constructor body.
- `crates/ui/src/shell.rs:659-663, 680` — `Shell::dispatch_intent` retry
  and disabled-command error arms; `shell.rs:588-590` — `keep`'s error
  sink.

### 2. `NeedsInput` flow (pickers and dialogs)

- `crates/ui/src/dialogs.rs:125-128, 130-143` — the `NativeFilePicker`
  `open`/`save` bodies, including the save-write error. `dialogs.rs:148-162`
  — `open_dialog` (extension filter, suggested path). These run only in a
  real desktop session (P4-03 recorded this); the flows are tested through
  the fake picker.
- `crates/ui/src/shell.rs:659-680` — the `Ask` match: `Again` (retry with
  the picked path), `Pending` (async wasm pick), `Cancelled`, `NoDialog`
  (the `CommandError::NeedsInput` contract). Partially covered.
- `crates/ui/src/shell.rs:710, 737, 792-793` — `ask_picker` poll fallback,
  the save arm, and `finish_pick`.
- `crates/ui/src/shell.rs:457, 487` — the close-confirmation dialog arms.
- `crates/commands/src/builtins/file.rs:366, 371` — `run_close`'s
  dirty-confirm branches (the `UiRequest::ConfirmClose` contract).

### 3. Layout switching (`crates/ui`)

- `crates/ui/src/shell.rs:515, 518` — the two `view.ui_layout.*` radio
  branches in the View menu; `shell.rs:542, 551` — `menu_item` error and
  disabled-reason arms.
- `crates/ui/src/shell.rs:142-143` — `Shell::new` recording shortcut-load
  warnings.
- `crates/ui/src/tab_bar.rs:408-409, 428` — `show_with` and
  `show_quick_access_with` error propagation; `command bar.rs:451, 458, 461` —
  `show_backstage_with` with no backstage tabs and its fallback.
- The switch itself (`Shell::set_layout_kind`) is covered; only these arms
  around it are not.

### 4. Canvas raster path (`crates/ui/src/canvas.rs`)

- `canvas.rs:708-710` — `rasterize`'s inconsistent-size guard (a corrupt
  raster from the renderer).
- `canvas.rs:803-807` — `render_error`, the `larrygram_render::Error` to
  `UiError::Render` mapping, never fires.
- `canvas.rs:265-270` — the middle-drag / space-drag pan branch of `show`.
- `canvas.rs:622-623, 627` — `wheel_zoom_factor`: `MouseWheelUnit::Page`
  and `Event::Zoom` arms.
- `canvas.rs:188` — `zoom_is_far` with no raster yet.
- `canvas.rs:772, 778` — `tick_positions` non-finite guards;
  `canvas.rs:586` — `viewport_rect` with rulers shown;
  `canvas.rs:678-679` — `intent`'s bad static id mapping.

### 5. Parity derivation (`xtask`)

- `xtask/src/extract/parity.rs:43-50` — `run`: directory-create and atomic
  write failures. `parity.rs:100` — `generate`'s error arm.
- `xtask/src/extract/parity.rs:212-214` — `implemented_ids` when the
  builtin registry does not build (the source of truth for `implemented`
  rows, ADR 0037).
- `xtask/src/extract/parity.rs:272, 279` — `load_command_table`: a
  non-list `commands.yaml` and entries without an id.
- `xtask/src/extract/parity.rs:318-320` — `load_out_of_scope` read error.
- `xtask/src/extract/parity.rs:393, 402-404` — `glob_match` with a bad
  regex and `md_escape` control-character arms.
- `xtask/src/parity.rs:34-36, 51` — the gate-step entry and its error arm.
- The derivation happy path is covered end to end
  (`builds_parity_from_real_layouts`,
  `the_generated_matrix_agrees_with_the_builtin_registry`).

## Other findings

Severity counts: **0 blocker, 6 major, 10 minor**.

### Major

1. **Native picker bodies** (`crates/ui/src/dialogs.rs:125-162`): the real
   `rfd` open/save dialogs and the save-write error never run under test
   (see area 2). A crash here breaks every file open/save on the desktop.
2. **`Ask` flow arms** (`crates/ui/src/shell.rs:659-680, 710, 737, 792-793`):
   the retry, pending and no-dialog arms of the `NeedsInput` contract are
   only partly covered (see area 2).
3. **Shortcut key-token table** (`crates/ui/src/shortcuts.rs:524-618`):
   most `key_token` arms are unexercised (arrow keys, `F1`..`F12`, digits
   `4`..`9`, shifted symbols). A wrong token breaks one binding silently.
4. **Canvas raster failure path** (`crates/ui/src/canvas.rs:708-710,
   803-807`): the corrupt-raster guard and the render-error mapping never
   fire (see area 4).
5. **Gate step runner** (`xtask/src/gate.rs:51-183`, 107 of 150 lines
   uncovered): step selection, `--quick` skip rules and failure aggregation.
   `xtask` has no threshold, but the gate is the phase-exit tool itself.
6. **Parity derivation error arms** (`xtask/src/extract/parity.rs:43-50,
   100, 212-214, 272, 279, 318-320`): every failure branch of the parity
   generator (see area 5).

### Minor

1. `CommandId::intern` poisoned-lock and pool-full branches
   (`crates/commands/src/lib.rs:92-95, 102-105`).
2. Two `validate_id` reject arms (`crates/commands/src/lib.rs:155, 170`).
3. Builtin fallback arms: `meta_for` parse failure
   (`crates/commands/src/builtins/mod.rs:183-185`), `kind_text` default,
   `warning_line` and `value_text` fallbacks
   (`crates/commands/src/builtins/file.rs:612, 652, 684`).
4. `view` error arms: `with_undo`'s no-document branches
   (`crates/commands/src/builtins/view.rs:58, 61`), `positive_cell`'s
   fallback (`view.rs:143`), `move_page`'s no-page and missing-index
   errors (`view.rs:215, 222-223, 226`).
5. `UndoStack`: `undo`'s missing-entry guard (`crates/commands/src/undo.rs:132`)
   and the `Debug` impl (`undo.rs:41-47`).
6. Layout radio menu arms (`crates/ui/src/shell.rs:515, 518, 542, 551`).
7. Command bar nesting arms (`crates/ui/src/tab_bar.rs:592-593, 651-655,
   673-677, 684, 713, 721`): the `MAX_DEPTH` label fallback, the closed
   dropdown (`None`) branch, gallery and caption arms.
8. Canvas formatting fallbacks (`crates/ui/src/canvas.rs:503, 527-530,
   772, 778`): non-finite numbers, page-label fallback, tick guards.
9. `glob_match`'s bad-regex arm and `md_escape`'s control-character arms
   (`xtask/src/extract/parity.rs:393, 402-404`).
10. Denominator noise: inline test panic arms count as instrumented lines
    (e.g. `crates/ui-spec/src/loader.rs:620, 847, 875, ...` are
    `other => panic!(...)` arms in tests). They are not product gaps.

## Proposed ratchet (Phase 5)

The master applies these with `cargo xtask coverage --update-ratchet` and
commits `coverage.toml`. Values are the measured coverage at one decimal
(the tool's own rounding). `larrygram-control` and `larrygram-vsd-binary`
have no ratchet entry (zero instrumented lines).

| Crate | Phase 4 ratchet (proposed) | Previous |
|---|---|---|
| larrygram | 80.0 | 66.6 |
| larrygram-cli | 96.8 | 96.8 |
| larrygram-commands | 97.2 | — (new) |
| larrygram-geom | 100.0 | 100.0 |
| larrygram-model | 99.7 | 99.7 |
| larrygram-render | 98.5 | 98.4 |
| larrygram-shapesheet | 100.0 | 100.0 |
| larrygram-text | 98.9 | 98.8 |
| larrygram-ui | 94.8 | — (new) |
| larrygram-ui-spec | 99.4 | 99.3 |
| larrygram-vsdx | 98.0 | 98.0 |
| larrygram-web | 77.9 | 0.0 |
| xtask | 91.9 | 91.6 |

Note: `larrygram-vsdx` measures 97.954%, so a ratchet of 98.0 leaves only
0.046 of margin against the 0.05 epsilon. If Phase 5 touches `vsdx`, write
97.9 instead.

## Verdict

**pass** — 0 blocker. Every crate clears both its threshold and its
ratchet. The majors above are coverage gaps, not threshold failures; they
belong in the Phase 5 fix pass (file picker tests behind a seam, key-token
table tests, canvas raster error tests, gate-runner and parity error-arm
tests).
