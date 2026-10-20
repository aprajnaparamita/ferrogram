# Phase 5 coverage audit

- Task: P5-13 (coverage-auditor), branch `agent/P5-13`, 2026-10-20.
- Scope: the whole phase, `git diff phase-4..main`, with the new surface
  named in the task: `model::mutate`, `geom::{hit,snap,routing}`,
  `render::{hit,overlay}`, `commands` editing, `ui` tools.
- Numbers below come from this run in the worktree (quoted as required):

```sh
cargo llvm-cov --workspace --all-features
```

  which `cargo xtask coverage` executes as
  `cargo llvm-cov --workspace --all-features --json --summary-only
  --output-path target/llvm-cov-summary.json`. Per-file detail was taken
  from `cargo llvm-cov report --lcov --output-path target/lcov.info`
  (same profile data, no re-run). Zero-hit lines are exact; llvm-cov
  folds partially covered lines (a covered body with an uncovered `?`
  arm) into the covered count, so a file's misses understate its dead
  arms. As in Phase 4, inline `#[cfg(test)]` code counts as instrumented
  lines.

## Per-crate results

| Crate | Lines | Covered | Coverage | Threshold | Phase 4 | Phase 4 ratchet | Delta |
|---|---:|---:|---:|---:|---:|---:|---:|
| ferrogram-geom | 2501 | 2501 | 100.0% | 85 | 100.0% | 100.0 | ±0.0 |
| ferrogram-shapesheet | 45 | 45 | 100.0% | 85 | 100.0% | 100.0 | ±0.0 |
| ferrogram-model | 10668 | 10627 | 99.6% | 85 | 99.7% | 99.6 | −0.1 |
| ferrogram-ui-spec | 1884 | 1872 | 99.4% | 85 | 99.4% | 99.3 | ±0.0 |
| ferrogram-text | 1768 | 1748 | 98.9% | 75 | 98.9% | 98.8 | ±0.0 |
| ferrogram-render | 7221 | 7105 | 98.4% | 75 | 98.5% | 98.4 | −0.1 |
| ferrogram-commands | 7875 | 7723 | 98.1% | 85 | 98.2% | 98.1 | −0.1 |
| ferrogram-vsdx | 7498 | 7343 | 97.9% | 85 | 98.0% | 97.9 | −0.1 |
| ferrogram-cli | 1266 | 1225 | 96.8% | 50 | 96.8% | 96.7 | ±0.0 |
| ferrogram-ui | 10590 | 10228 | 96.6% | 50 | 96.5% | 96.4 | +0.1 |
| xtask | 10443 | 9757 | 93.4% | — | 93.3% | 93.3 | +0.1 |
| ferrogram | 250 | 200 | 80.0% | 50 | 80.0% | 80.0 | ±0.0 |
| ferrogram-web | 140 | 109 | 77.9% | — | 77.9% | 77.8 | ±0.0 |
| ferrogram-control | 0 | 0 | n/a | 50 | — | — | placeholder |
| ferrogram-vsd-binary | 0 | 0 | n/a | 85 | — | — | placeholder |

Phase 4 numbers are the table in `docs/progress/phase-4.md`; the ratchet
column is `[ratchet]` in `coverage.toml`. Deltas are against the Phase 4
recorded percentage, one decimal place.

## Statement per crate

- **At threshold and at ratchet:** geom (100.0, 2501/2501), shapesheet
  (100.0), model (99.6), ui-spec (99.4), text (98.9), vsdx (97.9), cli
  (96.8), ferrogram (80.0), web (77.9). All clear the threshold with a
  wide margin and sit on or above their ratchet.
- **Above threshold and above ratchet:** ui (96.6 vs ratchet 96.4),
  xtask (93.4 vs 93.3). Both gained from the Phase 5 work.
