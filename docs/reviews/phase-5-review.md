# Phase 5 code review (P5-12)

Task P5-12, reviewer, 2026-10-20, branch `agent/P5-12`. Review of
`git diff phase-4..main` against AGENTS.md section 6, the layering (ADR
0002) and ADRs 0038-0049. The reviewer role is read-only: this task
changed no code.

Scope covered: `crates/model` (`mutate`, `connection`, `lookup`,
`error`), `crates/geom` (`hit`, `snap`, `routing`), `crates/render`
(`hit`, `overlay`), `crates/vsdx` (`package/edit`, `write/patch`,
`write/mod`), `crates/commands` (`undo`, `edit`, `clipboard`, `app`,
`builtins/*`), `crates/ui` (`tools/*`, `snap`, `handles`, `text_edit`,
`canvas`, `shell`, `panes`), the ADRs 0038-0049 and the task files.
The P5-11 product fixes were checked one by one (see "Checks that
passed").

## Verdict

**Approve with fixes: 0 blocker, 3 major, 7 minor.** The phase's hard
rules hold: no panic path outside test modules, bounded recursion and
capped input-derived counts everywhere the new code walks a document,
the ADR 0039 invariants are the ones implemented, the undo seam is one
edit per command with a working recording gate, the hit-test and snap
policy match ADR 0041/0043 including the cross-kind priority, and the
writer's additive changes follow ADR 0048. The three majors are one
non-atomic delete path (a nested selection deletes shapes with no undo
step), one non-atomic group path on extreme coordinates (it can also
write a non-finite cell the model's own write API rejects), and the
edit dispatcher living in `crates/ui` where every programmatic caller
must re-implement it.

## Checks that passed

- **Undo shape (ADR 0040).** Every editing command answers exactly one
  `Outcome::edit`; P5-11's generated registry walk
  (`crates/commands/tests/undo_prop.rs`) fails on a new untested id, so
  the rule is enforced, not hoped. `AppCtx::push_edit` is the only push
  path (`crates/ui/src/shell.rs:1177`, `crates/commands/src/app.rs:491`),
  it finds the session by `SessionId`, and it drops plus warns while the
  recording gate is closed. `run_suspended` saves and restores the flag
  (`crates/commands/src/undo.rs:19`), so nested `CompositeEdit` runs
  keep the gate closed, and the runner's zero-capacity placeholder
  stack is inert (the P4-10/P4-15 items are closed). `UndoStack::
  default` now has the default capacity (`undo.rs:66`), `with_undo`
  puts the stack back by `SessionId` (`builtins/view.rs:84`).
- **Undo inverses.** `CellEdit` captures the exact prior entries of the
  target and its derived cells and `restore_cells` puts them back entry
  for entry (`builtins/edit.rs:137`); `StructuralEdit` is
  detach/restore over `DetachedShapes`; `GeometryEdit` keeps the prior
  section list. The inverse property over random command sequences and
  the per-command pair tests (P5-11) compare reachable state and pass
  on a 900-sequence soak.
- **ADR 0039 invariants against the implementation.** Ids are stable:
  delete is `detach_shape` and the arena never drops an entry
  (`mutate.rs:732`, `:796`). List indices clamp everywhere
  (`insert_index`, `set_z_order`, `restore_shapes`, `move_page`).
  `reparent_shape` refuses self- and ancestor-cycles
  (`mutate.rs:849-870`). Ids are allocated (`next_visio_id` walks the
  page tree, bounded by `MAX_WALK`; `unique_name_u` appends ` (n)`).
  `add_connect` de-duplicates per endpoint and `detach_shape`/
  `delete_page` remove and return the records naming removed shapes.
- **Never-crash.** No `unwrap`/`expect`/`panic!` outside `#[cfg(test)]`
  modules in the new code (a full scan of every `crates/*/src` file
  agrees with the workspace clippy lints). Recursion is bounded
  (`MAX_WALK`, `MAX_CHAIN` in `mutate.rs:120-127`, `MAX_DEPTH` in
  `render/src/hit.rs:22` and `handles.rs:253`, `MAX_FLATTEN_DEPTH` in
  `geom/src/hit.rs:33`). Parser crates use `get()` (`write/patch.rs`
  `claim`, `render/src/hit.rs` `resolve`). Input-derived counts are
  capped (`MAX_SHAPES`, `MAX_LIST`, `MAX_PATH_POINTS`), snap and cell
  writes reject non-finite values (`checked_number`, `checked_distance`),
  and a stale `ShapeId`/`PageId`/`SessionId` on every command path is a
  typed error, not a panic (`document_mut`, `sheet_cells`,
  `selection_targets`).
- **Hit-test and snap policy (ADR 0041, ADR 0043).** `hit_test` walks
  in paint order and answers topmost first, one hit per shape node with
  the strongest part winning (`render/src/hit.rs:107`); the marquee is
  intersection of transformed bounds (`:128`); groups map to the
  top-level ancestor through `selectable_shape` (`ui/tools/pointer.rs:
  245-249`). Tolerances convert at the edge with the 96 px-per-inch
  constant: 4 px pointer, 6 px handles, 8 px snap. `crates/ui::snap`
  implements ADR 0043 exactly: delta snapping for moves with the
  stationary/moving split, per-axis resize snapping, `ConnectionPoint`
  over `Alignment` over `Grid` even when farther
  (`snap_axis_priority`, `ui/src/snap.rs:298`), Alt off for the
  gesture. `geom::snap` is unit-pure and rejects bad distances.
- **Writer additions (ADR 0048).** `write_document` patches the
  preserved package; a model entity no source element claims is emitted
  at its canonical position (`write/mod.rs:190-276`,
  `write/patch.rs` `sync_children`). Deleted pages and masters lose
  their part, their relationships and their content-type override
  (`prune_orphan_parts` + `Package::remove_part`,
  `package/edit.rs:21`; internal targets resolve against their source
  and external relationships survive). Relationship ids are allocated
  against the ids the source part already declares (`fresh_rel_id_from`).
- **P5-11's eight product fixes** are correct and narrow: `Connect`
  records return at their recorded list indices (`mutate.rs:826-840`,
  ascending insert); `ZOrderEdit` records its apply-time index before
  it moves (`arrange.rs:1013`); group and ungroup undo are exact
  inverses (`capture_shapes`/`dissolve`/`reuse_slot`/`place_at`,
  `arrange.rs:455-560`; `capture_group`/`regroup_exact`, `:727-790`);
  `PagesEdit` keeps the live `shapes`/`Connects` of surviving pages and
  resolves pages by Visio `ID` (`page.rs:100-140`, `PageKey` in
  `builtins/edit.rs:177-214`); paste, duplicate and glue-on-drop
  re-route; a grouped connector maps through the parent chain; the
  re-route pass no longer hijacks glue-on-drop instances; and the drag
  frame caches the page scene per `(session, changes, page)`
  (`ui/tools/pointer.rs:39-126`) with the moving connection points
  hoisted out of the snap loop. Each fix keeps to its layer.

## Findings

| id | severity | file:line | problem | suggested fix |
|----|----------|-----------|---------|---------------|
| F1 | major | crates/commands/src/builtins/edit.rs:607-619 | `edit.delete` of a shape and its own group applies half the steps, then errors with no undo step | filter nested targets (or roll the applied steps back on error) |
| F2 | major | crates/model/src/mutate.rs:1410-1470 | `group_shapes` writes non-finite cells and leaves a half-built group when a write fails | validate `width`/`height`/`centre` before mutating; make the failure atomic |
| F3 | major | crates/ui/src/shell.rs:1166 | the `Outcome::edit` dispatcher lives in L6 and is re-implemented by every programmatic caller | move `dispatch_one` into `larrygram_commands` |
| F4 | minor | crates/commands/src/builtins/page.rs:395 | dead `document_pages` helper kept alive by `#[allow(dead_code)]` | delete it and its test line |
| F5 | minor | crates/commands/src/builtins/edit.rs:303,352 | stale `#[allow(dead_code)]` on `Polarity::Insert`/`StructuralEdit::insertion`, which now have callers | drop the attributes |
| F6 | minor | crates/commands/src/builtins/edit.rs:60 | `number_literal` duplicates `model::mutate`'s `literal` formula spelling | one exported spelling helper, or a pinned mirror test |
| F7 | minor | crates/model/src/mutate.rs:1567,1672 | `write_connector_path` drops a connector's extra `Geometry` sections | write one path per section, or document the collapse |
| F8 | minor | crates/ui/src/snap.rs:332 | a snap-math failure surfaces as `UiError::Io` | add a geometry/invalid-argument variant |
| F9 | minor | crates/commands/src/builtins/arrange.rs:625,843 | `Edit::apply`/`revert` mutate the selection as a side effect | document it in ADR 0040 or restore the prior selection |
| F10 | minor | crates/model/src/mutate.rs:1617,1703,1761 | id and subtree walks are O(n²) and run per insert | a seen set (or an id counter) instead of `Vec::contains` |
| F11 | minor | crates/model/src/mutate.rs:672-680 | an `insert_shape` error after `add_shape` leaves an unnamed arena entry | validate the children before adding the shape |

### Major

1. **`edit.delete` of a nested selection deletes with no undo step.**
   `run_delete` builds one `StructuralEdit::removal` per selected shape
   and runs them with one `CompositeEdit::apply`
   (`crates/commands/src/builtins/edit.rs:607-619`). When the selection
   holds a group **and** one of its members, the first step detaches the
   group (the member goes with it) and the second step fails:
   `Document::detach_shape` answers `UnknownShape` for a shape no page
   reaches (`crates/model/src/mutate.rs:735`). The `?` then returns the
   error with the group already gone and no edit recorded, so the
   document lost shapes and `edit.undo` says "Nothing to undo".
   *Reproducing path:* open a document with a group; `edit.select`
   `shapes` = [the group's Visio `ID`, its member's Visio `ID`] (the
   `edit.select` command resolves sub-shapes, `shape_by_visio_id`
   walks children, `crates/model/src/document.rs:125`); `edit.delete`.
   Order matters: member-first deletes both correctly, group-first
   loses the group. The UI cannot build this selection (a click maps to
   the top-level ancestor) but `edit.select` is a shipped command and
   the Phase 8 control channel is exactly this path. **Fix:** in
   `run_delete` (or `selection_targets`) drop any target whose
   ancestor is also a target, with one warning line; alternatively roll
   back the applied steps when the first `apply` fails, so a failed
   command is always a no-op.

2. **`group_shapes` is not atomic on its failure path and writes
   non-finite cells.** The bounding-box check tests the `min`/`max`
   corners for finiteness but not the derived `width`, `height` and
   `centre` (`crates/model/src/mutate.rs:1410-1415`): two shapes whose
   cached pins sit near +1e308 and -1e308 give `max.x - min.x = inf`,
   and `number_cell("Width", inf)` writes it (and the formula `"inf"`)
   into the group's sheet **without** the `checked_number` rule every
   other write path applies. The member loop then re-pins each member
   (`:1444-1470`); its first `write_cell` fails `checked_number`
   (`pin.x - min.x` overflows) **after** the group is on the page and
   the member is already re-parented into it, so `arrange.group`
   answers an error with a half-built group left on the page and no
   undo step. `ungroup_shape` has the same shape: it writes the
   members' cells before it detaches (`:1487-1520`), so a mid-loop
   failure leaves some members re-based and the group intact.
   *Reproducing path:* a readable file with two shapes at extreme
   cached `PinX` values (our reader accepts any finite cached number),
   select both, `arrange.group`. **Fix:** check `width`, `height` and
   the member deltas for finiteness before the first mutation (they are
   all computable up front), and return `Error::BadCell` early; the
   atomicity half of this finding overlaps F1's rollback suggestion —
   one rule ("a failed mutation changes nothing") recorded in ADR 0039
   would close both.

3. **The `Outcome::edit` dispatcher lives in `crates/ui`.**
   `dispatch_one` (`crates/ui/src/shell.rs:1166`) is the only
   production code that pushes an edit: it records the session before
   `execute`, takes the edit from the outcome and calls `push_edit`
   with that session. ADR 0040 names this "the dispatcher of
   `Outcome::edit`", but the rule lives one layer above the crate that
   owns it, so every programmatic caller must re-implement it: the ten
   `crates/commands/tests/*` files each carry their own `dispatch`
   helper doing the same dance (21 `push_edit` sites), and a Phase 8
   control-channel or CLI editing command that calls
   `Registry::execute` directly will silently drop its undo step (or
   worse, push against the wrong session if it copies the wrong half).
   **Fix:** move the helper into `larrygram_commands` (e.g.
   `commands::dispatch(registry, ctx, command, invocation)` returning
   `Result<Outcome, CommandError>` with the push inside), have
   `shell::dispatch_one` wrap it, and let the test helpers call it.
   This is the one real `commands`/`ui` duplication of the phase.

### Minor

4. **Dead helper kept alive by an allow.** `document_pages`
   (`crates/commands/src/builtins/page.rs:395-398`) has no caller; only
   its own test asserts it. Delete both, or give the writer rule a real
   caller.

5. **Stale `#[allow(dead_code)]`.** `Polarity::Insert`
   (`crates/commands/src/builtins/edit.rs:303`) and
   `StructuralEdit::insertion` (`:352`) say "until P5-06..P5-08 use
   them", and they now do (`builtins/insert.rs:122,189`,
   `builtins/clipboard.rs:394`). Drop the attributes so a later
   genuinely-dead arm is still reported.

6. **One spelling rule, two copies.** `commands::builtins::edit::
   number_literal` (`crates/commands/src/builtins/edit.rs:60`) and
   `model::mutate::literal` (`crates/model/src/mutate.rs:200`) both
   implement "0 for zero, shortest round-trip otherwise" for the
   written formula text; a divergence would make undo-redo write a
   different `F` than the first apply. Export the model's spelling (or
   a `CellWrite::number` constructor) and use it in both, or pin the
   mirror with a test the way `touched_cells` is pinned.

