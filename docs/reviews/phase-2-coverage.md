# Phase 2 coverage audit

- Task: P2-12 (coverage-auditor), 2026-10-14
- Scope: `main` at `fe6e328` (after P2-10), crates with Phase 2 code:
  `geom`, `model`, `shapesheet`, `vsdx`, `ui-spec`, plus the apps and `xtask`.
- Verdict: **PASS** — every crate with instrumented lines is above its
  threshold and above its ratchet. Findings: 0 blocker, 3 major, 11 minor.

## How measured

- `cargo xtask coverage` (which runs
  `cargo llvm-cov --workspace --all-features --json --summary-only`) in the
  P2-12 worktree with `CARGO_TARGET_DIR=/Volumes/Dara/larrygram/target/P2-12`.
  All tests green in that run (230 xtask tests plus every crate's tests).
- `cargo llvm-cov report --lcov` for exact per-line execution counts.
  Findings below list only lines with a count of 0, i.e. lines no test
  executes at all. `cargo-llvm-cov 0.9.1`.
- The corpus is present in the worktree (`corpus/vsdx`, 15 files), so the
  corpus tests ran and their execution counts are included; `cargo test
  --test read_corpus` was re-run separately to confirm it does not skip.
- Everything large stayed on `/Volumes/Dara/larrygram/` (worktree, target dir);
  this audit changed no code.

## Per-crate table

Line counts from the llvm-cov summary. "Ratchet" is the value currently in
`coverage.toml` (set at the end of Phase 0/1; the Phase 2 ratchet for
`geom`, `model`, `shapesheet` and `vsdx` is set at the Phase 2 end per the
P2-01 note in `docs/STATUS.md`). "Delta vs threshold" is measured minus the
`[thresholds]` value.

| Crate | Lines | Covered | Pct | Threshold | Ratchet | Delta vs ratchet | Delta vs threshold |
|---|---:|---:|---:|---:|---:|---:|---:|
| larrygram | 9 | 6 | 66.7 | 50.0 | 66.6 | +0.1 | +16.7 |
| larrygram-cli | 9 | 6 | 66.7 | 50.0 | 66.6 | +0.1 | +16.7 |
| larrygram-commands | 0 | 0 | n/a | 85.0 | — | — | pass (empty) |
| larrygram-control | 0 | 0 | n/a | 50.0 | — | — | pass (empty) |
| larrygram-geom | 542 | 542 | 100.0 | 85.0 | — | — | +15.0 |
| larrygram-model | 2479 | 2471 | 99.7 | 85.0 | — | — | +14.7 |
| larrygram-render | 0 | 0 | n/a | 75.0 | — | — | pass (empty) |
| larrygram-shapesheet | 45 | 45 | 100.0 | 85.0 | — | — | +15.0 |
| larrygram-text | 0 | 0 | n/a | 75.0 | — | — | pass (empty) |
| larrygram-ui | 0 | 0 | n/a | 50.0 | — | — | pass (empty) |
| larrygram-ui-spec | 1738 | 1726 | 99.3 | 85.0 | 99.3 | 0.0 | +14.3 |
| larrygram-vsd-binary | 0 | 0 | n/a | 85.0 | — | — | pass (empty) |
| larrygram-vsdx | 5889 | 5717 | 97.1 | 85.0 | — | — | +12.1 |
| larrygram-web | 2 | 0 | 0.0 | — | 0.0 | 0.0 | — (no threshold) |
| xtask | 9232 | 8457 | 91.6 | — | 91.3 | +0.3 | — (no threshold) |

No crate is below its threshold or below its ratchet, so there is no blocker.
`larrygram-geom` and `larrygram-shapesheet` are at 100%; `larrygram-vsd-binary`
is still an empty placeholder (its 85% threshold becomes meaningful in
Phase 7, per the P1-07 note).

## Recommended `[ratchet]` for the end of Phase 2

The master updates `coverage.toml`; this task does not edit it. These are the
values `cargo xtask coverage --update-ratchet` would write (percent floored to
one decimal, empty crates omitted):

```toml
[ratchet]
larrygram = 66.6
larrygram-cli = 66.6
larrygram-geom = 100.0
larrygram-model = 99.6
larrygram-shapesheet = 100.0
larrygram-ui-spec = 99.3
larrygram-vsdx = 97.0
larrygram-web = 0.0
xtask = 91.6
```

Notes:

- The floored values (`larrygram-model` 99.6, `larrygram-vsdx` 97.0) are one
  tenth below the displayed table numbers (99.7, 97.1). `--update-ratchet`
  floors deliberately so the ratchet never exceeds what was measured.
- A ratchet of 100.0 for `geom` and `shapesheet` means any new line must come
  with a test that runs it. That is the intent; do not hand-round down.

## Findings

Findings are graded blocker / major / minor. "Untested" means every test
suite (inline unit tests, `tests/` integration tests, corpus tests) leaves
the line at zero executions. Where the likely cause is defensive or
unreachable code rather than a missing test, that is said so.

### Blocker

None.

### Major

1. **The ShapeSheet-level preservation fallbacks of the reader are
   untested.** `crates/vsdx/src/read/sheet.rs:42`, `:91`, `:121`,
   `:135-141`, `:151`, `:171`. When `map_cell`, `map_row` or `map_section`
   declines a child (`Cell`/`Row`/`Section` it does not model), the child is
   pushed into `Preserved`; unknown cell attributes go to
   `cell.preserved.attributes` (`:91`); `map_cell`/`map_row` returning
   `Ok(None)` (`:121`, `:151`) keeps the element preserved. These branches
   are the mechanism behind the Phase 2 exit criterion (every corpus file
   round-trips without loss), and finer-grained than the parts-level
   preservation tests in `tests/read_edge_cases.rs`, which do pass. Nothing
   exercises a sheet child or a cell attribute that the mapper refuses.
   A corpus file with an unmapped `Row`/`Cell` variant, or a fixture that
   adds one, would close this.

2. **Writing into a package that lacks the usual plumbing is untested.**
   `crates/vsdx/src/write/mod.rs:393-399` (build a missing pages/masters
   index part), `:418-419`, `:436` (`fresh_rel_id_from` collision loop),
   `:539-543` (`refresh_content_types` pushes an `Override` for the main part
   when `[Content_Types].xml` has none). The reader deliberately accepts
   packages without `[Content_Types].xml` and with missing plumbing (P2-05
   decision, two libvisio corpus files), but no test writes such a package
   back. The fuzz target does (`write_document(&document, &Package::new())`
   in `crates/vsdx/fuzz/fuzz_targets/read_package.rs`), but the fuzz gate
   step only builds fuzz targets, it does not run them. Add unit tests that
   write into `Package::new()` and into a package without a content-types
   part.

3. **The writer's error-returning paths are untested.** The `?` arms in
   `patch_lists` at `crates/vsdx/src/write/mod.rs:268`, `:282`, `:289`,
   `:307`, `:321`, `:328` (`contents_part`, `ensure_contents`, `ensure_list`)
   never fire in any test; the only write error with a test is `MissingPart`
   (`write/mod.rs:817`, inline). If a write fails halfway (dangling
   relationship, missing part named by a relationship), what the caller gets
   is unproven. One test per error variant of `write_document` would settle
   it.

### Minor

**Untested error paths and edge cases (hostile or odd input)**

1. `crates/vsdx/src/package/read.rs:96-100` — the streaming
   `max_part_uncompressed` enforcement site (the pre-check site is tested at
   `tests/package_read.rs:226`, this one never fires).
2. `crates/vsdx/src/package/read.rs:318`, `:323` — unknown element in
   `[Content_Types].xml`: `:318` skips an unknown child, `:323` rejects an
   unknown element. The `BadContentTypes` fixture truncates the file instead,
   so neither arm runs.
3. `crates/vsdx/src/package/read.rs:156` — content-type inference for an
   unknown extension when the package has no `[Content_Types].xml`;
   `:214`, `:220` — the odd shapes of a relationships entry name
   (root rels, no directory part).
4. `crates/vsdx/src/package/xml.rs:152-153` — escaping `\n` and `\r` as
   `&#xA;`/`&#xD;` on write. If text or attribute values may contain raw
   newlines (multi-line shape text), this is lossless-preservation code with
   zero coverage; worth one round-trip test with a newline in cell text.
5. `crates/model/src/lookup.rs:250` — the parent-cycle guard
   (`visited.contains`) in the master-path walk. A corpus file named
   `recursion-cycle.vsdx` exists, yet the guard never runs; either the file
   does not create a parent cycle (likely) or the walk is never reached that
   way. One test with a parent cycle would cover it.

**Dead or effectively dead code the coverage hides**

6. `crates/vsdx/src/write/patch.rs:548`, `:597-602`, `:621`, `:656`,
   `:679-684`, `:761`, `:833`, `:928`, `:1131`, `:1169` — the
   `map_or_else(|| emit::empty(...), ...)` miss arms. By code reading the
   `build(entity)` closures are only ever called with an index inside the
   model list they index (`sync_children` enumerates `entity_keys`, which is
   built from the same list), so these arms appear unreachable. They exist to
   satisfy `clippy::indexing_slicing`, which is fine, but they are invisible
   dead weight: coverage can never notice them, and a future refactor that
   makes them reachable gets no test signal. Mark them as defensive in a
   comment, or add a test that makes one reachable and deletes the doubt.
7. `crates/model/src/lookup.rs:482` — the `Err(_)` arm of the default-value
   cache `Mutex`: only reachable after a poisoned lock, i.e. after an earlier
   panic in a lock holder. Effectively dead by design; fine, but it will
   never be covered.
8. `crates/vsdx/src/write/emit.rs:774`, `:809`, `:830`, `:878`, `:906`,
   `:933` — the `_ => "?"` / `_ => None` unit-code fallbacks. A unit the
   writer cannot spell is written as `?`; nothing tests which units those
   are, so a wrong fallback would ship silently.
9. `crates/vsdx/src/write/patch.rs` ordering fallbacks: `:287`, `:309`,
   `:357`, `:388-394` (`page_rank`), `:445`, `:719`, `:858`, `:1001`,
   `:1019`, `:1069`, `:1212`, `:1234`. These rank or classify an unknown
   sibling element during insert; no test inserts next to an element the
   ranker does not know. Low risk, but they are the kind of ordering code
   that silently regresses.
10. `crates/vsdx/src/write/patch.rs:486-487` (`remove(node, "U")` — the model
    takes over a cell unit and the file's `U` must go), `:1240-1241`
    (wrapping loose `Connect` elements in a new `Connects` wrapper). Both are
    write-back semantics with no test.
11. `crates/model/src/document.rs:129`, `crates/model/src/raw.rs:239`,
    `:261` — defensive `continue`/`return 1` fallbacks (already-seen shape
    slot, occurrence of an out-of-range child index). Defensive by design;
    they will stay uncovered.

**Tests that assert nothing**

The test suites were scanned for `#[test]` functions with no assertion of
any kind. Six candidates, all intentional and none is a real problem:

- `crates/ui-spec/tests/malformed.rs:148`, `:157` (`arbitrary_bytes_never_panic`,
  `arbitrary_text_never_panics`): the property is "no panic", the call is the
  assertion. The sibling test `non_utf8_bytes_are_always_an_error` asserts an
  outcome, so the shape is right.
- `crates/vsdx/tests/read_fixtures.rs:126` (`malformed_packages_never_panic`):
  loops over six fixtures and swallows whatever comes back. `tests/malformed.rs`
  already records the expected outcome per fixture (`OUTCOMES`), so this test
  adds only the no-panic property. Consider asserting the recorded outcome
  here too, or dropping the duplication.
- `crates/vsdx/tests/fixtures.rs:278` (`regenerate_committed_fixtures`):
  `#[ignore]`d maintenance tool, panics via `expect` on failure.
- `crates/ui-spec/src/lib.rs:1403` and `crates/ui-spec/tests/committed.rs:181`
  (`model_types_round_trip_*`): both delegate to a `round_trip` helper that
  asserts equality.

### Measurement caveats (for the phase report)

- The percentage counts `#[cfg(test)]` inline test code as ordinary
  instrumented lines. Inline tests are roughly a third of `crates/vsdx/src`
  and almost half of `crates/ui-spec/src` by lines, and they are nearly all
  covered, so the product-only percentage is modestly lower than the table
  shows. Conversely `tests/` integration test files (about 5.8k lines for
  `vsdx` alone) contribute execution counts but no lines: they can only push
  the number up. The two effects do not cancel exactly; treat the table as
  "src coverage", not "product coverage".
- The uncovered test-code lines themselves (`crates/ui-spec/src/loader.rs`
  `other => panic!(...)` arms at 579, 806, 834, 860, 898, 912, 929, 946, 963,
  977, 1028; `crates/model/src/lookup.rs:1539`;
  `crates/vsdx/src/read/mod.rs:1472-1480`) are never-hit panic arms and test
  helpers. They are harmless and not worth chasing.
- `larrygram` and `larrygram-cli` remain one uncovered line above their
  ratchet (6/9 lines, `main` itself). The 66.6 ratchet stays fragile until
  the apps have real entry points (same note as Phase 1).
