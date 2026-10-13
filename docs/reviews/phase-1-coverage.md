# Phase 1 coverage audit

- Task: P1-10 (coverage-auditor), 2026-10-13
- Scope: `git diff phase-0..main`
- Verdict: **PASS** — every crate with instrumented lines is above its
  threshold and above its ratchet. Findings: 0 blocker, 2 major, 7 minor.

## How measured

- `cargo xtask coverage` (which runs
  `cargo llvm-cov --workspace --all-features --json --summary-only`)
  in the P1-10 worktree with
  `CARGO_TARGET_DIR=/Volumes/Dara/ferrogram/target/P1-10`.
- `cargo llvm-cov report --json` (full report, segments) for line-level
  detail. `cargo-llvm-cov 0.9.1`.
- All tests green in the same run (184 xtask tests, plus every crate).

## Per-crate table

Line counts from the llvm-cov summary. "Ratchet" is the value currently
in `coverage.toml` (recorded at the end of Phase 0); "delta" is the
measured percentage minus the ratchet.

| Crate | Lines | Covered | Pct | Threshold | Ratchet | Delta vs ratchet | Delta vs threshold |
|---|---:|---:|---:|---:|---:|---:|---:|
| ferrogram | 9 | 6 | 66.7 | 50.0 | 66.6 | +0.1 | +16.7 |
| ferrogram-cli | 9 | 6 | 66.7 | 50.0 | 66.6 | +0.1 | +16.7 |
| ferrogram-commands | 0 | 0 | n/a | 85.0 | — | — | pass (empty) |
| ferrogram-control | 0 | 0 | n/a | 50.0 | — | — | pass (empty) |
| ferrogram-geom | 0 | 0 | n/a | 85.0 | — | — | pass (empty) |
| ferrogram-model | 0 | 0 | n/a | 85.0 | — | — | pass (empty) |
| ferrogram-render | 0 | 0 | n/a | 75.0 | — | — | pass (empty) |
| ferrogram-shapesheet | 0 | 0 | n/a | 85.0 | — | — | pass (empty) |
| ferrogram-text | 0 | 0 | n/a | 75.0 | — | — | pass (empty) |
| ferrogram-ui | 0 | 0 | n/a | 50.0 | — | — | pass (empty) |
| ferrogram-ui-spec | 1320 | 1309 | 99.2 | 85.0 | — | — | +14.2 |
| ferrogram-vsd-binary | 0 | 0 | n/a | 85.0 | — | — | see "misleading" below |
| ferrogram-vsdx | 0 | 0 | n/a | 85.0 | — | — | pass (empty) |
| ferrogram-web | 2 | 0 | 0.0 | — | 0.0 | 0.0 | — (no threshold) |
| xtask | 8027 | 7198 | 89.7 | — | 55.6 | +34.1 | — (no threshold) |

No crate is below its threshold or below its ratchet, so there is no
blocker.

## Recommended `[ratchet]` for the end of Phase 1

The master updates `coverage.toml`; this task does not edit it. The
recommended values are what `cargo xtask coverage --update-ratchet`
would write (percent floored to one decimal, empty crates omitted):

```toml
[ratchet]
ferrogram = 66.6
ferrogram-cli = 66.6
ferrogram-ui-spec = 99.1
ferrogram-web = 0.0
xtask = 89.6
```

Notes:

- The floored values (99.1 for `ferrogram-ui-spec`, 89.6 for `xtask`)
  are one tenth below the displayed table numbers (99.2, 89.7). The
  `--update-ratchet` command floors deliberately so the ratchet never
  exceeds what was measured. Do not hand-round them up.
- `xtask` has no threshold. It is dev tooling; the ratchet alone guards
  it. Consider keeping it that way.
- `ferrogram` and `ferrogram-cli` sit one uncovered line above their
  ratchet (6/9 lines). See "misleading measurements": their ratchet of
  66.6 is fragile.

## Findings

### Blocker

None.

### Major

