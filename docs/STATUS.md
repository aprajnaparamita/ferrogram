# Status

Task board. Only the master agent edits this file, on `main`.

- Current phase: **3 in progress** (group B)
- Phase 3 goal: rendering from cached values, plus the CLI (KICKOFF.md). Exit: every corpus file renders without panics, and every geometry row type has tests.
- Phase 2 complete (2026-10-14, report: [progress/phase-2.md](progress/phase-2.md), tag `phase-2`).

## Phase 3 task board

| Task | Role | Depends on | Group | State | Branch |
|---|---|---|---|---|---|
| [P3-01](tasks/P3-01.md) | architect | phase-2 | — | merged (ADRs 0019–0027, stubs, task files) | agent/P3-01 |
| [P3-02](tasks/P3-02.md) | implementer | P3-01 | A | merged (ADR 0021 amended: `Themed`, colour indices) | agent/P3-02 |
| [P3-03](tasks/P3-03.md) | implementer | P3-01 | A | merged (knot rule flagged for P3-14) | agent/P3-03 |
| [P3-04](tasks/P3-04.md) | implementer | P3-01 | A | merged (sources rows renumbered 11–12 on main) | agent/P3-04 |
| [P3-08](tasks/P3-08.md) | implementer | P3-01 | A | merged (added `Error::InvalidStyleData` in `error.rs`) | agent/P3-08 |
| [P3-05](tasks/P3-05.md) | implementer | P3-02, P3-03, P3-04, P3-08 | B | merged (added `CellReader` row readers in model; `TextNode` gained `runs` and `rotation`) | agent/P3-05 |
| [P3-15](tasks/P3-15.md) | implementer | P3-05 | B | ready (model fixes: geometry row union, theme variants, quiet defaults) | — |
| [P3-06](tasks/P3-06.md) | implementer | P3-02, P3-03, P3-04 | B | merged (conflict with P3-05 resolved on main; glyphs use their run colour) | agent/P3-06 |
| [P3-07](tasks/P3-07.md) | implementer | P3-02, P3-03, P3-04 | B | merged (stale base; glyph size, rotation and run colour wired on main) | agent/P3-07 |
| [P3-09](tasks/P3-09.md) | implementer | P3-05, P3-06, P3-07 | C | merged (page size fallback fixed on main) | agent/P3-09 |
| [P3-10](tasks/P3-10.md) | implementer | P3-05..P3-08, P3-15 | C | waits for B | — |
| [P3-11](tasks/P3-11.md) | test-engineer | P3-09, P3-10 | — | waits for C | — |
| [P3-12](tasks/P3-12.md) | reviewer | P3-11 | D | waits | — |
| [P3-13](tasks/P3-13.md) | coverage-auditor | P3-11 | D | waits | — |
| [P3-14](tasks/P3-14.md) | cleanroom-auditor | P3-11 | D | waits | — |

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
