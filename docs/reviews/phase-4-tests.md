# Phase 4 test pass (P4-10)

Task: P4-10 (test-engineer), branch `agent/P4-10`, 2026-10-17.
Gate: `cargo xtask gate` green in the worktree (codegen, parity, fmt,
clippy, test, doc, coverage, deny, layers, wasm, fuzz).

Scope: untested paths in the Phase 4 code — property tests for the
command registry and the undo stack, shortcut-trie and chord
normalization edges, hostile inputs (settings slot, shortcut tables,
zip entry names), egui_kittest interaction tests, and fuzz targets. The
master's named hot spots were all touched: the scope-aware shortcut
trie and the text-edit defer rule (ADR 0036, amended twice), the
pointer-press focus tracking, `Shell::dispatch`'s `NeedsInput` flow and
the `app.quit` / `app.office.center.options` intercepts, the settings
slot with truncated or hostile JSON, and the zip 9 entry-name error
path in `crates/vsdx/src/package/read.rs`.

## What was tested

**`crates/commands/tests/undo_properties.rs` (4 tests, 1 property).**
Model-checked `UndoStack` against a reference (entries + cursor) over
random push/undo/redo/clear sequences: `revert` is the inverse of
`apply` (a shared counter equals the applied-and-not-reverted delta
sum), `push` truncates the redo tail and clamps to the capacity, a
failing `revert`/`apply` leaves the cursor where it was, and entries
dropped by the clamp stay applied forever (tracked as "stranded" in the
model). Plus the ADR 0028 inverse rule spelled out on one edit and the
failing-redo twin of the failing-revert unit test.

**`crates/commands/tests/registry_properties.rs` (7 tests, 4
properties).** `CommandId::parse` accepts exactly the ADR 0006 grammar
(an independent oracle over arbitrary char vectors); `CommandId::intern`
never panics, is idempotent and round-trips; every `ArgValue` variant
round-trips through `Args` under hostile names with the typed
accessors answering `None` for the other variants; a random
register/execute sequence agrees with a per-id state model
(absent/stub/real), pinning the P4-02 rules (a real command replaces
its stub, a duplicate is `RegistryError::Duplicate`, and `execute`
answers `UnknownCommand` before `Unsupported` before `NotEnabled`);
`from_mappings` builds the table it is given.

**`crates/ui/tests/settings_slot.rs` (7 tests).** The settings slot is
untrusted text (eframe storage / `localStorage`). Covered: every
byte-length truncation of a valid settings JSON; a hostile battery
(wrong types, missing and unknown fields, deep nesting, huge strings,
duplicate keys, BOM, NUL and control bytes, non-object roots,
`1e999`/`NaN`); the rule that a corrupt slot falls back to
`Settings::default()` with exactly one warning and never half-loads;
unknown fields are ignored and dropped by the next save; a failing
`SettingsStore` falls back the same way; and a save/load round-trip
property over 200 generated settings (hostile `recent_files` included).
The property runs on a seeded LCG, not proptest: `crates/ui/Cargo.toml`
belongs to P4-03 (Open issues).

**`crates/ui/tests/shortcuts_edge.rs` (21 tests).** Chord
normalization (modifier order, duplicates, shifted symbols `+`/`<`/`>`
absorbing `Shift` on both the file and the event side, unordered token
sets); every load-error class naming `shortcuts.yaml` (empty sequence,
no key, two keys, unknown token, bad command id); trie edges (a prefix
of longer sequences defers, fires on a mismatch and on the 2 s timeout,
state persists across `resolve` calls, unmatched chords are left alone,
a completed sequence with no active binding is still consumed); scope
precedence (the most specific active binding wins across all six
scopes, exact repeat silent, conflict warns once); the text-edit defer
rule exactly as amended twice (egui's editing chords are never stolen,
`Ctrl+Shift+C` and `Alt` chords dispatch inside a text edit, plain
typing passes through); and hostile tables (a 500-chord sequence, 2000
entries over 8 command ids, modifier-duplicate chords).

