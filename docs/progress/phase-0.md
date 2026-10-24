# Phase 0 report: Bootstrap

Date: 2026-10-12

## What was built

- **Workspace:** Cargo workspace (edition 2024, toolchain pinned to 1.95.0 with rustfmt, clippy, llvm-tools and the wasm32 target), workspace lints (`unsafe_code` forbidden; `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`, `unreachable` denied; `missing_docs` warn), `clippy.toml` exempting tests, `deny.toml`, `.gitignore`, `coverage.toml`.
- **Crates (empty placeholders, each `#![forbid(unsafe_code)]`):** geom, model, vsdx, vsd-binary (MPL-2.0 header), shapesheet, text, render, commands, ui-spec, control, ui. Parser crates (vsdx, vsd-binary, shapesheet) deny `clippy::indexing_slicing`. Apps: larrygram, larrygram-web, larrygram-cli.
- **xtask:** `gate` (fmt, clippy, test, doc, coverage, deny, layers, wasm, fuzz; `--quick`, `--skip`), `coverage` (per-crate thresholds and ratchet from `coverage.toml`, parsed from the llvm-cov JSON summary; `--update-ratchet`), `layers` (layering check, ADR 0002).
- **CI:** `.github/workflows/ci.yml` runs `cargo xtask gate` on Ubuntu, macOS and Windows (fuzz skipped on Windows: libFuzzer does not support MSVC). Not yet run: no remote configured.
- **Process:** AGENTS.md (rules for all harnesses), CLAUDE.md (`@AGENTS.md`), seven role files in `.claude/agents/`, task template, `docs/STATUS.md` task board, Phase 1 tasks P1-01 and P1-02.
- **Docs:** README (honest pre-alpha status), LICENSE (MIT), NOTICE (MPL-2.0 for vsd-binary), ATTRIBUTION.md, `docs/sources.md`, `corpus/PROVENANCE.md`.
- **Local setup:** `reference/` is a gitignored symlink to `/Volumes/Dara/larrygram/reference`; worktrees and target dirs live under `/Volumes/Dara/larrygram/`. Installed cargo-deny, cargo-llvm-cov, cargo-fuzz 0.13.2, trunk 0.21.14, nightly toolchain.

Borrowed from ArtCraft (gridcraft, wordcraft; patterns only, no code copied): enforced crate layering with GUI crates only at the top, single command registry, never-crash lint set, separate `CARGO_TARGET_DIR` per parallel agent, `ATTRIBUTION.md` for assets, macOS shell tips.

## Gate results

```
PASS  fmt
PASS  clippy
PASS  test      (14 xtask tests, 2 app tests)
PASS  doc
PASS  coverage
PASS  deny      (advisories, bans, licenses, sources)
PASS  layers    (15 crates)
PASS  wasm
SKIP  fuzz      no fuzz targets yet
gate: GREEN
```

Also verified: setting `license = "MPL-2.0"` on `larrygram-model` makes `cargo deny check licenses` fail, so MPL is confined to vsd-binary.

## Coverage

| Crate | Lines | Coverage | Threshold | Previous |
|---|---|---|---|---|
| larrygram | 9 | 66.7% | 50 | — |
| larrygram-cli | 9 | 66.7% | 50 | — |
| larrygram-web | 2 | 0.0% | — | — |
| xtask | 579 | 55.6% | — | — |
| all library crates | 0 | n/a | 75–85 | — |

Ratchet recorded in `coverage.toml`.

## Reviewer findings

Phase 0 was done by the master agent directly; no separate review tasks were run (the user chose to launch every worker session; reviewing an empty scaffold did not justify one). Phase 1 runs the full review/audit set.

## ADRs added

- 0000 template
- 0001 workspace layout and crate naming (adds `crates/ui`)
- 0002 crate layering
- 0003 licensing and MPL-2.0 confinement
- 0004 multi-agent workflow on a shared local repository

## Known issues

- CI has never run: no GitHub remote yet. `repository` is unset in manifests until the remote is known.
- The wasm step builds a placeholder binary; trunk packaging arrives with the web app in Phase 4.
- xtask is not under a coverage threshold, only the ratchet (55.6%).
- `larrygram-web` has no tests (wasm placeholder).
- The fuzz step's `cargo +nightly fuzz build` path is untested until the first fuzz target exists (Phase 2).

## Plan for Phase 1

1. P1-01 researcher: download all sources into `reference/`, provenance, corpus triage.
2. P1-02 architect: formats, schema, command-id scheme, extractor interfaces, ADRs; refines P1-03..P1-07.
3. P1-03..P1-07 implementers in parallel on disjoint files: command bar extractors, enumerations/ShapeSheet/commands/shortcuts extractors, VSDX inventory, ui-spec loader, libvisio oracle harness.
4. P1-08 test-engineer, then P1-09 reviewer, P1-10 coverage-auditor, P1-11 cleanroom-auditor in parallel.
5. Fix blockers/majors, gate green, `docs/progress/phase-1.md`.