1. **The corpus fetcher's network path has zero coverage.**
   `xtask/src/corpus.rs:243-251` (the shared `ureq` agent and TLS
   config), `xtask/src/corpus.rs:254-262` (loading the OS trust store
   via `rustls-native-certs`) and `xtask/src/corpus.rs:265-279`
   (`download`: GET, HTTP status check `271-273`, body read
   `274-277`) are never executed by any test. All TLS and network
   error handling is therefore unproven: trust-store load failure,
   DNS/connect/TLS failures, non-200 responses, truncated bodies.
   The tests cover `file_state`, `write_atomic` and the offline copy
   path only. A unit test can at least cover the HTTP-status and
   body-read error mapping against a local listener; the rest needs
   the ignored/integration tier.

2. **The fetch loop's post-fetch verification and checksum failure
   arms are untested.**
   `xtask/src/corpus.rs:347-348` (unknown source name),
   `xtask/src/corpus.rs:361` (the `download` call site),
   `xtask/src/corpus.rs:367` (fetch produced no file),
   `xtask/src/corpus.rs:369-373` (size/sha256 mismatch after fetch,
   including the `remove_file` cleanup) and `xtask/src/corpus.rs:378`
   (stat error while verifying) are all uncovered. The happy path
   (offline copy of a good file) and `--verify-only` are covered, so
   the exact behaviour the corpus gate depends on — a bad download is
   deleted and reported, never kept — is unverified. This is the
   checksum-error path the task calls out.

### Minor

3. **The xtask CLI dispatcher is 0% covered.**
   `xtask/src/main.rs:44-118` (all 59 lines): argument parsing,
   unknown subcommand, usage text, and the dispatch into `extract`,
   `corpus`, `oracle`, `gate`, `coverage`, `layers`. Cheap to test by
   calling the arg-handling function with crafted argv.

4. **Gate orchestration is barely covered.**
   `xtask/src/gate.rs:50-179` (38 of 148 lines covered): per-step
   invocation, the `--quick` short-circuit, step-failure reporting and
   the summary. Understandable — the gate runs outside the test suite —
   but step selection and failure handling could be unit-tested with a
   stub runner.

5. **`cargo xtask coverage` itself is untested at the entry point.**
   `xtask/src/coverage.rs:44-96` (`run`): reading the llvm-cov JSON
   report, printing the table, and the `--update-ratchet` write path
   (including the "refusing to update while coverage fails" guard).
   The pure helpers (`aggregate`, `parse_config`, `check`,
   `rewrite_ratchet`) are well covered.

6. **Subprocess helpers' error branches are untested.**
   `xtask/src/util.rs:16-18` (`$CARGO` fallback),
   `xtask/src/util.rs:21-32` (`run`: spawn failure, non-zero exit),
   `xtask/src/util.rs:35-58` (`output`: spawn failure, stderr capture,
   non-UTF-8 stdout, `has_cargo_subcommand` failure) and
   `xtask/src/util.rs:72-76` (`workspace_packages`). Every downstream
   error message routes through these.

7. **Oracle wrapper dispatch arms are untested.**
   `xtask/src/oracle.rs:80-88` (status printing),
   `xtask/src/oracle.rs:92-110` (the `build`/`clean` dispatch and the
   unknown-command error at line 110) and `xtask/src/oracle.rs:127-135`
   (the thin `build()`/`clean()` wrappers that would execute
   `build.sh`). The core logic (`build_in`, `clean_in`, `status_in`)
   is tested. Accepted risk noted in the task: running `build.sh` for
   real is out of scope for unit tests.

