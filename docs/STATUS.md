# Status

Task board. Only the master agent edits this file, on `main`.

- Current phase: **5 in progress** (group E: P5-10)
- Phase 5 goal: editing core (KICKOFF.md). Exit: a user can build a 10-shape flowchart with connected shapes across two pages, save it, reopen it and keep editing.
- Phase 4 complete (2026-10-18, report: [progress/phase-4.md](progress/phase-4.md), tag `phase-4`).
- Phase 3 complete (2026-10-16, report: [progress/phase-3.md](progress/phase-3.md), tag `phase-3`).
- Phase 2 complete (2026-10-14, report: [progress/phase-2.md](progress/phase-2.md), tag `phase-2`).

## Phase 5 task board

| Task | Role | Depends on | Group | State | Branch |
|---|---|---|---|---|---|
| [P5-01](tasks/P5-01.md) | architect | Phase 4 | — | merged (ADRs 0038–0049, stubs, task files) | agent/P5-01 |
| [P5-02](tasks/P5-02.md) | implementer | P5-01 | A | merged (mutation API, connects, writer deletes and text) | agent/P5-02 |
| [P5-03](tasks/P5-03.md) | implementer | P5-01 | A | merged (hit-testing, snapping, overlay; error variants fixed by the master) | agent/P5-03 |
| [P5-04](tasks/P5-04.md) | implementer | P5-02 | B | merged `c800fb4` (push_edit, undo steps, editing commands) | — |
| [P5-15](tasks/P5-15.md) | test-engineer | P5-02 | B | merged `22342d5` (model 97.70% to 99.62%, tests only) | — |
| [P5-05](tasks/P5-05.md) | implementer | P5-03, P5-04 | C | merged `199132a` (pointer tool, handles, marquee, snapping; P4-05, P4-07, P4-14 closed) | — |
| [P5-06](tasks/P5-06.md) | implementer | P5-05 | D | merged `73f0265` (insert.shape, drawing commands, draw and stencil tools; state moved into `ToolMachine` by the master) | — |
| [P5-07](tasks/P5-07.md) | implementer | P5-05 | D | merged `aba6efd` (shape.set.text, text.* commands, TextEditor, text tool; editor not yet shown, see P5-17) | — |
| [P5-16](tasks/P5-16.md) | implementer | P5-05 | D | merged (`edit.select`, `Document::connection_points`) | — |
| [P5-08](tasks/P5-08.md) | implementer | P5-05, P5-16 | D | merged `91124b0` (routing, glue commands, re-route hook, connector tool; master fixed the re-route document lookup) | — |
| [P5-09](tasks/P5-09.md) | implementer | P5-05 | D | merged (clipboard, arrange, pages, recent files, page-tab menu) | — |
| [P5-17](tasks/P5-17.md) | implementer | P5-06, P5-07, P5-08 | D2 | merged `b9fbb07` (tool commands, Shapes-window drag, text editor in the shell, indexed glue, `connector.draw`) | — |
| [P5-18](tasks/P5-18.md) | test-engineer | P5-08 | D2 | merged `3c9da58` (tests only; commands coverage 98.1% on main, at the ratchet; merge gate green) | — |
| [P5-10](tasks/P5-10.md) | implementer | P5-06..P5-09, P5-16..P5-18 | E | ready (flowchart exit harness, parity commands table) | — |
| [P5-11](tasks/P5-11.md) | test-engineer | P5-10 | F | waiting | — |
| [P5-12](tasks/P5-12.md) | reviewer | P5-11 | G | waiting | — |
| [P5-13](tasks/P5-13.md) | coverage-auditor | P5-11 | G | waiting | — |
| [P5-14](tasks/P5-14.md) | cleanroom-auditor | P5-11 | G | waiting | — |

Tasks in one group run in parallel on disjoint files. Shared files: `crates/commands/src/builtins/mod.rs`, `crates/commands/src/app.rs` and `xtask/src/extract/commands.rs` belong to P5-04 (later tasks use their own `TABLE` modules); `crates/ui/src/{lib,shell,canvas}.rs` and `crates/ui/Cargo.toml` to P5-05 (later tasks request lines in their Results); `crates/model/src/mutate.rs` and the `vsdx` writer to P5-02. The master resolves `Cargo.lock`. Fix tasks follow the audits.

## Phase 4 task board

