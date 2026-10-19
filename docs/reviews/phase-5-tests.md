# Phase 5 test pass (P5-11)

Task: P5-11 (test-engineer), branch `agent/P5-11`, 2026-10-20.
Gate: `cargo xtask gate` green in the worktree (codegen, parity, fmt,
clippy, test, doc, coverage, deny, layers, wasm, fuzz).

Scope: the Phase 5 test gaps named by ADR 0049 — the undo inverse
property (rule 2), the connector glue tests (rule 3), the
`egui_kittest` interaction suite (rule 4), the ADR 0042 drag timing
check, and the fuzz seed housekeeping (rule 8). The property tests
found eight product bugs; each is fixed at its narrowest layer with a
regression test and listed under "Product fixes".

## What is covered

### The undo property (`crates/commands/tests/undo_prop.rs`, 10 tests + 1 property)

- **The generated list (ADR 0040 rule 4).** `every_implemented_id_is_classified`
  walks the registry and fails when an implemented id has neither an
  undo property case nor a non-editing reason: 43 editing command ids
  have cases, 41 non-editing ids carry a reason (`edit.select` and
  `tools.*` are not edits (P5-16, P5-17), `edit.undo`/`edit.redo` are
  the stack itself, `connector.reroute_all` is not undoable on its own
  (ADR 0045 rule 3), the `file.*`, `view.*`, `app.*` and `help.*` ids
  never edit the document). A new registered command without a case
  fails the test.
- **One edit per command.** `every_case_produces_exactly_one_edit_on_the_seed`
  runs every case over the seeded document and asserts exactly one undo
  step (ADR 0040 rule 1); `non_editing_commands_never_produce_an_edit`
  runs every non-editing id and asserts it never answers an
  `Outcome::edit` or pushes a step.
- **The hand-written pair test (ADR 0040 rule 4).**
  `every_case_inverts_on_the_seed` runs each case at three argument
  picks, undoes it and compares with the seed, then redoes it and
  compares with the post-command state. The assertion names the
  command.
- **The sequence property.**
  `undo_is_the_inverse_of_random_command_sequences` (proptest, 48 cases
  per run, seeded regression file committed) draws random command
  sequences of one to five steps over the seed, undoes every pushed
  step and compares with the seed, then redoes the sequence step by
  step and compares with the state recorded after each step. A soak
  (`PROPTEST_CASES=300`, three runs ≈ 900 sequences) was clean.

Both compare the **reachable** state (task P5-11 note), never
`Document` equality: the pages in order, the shapes reachable from them
(arena renumbered into wrapper order), their sheets and text, the
`Connect` records of those shapes (with their list positions), and the
masters their instances use (renumbered into document order). The
normalisation follows `apps/ferrogram/tests/flowchart.rs`: `Preserved`
attribute order, preserved `Rel` children and a cell's `U` kind
spelling are not content, and the volatile document properties are not
compared.

Three extra property tests pin the exact-inverse fixes this task made:
a nested group and its undo (the members return to their parent's
list), an ungroup undo (the group's own shape, its slot and the
`Connects` records that name it), and a group-child delete undo. Two
more pin the arrange skip warnings and the thin selection shapes (a
one-shape align or distribute, a group copy/paste/duplicate, a nested
cut).

### Connector glue (`crates/commands/tests/connector.rs`, 21 tests: 10 new)

The ADR 0045 rule 3 list, all through `Registry::execute`:

- **Both glue kinds.** `a_glue_without_a_point_binds_the_shape_outline`
  pins the outline glue record (`ToCell` `PinX`, `ToPart`
  `VIS_WHOLE_SHAPE`) and its routed point; `a_five_connector_flow_is_built_headlessly`
  (P5-08) pins the connection-point kind. A drop with an explicit
  `glue_point` (ADR 0046) names the row's one-based cell and
  `VisToParts` value.
- **Glue replacement.** `a_new_glue_replaces_the_old_record_on_the_same_end`:
  one record per end (ADR 0039 invariant 5), the second glue wins, one
  undo restores the replaced one.
- **Unglue.** `unglue_removes_both_end_records_and_keeps_the_free_endpoints`:
  both records go, the free endpoints keep their routed coordinates
  across a later re-route.
- **Drop-on-detached.** `stale_and_detached_glue_records_are_dropped`
  (P5-08) covers the re-route pass dropping dangling records with one
  warning line; `deleting_the_target_drops_its_glue_and_undo_restores_it`
  covers the delete path (the record goes with the deleted target and
  undo restores both).
- **Re-route stability (the task's fixes).** A glue-on-drop instance
  keeps its geometry (it is shape-to-shape glue, not a connector end);
  a grouped connector's re-route is idempotent (the path maps through
  the parent chain); a duplicated connector leaves the page
  re-route-stable. `connector.draw` with both ends glued and with one
  glued end is one undo step whose undo restores the page exactly.

### UI interaction (`crates/ui/tests/phase5/`, 27 tests: 2 new)