7. **`write_connector_path` collapses multi-path geometry.**
   `replace_geometry_sections` with one fresh section drops every
   trailing `Geometry` section (`crates/model/src/mutate.rs:1672-1681`),
   so a connector instance that carries two geometry paths loses the
   second on the first re-route. The doc comment says the path "lands
   in the shape's own `Geometry` section" but not that the other
   sections go away. Either write one routed path per existing section
   (or merge the paths into the first) or record the collapse in the
   comment and ADR 0045.

8. **A snap-math failure is an `Io` error.** `geom_error`
   (`crates/ui/src/snap.rs:328-335`) maps `geom::Error::InvalidArgument`
   onto `UiError::Io { message: "snap math failed: …" }`. Nothing does
   IO; the variant misleads the error report. Add an
   `InvalidArgument`/`Geometry` variant to `UiError` (the P5-03 master
   fix introduced the typed `geom::Error`; the UI layer drops it again).

9. **Undo changes the selection.** `GroupEdit`/`UngroupEdit`
   `apply`/`revert` call `select(...)`
   (`crates/commands/src/builtins/arrange.rs:625,843`) and
   `PagesEdit::restore` clears the selection when the active page
   changed (`crates/commands/src/builtins/page.rs:134-137`). The undo
   property ignores `AppCtx` state so this is untested behaviour the
   user sees: undoing a group leaves the members selected. That may be
   the desired UX, but it is a rule nobody wrote down. Record it in ADR
   0040 (or capture and restore the selection in the edit).

