# Status

Task board. Only the master agent edits this file, on `main`.

- Current phase: **1 in progress**
- Phase 1 goal: gather resources and definitions (KICKOFF.md)

## Phase 1 task board

| Task | Role | Depends on | State | Branch |
|---|---|---|---|---|
| [P1-01](tasks/P1-01.md) | researcher | — | merged (corpus binaries excluded, ADR 0005) | agent/P1-01 |
| [P1-02](tasks/P1-02.md) | architect | P1-01 | changes requested ([review](tasks/P1-02-review.md)) | agent/P1-02 |
| P1-03 | implementer | P1-02 | outline (in P1-02) | — |
| P1-04 | implementer | P1-02 | outline (in P1-02) | — |
| P1-05 | implementer | P1-02 | outline (in P1-02) | — |
| P1-06 | implementer | P1-02 | outline (in P1-02) | — |
| P1-07 | implementer | P1-02 | outline (in P1-02) | — |
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

## Deferred minor findings (for the phase report)

- P1-12: `download` writes the file before verifying sha256; verifying the in-memory body first would avoid ever placing a bad file at the final path.
- P1-12: `ureq` uses the private `_ring` feature; it may break on a ureq minor update. Revisit if the webpki-roots licence (CDLA-Permissive-2.0) is accepted in `deny.toml`.