| Task | Role | Depends on | Group | State | Branch |
|---|---|---|---|---|---|
| [P4-01](tasks/P4-01.md) | architect | Phase 3 | — | merged (ADRs 0028–0037, stubs) | agent/P4-01 |
| [P4-02](tasks/P4-02.md) | implementer | P4-01 | A | merged (CLI on the registry; zoom range shared by the master) | agent/P4-02 |
| [P4-04](tasks/P4-04.md) | implementer | P4-01 | A | merged (registry-backed test added by the master) | agent/P4-04 |
| [P4-05](tasks/P4-05.md) | implementer | P4-01 | A | merged (`show_rulers` takes units; notes for P4-03) | agent/P4-05 |
| [P4-06](tasks/P4-06.md) | implementer | P4-01 | A | merged (12 + 9 original masters) | agent/P4-06 |
| [P4-03](tasks/P4-03.md) | implementer | P4-02 | B | merged (shell, settings, pickers, both apps) | agent/P4-03 |
| [P4-07](tasks/P4-07.md) | implementer | P4-03 | C | merged (read-only panels) | agent/P4-07 |
| [P4-08](tasks/P4-08.md) | implementer | P4-03 | C | merged (shortcut map and dispatch; shell pass in P4-14) | agent/P4-08 |
| [P4-14](tasks/P4-14.md) | implementer | P4-08 | C2 | merged (scope-aware bindings, shell shortcut pass) | agent/P4-14 |
| [P4-09](tasks/P4-09.md) | implementer | P4-03..P4-08, P4-14 | D | merged (parity generator and gate step, Phase 4 harness) | agent/P4-09 |
| [P4-10](tasks/P4-10.md) | test-engineer | P4-09 | E | merged (54 tests, ui fuzz crate, one interner fix) | agent/P4-10 |
| [P4-11](tasks/P4-11.md) | reviewer | P4-10 | F | merged (approve with fixes: 0 blocker, 4 major, 12 minor) | agent/P4-11 |
| [P4-12](tasks/P4-12.md) | coverage-auditor | P4-10 | F | merged (pass: 0 blocker, 6 major, 10 minor) | agent/P4-12 |
| [P4-13](tasks/P4-13.md) | cleanroom-auditor | P4-10 | F | merged (pass: 0 blocker, 0 major, 1 minor, fixed on main) | agent/P4-13 |
| [P4-15](tasks/P4-15.md) | implementer | P4-11, P4-12 | G | merged (commands fixes, `view.units`, options/quit commands) | agent/P4-15 |
| [P4-17](tasks/P4-17.md) | test-engineer | P4-12 | G | merged (gate.rs 90.6%, parity 99.1%, ui fuzz seeds) | agent/P4-17 |
| [P4-16](tasks/P4-16.md) | implementer | P4-15 | H | merged (shell/UI fixes, Options through the registry, ui 96.5%) | agent/P4-16 |

Tasks in the same group run in parallel on disjoint files. Shared files: `crates/ui/src/lib.rs` and `crates/ui/Cargo.toml` belong to P4-03 (others request additions under Open issues); `xtask/src/extract/*` mapping rules to P4-02; `xtask/src/{main,gate}.rs` and `xtask/Cargo.toml` to P4-09; `ATTRIBUTION.md` rows come from P4-03 and P4-06; the master resolves `Cargo.lock`. Fix tasks follow the audits.

## Phase 3 task board

| Task | Role | Depends on | Group | State | Branch |
|---|---|---|---|---|---|
| [P3-01](tasks/P3-01.md) | architect | phase-2 | — | merged (ADRs 0019–0027, stubs, task files) | agent/P3-01 |
| [P3-02](tasks/P3-02.md) | implementer | P3-01 | A | merged (ADR 0021 amended: `Themed`, colour indices) | agent/P3-02 |
| [P3-03](tasks/P3-03.md) | implementer | P3-01 | A | merged (knot rule flagged for P3-14) | agent/P3-03 |
| [P3-04](tasks/P3-04.md) | implementer | P3-01 | A | merged (sources rows renumbered 11–12 on main) | agent/P3-04 |
| [P3-08](tasks/P3-08.md) | implementer | P3-01 | A | merged (added `Error::InvalidStyleData` in `error.rs`) | agent/P3-08 |
| [P3-05](tasks/P3-05.md) | implementer | P3-02, P3-03, P3-04, P3-08 | B | merged (added `CellReader` row readers in model; `TextNode` gained `runs` and `rotation`) | agent/P3-05 |
| [P3-15](tasks/P3-15.md) | implementer | P3-05 | B | merged (themed patterns and weights included) | agent/P3-15 |
| [P3-06](tasks/P3-06.md) | implementer | P3-02, P3-03, P3-04 | B | merged (conflict with P3-05 resolved on main; glyphs use their run colour) | agent/P3-06 |
| [P3-07](tasks/P3-07.md) | implementer | P3-02, P3-03, P3-04 | B | merged (stale base; glyph size, rotation and run colour wired on main) | agent/P3-07 |
| [P3-09](tasks/P3-09.md) | implementer | P3-05, P3-06, P3-07 | C | merged (page size fallback fixed on main) | agent/P3-09 |
| [P3-10](tasks/P3-10.md) | implementer | P3-05..P3-08, P3-15 | C | merged (render fuzz seed corpus re-included by the master) | agent/P3-10 |
| [P3-11](tasks/P3-11.md) | test-engineer | P3-09, P3-10 | — | merged (fixed a negative zero-advance line width in text) | agent/P3-11 |
| [P3-12](tasks/P3-12.md) | reviewer | P3-11 | D | merged (approve with fixes: 0 blocker, 4 major, 8 minor) | agent/P3-12 |
| [P3-13](tasks/P3-13.md) | coverage-auditor | P3-11 | D | merged (pass: 0 blocker, 5 major, 10 minor; ratchet proposal in report) | agent/P3-13 |
| [P3-14](tasks/P3-14.md) | cleanroom-auditor | P3-11 | D | merged (pass: 3 minor; F4 crash fixed by the master) | agent/P3-14 |
| [P3-16](tasks/P3-16.md) | implementer | P3-12..P3-14 | E | merged (also fixed PNG tile marks drawn pitch times too small) | agent/P3-16 |
| [P3-18](tasks/P3-18.md) | implementer | P3-12, P3-14 | E | merged (docs only; knot rule re-sourced) | agent/P3-18 |
| [P3-17](tasks/P3-17.md) | test-engineer | P3-16 | — | merged (56 tests; render 98.5%) | agent/P3-17 |

Tasks in the same group run in parallel on disjoint files. Shared files: `docs/sources.md` is owned by P3-04, and P3-02 and P3-08 each append rows (the master merges them). `ATTRIBUTION.md` is edited by P3-04 first, then P3-10. In `crates/render`, P3-05 owns `lib.rs`, `scene.rs`, `build.rs` and `Cargo.toml`. The master resolves `Cargo.lock`. Fix tasks follow the audits.

## Phase 2 task board