**`crates/ui/tests/phase4/interactions.rs` (7 tests) and
`crates/ui/tests/phase4/flows.rs` (4 tests).** egui_kittest clicks
through the real `Shell`: the View tab's `ShowGrid` checkbox toggles
`view.toggle_grid` twice; the View/Options dialog switches to the 365
command bar with no restart (same `Shell`, same `AppCtx`, the 365-only tab
appears); the Help menu opens the About window; the close-confirmation
dialog closes a dirty document on "Close without saving"; the command bar
File button opens the backstage with its committed placements (each
"File" button gets its own rig: the menu-bar button's popup covers the
command bar one); the QAT `FileNewDefault` button opens a document; a page
tab click switches the active page. Flows: an asynchronous pick (the
wasm upload shape) completes `file.open` across frames through
`finish_pick`, a cancelled async pick clears the pending command, the
window commands are `Unsupported` for programmatic callers through
`dispatch_one` while `Shell::dispatch` runs them (P4-03 deviation 4),
and `app.quit` resets its flag when the close goes to the viewport.
`ShellRig` gained `large()` (12000x4000, the P4-04 note about the
command bar width) and `click`/`try_click`/`try_click_nth` helpers.

**`crates/vsdx/tests/zip_entry_names.rs` (4 tests).** The zip 9
entry-name path: an entry name with invalid UTF-8 bytes (leading 0xFF,
lone continuation 0x80, a dangling 0xC3, all-bytes-0xFF) never panics
and never drops the entry silently; hostile UTF-8 names (traversal,
absolute, backslashes) normalize to one absolute part name; a NUL in
the name is tamed the same way; the entry-count cap applies whatever
the names decode to.

**Not touched (as instructed):** `SKIP_RASTER` / `KNOWN_BLANK` in
`apps/larrygram/tests/open_corpus.rs`; the corpus-open coverage of the
exit criterion stays P4-09's test.

## Fuzz targets

