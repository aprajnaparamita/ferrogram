# Phase 5 report: Editing core

Phase 5 turns the Phase 4 viewer into an editor. Shapes can be selected, moved, resized, rotated, drawn, dropped from the Shapes window, connected with glued connectors, given text, arranged and grouped, across several pages, and every edit undoes. Saving keeps the ADR 0048 identity, so a document can be saved, reopened and edited again. Formulas are still not evaluated (ADR 0038): edits write literal values. Started 2026-10-18, closed 2026-10-21, tag `phase-5`.

## What was built

| Area | Content | Tasks |
|---|---|---|
| `crates/model` mutation API (L1) | `Document` mutations with the ADR 0039 invariants: stable arena ids (delete detaches, never drops), clamped list indices, no reparenting cycles, allocated Visio ids and unique names. `Connect` records with the transcribed `VisFromParts`/`VisToParts` constants. Group and ungroup, master instantiation, connector path writes. Connection points from the shape's own rows or the nearest master's, scaled to the instance and indexed by row `IX`. A failed mutation changes nothing (ADR 0039 amendment). | P5-02, P5-16, P5-17, P5-19 |
| `crates/vsdx` writer (L2) | Patching instead of re-emitting: a model entity no source element claims is emitted at its canonical position; deleted pages and masters lose their part, relationships and content-type override; shape text is written back (ADR 0048). | P5-02, P5-20 |
| `crates/geom` and `crates/render` (L0, L3) | Hit testing in paint order with the strongest part winning, marquee intersection, snapping primitives, the deterministic one-elbow router (ADR 0041, 0043, 0045), and the selection and preview overlay. | P5-03, P5-08 |
| `crates/commands` (L4) | One undo seam: `Outcome::edit`, `AppCtx::push_edit` and `ferrogram_commands::dispatch`, used by the shell, the CLI and every test (ADR 0040). Selection through `edit.select`; move, resize, rotate and cell edits; `insert.shape` and the drawing inserts; `shape.set.text` and the `text.*` commands; connector glue, unglue, style, re-route and `connector.draw`; cut, copy, paste, duplicate, group, ungroup, align, distribute and z-order; page add, delete, rename, reorder and navigation; recent files; tool choice (`tools.*`). Re-routing runs after every edit that moves a glued shape. | P5-04, P5-06..P5-09, P5-16, P5-17, P5-19 |
| `crates/ui` (L6) | The `ToolMachine` with pointer, rectangle, ellipse, line, text, connector and stencil tools, each keeping its drag in its own field. Handles, marquee, snapping with Alt to disable, an 8 ms drag frame on a 1000-shape page (ADR 0042). Editable Size & Position and Shape Data panels. Shapes-window drag and drop with glue on drop (ADR 0046). The in-place text editor with the Text shortcut scope (ADR 0044). The page-tab context menu. | P5-05..P5-09, P5-11, P5-17 |
| Parity | `docs/parity/controls.md` gains a second table with every registry id, command-only ids included; the gate checks both tables. | P5-10 |
| Test harness | The exit-criterion flowchart test, the undo inverse property over random command sequences (every implemented id has a case or a reason), glue and re-route tests, the drag timing harness, writer insert-direction round trips, per-target fuzz seed directories. | P5-10, P5-11, P5-15, P5-18, P5-20 |

## Exit criterion

**A user can build a 10-shape flowchart with connected shapes across two pages, save it, reopen it and keep editing: met.**

`apps/ferrogram/tests/flowchart.rs` (`the_flowchart_exit_criterion`) drives the real `Shell` and the builtin registry with `Source::Test`:

- `file.new`, `page.add`, ten `insert.shape` dispatches (Terminator, Process and Decision masters), six shapes on page one and four on page two;
- five glued connectors in both forms: three `connector.draw` (outline and connection-point glue) and two `insert.shape` plus `connector.glue_begin`/`connector.glue_end`;
- `file.save` to a temporary path, `file.open` of the same path, and the ADR 0048 model equality of the saved and reopened documents, plus the literal `read(write(read(write(d)))) == read(write(d))` identity and byte-identical re-saves;
- `edit.select` and `shape.move` of the decision shape, with the change counter bumped and both glued connectors' routed ends following the move, then a successful second `file.save`.