| Task | Role | Depends on | State | Branch |
|---|---|---|---|---|
| [P2-01](tasks/P2-01.md) | architect | phase-1 | merged (ADRs 0011–0017, stubs, task files) | agent/P2-01 |
| [P2-02](tasks/P2-02.md) | implementer | P2-01 | merged | agent/P2-02 |
| [P2-03](tasks/P2-03.md) | implementer | P2-01 | merged (use `UnitKind` for kind-checked conversion) | agent/P2-03 |
| [P2-05](tasks/P2-05.md) | implementer | P2-01 | merged (XML declaration added on main) | agent/P2-05 |
| [P2-08](tasks/P2-08.md) | implementer | P2-01 | merged (ATTRIBUTION row added on main) | agent/P2-08 |
| [P2-04](tasks/P2-04.md) | implementer | P2-02, P2-03 | merged | agent/P2-04 |
| [P2-06](tasks/P2-06.md) | implementer | P2-04, P2-05 | merged (angles are radians; 25 name aliases) | agent/P2-06 |
| [P2-06b](tasks/P2-06b.md) | implementer | P2-06 | merged (`Inh` fix on main) | agent/P2-06b |
| [P2-07](tasks/P2-07.md) | implementer | P2-06b | merged (exit criterion met: 15/15 corpus files round-trip) | agent/P2-07 |
| [P2-09](tasks/P2-09.md) | implementer | P2-06, P2-08 | merged (docs XSD is 1.1 and out of date; see P2-10 rule) | agent/P2-09 |
| [P2-10](tasks/P2-10.md) | test-engineer | P2-07, P2-09 | merged (1 writer bug fixed; vsdx 97.1%) | agent/P2-10 |
| [P2-11](tasks/P2-11.md) | reviewer | P2-10 | merged (approve with fixes: 0 blocker, 1 major, 6 minor; finding 7 fixed on main) | agent/P2-11 |
| [P2-12](tasks/P2-12.md) | coverage-auditor | P2-10 | merged (pass: 0 blocker, 3 major, 11 minor) | agent/P2-12 |
| [P2-14](tasks/P2-14.md) | implementer | P2-11, P2-12 | merged | agent/P2-14 |
| [P2-13](tasks/P2-13.md) | cleanroom-auditor | P2-10 | merged (pass: 0 blocker, 0 major, 2 minor; F1 fixed on main) | agent/P2-13 |

Tasks in the same group run in parallel on disjoint files. Fix tasks (P2-14 onward) follow the audits, as in Phase 1.

## Phase 1 task board

| Task | Role | Depends on | State | Branch |
|---|---|---|---|---|
| [P1-01](tasks/P1-01.md) | researcher | — | merged (corpus binaries excluded, ADR 0005) | agent/P1-01 |
| [P1-02](tasks/P1-02.md) | architect | P1-01 | merged ([review](tasks/P1-02-review.md)) | agent/P1-02 |
| [P1-03](tasks/P1-03.md) | implementer | P1-02 | merged ([review](tasks/P1-03-review.md)) | agent/P1-03 |
| [P1-04](tasks/P1-04.md) | implementer | P1-03 | merged (align ids fixed on main) | agent/P1-04 |
| [P1-05](tasks/P1-05.md) | implementer | P1-02 | merged (OPC plumbing parts set to supported on main) | agent/P1-05 |
| [P1-06](tasks/P1-06.md) | implementer | P1-02 | merged (schema issues 1–2 fixed on main) | agent/P1-06 |
| [P1-06b](tasks/P1-06b.md) | implementer | P1-06 | merged | agent/P1-06b |
| [P1-07](tasks/P1-07.md) | implementer | P1-02 | merged (oracle built locally, smoke test passes) | agent/P1-07 |
| [P1-12](tasks/P1-12.md) | implementer | — | merged | agent/P1-12 |
| [P1-08](tasks/P1-08.md) | test-engineer | P1-03..07 | merged | agent/P1-08 |
| [P1-09](tasks/P1-09.md) | reviewer | P1-08 | merged (approve with fixes: 0 blocker, 2 major, 8 minor) | agent/P1-09 |
| [P1-10](tasks/P1-10.md) | coverage-auditor | P1-08 | merged (pass: 0 blocker, 2 major, 7 minor) | agent/P1-10 |
| [P1-11](tasks/P1-11.md) | cleanroom-auditor | P1-08 | merged (pass with findings: 0 blocker, 1 major, 4 minor) | agent/P1-11 |
| [P1-13](tasks/P1-13.md) | implementer | P1-09..11 | merged | agent/P1-13 |
| [P1-14](tasks/P1-14.md) | implementer | P1-09..10 | merged | agent/P1-14 |

States: not written · outline · ready · in progress · in review · merged · blocked.

## Phase history

| Phase | Report | Gate |
|---|---|---|
| 0 | [phase-0.md](progress/phase-0.md) | green (2026-10-12) |
| 1 | [phase-1.md](progress/phase-1.md) | green (2026-10-13) |
| 2 | [phase-2.md](progress/phase-2.md) | green (2026-10-14) |
| 3 | [phase-3.md](progress/phase-3.md) | green (2026-10-16) |
| 4 | [phase-4.md](progress/phase-4.md) | green (2026-10-18) |

Shared file note: P1-03, P1-04, P1-05 all add dependencies to `xtask/Cargo.toml` (and `Cargo.lock`); P1-07 adds an arm to `xtask/src/main.rs`. The master resolves these merge conflicts.

Open master items: none (the codegen task is part of the P2-01 plan).

## Deferred minor findings (for the phase report)

