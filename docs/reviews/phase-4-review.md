# Phase 4 code review (P4-11)

Task P4-11, reviewer, 2026-10-17, branch `agent/P4-11`. Review of
`git diff phase-3..main` against AGENTS.md section 6, the layering (ADR
0002) and ADRs 0028–0037. The reviewer role is read-only: this task
changed no code.

Scope covered: `crates/commands` (app, error, registry, undo, builtins),
`crates/ui` (shell, ribbon, canvas, dialogs, settings, panels,
shapes_window, shortcuts, icons), `apps/ferrogram`, `apps/ferrogram-web`,
`apps/ferrogram-cli` (the registry dispatch move), the P4-02 extractor
changes (`xtask/src/extract/commands.rs`, `fluent.rs`, `parity.rs`,
`xtask/src/parity.rs`), the P4-05 raster seam (`crates/render/src/raster.rs`),
the P4-06 stencils (`crates/model/src/stencils/`) and the P4-04 embedded
loader (`crates/ui-spec/src/embedded.rs`, `loader.rs`).

## Verdict

**Approve with fixes: 0 blocker, 4 major, 12 minor.** The hard rules
hold: no panic path outside tests, no command opens a dialog (the picker
is reached only from the shell, and only for interactive sources on the
`NeedsInput` path), layering holds (nothing below L6 sees egui/eframe/
winit/rfd), the ADR 0028–0037 interfaces are the ones implemented
(only documented extensions), and the ribbon, shortcut and raster code
is bounded and careful. The four majors are one dead control, one silent
failure path, and two rule seams (registry bypass in the Options dialog,
payload-triggered dialog) that must be closed before Phase 5 spreads
them.

## Checks that passed

- **Panic safety.** `clippy --workspace --all-targets --all-features --
  -D warnings` is clean, so the never-crash lints hold outside tests; a
  manual grep of `unwrap`/`expect`/`panic!` in the new code finds hits
  only inside `#[cfg(test)]` modules. Input-derived work is bounded:
  the ribbon caps control nesting at 8 and claims each placement once
  (`crates/ui/src/ribbon.rs:44`, `:143`), the canvas caps ticks at 4096
  and clamps the raster scale under `MAX_DEVICE_EXTENT`, a 256 MiB byte
  cap and `max_texture_side` with checked arithmetic
  (`crates/ui/src/canvas.rs:730`), `shape_count` walks cycles once
  (`crates/commands/src/builtins/file.rs:619`), the id interner is
  capped at 4096 (`crates/commands/src/lib.rs:60`), and warnings ring at
  200 (`crates/commands/src/app.rs:205`).
- **No dialog from a command (AGENTS.md section 6).** `crates/commands`
  has no rfd/egui dependency at all; `file.open`/`file.save*` return
  `CommandError::NeedsInput`, `help.about` and `file.close` return
  `UiRequest`, and the only `FilePicker` calls are `Shell::ask_picker`
  (gated on `Source::is_interactive`) and `Shell::apply_outcome` (the
  one gap: major 3). Programmatic dispatch (`Registry::execute`,
  `dispatch_one`, the CLI) reaches no dialog.
- **One command registry.** Every user action resolves to a
  `CommandId`: the menus, the ribbon/QAT/backstage, the canvas keys and
  the shortcut map all produce `Intent`s and only `Shell::dispatch_one`
  calls `Registry::execute` (`crates/ui/src/shell.rs:849`). The CLI's
  `Command` enum now dispatches `file.inspect`/`file.render`/
  `file.export` through the same registry (`apps/ferrogram-cli`).
  `builtins::registry()` is the single implementation list
  (`crates/commands/src/builtins/mod.rs:137`), and the parity matrix is
  derived from it (ADR 0037, gate step `parity`).
- **ADR interfaces.** `CommandId`/`Args`/`Invocation`/`Enabled`/
  `Outcome`/`UiRequest`/`Command`/`Registry` (ADR 0028), `AppCtx`/
  `DocumentSession`/`ViewState`/`Settings`/`SettingsStore` (ADR 0029),
  `RasterImage`/`RasterOptions`/`render_raster` sharing the PNG
  rasterizer (ADR 0033, `crates/render/src/raster.rs:69`),
  `StencilDef`/`MasterDef`/`basic_shapes`/`basic_flowchart`/`catalog`/
  `stencil_document`/`instantiate` (ADR 0034), `FilePicker`/`FileChoice`
  (ADR 0036) and `ShortcutMap` (ADR 0036) all match their ADR shapes.
  Deviations are documented where they occur: `ArgValue::Bytes` and
  `Outcome.payload` (wasm upload/download), `Outcome.warnings`,
  `DocumentSession.preserve` (the ADR 0018 patch writer),
  `ShortcutMap::resolve(&ActiveScopes)` and the P4-14 conflict rule
  (both recorded as ADR 0036 amendments), `show_rulers(…, units)`
  (P4-05 note). The registry container is the one silent deviation
  (minor 5).
