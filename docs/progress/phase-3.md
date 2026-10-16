# Phase 3 report: Rendering from cached values, plus the CLI

Phase 3 renders pages from cached cell values (no formula evaluation) to PNG and SVG, and adds the `ferrogram-cli` tool. Started 2026-10-14, closed 2026-10-16, tag `phase-3`.

## What was built

| Area | Content | Tasks |
|---|---|---|
| `crates/geom` curves (L0) | Every Geometry row's curve math into cubic Béziers: arcs, elliptical arcs, NURBS, splines, quadratic and cubic Béziers, polylines. Degenerate input degrades to the chord, never panics; segment, span and degree caps. | P3-03, P3-18 |
| `crates/model` additions (L1) | Typed geometry rows (`GeometryRow`, 15 kinds plus `Unknown`) with the master row union by `IX` (`Del`, takeover); row-aware cell lookup; `CellReader` with documented defaults and warnings; theme colours and fonts, `Themed` values, palette and document colour indices; layers; text content with run markers by `IX`. | P3-02, P3-05, P3-15, P3-16 |
| `crates/text` (L1) | Text layout with `rustybuzz` shaping: runs, paragraphs, alignment, wrapping, bullets, tabs, rotation. Bundled Carlito (for Calibri) and Selawik (for Segoe UI) under the OFL. | P3-04 |
| `crates/vsdx` read additions (L2) | Theme part, `Text` mixed content with `cp`/`pp`/`tp`/`fld` markers, colour table, layers | P3-02 |
| `crates/render` (L3) | Scene graph (ADR 0020) with parent-relative transforms; the builder from model to scene (geometry, fills, gradients, patterns, lines, line ends, shadows, text, masters, themes, layers, page size, background pages); our own visual style data (`visual-styles.yaml`, ADR 0023); PNG backend on `tiny-skia`; SVG backend with a fixed output format. | P3-05..P3-08, P3-15, P3-16 |
| `apps/ferrogram-cli` (L7) | `inspect`, `render` and `export` subcommands; exit codes 0 OK, 1 usage, 2 read, 3 render, 4 warnings with `--strict` | P3-09 |
| Test harness | 11 rendering fixtures, 12 golden images (ADR 0027 metric), 3 SVG snapshots, corpus render test, libvisio `vsd2xhtml` structural comparison, scene and layout property tests, boundary tests, `render_document` fuzz target | P3-10, P3-11, P3-17 |

## Exit criterion

**Every corpus file renders without panics, and every geometry row type has tests: met.**

- `FERROGRAM_REQUIRE_CORPUS=1 cargo test -p ferrogram-render --test corpus_render` renders every page of all 15 corpus files through both backends with `Ok` and no panic.
- The P3-10 Result has the geometry row test table: all 16 `GeometryRow` variants have typing, curve-math (where a curve applies) and scene-mapping tests. P3-11 added property tests over random row sequences.
- The oracle comparison (`FERROGRAM_REQUIRE_ORACLE=1`) passes. It compares page counts for every file and full per-page structure on three pages; the reasons for the others are under Known issues.

## Gate results

```
cargo xtask gate (full, 2026-10-16, main)
  PASS  codegen
  PASS  fmt
  PASS  clippy
  PASS  test      (1447 tests; 2 ignored: the fixture regenerator and the oracle smoke test)
  PASS  doc
  PASS  coverage
  PASS  deny
  PASS  layers
  PASS  wasm
  PASS  fuzz      (crates/ui-spec/fuzz, crates/vsdx/fuzz, crates/render/fuzz)
gate: GREEN

FERROGRAM_REQUIRE_CORPUS=1 cargo test -p ferrogram-render --test corpus_render --test golden: OK
FERROGRAM_REQUIRE_ORACLE=1 cargo test -p ferrogram-render --test oracle_render: OK
```

## Coverage

| Crate | Lines | Coverage | Threshold | Phase 2 ratchet | New ratchet |
|---|---|---|---|---|---|
| ferrogram-geom | 1677 | 100.0% | 85 | 100.0 | 100.0 |
| ferrogram-model | 6832 | 99.7% | 85 | 99.7 | 99.7 |
| ferrogram-text | 1768 | 98.9% | 75 | — | 98.8 |
| ferrogram-render | 6532 | 98.5% | 75 | — | 98.4 |
| ferrogram-vsdx | 7229 | 98.0% | 85 | 97.7 | 98.0 |
| ferrogram-cli | 1408 | 96.9% | 50 | 66.6 | 96.8 |
| ferrogram-shapesheet | 45 | 100.0% | 85 | 100.0 | 100.0 |
| ferrogram-ui-spec | 1738 | 99.3% | 85 | 99.3 | 99.3 |
| xtask | 9232 | 91.6% | — | 91.6 | 91.6 |
| ferrogram | 9 | 66.7% | 50 | 66.6 | 66.6 |

