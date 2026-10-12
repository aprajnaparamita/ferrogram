# 0003: Licensing and confinement of MPL-2.0 code

- Status: accepted
- Date: 2026-10-12
- Task: P0

## Context

Ferrogram is MIT. Binary .vsd import uses libvisio (expected MPL-2.0, to be verified from its COPYING file in Phase 1) as the reference. MPL-2.0 is file-level copyleft: derived files must stay MPL-2.0.

## Decision

- All crates are MIT except `ferrogram-vsd-binary`, which is MPL-2.0. Every source file in it carries the MPL-2.0 header; NOTICE records the libvisio attribution and commit.
- No libvisio-derived code anywhere else. libvisio's CLI tools may be built under `tools/libvisio-oracle/` as a dev-only test oracle, never linked into Ferrogram.
- `deny.toml` does not allow MPL-2.0 globally; a single exception allows it for `ferrogram-vsd-binary`. `licenses.private.ignore = false` makes cargo-deny check our own crates too.
- Third-party dependencies must use the permissive licenses listed in `deny.toml`. If a needed dependency is MPL (common in GUI stacks, e.g. via `dirs`), it requires a new ADR, because the kickoff requires vsd-binary to be the only MPL crate.

## Consequences

- Dependency choices for the GUI must be checked against cargo-deny early (Phase 4).

## Alternatives considered

- Whole project MPL-2.0: rejected by the kickoff.
