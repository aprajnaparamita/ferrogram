# Phase 3 coverage audit

- Task: P3-13 (coverage-auditor), 2026-10-15
- Scope: `main` at `255d730` (after P3-11), with the Phase 3 code in
  `geom`, `model`, `render`, `text`, `vsdx` and `ferrogram-cli`.
- Verdict: **PASS** — every crate with instrumented lines is above its
  threshold and above its ratchet. Findings: 0 blocker, 5 major, 10 minor.

## How measured

- `cargo xtask coverage` (which runs
  `cargo llvm-cov --workspace --all-features --json --summary-only`) in the
  P3-13 worktree with `CARGO_TARGET_DIR=/Volumes/Dara/ferrogram/target/P3-13`.
  All tests green in that run (230 xtask tests plus every crate's tests).
- `cargo llvm-cov report --lcov` for exact per-line execution counts.
  Findings below list only lines with a count of 0, i.e. lines no test
  executes at all. `cargo-llvm-cov 0.9.1`.
- The corpus is present in the worktree (`corpus/vsdx`, `corpus/vsd`), so the
  corpus tests ran and their execution counts are included.
- Everything large stayed on `/Volumes/Dara/ferrogram/` (worktree, target dir);
  this audit changed no code.

## Per-crate table

Line counts from the llvm-cov summary. "Ratchet" is the value currently in
`coverage.toml`, which is the Phase 2 value. "Phase 2" is the measured
percentage from `docs/progress/phase-2.md`.

| Crate | Lines | Covered | Pct | Threshold | Ratchet | Phase 2 | Pass/fail |
|---|---:|---:|---:|---:|---:|---:|---|
| ferrogram | 9 | 6 | 66.7 | 50.0 | 66.6 | 66.7 | pass |
| ferrogram-cli | 1322 | 1256 | 95.0 | 50.0 | 66.6 | 66.7 | pass |
| ferrogram-commands | 0 | 0 | n/a | 85.0 | — | n/a | pass (empty) |
| ferrogram-control | 0 | 0 | n/a | 50.0 | — | n/a | pass (empty) |
| ferrogram-geom | 1677 | 1677 | 100.0 | 85.0 | 100.0 | 100.0 | pass |
| ferrogram-model | 6633 | 6613 | 99.7 | 85.0 | 99.7 | 99.7 | pass |
| ferrogram-render | 5327 | 5027 | 94.4 | 75.0 | — | new | pass |
| ferrogram-shapesheet | 45 | 45 | 100.0 | 85.0 | 100.0 | 100.0 | pass |
| ferrogram-text | 1677 | 1627 | 97.0 | 75.0 | — | new | pass |
| ferrogram-ui | 0 | 0 | n/a | 50.0 | — | n/a | pass (empty) |
| ferrogram-ui-spec | 1738 | 1726 | 99.3 | 85.0 | 99.3 | 99.3 | pass |
| ferrogram-vsd-binary | 0 | 0 | n/a | 85.0 | — | n/a | pass (empty) |
| ferrogram-vsdx | 7188 | 7039 | 97.9 | 85.0 | 97.7 | 97.8 | pass |
| ferrogram-web | 2 | 0 | 0.0 | — | 0.0 | 0.0 | pass (no threshold) |
| xtask | 9232 | 8457 | 91.6 | — | 91.6 | 91.6 | pass |

No crate is below its threshold or below its ratchet, so there is no blocker.
`ferrogram-geom` and `ferrogram-shapesheet` stay at 100%;
`ferrogram-commands`, `ferrogram-control`, `ferrogram-ui` and
`ferrogram-vsd-binary` are still empty placeholders.

Comparison with Phase 2: `vsdx` grew (+0.1) while gaining Phase 3 code,
`model` and `ui-spec` are flat, `xtask` is flat. The apps changed shape:
`ferrogram-cli` went from 9 stub lines (66.7%) to 1322 lines (95.0%), and
`ferrogram-render` (94.4%) and `ferrogram-text` (97.0%) are new, both well
above their 75% thresholds.

## Recommended `[ratchet]` for the end of Phase 3

The master updates `coverage.toml`; this task does not edit it. These are the
values `cargo xtask coverage --update-ratchet` would write (percent floored to
one decimal, empty crates omitted):

```toml
[ratchet]
ferrogram = 66.6
ferrogram-cli = 95.0
ferrogram-geom = 100.0
ferrogram-model = 99.6
ferrogram-render = 94.3
ferrogram-shapesheet = 100.0
ferrogram-text = 97.0
ferrogram-ui-spec = 99.3
ferrogram-vsdx = 97.9
ferrogram-web = 0.0
xtask = 91.6
```

Notes:

- The floored values are one tenth below the displayed table numbers where the
  measured percentage does not land on a tenth: `ferrogram-model` 99.6
  (99.698% measured), `ferrogram-render` 94.3 (94.368%).
- `ferrogram-model` is the one crate where the floor (99.6) is below the
  current ratchet (99.7). The current gate passes only because `check` allows
  0.05 of float noise. Writing 99.6 is honest; writing 99.7 again would keep
  relying on the epsilon.
- `ferrogram-render` and `ferrogram-text` get their first ratchets now.

## Findings

Findings are graded blocker / major / minor. "Untested" means every test
suite (inline unit tests, `tests/` integration tests, corpus tests) leaves the
line at zero executions. Where the likely cause is defensive or unreachable
code rather than a missing test, that is said so.

### Blocker

None.

### Major

1. **The render builder's missing-cell fallbacks are untested.** When a
   cached cell is missing or has the wrong kind, the builder substitutes a
   default and records a `CellWarning`. None of these arms runs in any test:
   `crates/render/src/build.rs:246-253` (number), `:268-275` (bool),
   `:286-293` (colour), `:311-318` (row number), `:539-546` (`LayerMember`),
   `:592-599` (geometry section), `:1717-1724` (`Text` with a wrong-kind
   value). This is the degraded-input behaviour of the Phase 3 core promise
   ("rendering from cached values"), and it is exactly what hostile or damaged
   files exercise. One fixture per substituted kind would close this.

2. **The PNG backend's hostile-input guards are untested.**
   `crates/render/src/png.rs:555-566` rejects non-finite, sub-pixel,
   over-`MAX_DIMENSION` and over-`MAX_BYTES` pixmap sizes;
   `:484` and `:499-500` reject degenerate gradient endpoints (non-finite
   start/end, non-positive radius). These are the caps that stand between a
   malicious `.vsdx` and an unbounded allocation. No test feeds NaN geometry
   or an absurd page size to the PNG backend.

3. **The SVG backend's size cap and segment fallbacks are untested.**
   `crates/render/src/svg.rs:266-267` (`Error::TooLarge` when the document
   exceeds the output cap), `:776-788` (mapping `QuadTo`/`CubicTo` segments,
   with the non-exhaustive catch-all at `:788`), `:802-824` (first-segment
   direction extraction, quad/cubic arms at `:813-820`). Same class as
   finding 2 for the SVG output path.

4. **Line-end arrowheads are only half covered in the PNG backend.**
   `crates/render/src/png.rs:299` (drawing the start end), `:336-349`
   (colour override and the filled-versus-outlined arrow body). Arrowhead
   geometry is new Phase 3 code; the scene side is tested, the raster side is
   not.

5. **The CLI's usage-error paths and warning report formatting are
   untested.** `apps/ferrogram-cli/src/cli/parse.rs:134-141`, `:183-190`
   (unexpected argument, unknown option — the first thing hostile command
   lines hit), `apps/ferrogram-cli/src/cli/exec.rs:232-239` (warning-kind
   text), `:265-271` (`value_text` per `CellValue` kind). The happy paths of
   every subcommand are tested; the error and reporting edges are not.

### Minor

**Untested fallbacks and edge cases in new code**

1. `crates/text/src/layout.rs:1043`, `:1050`, `:1070-1078` — the
   `nominal()` fallback used when the shaper is unavailable or shaping fails.
   The whole fallback is uncovered, so text layout has no proven behaviour
   without the shaper.
2. `crates/text/src/font.rs:209-218` (`advance_em` when the font does not
   parse), `:378-386` (language preference when picking a name string),
   `:444-450` (cubic glyph outlines), `:576-582` (cubic control points in the
   outline bounding box).
3. `crates/render/src/build.rs:1299-1306` (rectangular/path gradient
   direction warning), `:1414-1420`, `:1464-1475`, `:1497-1503` (unknown
   line pattern, arrow index, shadow pattern warnings), `:1701`
   (`VAlign::Bottom`), `:1887-1894` (line-spacing branches), `:1000-1002`
   (unmapped geometry row warning). Same warning machinery as major 1, but
   style-level.
4. `crates/render/src/styles.rs:314`, `:359`, `:392` — `Error::InvalidStyleData`
   on duplicate style indices. Error handling in the style set builder,
   untested.
5. `crates/render/src/png.rs:471-473` (drawing a pattern tile),
   `:764-788` (quad/cubic dash segments), `:587-589` (intersecting a second
   clip into the mask). Pattern fills and multi-clip in raster are new.

**Carried over from the Phase 2 audit, still uncovered**

6. `crates/vsdx/src/write/mod.rs:205`, `:219`, `:226`, `:244`, `:258`, `:265`
   (the `?` arms of index/part patching) and `:476-480`
   (`refresh_content_types` pushing a missing `Override`). Phase 2 major 2
   and 3 are still open: writing into a package without the usual plumbing,
   and the writer's error-returning paths, remain unproven.
7. `crates/vsdx/src/write/patch.rs` — 57 uncovered lines, mostly the
   `map_or_else(|| emit::empty(...), ...)` miss arms and ordering fallbacks
   listed in the Phase 2 report (finding 6 and 9). Nothing changed; they are
   still invisible dead weight.
8. `crates/vsdx/src/write/emit.rs:795`, `:830`, `:851`, `:899`, `:927`,
   `:954` — the unit-code fallbacks (`_ => "?"` / `_ => None`), Phase 2
   finding 8.
9. `crates/vsdx/src/package/read.rs:156`, `:214`, `:220` — content-type
   inference for unknown extensions and odd relationship entry names
   (Phase 2 minor 3); `:379` — non-element child of a relationships entry.
10. Defensive fallbacks that will stay uncovered by design:
    `crates/model/src/lookup.rs:654-659` (row index out of range in a cell
    lookup), `crates/model/src/document.rs:134`, `:171`,
    `crates/model/src/raw.rs:243`, `:265`, `crates/vsdx/src/write/value.rs:130`,
    `:149`, `crates/render/src/build.rs:454`, `:1056`, `:1170`, `:1207`.

**Improved since Phase 2**

- The ShapeSheet preservation fallbacks (Phase 2 major 1) are now mostly
  covered. Only `crates/vsdx/src/read/sheet.rs:121` (a section with loose
  content is declined) and `:141` (a non-row child is preserved) remain.
- `crates/vsdx/src/read/mod.rs` is down to 12 uncovered lines (from a much
  larger set), and `crates/model/src/lookup.rs` down to 6.

**Tests that assert nothing**

The scan for `#[test]` functions with no assertion found only never-hit
`panic!` arms in match positions (for example
`crates/render/src/build.rs:2216`, `crates/render/src/scene.rs:381-408`,
`apps/ferrogram-cli/src/cli/parse.rs:327-399`,
`crates/vsdx/src/read/sheet.rs:186-190`) and test helpers. These are
harmless: they are the "assert the shape" arms of tests that do assert, and
they are not worth chasing.

### Measurement caveats (for the phase report)

- Same as Phase 2: the percentage counts `#[cfg(test)]` inline test code as
  ordinary instrumented lines, and `tests/` integration files contribute
  execution counts but no lines. Treat the table as "src coverage", not
  "product coverage". The new crates are no different: about a third of
  `crates/render/src/build.rs` past line 2100 is inline tests, and its
  uncovered `panic!` arms are test code, not product code.
- `ferrogram` (the app) is unchanged at 6/9 lines, one uncovered line above
  its ratchet. Its `main` (`apps/ferrogram/src/main.rs:7-9`) and
  `ferrogram-cli`'s `main` (`apps/ferrogram-cli/src/main.rs:10-15`) are real
  process entry points that no test executes; the 66.6 ratchet for
  `ferrogram` stays fragile for the same reason as in earlier phases.
- The line totals for `geom`, `model`, `render`, `text` and `vsdx` are roughly
  three times the Phase 2 numbers for `geom`/`model` because Phase 3 added
  the geometry rows, the render builder and text layout with their inline
  test modules. Coverage percentages are comparable; line counts are not a
  code-size measure on their own.