- **At threshold, below ratchet by less than the gate's 0.05 epsilon
  (flagged as a blocker under the strict rule of the role file):**
  - `ferrogram-commands` 7723/7875 = **98.07%** against the 98.1
    ratchet, 0.03 pp below it. `cargo xtask coverage` is green only
    because it tolerates 0.05 for float noise.
  - `ferrogram-render` 7105/7221 = **98.39%** against the 98.4 ratchet,
    0.01 pp below it, same tolerance.
  Both crates have zero safety margin: one uncovered line in
  `ferrogram-commands` (about 8 lines per 0.1 pp) turns the gate red.
  The fix tasks that add the tests listed below should treat these two
  as their first target.
- **Placeholders:** control and vsd-binary have zero instrumented lines
  and pass by the "empty placeholder" rule (vsd-binary's 85% threshold
  becomes meaningful in Phase 7).

## Ratchet proposal for the master (numbers only)

`cargo xtask coverage --update-ratchet` floors to one decimal place.
Applying that to this run gives exactly the current value for every
crate except `ferrogram-ui` and `xtask`, and gives values *below* the
current ratchet for `ferrogram-commands` (98.0) and `ferrogram-render`
(98.3). Since a ratchet may never drop (AGENTS.md section 7), the
proposal keeps the current numbers where the floor is lower:

| Crate | Current | Proposed |
|---|---:|---:|
| ferrogram | 80.0 | 80.0 |
| ferrogram-cli | 96.7 | 96.7 |
| ferrogram-commands | 98.1 | 98.1 |
| ferrogram-geom | 100.0 | 100.0 |
| ferrogram-model | 99.6 | 99.6 |
| ferrogram-render | 98.4 | 98.4 |
| ferrogram-shapesheet | 100.0 | 100.0 |
| ferrogram-text | 98.8 | 98.8 |
| ferrogram-ui | 96.4 | **96.5** |
| ferrogram-ui-spec | 99.3 | 99.3 |
| ferrogram-vsdx | 97.9 | 97.9 |
| ferrogram-web | 77.8 | 77.8 |
| xtask | 93.3 | **93.4** |

Rationale for keeping commands at 98.1 and render at 98.4: their
measured values are inside the gate's float tolerance of the existing
ratchet, so the gate is green today; lowering the ratchet to the floor
would silently accept the Phase 5 drop. The gap (0.03 pp and 0.01 pp)
is the debt the fix tasks should close first.

## Critical uncovered paths

Severity follows the role file: blocker (below threshold or ratchet),
major (a critical path no test reaches), minor (defensive or
line-table). Line numbers are from this worktree.

### Undo and mutation failure paths

- **major** `crates/model/src/mutate.rs:1514`, `mutate.rs:1522` — the
  `write_cell(...)?` error arms for `FlipX`/`FlipY` in `ungroup_shape`.
  A failed ungroup must leave the document unchanged and the undo
  contract intact; nothing proves that today, because no test can make
  the group's sheet vanish between validation and the write. Worth one
  test through a stubbed model fault, or accept as unreachable and say
  so.
- **major** `crates/model/src/mutate.rs:1829` — `master_size`'s
  `Err(BadCell("non-finite master size"))` guard. Reachable from the
  public API (a hand-built master with a NaN `Width`), and it feeds
  `Document::connection_points` scaling (P5-16). This one *should
  become a test*: one fixture master with a non-finite size, one assert
  on the error.
- **major** `crates/commands/src/builtins/edit.rs:573`, `edit.rs:616`,
  `edit.rs:648` — `deleted_glue_warnings` with no active session, the
  closing `?` of `StructuralEdit::removal`, and the selection-refresh
  closing brace (P5-18 catalogued them; still open). The delete path's
  glue warnings are the user-visible half of undo-after-delete; one
  end-to-end delete with glue per arm would close the first two.
- **minor** `crates/model/src/mutate.rs:780`, `mutate.rs:838` — closing
  braces of the `Connect` detach/restore loops in `detach_shape` and
  `restore_shapes`. The bodies run (779, 830–837); llvm-cov maps the
  brace to the surrounding branch. Line-table artefact, not dead code.
- **minor** `crates/model/src/mutate.rs:1688` — `check_room`'s
  `TooManyShapes` (the AGENTS.md section 6 room check). Unreachable
  without `MAX_SHAPES` shapes. Keep: it is the bounded-allocation rule.