- **Ribbon (ADR 0031).** One widget per placement path, the
  `(id, label, order)` dedup is exactly the ADR rule, repeats render
  once per occurrence, out-of-scope ids get the out-of-scope tooltip and
  everything unimplemented gets "Not yet implemented", disabled widgets
  carry their `Enabled` reason as the tooltip, and widgets emit
  `Intent`s, never `execute` (`crates/ui/src/ribbon.rs:807`).
- **Shortcuts (ADR 0036 as amended).** The (sequence, scope) conflict
  key, the most-specific-active-scope order, the prefix-deferral with a
  2 second timeout, and the P4-14 rule that egui's own editing chords
  stay with egui inside a text edit are all implemented as amended
  (`crates/ui/src/shortcuts.rs:290`, `:398`). Unknown tokens fail the
  load with a warning fallback, never a silent gap
  (`crates/ui/src/shell.rs:815`).
- **Stencils and assets (AGENTS.md section 4).** The masters are
  hand-authored geometry in `crates/model/src/stencils/`, the two
  `ATTRIBUTION.md` rows land in the same commit as the data, the egui
  chrome fonts have their dependency row and `deny.toml` exception, and
  no Microsoft asset is committed.
- **Extractor (P4-02).** `OUR_COMMANDS` and `FLUENT_COMMANDS` are the
  ADR 0028/0006 "our own ids" rule in code, they claim only ids no
  mechanical derivation produced, and the moved Fluent ids, the
  confidence downgrade and the row-drop rules are documented at
  `xtask/src/extract/commands.rs:566`, `:614` and
  `xtask/src/extract/fluent.rs:44`.

## Findings

### Major

1. **The Options dialog's grid and rulers toggles are dead.**
   `crates/ui/src/settings.rs:145` edits `Settings::show_grid` and
   `Settings::show_rulers`, but the canvas reads `ViewState::show_grid`
   and `ViewState::show_rulers` (`crates/ui/src/canvas.rs:296`,
   `:334`), which `ViewState::new` hard-codes to `true`
   (`crates/commands/src/app.rs:50`) and `DocumentSession::new`
   (`crates/commands/src/app.rs:116`) never seeds from `Settings`. The
   `view.toggle_grid` / `view.rulers` commands touch the per-document
   `ViewState` only (`crates/commands/src/builtins/view.rs:149`,
   `:159`). Toggling "Show grid" in Options therefore changes persisted
   state that nothing reads and has no visible effect. **Fix:** seed
   `ViewState::show_grid/show_rulers` from `Settings` where a session is
   created (`file.new`/`file.open`, `crates/commands/src/builtins/file.rs:240`,
   `:247`), and have the two toggle commands also update `Settings` (or
   drop the `ViewState` copies and give the dialog rows the two commands
   to dispatch).

2. **A failed command from the UI is dropped silently.**
   `Shell::dispatch` returns one `Result<Outcome, CommandError>` per
   intent, but both apps ignore the inner errors
   (`apps/ferrogram/src/main.rs:173`, `apps/ferrogram-web/src/main.rs:93`)
   and `Shell::dispatch_intent` records only `NotEnabled` in the status
   bar (`crates/ui/src/shell.rs:256`, `:677`). A `file.open` on a
   corrupt file (a `CommandError::Failed` with the ADR 0026 one-line
   report) therefore shows nothing at all; the user sees a closed
   dialog and no feedback. **Fix:** in `Shell::dispatch` (or in both
   `update_ui`s) feed every inner `Err` to `note_error`; ADR 0021's
   "errors print as one line" applies to UI runs too.

3. **`Shell::apply_outcome` opens a save dialog for a payload whatever
   the source.** `crates/ui/src/shell.rs:758` calls `self.picker.save`
   for `Outcome::payload` without checking `invocation.source`. On wasm
   a `file.save` without `path` produces exactly that payload
   (`crates/commands/src/builtins/file.rs:333`), so a future
   `Source::Control` / `Source::Mcp` intent routed through
   `Shell::dispatch` would open a dialog (or trigger a browser
   download) programmatically — the AGENTS.md section 6 rule the rest of
   the phase respects. Latent today (nothing programmatic enters
   `Shell::dispatch` yet), but the seam will spread in Phase 8. **Fix:**
   pass the source into `apply_outcome` and handle the payload only for
   `source.is_interactive()`, else return it to the caller.

4. **The Options dialog bypasses the command registry.**
   `show_options` (`crates/ui/src/settings.rs:128`) writes
   `Settings::ui_layout`, `show_grid`, `show_rulers` and `units` by
   direct mutation, including the two layout radios
   (`crates/ui/src/settings.rs:134`), while `view.ui_layout.visio_2016`
   / `view.ui_layout.visio_365` are the registry commands for that
   change (ADR 0032 says both appear in the dialog "as a radio choice"
   and ADR 0029 says UI code never mutates `AppCtx` directly). The
   dialog path is how most users will switch the ribbon, and it is the
   one path that never produces a status message, an undoable step or a
   parity-visible dispatch. **Fix (pick one and record it):** have the
   dialog return `Intent`s (or a changed-flag plus the ids) and let the
   shell dispatch `view.ui_layout.*` and the toggles; or amend ADR
   0029/0032 to exempt the settings dialog from the command rule and
   drop the claim that the two commands appear there.