8. **Extractor failure paths (file I/O, parse, serialize) are almost
   entirely untested.** These run offline on trusted `reference/`
   input, so the risk is low, but a corrupted or updated reference
   download would surface these paths first. Representative uncovered
   ranges:
   - `xtask/src/extract/fluent.rs:65-72` (write errors),
     `xtask/src/extract/fluent.rs:212-235` (xlsx open / no sheets /
     worksheet-range errors), `xtask/src/extract/fluent.rs:307-311`
     (missing header columns), `xtask/src/extract/fluent.rs:346-352`
     (unknown control type);
   - `xtask/src/extract/commands.rs:386-389,398-407` (write and YAML
     serialize errors), `xtask/src/extract/commands.rs:479-492`
     (first-alias and same-command arms of alias resolution),
     `xtask/src/extract/commands.rs:747-752`
     (id-collision guard), `xtask/src/extract/commands.rs:1092-1096`
     (YAML serialize error);
   - `xtask/src/extract/shortcuts.rs:815-819` (YAML serialize error);
   - `xtask/src/extract/shapesheet.rs:139-174` (write, read_dir,
     read_to_string and table-parse errors);
   - `xtask/src/extract/vsdx_schema.rs:1339-1374` (read error
     `1339-1342`, create_dir_all and write errors `1365-1374`);
   - `xtask/src/extract/schema.rs:46-53,64-71,93-96,118-123`
     (read, JSON parse, unknown file name, missing
     `definitions/<kind>`, sub-schema compile errors);
   - `xtask/src/extract/diff.rs:118-143,267-285` and
     `xtask/src/extract/parity.rs:112-131,249-251` (read and parse
     error arms).

9. **Two small gaps in the ui-spec loader.**
   `crates/ui-spec/src/loader.rs:212` — the case where the primary
   path does not end in `.yaml` and is missing (the `.json`
   alternative fallback is covered; the plain-miss fallthrough is
   not), and `crates/ui-spec/src/loader.rs:70,72` — the `"layout"` and
   `"shortcuts"` arms of `DocKind::noun`, meaning kind-specific error
   messages are only built for `commands`. Overall the loader is
   613/623 (98.4%) with all real error branches exercised.

## Misleading measurements

- **`ferrogram-vsd-binary` (0 lines).** Its only code is under
  `crates/vsd-binary/tests/`, which `cargo llvm-cov` excludes from the
  report by default, and its `src/` is an empty placeholder. The 0
  lines say nothing about the tests' quality. This matches the
  deferred note in `docs/STATUS.md` (P1-07): the number becomes
  meaningful once the crate has library code (Phase 7). Rated the
  same as the master; not counted as a finding.
- **`ferrogram` and `ferrogram-cli` (9 lines each).** One uncovered
  line swings the percentage by 11 points. Their 66.7% against a 66.6
  ratchet has a margin of zero: any edit to `main.rs` can flip the
  gate. Do not tighten these ratchets; consider a "small denominator"
  grace in the coverage step later.
- **`ferrogram-web` (2 lines).** The wasm entry point is not run under
  `wasm32`; 0.0% is expected. Its ratchet of 0.0 is correct.
- **Test code in `src/` counts toward the denominator.** Unit tests
  are inline, so unreachable match arms in tests inflate "uncovered"
  counts — for example the `other => panic!(...)` arms at
  `crates/ui-spec/src/loader.rs:554,781,809,835,873,887,904` and the
  fixture skip guards at `xtask/src/extract/mod.rs:359-360`,
  `xtask/src/extract/commands.rs:1448-1449`. They can never be covered
  and should be read as noise, not gaps.
- **Zero-line crates are not "passing" in a meaningful sense.**
  `ferrogram-geom`, `ferrogram-model`, `ferrogram-vsdx`,
  `ferrogram-vsd-binary`, `ferrogram-shapesheet`, `ferrogram-commands`,
  `ferrogram-render`, `ferrogram-text`, `ferrogram-control` and
  `ferrogram-ui` are empty placeholders. The gate passes them by
  design ("a crate with zero instrumented lines passes"). Phase 2
  onwards numbers will appear; the thresholds in `coverage.toml`
  (85/75/50) are the real target then.
- **`xtask`'s 89.7% is dominated by the extractors' happy paths.**
  The number would drop quickly if the failure-path tests recommended
  above were added and the denominators grew into the network layer
  (finding 1).