- **minor** `crates/commands/src/undo.rs:164` — `entries.get_mut(index)`
  miss arm in `undo()`, unreachable because the cursor bounds the
  index. Keep as the defensive pair of the `checked_sub` above it.
- **minor** `crates/commands/src/edit.rs:118` — the inline test
  double's failing `revert` arm; the revert-failure contract is tested
  in `crates/commands/tests/undo_properties.rs:91` instead. Either wire
  one inline test to it or delete the arm.

### Glue and re-route error paths

- **major** `crates/commands/src/builtins/connector.rs:934-935` —
  `reroute_quietly`'s "connectors were not re-routed" warning. The
  suite forces a route failure only after edit, delete and undo, which
  each keep their own copy of the warning (`builtins/edit.rs:623`,
  `builtins/shape.rs:92`, `builtins/view.rs:69`); a paste, duplicate,
  drop-glue or arrange that fails routing never records the warning.
  Related: the P5-11 note "the three copies of the re-route warning
  helper became one" is only half true — four identical `format!`
  strings remain in `crates/commands/src/builtins/`. The fix task should
  finish the dedup and point one failing-route test at the
  clipboard/arrange copy.
- **major** `crates/commands/src/builtins/connector.rs:975` — the
  stale-target arm of the re-route reference loop (`continue` when a
  `Connect` record names a shape that is no longer on the page). This
  is exactly P5-11's open item about a pasted fragment whose `Connect`
  record names a target that was not copied. One paste fixture with a
  dangling record closes both.
- **major** `crates/commands/src/builtins/insert.rs:421-423` —
  "the dropped master has no connection points to glue". Every shipped
  catalog master has connection points, but `insert.shape` also takes
  document masters from a read file, and a foreign master may have no
  `Connection` rows. The arm silently skips glue; that deserves one
  fixture master, not silence.
- **minor** `crates/commands/src/builtins/insert.rs:495`, `518` —
  non-finite source connection-point transforms in the drop-glue path.
  Unreachable from `insert.shape`'s finite inputs (P5-18). Keep.
- **minor** `crates/commands/src/builtins/connector.rs:1058`, `1070`,
  `1094` — the page-missing arms of `drop_all_for` and
  `drop_detached_targets` (closing braces and one `return`).
  `reroute_after_edit` returns before calling either when the page is
  gone. Keep as guards; one `if let` consolidation would remove the
  brace artefacts. The lost-glue warning body itself (1086-1089) is
  covered.
- **minor** `crates/model/src/connection.rs:68`, `160` — the
  master-chain lookup `continue` (a chain naming a missing shape) and
  `direct_number`'s non-`Number` arm. Both silently skip size scaling
  in `Document::connection_points`; one fixture each would pin the
  behaviour.

### Writer delete and patch paths (vsdx)

The delete direction of the writer is covered (`crates/vsdx/tests/roundtrip_edit.rs`);
the gaps are its failure arms and its **insert** direction.

- **major** `crates/vsdx/src/write/patch.rs:539`, `588-593`, `612`,
  `647`, `670-675`, `790`, `862`, `957`, `1160`, `1198` — the
  `build(entity)` closures of `sync_children` (the `map_or_else` arms
  that `emit::empty`/`emit::cell_element`/`emit::row_element`/
  `emit::section`/`emit::shape_element`/`emit::connect_element`/
  `emit::style_element` a *new* child into an existing part), together
  with the insert ranks `page_rank` (`patch.rs:391-397`, all arms) and
  the catch-all of `shapes_rank` (`patch.rs:360`). Zero hits in the
  whole workspace run: no test patches a read part that *lacks* a
  cell, row, section, shape, connect, style, page or master the model
  has. Adding an element to a read document (insert shape onto a read
  page, set a cell a read file does not define, add a page to a read
  file) is the exit scenario's write path; the in-memory round-trip
  tests write whole parts through `emit::*` and never reach these
  closures. Add `roundtrip_edit` cases that mutate the corpus fixtures
  (insert shape, write a missing cell, add a page) and round-trip them.