The task's condition on a ui-spec fuzz target: `Loader::from_embedded`
parses `include_str!` build-time text — not untrusted input — so it
needs no target; the hostile-YAML case is already
`crates/ui-spec/fuzz`'s `loader_fuzz` (P1-08). The untrusted text
surfaces that had no fuzzing got two new targets in the new
`crates/ui/fuzz` crate (edition 2024, closing the P1-08 note for "the
next fuzz target"):

- `settings_json` — the settings slot (`TextStore::load`,
  `load_settings` fallback, `save_settings` write-back) on arbitrary
  bytes.
- `shortcuts_map` — `ShortcutMap::from_shortcuts` and `resolve` on
  decoded hostile tables built straight from input bytes (bounded
  entries, sequences and chord widths, plus a 512-chord deep shape),
  with rotating active scopes.

Both `fuzz/.gitignore` carry `!corpus/` (AGENTS.md lesson). No seed
corpus is committed for the new targets (hand-written seeds would need
a `corpus/PROVENANCE.md` row, which is not this task's file; add seeds
with the next parser change). Both targets ran 2000 runs clean, and
`cargo +nightly fuzz build` passes for every `crates/*/fuzz`.

## Bugs and findings

1. **Fixed** (`fix(commands)`): `CommandId::intern` leaked the boxed
   string of every *rejected* id — `Box::leak` ran before
   `validate_id`. A hostile `shortcuts.yaml` (or any future
   user-supplied table) could grow the process's memory without bound
   one small string per bad command id. Now the id is validated first.
   The reject path is covered by
   `intern_rejects_an_invalid_id_with_a_reason` and the
   `intern_never_panics_and_is_idempotent` property; the leak itself
   has no unit-observable.
2. **Documented, open issue**: `ShortcutMap::step` drops the chord
   that mismatches a half-typed sequence when the mismatch fires the
   held-back (deferred) completion — that chord is consumed without
   dispatching its own binding. Pinned as-is by
   `a_prefix_sequence_defers_and_fires_on_a_mismatch` with a comment;
   the right rule (retry as a fresh start vs. drop) is an ADR 0036
   decision. No committed binding can hit it today (the committed
   table has no multi-chord sequences).
3. **Pinned trap**: `UndoStack::default()` has capacity 0 (a black
   hole that keeps nothing). That is exactly what
   `builtins::with_undo`'s `mem::take` placeholder wants; any other
   `Default` caller silently loses edits.
   `a_default_stack_keeps_no_entries` documents the semantics.
4. **Open issue** (Phase 5): `builtins::with_undo` takes the undo
   stack out of `documents[index]` and puts it back by index; an
   `Edit` whose `apply`/`revert` changes `AppCtx::documents` would put
   the stack into the wrong session.
5. **Testing hazard**: `CommandId::intern` is a process-global pool
   with a 4096 cap, shared by every test in one binary, and
   `builtins::registry()` alone interns 943 ids. Property tests must
   draw their ids from small fixed pools (done in
   `registry_properties.rs` and `shortcuts_edge.rs`); a test that
   interns arbitrary valid ids will eventually fail spuriously with
   "id pool is full". A test-only reset hook or a documented higher
   cap would remove the hazard.
6. **zip 9 entry-name arm is defensive**: zip 9.0.4's `ZipFile::name()`
   decodes any byte sequence (UTF-8, else CP437) and cannot fail in
   practice, so `read_package`'s `"zip entry name is malformed"` arm is
   unreachable today. The reachable contract on those exact inputs is
   pinned instead (decoded name kept under an absolute part name, or
   the strict content-type rule's mapped error). If a future zip
   release makes `name()` fallible, the arm is already mapped.
7. **Observation**: `normalize_name` keeps `..` segments
   (`/part0000/../evil.xml`). Inert here — the reader is byte-based
   and never extracts to a file system — but never use a part name as
   a filesystem path without sanitizing (zip's own `mangled_name`
   warning).

## Coverage before/after

`cargo llvm-cov` (the gate's coverage step) counts `src/` only (the
P1-07 note), so test-only additions move the numbers only where they
exercise previously-uncovered source lines.

| crate | before | after | threshold / ratchet |
|---|---|---|---|
| larrygram | 80.0 | 80.0 | 50 / 66.6 |
| larrygram-cli | 96.8 | 96.8 | 50 / 96.8 |
| larrygram-commands | 97.2 | 97.2 | 85 |
| larrygram-render | 98.5 | 98.5 | 75 / 98.4 |
| larrygram-text | 98.9 | 98.8 | 75 / 98.8 |
| larrygram-ui | 92.3 | **94.8** | 50 |
| larrygram-ui-spec | 99.4 | 99.4 | 85 / 99.3 |
| larrygram-vsdx | 98.0 | 98.0 | 85 / 98.0 |
| others (geom, model, shapesheet, web, xtask) | — | unchanged | at/above |

`larrygram-ui` gains 2.5 points from the newly covered settings,
shortcut and shell error paths. No threshold or ratchet change is
proposed (the master owns `coverage.toml`).

## Test totals

- `crates/commands`: +11 tests (2 property).
- `crates/ui`: +39 tests (`settings_slot` 7, `shortcuts_edge` 21,
  `phase4/flows` 4, `phase4/interactions` 7).
- `crates/vsdx`: +4 tests.
- `crates/ui/fuzz`: 2 new fuzz targets.
- 54 tests total; one production fix (see above). No existing
  assertion was weakened.

## Open issues (for the fix tasks / the parity pass)

- The deferred-completion mismatch chord is dropped (finding 2): decide
  the rule in the ADR 0036 review.
- `builtins::with_undo`'s stack put-back by index (finding 4), with
  Phase 5 edits.
- `UndoStack::default()` capacity 0 (finding 3): document in ADR 0028
  or give `Default` the `DEFAULT_CAPACITY`.
- The interner's per-process 4096 cap is a test hazard (finding 5).
- `crates/ui/Cargo.toml` (P4-03's file) could gain `proptest` so the
  settings round-trip property and the shortcut properties use the
  workspace's property-test convention instead of a hand-rolled LCG.
- `crates/ui-spec/fuzz/.gitignore` has no `!corpus/` line (it has no
  seed corpus, so nothing is lost today; align it when seeds land).
- The new fuzz crate adds a one-time cold build (about 4-8 minutes)
  to the gate's `fuzz` step per target directory.
- The committed `shortcuts.yaml` still has no multi-chord sequence, so
  real keytip data does not exercise the trie's deferred completion
  (the shapes are covered by tests and the `shortcuts_map` fuzzer).
