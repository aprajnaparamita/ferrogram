# 0005: Third-party test corpus is fetched, not committed

- Status: accepted
- Date: 2026-10-12
- Task: P1-01 review

## Context

P1-01 collected 23 openly licensed Visio files from libvisio and LibreOffice core test data. Review found:

- 14 of 17 .vsdx files embed master shapes, several from Microsoft templates (`BASICD_M.VSTX`, `XFUNC_M.VSTX`, `BASFLO_M.vstx`). Clean-room rule 3 forbids committing Microsoft stencils; committing drawings that carry copies of their masters sits too close to that line.
- The three LibreOffice core files had no pinned commit and an unclear license (core mixes MPL-2.0 and LGPL-era files).

The ArtCraft apps solve the same problem by fetching corpora from a pinned, sha256-verified source.

## Decision

- Third-party test files are never committed. `corpus/manifest.toml` lists each file: upstream source, path, pinned commit, sha256, size.
- `cargo xtask corpus` fetches them into `corpus/vsd/`, `corpus/vsdx/` (gitignored), verifies sha256, and is idempotent. CI runs it before the gate.
- Tests that need the corpus skip with a clear message when it has not been fetched, except in CI where `FERROGRAM_REQUIRE_CORPUS=1` turns a missing corpus into a failure.
- Hand-authored fixtures (our own work, including malformed files) are committed under `corpus/fixtures/` and listed in `corpus/PROVENANCE.md`.
- Only libvisio files at the commit recorded in NOTICE are in the manifest for now. LibreOffice core files are dropped.

## Consequences

- No Microsoft-derived master content in our git history.
- Test runs need network once. CI depends on GitHub's raw mirror of libvisio (fallback: git.libreoffice.org).

## Alternatives considered

- Commit libvisio files as-is: simpler, but puts Microsoft-template masters in history.
- Commit only master-free files: too few files to be useful.