- **minor** `crates/vsdx/src/write/patch.rs:138-140`, `149` —
  `sync_uint`'s "already equal" / "overwrite" / "set new" arms and
  `bool_matches`'s catch-all for an unparseable boolean spelling.
  Patching existing numeric/boolean attributes of a read part is barely
  exercised. One fixture with a spelled-out `TRUE` and one with junk
  text closes them.
- **minor** `crates/vsdx/src/write/patch.rs:515` — `row_wanted`'s
  `(None, None)` arm (a model row with neither `IX` nor `N` cannot
  match a source row). Keep.
- **minor** `crates/vsdx/src/write/patch.rs:248`, `290`, `312`, `748`,
  `887`, `1030`, `1241`, `1263`, `1340` and the `is_property` /
  `patch_contents_root` / `property_template` arms (`1048`, `1071`,
  `1074`, `1098`, `1315-1317`, `1351-1361`) — text-node-where-element
  expected arms and the `DocumentProperties` patching paths (the P2-07
  deferred `Property` write). Defensive against foreign parts; keep.
- **minor** `crates/vsdx/src/write/mod.rs:209`, `224`, `231`, `251`,
  `266`, `273` — the `?` error arms of `ensure_list`, `contents_part`
  and `ensure_contents` in `patch_lists` (page/master create and
  delete). Error propagation for a corrupted package; no test can
  reach them without fault injection. Keep.
- **minor** `crates/vsdx/src/package/edit.rs:64-67` —
  `next_relationship_id`'s fallback when the bound is full; the doc
  comment proves one of `rId1..rIdN+1` is always free, so the arm is
  the loop's bound, not a bug. Keep.
- **minor** `crates/vsdx/src/write/patch.rs:1566`, `1583`, `1657`,
  `1687`, `1711`, `1727` — arms of the inline test helpers that never
  fail. Same class as P5-15's test-module note: they inflate the
  denominator (inline test code counts). Leave or delete, never
  `expect`.

### Tool-state edge cases (ui)

- **major** `crates/ui/src/canvas.rs:728-739`, `748-755` — the page-tab
  context menu (Rename/Delete/Move Page Left/Move Right) is built and
  dispatched but never exercised, and neither are tab clicks and the
  `page.previous` / `page.next` intents. Page management is part of the
  Phase 5 exit scenario ("across two pages"); the P5-09 menu is
  effectively untested UI surface. (`Shell`'s context menus being
  parked is the known P4-16 item; this is the separate page-tab menu.)
- **major** `crates/ui/src/tools/pointer.rs:490-502` — the entered-group
  hit walk (`entered == Some(parent)` early return and the parent
  climb). Getting in and out of a group is core editing state; a bug
  here traps selection inside a group. One `CanvasRig` test with a
  nested group closes it.
- **major** `crates/ui/src/tools/connector.rs:168-170` — "the Basic
  Flowchart Connector master is missing" `UiError::Io`. The connector
  tool finds its master by name (P5-08 open issue); a renamed or
  missing stencil raises this and nothing tests it.
- **minor** `crates/ui/src/tools/pointer.rs:116-117`, `178`, `186`,
  `205-243`, `305-351`, `379-382`, `434-455`, `534`, `556`, `726-727`,
  `809` — no-page scene reset, `Pressed` without drag, drag-end effect
  returns, handle/marquee effect paths, the invalid-tool-id error and
  an overlay catch-all. The early returns guard the drag state machine
  (ADR 0042); a stuck drag is the classic tool bug, but each arm needs
  an artificial event sequence. Worth a sweep in a tool-state test
  pass, not a blocker.
- **minor** `crates/ui/src/tools/stencil.rs:83-84`, `129-130`, `156`,
  `180`, `188`, `262-263`, `297-315` and
  `crates/ui/src/tools/draw.rs:72-73`, `78`, `151-152`, `176`, `225`,
  `260`, `271`, `274-275` — cancel-without-active-drag arms,
  double-click arms, scale fallbacks (`1.0`), the invalid-command-id
  errors (`stencil.rs:262-263`, `draw.rs:274-275`). Keep; a few are
  reachable with an extra event each.
