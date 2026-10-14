# Status

Task board. Only the master agent edits this file, on `main`.

- Current phase: **2 in progress** (planning)
- Phase 2 goal: document model and .vsdx I/O (KICKOFF.md). Exit: every corpus .vsdx file round-trips without loss and without panics.

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
| [P2-11](tasks/P2-11.md) | reviewer | P2-10 | ready (group D) | — |
| [P2-12](tasks/P2-12.md) | coverage-auditor | P2-10 | ready (group D) | — |
| [P2-13](tasks/P2-13.md) | cleanroom-auditor | P2-10 | ready (group D) | — |

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