The ADR 0049 rule 4 list is covered by the P5-05..P5-09 suites, which
this task extends rather than duplicates:

| Gesture | Test |
|---|---|
| drag (one undo step) | `pointer::one_move_drag_is_one_undo_step`, `a_drag_does_not_re_rasterize_until_the_commit` |
| resize | `pointer::one_resize_drag_sets_the_new_size` |
| rotate | `pointer::one_rotate_drag_turns_the_shape` |
| marquee | `pointer::a_marquee_selects_intersecting_shapes` |
| snapping, Alt off | `pointer::alt_disables_snapping_for_the_drag`, `snap::alt_keeps_the_unsnapped_delta` |
| text edit commit | `text::escape_commits_and_leaves_the_editor_idle` (editor) and the new `text::the_text_edit_cancels_and_commits_through_the_shell` (Escape through the real `Shell`, P5-17's wiring) |
| text edit cancel | `text::the_text_edit_cancels_and_commits_through_the_shell`: `text.begin`, typing, `text.cancel` — the shape's text is unchanged |

### The drag timing check (`crates/ui/tests/phase5/drag_timing.rs`)

**Measurement method.** A 1000-shape page (a 25-column grid of 1 x 0.5
in rectangles), a press on one shape, then 30 drag runs walking the
pointer across the page. Each run is one `egui_kittest` frame: the real
canvas closure (`canvas::show_with_overlay` + `ToolMachine` + command
dispatch) plus the egui paint — the per-frame UI work of ADR 0042 rule
3. `Harness::run` can settle a run in more than one frame; the settle
count divides the elapsed time, so every recorded number is one frame.
The setup's first rasterization (the 1000-shape page raster) and the
commit after the release are outside the measurement, exactly as ADR
0042 allows. The test prints the median, p95 and worst frame.

**The budget.** The ADR 0042 number is 8 ms per frame on the reference
machine. The assertion runs the median against `8 ms x 5` in CI (the
generous CI multiplier) and `8 ms x 2` on a local machine, and only on
a known CPU/OS pair (`aarch64`/`x86_64` x `macos`/`linux`) or with an
explicit `FERROGRAM_DRAG_BUDGET_MS`; anywhere else the test still runs
and prints the measurements, then skips the assertion with its reason
(unknown hardware would turn the check into noise).

**What it found.** On this machine (Apple silicon macOS) the check
started at **80-110 ms per frame** (debug and release), ~10x over the
budget. Two scene rebuilds per frame were responsible (see the fixes
below); the same drag frame measures **0.6-1.6 ms** after them, inside
the budget with its multiplier.

### Fuzz housekeeping (ADR 0049 rule 8)

- `crates/render/fuzz` and `crates/vsdx/fuzz` seed corpora moved from
  flat files into per-target directories (`corpus/render_document/`,
  `corpus/read_package/`), matching the `ui` crate (P4-17). The path
  rows in `ATTRIBUTION.md` and `corpus/PROVENANCE.md` follow the move.
- Already closed before this task (the master's note): `crates/ui` has
  the `proptest` dev-dependency (P4-16) — the P5-11 line in "Owns" was
  not needed; `crates/ui-spec/fuzz/.gitignore` has `!corpus/`; the
  `crates/ui/fuzz` seeds are per-target.
- **No new fuzz target.** `vsdx` gained no public parser surface in
  Phase 5 (`read_document`/`write_document` are unchanged), and the
  `read_package` target already round-trips `write_document` over what
  it reads. `cargo fuzz build` is green for all four fuzz crates.

## Product fixes (each with a regression test above)

1. **`fix(model)`: `restore_shapes` put removed `Connect` records back
   at the end of the list.** `DetachedShapes.connects` now carries each
   record's list index and the restore puts it back exactly — the same
   invariant `GlueEdit` already documented. A delete or cut undo now
   restores the record order (regression: `tests/connects.rs`, the undo
   property).
2. **`fix(commands)`: a multi-shape z-order undo interleaved the page
   list.** `ZOrderEdit` recorded the shape's index before any sibling
   moved; it now records the index its own `apply` saw (regression: the
   undo property).
3. **`fix(commands)`: group and ungroup undo were not exact inverses.**
   `GroupEdit::revert` went through `ungroup_shape`, which re-bases the
   members' cells and leaves them at the group's slot: undo changed the
   members' sheets and their z-order, and a redo re-created the group
   under a fresh arena id (so a later edit recorded against the old
   group lost its subject). Both group edits now snapshot the shapes
   and positions they touch, restore them exactly (`dissolve`,
   `regroup_exact`, `place_at`), and re-create the group at its own
   arena slot (`reuse_slot`) (regression: three undo property tests).
4. **`fix(commands)`: a page snapshot restore named dead arena ids.**
   `PagesEdit` replaced the whole page list with its snapshot,
   including `shapes` and `Connects` lists that later edits own; after
   an undo and redo of an earlier group or insert, the restored lists
   pointed at replaced arena ids. The restore now takes the pages'
   names and membership from the snapshot and keeps the live lists of
   pages that are still open (regression: the undo property sequence
   cases).
5. **`fix(commands)`: paste, duplicate and glue-on-drop did not
   re-route.** The copies land at new positions carrying their copied
   glue, and a glue-on-drop adds a `Connect` record; all three now call
   the one re-route hook (ADR 0045 rule 3, ADR 0046's "any glued
   connector endpoints on the page re-route") (regression:
   `a_duplicated_connector_leaves_the_page_re_route_stable`).
6. **`fix(commands)`: a connector inside a group routed through the
   wrong transform.** `write_connector_path` maps page points into the
   shape's own local box, but a grouped connector is addressed through
   its parents (`geometry_endpoints` reads through the same chain), so
   grouped connectors got wrong and non-convergent paths. `write_path`
   now maps into the parent's space first (regression:
   `a_grouped_connector_re_route_is_idempotent`).
7. **`fix(commands)`: a glue-on-drop instance was hijacked as a
   connector.** The re-route pass treated every `Connect` record's
   owner as a connector and rewrote the dropped instance's geometry as
   a routed path (ADR 0046 glue is shape-to-shape). Only
   `VisFromParts` begin/end records are connector ends now
   (`is_connector_end`) (regression:
   `a_drop_glued_instance_keeps_its_geometry`).
8. **`fix(ui)`: the drag frame was 10x over the ADR 0042 budget.** The
   snapper called `scene_for` (a full `build_scene` over the page) on
   every `snap_move`/`snap_point` and `selection_frame` did the same,
   and `snap_move` recomputed the moving shapes' connection points
   inside the O(n) stationary loop. The two now take the caller's page
   scene (the pointer tool caches one per change counter) and the
   moving points are read once (regression: `drag_timing.rs`, which is
   the check that found it).

A ninth change is a refactor, not a fix: the three copies of the
re-route warning helper became one (`connector::reroute_quietly`).

## What is not covered, and why

- **Free connector ends compare within 12 significant digits** (and
  values within a nano-inch of zero fold to zero). A re-route pass
  re-derives a *free* end through the shape's transform and the f64
  round trip drifts about one ulp per pass (ADR 0045 keeps a free
  point "until the connector itself moves" within rounding), so exact
  bit equality of routed paths is not achievable. Everything else —
  structure, ids, text, formulas that are not number literals, the
  `Connect` records and their positions — compares exactly.
- **The drag budget covers the pointer drag.** The drawing, connector
  and stencil tools still build a page scene per `snap_point` call
  (they have no scene cache of their own); the pointer drag is the one
  ADR 0042 budgets and the one the harness measures. See "Open
  issues".
- **The budget assertion skips on unknown hardware** (with its reason
  and the measured numbers printed). `FERROGRAM_DRAG_BUDGET_MS` forces
  it anywhere.
- **Defensive arms stay uncovered**: the re-route failure warnings
  (the pass cannot fail from public-API state: non-finite cells read as
  their documented defaults and every shape transform is invertible),
  the poisoned id-interner arm, and the `usize` overflow arms of
  `need_index` on 64-bit. They are listed for P5-13 (coverage audit) to
  judge whether to delete or keep them.

## Known differences of the undo contract (the master's note)

The property compares the reachable state because these differences
are **by design**, and `known_undo_differences_stay_outside_the_reachable_state`
pins them as observable:

- group, insert and delete undo leave detached arena entries behind;
- the first drop of a catalog master leaves the materialised master in
  the document (P5-06 open issue) — the reachable state keeps only the
  masters that reachable instances use, so the leftover drops out.

## Open issues

- **The non-pointer tool drags rebuild the page scene per snap call.**
  `draw`, `connector` and `stencil` call `snap_point(ctx, None, ...)`,
  which builds the scene; on a 1000-shape page that is the same cost
  the pointer drag had. The fix is the scene threading the pointer
  already has (or one shared cache on `ToolMachine`).
- **A pasted fragment's `Connect` records name the copied targets**
  through a visio-id map (`paste_content`); whether a record whose
  target was *not* copied keeps a stale id was not verified here and
  deserves one test.
- **The defensive arms above** (re-route warnings, `need_index`
  overflow) are unreachable from the public API; P5-13 may prefer
  deleting them to keeping uncovered code.
- **`Edit::label` on the inner `PathEdit`** (`"Route connector"`) is
  never read: the undo stack reports the composite's label. Either the
  runner should surface inner labels (for a future undo menu) or the
  method can return the composite's.

## Coverage

| crate | before (main) | after | ratchet |
|---|---|---|---|
| ferrogram-commands | 98.1% | 98.1% | 98.1 |
| ferrogram-model | 99.6% | 99.6% | 99.6 |
| ferrogram-ui | 96.4% | 96.6% | 96.4 |

(`cargo xtask coverage`: every crate is at or above its threshold and
its ratchet. The commands crate is at its ratchet exactly; the
product fixes' uncovered defensive arms are the reason it does not sit
higher.)