- **minor** `crates/ui/src/snap.rs:83`, `100`, `119`, `204`, `218`,
  `225`, `245`, `274` — `SnapResult::none` fallbacks (no scene, no
  candidates) and skip arms of the candidate loop. Snap behaviour is
  otherwise well covered (`crates/ui/tests/phase5/snap.rs`).
- **minor** `crates/ui/src/handles.rs:203`, `255`, `276`, `308` —
  `Ok(None)` and no-target returns in handle building.
- **minor** `crates/ui/src/text_edit.rs:173-176`, `183`, `280`, `296`,
  `315`, `357` — the click-outside-the-editor commit condition and
  `Pending`/`None` fallbacks. Whether a click outside commits is one of
  P5-07's black-box checks; the behaviour exists but is untested at
  the egui seam.
- **minor** `crates/ui/src/canvas.rs:333-338`, `939-944`, `1049-1050`,
  `1066-1068`, `1137` — pan-by-drag, the `Page` wheel unit and `Zoom`
  events, and the "inconsistent raster size" guard P4-16 already
  proved unreachable through `render_raster`.
- **minor** `crates/ui/src/shell.rs` (77 lines), `dialogs.rs` (35),
  `shapes_window.rs` (31) — mostly the desktop-only file pickers and
  dialog plumbing P4-16 recorded as untested behind
  `picked_path`/`save_picked`. Not Phase 5 surface; citing the known
  item, not re-reporting it.
- **minor** `crates/render/src/hit.rs:432-439` — the degenerate-points
  catch-all for an unexpected geometry row kind in hit testing, and
  `hit.rs:569` (`_ => continue`). Hit testing drives selection
  (P5-03); one fixture with an unmapped row kind would pin the
  fallback.
- **minor** `crates/render/src/overlay.rs:192`, `199`, `221`, `246`,
  `275` — five `panic!` arms in the inline test module (assertions that
  never fail). Same class as the P5-15 test-module arms.

### `ferrogram-commands` defensive arms (the P5-11 questions)

The task asks for a verdict on each arm `docs/reviews/phase-5-tests.md`
judged unreachable:

1. **Re-route failure warnings** (`connector::reroute_quietly` and the
   three copies): **stay + become a test**. The warning must stay —
   AGENTS.md section 6 forbids crashing and the user needs to know a
   connector kept its old path. But the clipboard/arrange/insert copy
   (`connector.rs:934-935`) should get one failing-route test, and the
   four copies of the message string should become one helper. The
   page-missing arms of `drop_all_for`/`drop_detached_targets`
   (`connector.rs:1058`, `1070`, `1094`) may stay as guards.
2. **Poisoned id-interner arm** (`crates/commands/src/lib.rs:94-100`):
   **go**. Replace the `Err` arm with `PoisonError::into_inner()`
   recovery: the pool holds immutable leaked ids, recovery is safe, and
   the arm can never be exercised without poisoning a mutex on purpose.
   Keep `CommandIdError` for the 4096-id cap. (Never by switching to
   `expect`, per the P5-15 rule.)
3. **`need_index` `usize` overflow arms**
   (`crates/commands/src/builtins/page.rs:293-306`): **stay, no test**.
   Checked conversion is mandated by AGENTS.md section 6; on 64-bit the
   arms need an out-of-range `i64`/`f64`, and they are partially
   covered on the same lines as the covered conversion.

Also still open from P5-15 (unchanged judgement): `lookup::repeated_cell`'s
`Some(row)` arm is dead and can go; the defensive fall-throughs in
`document.rs:134`, `raw.rs:243/265` and `stencils/mod.rs:189` are
malformed-input guards and stay.

## Gate

- `cargo xtask coverage`: green (the per-crate table above; every crate
  at or above its threshold and, within the 0.05 tolerance, its
  ratchet).
- `cargo xtask gate --quick`: green in the worktree (codegen, parity,
  fmt, clippy, test, doc, deny, layers). No code changed in this task,
  so wasm and fuzz were not triggered and coverage is not part of
  `--quick`; it was run separately as quoted above.