- P1-12: `download` writes the file before verifying sha256; verifying the in-memory body first would avoid ever placing a bad file at the final path.
- P1-12: `ureq` uses the private `_ring` feature; it may break on a ureq minor update. Revisit if the webpki-roots licence (CDLA-Permissive-2.0) is accepted in `deny.toml`.
- P1-07: CI does not build the oracle yet. When the GitHub remote exists, add an oracle build step on Linux and macOS (apt/brew deps from `tools/libvisio-oracle/README.md`), set `FERROGRAM_LIBVISIO_ORACLE` and `FERROGRAM_REQUIRE_ORACLE=1`, and run the ignored oracle tests. Windows skips.
- P1-07: `cargo llvm-cov` excludes `tests/` by default, so a crate's coverage counts only `src/`. The vsd-binary threshold becomes meaningful once it has library code (Phase 7).
- P1-06b: a test that loads the committed `ui-spec/*.yaml` with real `commands.yaml` and `shortcuts.yaml` and pins the real control counts. P1-03 covers the layouts with empty tables; P1-04 adds a loader check; P1-08 (test-engineer) owns the final version.
- P1-04: 15 VisUICmds constants contain underscores (`visCmdSetPageLineJumpCode_None` family and similar) and are skipped because the `commands.yaml` pattern is `^visCmd[A-Za-z0-9]+$`. Widen the pattern if Phase 4 needs them.
- P1-04: `xtask/src/extract/shortcuts.rs` `ACTION_TO_COMMAND` uses ~61 short phrases from the support page as lookup keys. They are needed to match rows and are not shipped. P1-11 (cleanroom-auditor) must confirm this is acceptable.
- P1-04: `shapesheet.toml` has no row-type families (MoveTo, LineTo, ...); `shortcuts.yaml` has no way to mark held keys such as "Arrow Keys". Revisit in Phase 2 (codegen) and Phase 4 (shortcuts).
- P1-08: the fuzz crate `crates/ui-spec/fuzz` uses edition 2021 (cargo-fuzz default); move to 2024 with the next fuzz target. Seed a fuzz corpus for CI runs later.
- P1-08: `cargo xtask extract --check` is not in the gate (CI has no `reference/`). The master runs it at each phase end.
- P1-14 (decided): a Fluent id is not unique within a layout. The same control appears in several places (ribbon, QAT, backstage), so a placement is identified by its path in the layout, not by its id. Command ids stay unique (enforced). Repeats inside one container (`TabSave` x9 in one backstage tab, `LineStylesDialog` twice in one group) look like extractor artefacts; review them when Phase 4 builds the ribbon, and record the rule in ADR 0007 then.
- P1-14: serialized layouts are not schema-valid for write-back (the model lacks `backstage.kind` and `quick_access_toolbar.id`). Fix before the screenshot pass writes layouts back.
- History rewrite (P1-11 F1), done 2026-10-13 with the user's approval: `git filter-repo` replaced the blob of `docs/tasks/P1-04.md` that held the verbatim support-page text with the cleaned version. The final tree is unchanged; commit hashes from `7097013` onward changed, and hashes quoted in docs were updated. A pre-rewrite mirror is at `/Volumes/Dara/ferrogram/backup/ferrogram-pre-rewrite.git`. Delete it before the first push.
- P2-01: the canonical angle unit is radians (ADR 0011, amended by P2-06 from corpus evidence; P2-06b moves `geom` to radians). The MS-VSDX XSD licence is checked in P2-09; until then the XSD is never committed. The coverage ratchet for geom, model, vsdx and shapesheet is set at the Phase 2 end.
- P2-02: the ShapeSheet table has gaps, so the generated defs use `Option`: 10 of 420 cells have no cell index, the Layer Membership section has no section index, and 8 of 51 rows have no owning section. Fill them if a later source gives the values.
- P2-05 (decided): the package reader accepts a package without `[Content_Types].xml` and infers declarations, because two libvisio corpus files have none. A dangling internal relationship is recorded, not an error. The XML helpers live in `crates/vsdx/src/package/xml.rs`; there is no `src/xml/` module.
- P2-04 (decided): the model keeps cell and section names as spelled in the `.vsdx` file; P2-06 adds a name-alias table in `crates/model/src/aliases.rs` for the ShapeSheet-table lookups. `ferrogram_model::Error::NotImplemented` and the `vsdx` one are removed after P2-07. Table defaults are parsed lazily and interned with `Box::leak` (bounded by the static table); move the parsing into codegen once `shapesheet.toml` records default values.
- P2-06 (decided): the writer patches the source part trees (P2-07, ADR 0018), because unmapped children interleave with mapped ones in the corpus. Model gaps (section-level cells, several master shapes, style references by ID, `Connect` preservation, radians in geom) go to P2-06b. `tests/roundtrip.rs` moved from P2-09 to P2-07; P2-10 adds schema validation of written parts.
- P2-06b (decided on main): a cell whose formula is `Inh` does not define the cell for the lookup (ADR 0013 amendment); its cached snapshot is not used as a fallback yet. Duplicate style IDs resolve first-wins. A chained master rung reuses the instance's sibling position; revisit if a file needs more.
- P2-07 (deferred to Phase 4): deleting a page or master leaves its part behind; a new `Property` is written into `DocumentProperties`, not `docProps/*`; edits to `Preserved` are not written back; only the first `Shapes` wrapper of a shape is patched; changed booleans are written as `0`/`1`. The generator silently stops at 128 sub-shape levels; make that an error when Phase 4 can build deep models.
- P2-09 (decided): the only public XSD (Office developer docs) is XSD 1.1, uses an older namespace and is out of sync with files Visio writes; 3 of 106 corpus source parts validate against the derived schema. The schema test checks that the writer adds no new validation errors, not that parts are valid. The libvisio oracle compares only page counts and page sizes for `.vsdx`; full comparison waits for Phase 7.
- P2-10 (open, for the audits and fix tasks): a model built in memory loses `Bool`, `List` and number-like `Str` kinds on write then read, because the writer emits no `U` for them; the arena holds nested sub-shapes in wrapper order, not document order; the schema test skips without `xmllint` and `reference/` (CI should install libxml2 and set `FERROGRAM_REQUIRE_SCHEMA=1`).
- P2-14 (open): a hand-built `Color`/`List` value with a unit, or a `Bool`/`Str` with a real unit, does not read back the same (formula-engine task); the relationships writer always uses the default namespace for the relationships vocabulary.
- P3-01 (open): the fixed palette 0..23 is not transcribed yet (P3-02). Per-shape `ThemeIndex` is deferred; the renderer uses `themes[0]`. The `Ellipse` row reading (conjugate semi-diameters) needs a black-box check for the oblique case. Linux golden drift may need the metric loosened in `tests/golden.rs`. Corpus colour cells cache `Themed` and palette indices; ADR 0021 did not cover them (master note in P3-02).
- P3-08 (open): dashes scale with line width (ADR 0023), which may not match Visio; line ends 25..45, fill patterns 25..40 (gradients) and patterned shadow bodies are undefined and return `None` with a warning; the `.vsd` corpus was not scanned for style indices. The scene builder (P3-05) must follow the unit-box and tile-box conventions documented in `styles.rs` and `visual-styles.yaml`.
- Master (2026-10-14): new advisories RUSTSEC-2026-0346..0349 against `rustls` 0.23.45 (xtask via `ureq`) failed the deny step; fixed with `cargo update -p rustls` (0.23.46).
- P3-04 (decided, ADR 0024 amendment): `layout` bakes the block rotation into glyph positions, and the backends (P3-06, P3-07) rotate each glyph outline; `Warning::MissingGlyph` carries the `char`; a bullet sits at `indent_left + bullet.indent`; Selawik has no italic faces, so an italic Segoe UI run falls back to Carlito Italic with a `MissingFont` warning (a synthetic slant needs an API extension). The bundled fonts are 2.96 MB and are compiled into every binary, including the web build; revisit lazy loading in Phase 6.
- P3-03 (open): the knot-vector rule (stored `n + 1` knots padded with the last knot) is unconfirmed: no corpus file has `NURBSTo` or spline rows, and the Result cites libvisio's reading, so P3-14 checks its provenance (master note). Spline and NURBS curves come out as straight chords within tolerance, not as fitted curves. The `NURBSTo` fallback rule and the knot-list assembly go to P3-05 (master note).
- P3-02 (open): palette indices 0..23 have names from `VisDefaultColors` but no Microsoft source gives their RGB values, and the corpus has none below index 24; the RGB values are the conventional ones and are flagged unverified (check in the screenshot pass). Deferred: variant theme colours (`QuickStyle*` values 100+ and 200+), the `a:fmtScheme` line and effect matrices, the `dk1`/`lt1` slot check, the packed-RGB index guess, `PolylineTo` `xType`/`yType` (dropped by the fixed enum), and addressing several sections of the same name through the public API.
- P3-05 (open, the rest go to P3-15): `Txt*` placement cells are used only when the shape defines them; `TextBkgnd` has no slot in `TextNode`; tab stops ignore the `tp` row and multi-stop rows; bullets use the run family and our own symbols. Approximations for the screenshot pass: `Rounding > 0` becomes a round join; line gradients are ignored; rectangular and path gradients degrade to linear with a warning; the arrow size scale and the pattern tile pitch are ours. `Scene::background` is always `None`. A group with its own geometry emits its shape as the first child of its `Group`.
- P3-06 (open): underline and strike-through are not drawn yet (`TextRun` carries them; no backend draws them); `Shadow::blur` is not applied (tiny-skia has no blur); a shape clip does not clip its own shadow; the pattern tile is a background box plus a filled tile path. `text::Layout` now carries `rotation` and `GlyphPos::size`, which overlap with `TextNode::rotation` and `TextRun::size` from P3-05: the backends use the layout fields. The reviewer (P3-12) should decide whether to keep one.
- P3-07 (open): `PatternPaint` has no `filled` flag or tile stroke width, so the SVG backend strokes open tiles as hairlines while the PNG backend fills the tile path; the two backends can differ on pattern fills (P3-12 decides the fix). Underline and strike-through are drawn by neither backend. Shadow blur needs `filter` in the ADR 0025 SVG subset. Line ends on dashed strokes are drawn solid. `render_svg`/`render_png` have no `InvalidOptions` error for a bad scale.
- P3-09 (open): without `--background`, PNG output is transparent (`Scene::background` is always `None`); `export` takes no `--background`; the inspect shape count, the main.rs error branch and the page/export exit-code split are listed in the P3-09 Result. Master smoke run (2026-10-15): all 15 corpus files render to PNG and SVG after the page-size fix, but themed shapes are invisible because `Themed` patterns resolve to 0 (added to P3-15).
- Master fix (2026-10-15): pages without a positive `PageWidth`/`PageHeight` (two corpus files have no page sheet) now render at 8.5 x 11 in instead of failing with `TooLarge`.
- P3-15 (open): the style-matrix colour modulation (about a 0.73 factor) is not applied; themed dash presets and shadow patterns fall back; cells with no documented default still warn `Missing`; inherited geometry row cells are the master's cached values (master-sized geometry on resized instances) until the formula engine (Phase 6); `QuickStyle*Color` 200..206 has no published meaning and is resolved like 100..106 from corpus evidence; a themed `LineWeight` falls back to 0.7 pt.
- Master fixes (2026-10-15): scene node transforms are now relative to their parent group (the builder stored page transforms, so nested shapes were offset twice); the fixed palette RGB values come from [MS-VSDX] 2.2.7.6 (indices 14, 16 and 18 corrected), which closes the P3-02 "unverified" item. Master smoke render: all 15 corpus files render to PNG and SVG with exit 0; themed fills, gradients and text are visible.
- P3-10 (open): the oracle comparison covers 3 of 15 corpus pages (`blue-box`, `qs-box`, `color-boxes`); the others skip on text drawn as paths, group flattening, unresolved themed paints, and a smaller geometry box than the oracle's on `testfile1`/`testfile5` arcs and ellipses (check the `Ellipse` and inherited-geometry cases). Tighten `skip_reason` in `crates/render/tests/oracle_render.rs` as those close. Golden metric may need loosening on Linux. The text fixture pins approximate tab and bullet rendering. Master fixes at merge: the root `**/fuzz/corpus/` ignore rule dropped the `render_document` seed corpus (now re-included by `crates/render/fuzz/.gitignore`), and `crates/vsdx/tests/fixtures.rs` now walks every fixture variant.
- P3-11 (open): `text::layout` can emit non-finite glyph positions for extreme finite input (`line_spacing: f64::MAX`); the backends sanitize, so it is hardening for Phase 6. `render_svg` has no upper bound on `scale` (PNG caps the raster); P3-12 decides whether both backends share one cap. Report: `docs/reviews/phase-3-tests.md`.
- P3-14 (fixed on main): `render_png` panicked in tiny-skia on huge finite coordinates under a rotation (F4). Fills, strokes and clips whose device-space bounds, widened by the stroke, pass +/-2^22 px are now skipped (`fits_device` in `png.rs`), with a regression test. Clean-room minors F1-F3 go to P3-18; the knot rule rests on [MS-VSDX] and the `DrawNURBS` remarks, not libvisio.
- P3-18 (open): the black-box check of the knot rule stays open (no corpus file has `NURBSTo` or spline rows); a periodic spline (first knot repeated fewer than `degree + 1` times) is read as clamped, a documented divergence.
- P3-16 (open): `text::Layout` `MissingFont`/`MissingGlyph` warnings do not reach `Scene::warnings` or the CLI; boolean and list coercions still accept non-finite values (Phase 6); `Character`/`Paragraph`/`Tabs` rows without an `IX` no longer match run markers by position (every corpus row has an `IX`); un-drawable glyphs are skipped by documented rule, not warned.
- P4-05 (open): the canvas cache detects document changes by comparing a `Document` snapshot every frame (O(document)) because `DocumentSession` has no generation counter; switch to a counter once `crates/commands` has one. Our own UI choices to revisit in the parity pass: 1/8 in grid and ruler ticks, zoom range 0.1..64, 10% per wheel notch, Ctrl+0 for `view.zoom.fit`. The raster scale is also capped by the GPU `max_texture_side`.
- P4-06 (open, Phase 5): stencil drag onto the page should set `PinX`/`PinY` at the drop point and glue to `MasterDef::connection_points`; `instantiate` has no size parameter; Shape Data values are strings until the formula engine. Master fix at merge: the preview test now requires a raster for every master (it predated `render_raster`).
- P4-02 (open): `file.open` does not update `Settings::recent_files`; `view.zoom.fit` repeats the renderer's 8.5 x 11 in page fallback (keep the two in step); a wasm `file.save` with a `path` argument fails at runtime; `app.quit` stays a stub (the shell quits itself). `DocumentSession` has no change counter, so the canvas keeps its per-frame document comparison: Phase 5 should bump a counter on every document edit (undo stack) and the canvas should use it.
- P4-04 (open): the ADR 0031 dedup key (id, label, order) does not collapse the extractor's nine `TabSave` placements (their orders differ), so nine widgets render; decide in an ADR 0007 review or fix in the extractor. A dropdown button with items does not dispatch its own command. P4-04 added `insta` and egui's `default_fonts` feature to `crates/ui/Cargo.toml` (P4-03's file).
- P4-03 (open): the shell does not host `shortcuts::dispatch` (P4-14) or call `Ribbon::set_context_menu` yet (the committed context menus carry no controls). `app.quit` and `app.office.center.options` are intercepted by `Shell::dispatch` because their registry rows are stubs; a `view.options` builtin returning `UiRequest::ShowOptions` and one quit id (`app.quit` vs `file.exit`) would remove the intercepts. `Shell::show(&Context)` panics inside `eframe::App::ui` (egui 0.37 `root_ui`), so the apps call `Shell::show_in`. On wasm only one async pick runs at a time; the desktop `NativeFilePicker` bodies are untested (flows tested through a fake picker). Settings persist on change, at `App::save` and on shutdown.
- P4-07 (open): panel warnings ride in a "Warning" row instead of `AppCtx::warnings` (the builders take `&AppCtx`; give them a warning sink in Phase 5); the panels show a non-positive page size as is while the canvas falls back to 8.5 x 11 in (one rule in the parity pass); property labels are the raw element names; the `egui::Grid` id is the panel title. Master fix at merge: the shell frame test recorded a warning and then asserted it; it now asserts a real frame raises no widget stub error.
- P4-08 (open): the first conflict rule keyed on the sequence alone and dropped 29 of 90 entries, so `Ctrl+C`/`Ctrl+V`/`Delete`/arrows worked in one scope only and `tools.format_painter` was lost; ADR 0036 is amended (key = sequence + scope, most specific active scope wins) and P4-14 implements it plus the shell pass. Load errors use `UiError::Layout(LoadError::Parse)` (no shortcut variant); `Ctrl+Backspace` in text defers to egui; shifted symbols vary by real keyboard layout.
- P4-14 (open): focus tracking is pointer-press based (the previous frame's canvas and ribbon rects), so a keyboard-only focus move does not change the scope; `ShortcutMap::resolve` gained an `&ActiveScopes` argument; the committed table has 74 bindings, 16 exact repeats and no real conflicts. Master fix at merge: P4-14 let every bound `Ctrl` chord dispatch inside a text edit (so `Ctrl+C`/`Ctrl+V`/`Ctrl+Z` ran registry commands instead of egui's copy, paste and undo in dialog fields); egui's own editing chords defer to egui again (ADR 0036 amended), and other bound chords such as `Ctrl+Shift+C` still dispatch.
- P4-09 (open): parity is now derived (7 implemented, 685 stub, 14 out-of-scope) and checked by the `parity` gate step. 5 of 15 corpus files rasterize blank: their shape or page cells have no cached values and the builder substitutes 0 (`FillPattern`/`LinePattern`/`LineWeight` on `dwg`, `fdo86664`, `testfile5`; `Width`/`Height`/`PinX`/`PinY` on `tab-short-prefix`), and `recursion-cycle.vsdx` has no page part; `SKIP_RASTER`/`KNOWN_BLANK` in `apps/ferrogram/tests/open_corpus.rs` go when the value pipeline (P3-15/P3-16, Phase 6 formula engine) closes this. `Shell` exposes no canvas raster, so the exit test reads it through a `canvas::show` frame. The 365 backstage has 205 placements but 127 unique ids (same as the P4-04 `TabSave` item). P4-09 also added `crates/commands/tests/headless.rs` (outside its listed files; test-only, kept). ADR 0037 now says out-of-scope beats implemented.
- Master fix (2026-10-17): RUSTSEC-2026-0367..0369 (zip `extract()` bugs) turned `cargo deny` red on main. `crates/vsdx` and `xtask` now use zip 9; zip 8.6 remains only through calamine in xtask, which never calls `extract()`, and `deny.toml` ignores the three ids with that reason (ADR 0014 updated).
- P4-10 (open): fixed `CommandId::intern` leaking every rejected id. Open for the fix pass: a chord that mismatches a half-typed sequence is dropped when the mismatch fires the held-back completion (retry it as a fresh start?); `builtins::with_undo` puts the undo stack back by document index (Phase 5 risk); `UndoStack::default()` has capacity 0; the 4096-id intern pool is a test hazard; `crates/ui` has no `proptest` dev-dependency; `crates/ui-spec/fuzz/.gitignore` lacks `!corpus/` and the new `crates/ui/fuzz` targets have no seeds. Report: `docs/reviews/phase-4-tests.md`.
- P4-13 (fixed on main): the two ui `insta` snapshots and the `proptest-regressions` seed files had no `ATTRIBUTION.md` rows; rows added. Report: `docs/reviews/phase-4-cleanroom.md`.
- P4-17 (open): the `render` and `vsdx` fuzz seed corpora are flat, but cargo-fuzz auto-loads only `corpus/<target>/`; move them to per-target directories in a later fuzz pass. Master fix (2026-10-18): RUSTSEC-2026-0370..0372 (zvariant, zbus_names, zvariant_derive unmaintained, no fix) under eframe's Linux accessibility bridge turned `cargo deny` red on main; ignored in `deny.toml` with that reason.
- P4-15 (open): the parity matrix has one row per Fluent control, so command-only ids (`app.office.center.options`, `app.quit`, `file.exit`, `view.units`) do not appear in `docs/parity/controls.md`; the master's P4-15 criterion asked for it by mistake. Phase 5: add a `| command | label | status |` table to the generator. `with_undo`'s placeholder stack is zero-capacity and visible to an `Edit` while it runs. The 4096-id global intern pool is still a test hazard (`intern_into` gives tests a private pool). `commands` dropped `serde-saphyr` and builds its stubs from `Loader::from_embedded()`; `DocumentSession::id` is the new session identity.
- P4-16 (open): the context menus stay parked (`Ribbon::show_context_menu` / `set_context_menu` never called; the committed menus carry no controls). The canvas "inconsistent raster size" guard is unreachable through `render_raster`. The rfd `pick_file`/`save_file` calls and `open_dialog`'s configuration are desktop-only and untested (the rest sits behind `picked_path`/`save_picked`). `ShortcutMap::resolve` and `shortcuts::dispatch` return `Vec<Intent>`: a mismatching chord fires the held-back completion and then starts a new sequence (ADR 0036 sketch updated by the master). A programmatic outcome still updates the status line; dialogs, downloads, quit and confirm are interactive-only. Closes P4-03's intercepts and `Shell::show` items, the P4-10 mismatch item and P4-15 open issue 4 (edits land on the session the command ran against).
- P5-02 (open): a new page with no shapes gets no contents part and no `Rel` (the P2-10 write-stability rule kept over ADR 0048's wording); check with Visio as a black box that such a file opens, and change the rule and `write_edge_cases.rs` together if it does not. `scale_geometry` drops cells the typed rows do not model (`NurbsTo` E, `PolylineTo` A, `Unknown` rows) and does not scale arc bows, elliptical ratios or spline knots, so non-uniform resizes distort them slightly (Phase 6 recalc replaces it). Master rows without `IX` are overridden by position. Group bounds use the shape frame, not the geometry. The writer still allocates relationship ids with `fresh_rel_id`, not `Package::next_relationship_id`. `model::mutate` is now `pub` (the glue constants).
- P5-03 (open): `snap_axis` takes the nearest candidate with ties broken by order, with no ADR 0043 kind priority; check against Visio in the screenshot pass. Text selection bounds are the layout box, not the glyphs (revisit with text editing, P5-07). A rotated group's bounds overestimate slightly. `hit_test` ignores clips. The marquee rule and the 4 px / 8 px constants are our own. Master fix at merge: `geom::Error::InvalidArgument` and `render::Error::InvalidNode` replace the borrowed `WrongUnit` and `InvalidOptions`, and `geom` exports `FLATTEN_TOLERANCE`. Merge gate after P5-02: model coverage 97.7% < 99.6% ratchet (P5-02 skipped coverage under the new trigger rules). P5-15 restores it, and AGENTS.md now runs coverage for every code change.
- P5-15 (open): `lookup.rs` `repeated_cell`'s `Some(row)` arm is dead (only caller passes `None`). Unreachable defensive fall-throughs in `document.rs` 134, `raw.rs` 243/265, `stencils/mod.rs` 189. Uncovered test-module arms in `geometry.rs` and `stencils/mod.rs`. A tidy pass may remove dead arms, but never by switching to `expect` (AGENTS.md section 6).
- P5-04 (open): undo restores cells straight into `Sheet::cells` (`restore_cells`) because `Document::write_cell` cannot express "no entry"; a later model task may add `Document::restore_cell`. A third undo step `GeometryEdit` exists for resize. `StructuralEdit::insertion` is `#[allow(dead_code)]` until P5-06..P5-08 use it. `touched_cells` mirrors the model's `DERIVED_CELLS` (pinned by a test). Our own semantics, to check in a black-box check: rotate is a delta, multi-shape resize gives each shape the target size, the enter-group state. `need_finite` accepts integers.
- P5-05 (open): the rotate handle sits 0.25 in above the shape, not a fixed screen distance (`handles_for` has no zoom). Editable panel fields take inches and degrees, not `Settings::units`. `builtins::view::page_size` is still private, so the panels keep their own page-size reader. Black-box checks: the rotate handle offset, the marquee intersection rule, Alt as the snap modifier. P5-11 can reuse `CanvasRig` (`crates/ui/tests/phase5/main.rs`) for the 8 ms drag budget. Selection through the registry and master connection points moved to P5-16.
- P5-16 (open): `edit.select` has no explicit entered-group argument (callers map a hit to the entered group's child first). `Document::connection_points` takes the nearest master that has rows and scales by direct `Width`/`Height` cells only; inherited sizes without a cached value stay unscaled until the Phase 6 formula engine.
- P5-06 (open): undoing the first drop of a catalog master leaves the materialised master in the document (needs a master-removal mutation). Glue records use `Connections.X<n>` and connection-point parts; compare 2D glue cell spelling with Visio in a black-box check. Master fix: the draw and stencil drags moved from address-keyed thread-locals into `ToolMachine` fields (`882e7b1`). Tool picking, the Shapes-window drag and indexed connection points moved to P5-17.
- P5-07 (open): the editor commits one run. Field text becomes literal, and multi-run formatting markers collapse to the first run. The editor box is axis-aligned for rotated text. A new text box is a fixed 2 x 0.5 in, our own size. Black-box checks: whether Escape or a click outside commits, the default box size, and where the caret starts. Master fix at merge: the text-tool test now uses P5-06's real `insert.drawing.text_box`. Showing the editor in the app moved to P5-17.
- P5-08 (open): the tool finds the connector through `insert.shape`'s selection and the Basic Flowchart "Connector" master name. Outline glue uses page-aligned bounds, not the rotated outline. A connector inside a group writes its path without the parent transform. `connector.reroute_all` bumps `changes` even when no path changes. Re-routing rewrites every glued connector on the page, so the first edit of a Visio-routed file replaces Visio's paths with ours (ADR 0045 accepts this). One-end unglue edits `Page::connects` directly, not through the model's mutation API. Master fix at merge: `reroute_after_edit` acts on the active document only. Indexed glue and the connector free end moved to P5-17. Merge gate after P5-08: `ferrogram-commands` coverage 97.9% < 98.1% ratchet (P5-08 handed off at exactly 98.1%; the P5-07 and P5-08 merges together fell under it). P5-18 restores it; main is red on that step only.
- P5-17 (open): the registry has no checked-state seam, so tool commands cannot report themselves checked. Typing on a selected shape does not open the editor, and Escape does not return to the pointer tool (no binding; check in a black-box check). The tool choice is application-wide, not per document. A free connector end lives in the connector's local geometry and moves with it. The shell takes Escape from `view.full_screen_mode` while the editor or a stencil drag is active. `connector.draw` is a new Ferrogram-only id (977 rows). The worker also removed two unreachable checks in `insert.rs`. Commands coverage was still 97.9% after the rebase; P5-18 restores it.
- P5-18 (closed): tests only, no bugs found. The branch measured 98.2% before P5-17; on main after both merges it is 98.1% (7491/7635), exactly at the ratchet, so the merge gate is green with no margin. The remaining uncovered lines are listed in its Result; most are `?` arms of constructors after validated ids.
- P5-09 (open): align and distribute ignore `Angle`; check Visio's rotated behavior in a black-box check. `arrange.align.center` (the Fluent row) is not implemented; `arrange.align.center_x` is. Group, ungroup and insert undo leave detached arena entries, so the P5-11 undo property must compare reachable state, not `Document` equality (or an arena-compaction ADR). Page undo snapshots the whole page list. Cross-document paste keeps a `Master` link the target may lack; the writer needs a dangling-master rule before that ships. The shortcut extractor drops "open the page tab context menu" (no mapped action).