The comparison renumbers `ShapeId`s into document order, because writing and reading renumber the arena, and drops two reader artefacts (see Known issues).

## Gate results

```
cargo xtask gate (full, 2026-10-21, main after the P5-20 merge, 688cb8b)
  PASS  codegen
  PASS  parity
  PASS  fmt
  PASS  clippy
  PASS  test      (2531 tests; 2 ignored: the fixture regenerator and the oracle smoke test)
  PASS  doc
  PASS  coverage
  PASS  deny
  PASS  layers
  PASS  wasm
  PASS  fuzz      (crates/ui-spec/fuzz, crates/vsdx/fuzz, crates/render/fuzz, crates/ui/fuzz)
gate: GREEN

FERROGRAM_REQUIRE_CORPUS=1 cargo test --workspace --all-features: OK
```

## Coverage

| Crate | Lines | Coverage | Threshold | Phase 4 ratchet | New ratchet |
|---|---|---|---|---|---|
| ferrogram-geom | 2501 | 100.0% | 85 | 100.0 | 100.0 |
| ferrogram-shapesheet | 45 | 100.0% | 85 | 100.0 | 100.0 |
| ferrogram-model | 10750 | 99.7% | 85 | 99.6 | 99.7 |
| ferrogram-ui-spec | 1884 | 99.4% | 85 | 99.3 | 99.3 |
| ferrogram-text | 1768 | 98.9% | 75 | 98.8 | 98.8 |
| ferrogram-render | 7221 | 98.5% | 75 | 98.4 | 98.5 |
| ferrogram-commands | 8047 | 98.3% | 85 | 98.1 | 98.3 |
| ferrogram-vsdx | 7498 | 98.1% | 85 | 97.9 | 98.1 |
| ferrogram-ui | 10582 | 96.8% | 50 | 96.4 | 96.8 |
| ferrogram-cli | 1266 | 96.8% | 50 | 96.7 | 96.7 |
| xtask | 10443 | 93.4% | — | 93.3 | 93.4 |
| ferrogram | 250 | 80.0% | 50 | 80.0 | 80.0 |
| ferrogram-web | 140 | 77.9% | — | 77.8 | 77.8 |

The new ratchet is each crate's exact value rounded down to one decimal place. `ferrogram-control` and `ferrogram-vsd-binary` are still empty placeholders. During the phase `ferrogram-commands` fell under its ratchet twice (97.8% after P5-09); P5-18 and P5-19 restored it with tests and gave it margin.

## Reviewer findings and resolution

| Report | Verdict | Blocker / major / minor | Resolution |
|---|---|---|---|
| [Test pass](../reviews/phase-5-tests.md) (P5-11) | — | eight product bugs | Fixed in P5-11: `Connect` restore order, z-order and group undo, page snapshot restore, missing re-route after paste/duplicate/drop glue, grouped connector transform, drop glue treated as a connector, and an 80–110 ms drag frame (now 0.6–1.6 ms). |
| [Code review](../reviews/phase-5-review.md) (P5-12) | approve with fixes | 0 / 3 / 7 (F1..F11) | All fixed in P5-19: nested delete, atomic group/ungroup, the dispatch seam moved into `ferrogram_commands`, and the minors. P5-19's F9 test found and fixed one more bug (ungroup undo selected a discarded id). |
| [Coverage](../reviews/phase-5-coverage.md) (P5-13) | pass, no margin | 2 / 10 / 22 | The two blockers were the commands and render ratchets, met only within the gate's 0.05 tolerance. They and the majors went to P5-19 (commands, model) and P5-20 (vsdx writer insert direction, UI tool state, render). |
| [Clean-room](../reviews/phase-5-cleanroom.md) (P5-14) | pass | 0 / 0 / 2 | Both fixed on main: a split `docs/sources.md` table and the proptest attribution row. |