The percentages count inline `#[cfg(test)]` code as instrumented lines (see the caveats in [the coverage audit](../reviews/phase-3-coverage.md)).

## Reviewer findings and resolution

| Report | Verdict | Blocker / major / minor | Resolution |
|---|---|---|---|
| [Test pass](../reviews/phase-3-tests.md) (P3-11) | — | one product bug | Negative line width for zero-advance glyphs, fixed in P3-11 |
| [Code review](../reviews/phase-3-review.md) (P3-12) | approve with fixes | 0 / 4 / 8 | P3-16 (all majors, minors 5–7 and 9–12); minor 8 by ADR 0022 amendment in P3-18 |
| [Coverage](../reviews/phase-3-coverage.md) (P3-13) | pass | 0 / 5 / 10 | P3-17 (all majors, minors 1–5; the rest with written reasons) |
| [Clean-room](../reviews/phase-3-cleanroom.md) (P3-14) | pass with findings | 0 / 0 / 3, plus F4 | F1–F3 in P3-18; F4 (a PNG crash, not a clean-room issue) fixed on main |

## Fixes made on main by the master

- Pages with no positive size fall back to 8.5 × 11 in.
- Scene node transforms are relative to the parent group (nested shapes had been offset twice).
- Palette RGB values for indices 14, 16 and 18 corrected from [MS-VSDX] 2.2.7.6.
- Glyphs take their run colour in both backends; SVG got glyph size and rotation.
- The `render_document` fuzz seed corpus was ignored by the root `.gitignore`; re-included.
- `render_png` skips paths whose device-space bounds pass ±2^22 px; `tiny-skia` asserted on huge finite coordinates under rotation (P3-14 F4).
- `rustls` updated to 0.23.46 for RUSTSEC-2026-0346..0349.

## Decisions made during the phase

- **Cached values only:** rendering reads cached values; inherited geometry cells use the master's cached values, so a resized instance keeps the master's geometry size until the formula engine (Phase 6).
- **Warnings, not failures:** a missing or bad cell substitutes the documented or hard default and records one warning (ADR 0021); non-finite numbers count as bad. A warning never fails a render; the CLI turns warnings into exit 4 only with `--strict`.
- **Themes:** `Themed` values and Quick Style indices resolve from the theme part; unresolved themed paints fall back to visible defaults, never to "no fill" or "no line" (ADR 0021 amendment).
- **Our own visual styles:** fill patterns, line patterns and line ends are our own drawings keyed by the documented indices (ADR 0023), not copied from Visio.
- **One source of drawing intent:** the scene carries everything a backend needs (pattern tile fill and stroke width, text rotation and glyph size live in `text::Layout`); PNG and SVG share one 16384 px scale cap and `Error::InvalidOptions`.
- **Knot vectors:** the stored `n + 1` knots come from [MS-VSDX]; the end padding from the `DrawNURBS` reference remarks. libvisio is corroboration only.
- **Group shapes:** a group's own geometry is drawn first, as the first child of its `Group` (ADR 0020 amendment).

## ADRs added

- 0019 model additions for rendering (amended: themed values, colour indices)
- 0020 scene graph (amended: parent-relative transforms, group own shape, pattern tiles)
- 0021 cached values, defaults and warnings (amended: themes, non-finite numbers)
- 0022 geometry semantics and curve flattening (amended: knot rule sources, silent degradation)
- 0023 visual styles we define ourselves
- 0024 text layout and bundled fonts
- 0025 PNG and SVG backends and determinism (amended: shared scale cap)
- 0026 the Phase 3 CLI
- 0027 Phase 3 test strategy

## Known issues

- **CI has still never run** (no remote). When it exists, set `FERROGRAM_REQUIRE_CORPUS=1` and provide the libvisio oracle.
- **Fidelity gaps** (all in `docs/STATUS.md` deferred items):
  - master-sized geometry on resized instances (Phase 6 formula engine);
  - the Quick Style matrix colour shading (about a 0.73 factor) is not applied;
  - long text overflows its box in `bgcolor`; `TextBkgnd` is not drawn;
  - underline and strike-through are not drawn; shadow blur is not applied;
  - tab stops ignore `tp` rows and multi-stop rows; bullets use our own symbols;
  - periodic splines are read as clamped; no corpus file has spline rows, so the knot rule has no black-box check.
- **Oracle coverage:** full per-page comparison on 3 of 15 corpus pages. The others skip on text drawn as paths, group flattening, unresolved themed paints, and smaller arc and ellipse boxes in `testfile1`/`testfile5`.
- **Warnings:** text layout's `MissingFont`/`MissingGlyph` warnings do not reach the scene or the CLI; boolean and list coercions still accept non-finite values.
- **Golden images** were made on macOS; the metric may need loosening on Linux.
- **Writer:** the Phase 2 writer error paths stay uncovered; their callees are infallible today.
