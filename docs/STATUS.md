# Status

Task board. Only the master agent edits this file, on `main`.

- Current phase: **1 in progress**
- Phase 1 goal: gather resources and definitions (KICKOFF.md)

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

Shared file note: P1-03, P1-04, P1-05 all add dependencies to `xtask/Cargo.toml` (and `Cargo.lock`); P1-07 adds an arm to `xtask/src/main.rs`. The master resolves these merge conflicts.

Open master items: create the Phase 2 task for `cargo xtask codegen` (ADR 0008). At phase end: update the coverage ratchet, run `cargo xtask extract --check`, write `docs/progress/phase-1.md`.

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