10. **Quadratic identity walks.** `next_visio_id`, `subtree_ids` and
    `walk_position` (`crates/model/src/mutate.rs:1617,1703,1761`) keep
    their visited set in a `Vec` and test with `contains`, and
    `next_visio_id` runs on every `insert_shape`. The `MAX_WALK`
    caps keep it bounded, but building a 10k-shape page is O(n²) in
    comparisons, and `instantiate_master` also clones every arena
    shape's `name_u` per drop (`:1244`). Use a `BTreeSet`/`HashSet`
    (or a monotonically increasing visit stamp) and index the taken
    names once per document build. No crash: performance only.

11. **`insert_shape` errors leave an unnamed arena entry.** When a
    `Shape::children` entry names a stale id, the function has already
    added the new shape to the arena and returns `UnknownShape(child)`
    (`crates/model/src/mutate.rs:672-680`); the caller never learns the
    new shape's id and cannot drop it. Harmless per ADR 0039 (detached
    entries are unreached), but the orphan is avoidable: validate the
    children first and add the shape afterwards.

## Known open items (not re-filed here)

The `docs/STATUS.md` "Deferred minor findings" section already tracks
these and this review agrees with their recorded severity: the writer's
`fresh_rel_id` allocation instead of `Package::next_relationship_id`
and the new page with no contents part (P5-02 — the added
`next_relationship_id` (`crates/vsdx/src/package/edit.rs:52`) is
currently public with no production caller, same finding, same fix);
`restore_cells` writing `Sheet` outside the mutation API and the third
`GeometryEdit` type (P5-04); the rotate-handle offset and the private
`builtins::view::page_size` (P5-05 — the 8.5 x 11 in fallback is now
spelled in `crates/render::build`, `builtins/view.rs` and
`crates/ui/src/canvas.rs:80`, which the fix pass should fold into one
exported helper); the materialised master left behind by an undone
first drop (P5-06); the one-run text editor (P5-07); the re-route pass
rewriting Visio-routed paths and `reroute_all` bumping `changes` when
nothing moved (P5-08); `snap_axis` ties and the rotated-bounds and
clip-free hit rules (P5-03, black-box checks); the dead defensive arms
(P5-15); the missing registry checked-state seam and Escape handling
(P5-17); the detached-arena comparison rule and the 12-significant-digit
path fold (P5-11). The `Edit::label` of the inner `PathEdit` being
unread (P5-11) is cosmetic and its fix is the same as F5's tidy.

## Gate

`cargo xtask gate --quick` in the worktree: green (codegen, parity,
fmt, clippy, test, doc, deny, layers). The reviewer changes no code, so
no slow step is triggered (AGENTS.md section 7).