### Minor

1. **`Registry` is Vec-backed with a linear scan**, while ADR 0028
   specifies "`BTreeMap`-backed" (`crates/commands/src/registry.rs:114`,
   `:136`). `get` is O(ids) (~700 rows) and runs twice per actionable
   control per frame from `action_for` (`crates/ui/src/ribbon.rs:96`).
   Correctness is fine; use `BTreeMap<CommandId, …>` (ids are `Ord`) or
   amend the ADR line.

2. **`UndoStack::default()` has capacity 0** (`crates/commands/src/undo.rs:33`
   derives `Default`), so `push` on a defaulted stack silently drops
   every edit; `with_undo` (`crates/commands/src/builtins/view.rs:53`)
   leaves exactly that default in the session while the step runs
   (`std::mem::take`). Flagged in the P4-10 report, still open.
   **Fix:** hand-written `Default` with `DEFAULT_CAPACITY`.

3. **`with_undo` restores the stack by document index.** An `Edit` whose
   `apply`/`revert` opens or closes documents misplaces or loses the
   stack (`crates/commands/src/builtins/view.rs:62`). Phase 5 risk
   (P4-10 note); fix before edits exist.

4. **`UndoStack::push` returns `Result` but never fails**
   (`crates/commands/src/undo.rs:112`, its own doc says "Never fails"),
   so the `map_err` in `dispatch_one` (`crates/ui/src/shell.rs:856`) is
   dead plumbing. Make `push` infallible (ADR 0028 does not require a
   `Result`).

5. **`AppCtx::close_document` re-aims `active` when a non-active
   document closes** (`crates/commands/src/app.rs:238`): closing index 0
   while index 2 is active moves the user to the old index 1. Shift
   `active` down when `active > index` and leave it otherwise.

6. **`optional_page` accepts `page: 0`** while its error claims pages
   are "counted from 1" (`crates/commands/src/builtins/file.rs:78`); 0
   then silently selects the first page (`select_page`, `:198`). Also
   `u32::try_from(number).unwrap_or(u32::MAX)` reports the saturated
   number, not the caller's. Reject `number < 1` and carry the original
   value into the error.

7. **The wasm no-path save mislabels a write failure.**
   `save_without_path` (`crates/commands/src/builtins/file.rs:333`)
   wraps `write_document` errors with `render_error`, producing
   `render failed: …` instead of the ADR 0026 `cannot write output: …`
   prefix the CLI maps to exit codes.

8. **Two parse paths for `ui-spec/commands.yaml`.** `crates/commands`
   embeds and parses the file itself (`include_str!` +
   `serde_saphyr`→`serde_json`, `crates/commands/src/builtins/mod.rs:33`,
   `:157`) while `ui-spec` embeds the same bytes and validates them
   through the loader (ADR 0031, `ui_spec::embedded`). The registry's
   copy never sees the schema validation, and the two can drift. Build
   the stubs from `Loader::from_embedded()`'s `commands`, or document
   why the JSON round trip is needed.

9. **`UiError::NotImplemented` has no producer left**
   (`crates/ui/src/lib.rs:59`): a P4-01 stub remnant, referenced only by
   tests and one doc comment. Likewise `Ribbon::show_context_menu` /
   `set_context_menu` are never called by the shell (P4-03 open item:
   the committed context menus carry no controls). Drop the variant and
   wire or park the context menu in the fix pass.

10. **`Shell::show` is a documented panic footgun.** It wraps
    `Context::root_ui` (`crates/ui/src/shell.rs:269`), which panics
    while the frame's root `Ui` is live; the apps correctly use
    `show_in`, but a public API that can crash on misuse cuts against
    the never-crash rule. Remove it or make it return an error outside a
    frame.

11. **A failing raster retries every frame.** `rasterize` errors leave
    `is_stale` true (`crates/ui/src/canvas.rs:685`), so a page that
    cannot raster (e.g. `TooLarge`) rebuilds the scene and re-reports
    the same error each frame. Remember the failed key and skip until
    the key changes.

12. **`CommandId::intern` checks the pool cap before validating the
    grammar** (`crates/commands/src/lib.rs:84`): a full pool reports
    "id pool is full" for an invalid id. Validate first (the P4-10
    leak fix already validates before leaking).

## Known open items (not re-filed here)

The P4-02..P4-10 Result notes and `docs/STATUS.md` already track these;
this review agrees with them and adds nothing new: the dropped chord on
a mismatch firing a held-back completion and the `UndoStack` capacity
(P4-10), the nine `TabSave` placements and the dropdown-button command
(P4-04), the per-frame `Document` snapshot comparison and the canvas
UI constants (P4-05), the `app.quit` / `app.office.center.options`
registry stubs and `Shell::dispatch` intercepts (P4-02/P4-03; minor 4
above covers the Options half), `Shell::show` usage in the exit test
(P4-09), the panel warning rows and the non-positive page size display
(P4-07), and the `SKIP_RASTER`/`KNOWN_BLANK` corpus exceptions (P4-09).
