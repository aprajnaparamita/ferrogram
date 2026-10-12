# Status

Task board. Only the master agent edits this file, on `main`.

- Current phase: **1 in progress**
- Phase 1 goal: gather resources and definitions (KICKOFF.md)

## Phase 1 task board

| Task | Role | Depends on | State | Branch |
|---|---|---|---|---|
| [P1-01](tasks/P1-01.md) | researcher | — | merged (corpus binaries excluded, ADR 0005) | agent/P1-01 |
| [P1-02](tasks/P1-02.md) | architect | P1-01 | merged ([review](tasks/P1-02-review.md)) | agent/P1-02 |
| [P1-03](tasks/P1-03.md) | implementer | P1-02 | ready (wave A) | — |
| [P1-04](tasks/P1-04.md) | implementer | P1-03 | blocked (wave B) | — |
| [P1-05](tasks/P1-05.md) | implementer | P1-02 | ready (wave A) | — |
| [P1-06](tasks/P1-06.md) | implementer | P1-02 | merged (schema issues 1–2 fixed on main) | agent/P1-06 |
| [P1-06b](tasks/P1-06b.md) | implementer | P1-06 | ready (wave A) | — |
| [P1-07](tasks/P1-07.md) | implementer | P1-02 | ready (wave A) | — |
| [P1-12](tasks/P1-12.md) | implementer | — | merged | agent/P1-12 |
| P1-08 | test-engineer | P1-03..07 | not written | — |
| P1-09 | reviewer | P1-08 | not written | — |
| P1-10 | coverage-auditor | P1-08 | not written | — |
| P1-11 | cleanroom-auditor | P1-08 | not written | — |

States: not written · outline · ready · in progress · in review · merged · blocked.

## Phase history

| Phase | Report | Gate |
|---|---|---|
| 0 | [phase-0.md](progress/phase-0.md) | green (2026-10-12) |

Shared file note: P1-03, P1-04, P1-05 all add dependencies to `xtask/Cargo.toml` (and `Cargo.lock`); P1-07 adds an arm to `xtask/src/main.rs`. The master resolves these merge conflicts.

Open master item: create the Phase 2 task for `cargo xtask codegen` (ADR 0008).

## Deferred minor findings (for the phase report)

- P1-12: `download` writes the file before verifying sha256; verifying the in-memory body first would avoid ever placing a bad file at the final path.
- P1-12: `ureq` uses the private `_ring` feature; it may break on a ureq minor update. Revisit if the webpki-roots licence (CDLA-Permissive-2.0) is accepted in `deny.toml`.