## Fixes made on main by the master

- The draw and stencil drags kept their state in address-keyed thread-locals; they moved into `ToolMachine` fields (`882e7b1`).
- `reroute_after_edit` re-routed a page by position in whichever document was active; it now acts on the active document only, with a test.
- The P5-07 text-tool test registered a fake insert that collided with the real `insert.drawing.text_box`; it uses the real command.
- Merge conflicts between unrebased P5-07 and P5-08 branches in `tools/mod.rs` were resolved by hand.
- RUSTSEC-2026-0382 (`rustls-webpki`, reached only through `ureq` in xtask) was published during the phase; bumped to 0.103.16.
- The three 8.5 x 11 in page-size fallbacks became one `ferrogram_render::FALLBACK_PAGE_SIZE`.

## Decisions made during the phase

- **Edits without a formula engine:** every edit writes literal values and cached results (ADR 0038). Phase 6 adds evaluation.
- **One undo seam:** a command returns its edit in `Outcome::edit`; `ferrogram_commands::dispatch` pushes it on the session the command ran against. Selection and tool choice are not edits (ADR 0040, amended).
- **A failed mutation changes nothing:** every derived value is checked before the first write (ADR 0039, amended).
- **Re-routing is ours:** connectors route straight or with one deterministic elbow and are re-routed after every edit that moves a glued shape; the first edit of a Visio-routed file replaces Visio's paths (ADR 0045).
- **Glue names rows by `IX`:** connection points are indexed by their row, so a malformed row never renumbers the glue.
- **`connector.draw`:** a Ferrogram-only command that inserts, glues and routes a connector in one undo step, so one drag is one undo step.
- **Undo selection rule:** group and ungroup undo select the affected shapes; a page restore clears a stale selection (ADR 0040, amended).

## ADRs added

- 0038 editing without a formula engine
- 0039 model mutation API (amended: failed mutations change nothing)
- 0040 edits, undo and the change counter (amended: dispatch seam, selection rule)
- 0041 selection and hit testing
- 0042 tools, handles and drag redraw
- 0043 snapping
- 0044 shape text editing
- 0045 dynamic connectors (amended: multi-path connector geometry collapses to one path)
- 0046 stencil drag and drop
- 0047 clipboard, arrangement and pages
- 0048 saving and round-trip identity
- 0049 Phase 5 test strategy

## Known issues

- **Formulas are not evaluated.** Moving a shape writes literal pins; dependent cells keep their old cached values. 5 of the corpus files still render blank for lack of cached values (Phase 6).
- **CI has still never run** (no remote).
- **Ribbon coverage:** 25 of 706 ribbon controls and 84 of 977 registry commands are implemented; the rest are stubs or out of scope.
- **Undo:** group, insert and drop undo leave detached arena entries; the first drop of a catalog master leaves the materialised master behind after undo.
- **Text:** the editor commits one run, so multi-run formatting collapses; typing on a selected shape does not open the editor.
- **Connectors:** outline glue uses page-aligned bounds, not the rotated outline; a free end moves with the connector; `connector.reroute_all` bumps the change counter when nothing changed.
- **Arrange:** align and distribute ignore rotation; a cross-document paste can keep a `Master` link the target lacks.
- **Tools:** the registry has no checked state for tool buttons; Escape does not return to the pointer tool; the draw, connector and stencil drags still rebuild the page scene per snap call.
- **Reader artefacts:** `crates/vsdx` keeps the writer's `U="BOOL"`/`U="STR"` spelling and a read page's `<Rel>` child in `Preserved`, which the exit test's comparison has to drop.
- **Black-box checks:** rotate handle offset, marquee rule, Alt as the snap modifier, `file.new` with no pages, text commit on click outside, glue cell spelling.
- The rest of the deferred items are in `docs/STATUS.md`.
