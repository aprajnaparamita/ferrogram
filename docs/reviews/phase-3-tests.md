# Phase 3 test pass (P3-11)

Task P3-11, test-engineer, 2026-10-15, branch `agent/P3-11`.

A written pass over every Phase 3 crate: each untested path found gets a
test or a written reason. Test names below are the ones added or changed
by this task. The one product bug the pass found is fixed with a
regression test and recorded under "Bug fixed" and "Open issues".

## Summary

| Crate | Untested paths found | Action |
|---|---|---|
| `larrygram-geom` | none material | written reason (below) |
| `larrygram-model` | row typing on random rows/cells/`IX`/`Del`; `NURBS()`/`POLYLINE()` edge spellings | 2 property tests, 2 unit tests |
| `larrygram-vsdx` | `Text` marker `IX` bounds; nested `fld`; theme colour fall-through, alpha range, `ColorEntry` bounds | 5 unit tests |
| `larrygram-text` | layout on random runs; validator on random invalid blocks; extreme-but-finite inputs | 3 property tests |
| `larrygram-render` | empty and single-shape scenes end to end; zero-size and huge pages; huge scale; text overflow; outputs parse | 6 tests in a new file, 2 property tests extended |
| `larrygram-cli` | argument boundaries (`--page`, `--scale`, `--format`, `--background`, non-UTF-8); huge scale/page exit codes | 4 parse tests, 2 exec tests |

Everything Phase 3 left untested and not listed here is covered by the
tests the implementing tasks wrote (the naming convention makes them easy
to find: one sentence per behavior).

## `larrygram-geom` (ADR 0022 math)

Untested paths found: none material. Every `Curve::to_cubics` variant has
its three levels per ADR 0027 section 1 (typing in `model`, math in
`geom`, mapping in `render`), the degenerate cases degrade to chords with
pinned tests, and `cargo xtask coverage` reports **100.0%** line
coverage — there is no uncovered line to name. Written reason: nothing to
add.

## `larrygram-model` (geometry typing)

Untested paths found:

1. **Row typing on random input.** `shape_geometry` was only tested with
   hand-built rows: random `T` spellings, random cell kinds (`Bool`,
   `List`, `Error`), random formula text, duplicate/negative `IX` values
   and `Del` rows combine into merge paths (`apply_rows`) that no single
   unit test walks. Added
   `tests/geometry_properties.rs::random_geometry_rows_type_without_panicking`
   and `::random_rows_through_a_master_chain_type_without_panicking`:
   typing never panics, returns one path per own `Geometry` section, and
   the merged rows never outnumber the rungs' rows.
2. **`NURBS()`/`POLYLINE()` edge spellings.** Space before the
   parenthesis, a leading `=`, a trailing comma, a stray closing
   parenthesis, quoted non-numbers, infinite `knotLast` (kept as written)
   vs infinite `degree` (rejected), every bare-list separator, odd and
   empty token lists, and the rule that a `POLYLINE(...)` call never falls
   back to the bare-list form. Added
   `src/geometry.rs::parse_nurbs_data_edge_spellings` and
   `::parse_polyline_data_edge_spellings`.

## `larrygram-vsdx` (Phase 3 decoders: `Text`, theme XML)

Untested paths found:

1. **`Text` mixed content, marker bounds.** `IX` is trimmed but bounds
   are strict (`u32`); empty, negative and out-of-range values must error
   naming the part. Added `src/read/text.rs::
   marker_ix_trims_whitespace_and_respects_the_bounds`.
2. **`Text` mixed content, nesting.** `fld` inside `fld` stacks its
   markers before the inner text. Added
   `src/read/text.rs::fields_nested_in_fields_stack_their_markers`.
3. **Theme colour slots.** A slot with an unparsable `srgbClr` must fall
   through to the next child (`sysClr/@lastClr`), and a slot whose only
   colour is broken must be `None`, not a default colour. Added
   `src/read/theme.rs::a_slot_falls_through_an_unparsable_colour_to_the_next_child`.
4. **Theme alpha range.** `a:alpha` outside `u32` (negative, overflow,
   junk, empty) falls back to the DrawingML default 100000. Added
   `src/read/theme.rs::an_unparsable_alpha_falls_back_to_opaque`.
5. **`ColorEntry` bounds.** The `IX` is trimmed and must fit `i32`; the
   `RGB` literal is *not* trimmed and must be a `#RGB`/`#RRGGBB`
   spelling. Added `src/read/theme.rs::
   color_entry_indices_and_colours_say_what_they_accept`.

Written reason for the rest of the theme surface: the scheme, font,
variation and `fmtScheme` mappings are pinned by the implementing task's
tests (`a_theme_maps_its_schemes_and_keeps_the_rest` and its seven
siblings); the theme parts are preserved-only, so unmapped content has no
behavior beyond staying in `Preserved`, which the same tests assert.

## `larrygram-text` (layout)

Untested paths found:

1. **Layout on random runs.** The golden-number tests cover each feature
   in isolation; nothing covered random combinations (wrapping, tabs,
   bullets, justification, vertical alignment, margins, rotation on
   random text with random sizes and families). Added
   `tests/layout_properties.rs::random_blocks_lay_out_without_panicking`:
   the layout never panics and every output value is well-formed (finite
   glyph positions and sizes, run indices into their paragraph or
   `BULLET_RUN`, font indices into the bundled stack).
2. **The validator as a property.** `validate` rejects non-finite and
   negative input; random corruptions of each field must error, not
   panic. Added `::invalid_blocks_error_without_panicking`.
3. **Extreme-but-finite inputs.** `f64::MAX` spacing, indents, tabs and
   sizes are accepted by the validator. Added
   `::extreme_but_finite_values_never_panic`, which pins no-panic
   behaviour and records the hardening candidate (Open issues).

Written reason for the feature tests ADR 0027 section 7 lists (wrapping,
alignment, spacing, bullets, tabs, vertical alignment, rotation,
warnings): `src/layout.rs::layout_tests` already pins each with golden
numbers; this task did not duplicate them.

## `larrygram-render` (scene builder, styles, backends)

Untested paths found:

1. **Empty and single-shape scenes end to end.** The backends were tested
   on hand-built `Scene`s and the corpus on full documents; the simplest
   documents were not run through build + both backends with the output
   parsed. Added `tests/boundaries.rs::an_empty_page_renders_in_both_backends`
   and `::a_single_shape_page_renders_in_both_backends`.
2. **Zero-size pages.** The letter-size fallback was pinned only by the
   unit test in `src/build.rs`. Added
   `tests/boundaries.rs::a_zero_page_size_falls_back_to_letter`.
3. **Huge pages and huge scale.** The caps (`render::Error::TooLarge`)
   were tested at the backend level only. Added
   `tests/boundaries.rs::a_huge_page_is_capped_not_a_panic` and
   `::a_huge_scale_caps_the_raster_not_the_svg_text` (see Open issues for
   the SVG asymmetry the second one pins).
4. **Text overflow.** `text::Layout::overflow` was tested in `text`; no
   test checked it survives the scene builder and still renders. Added
   `tests/boundaries.rs::text_overflows_its_box_and_still_lays_out`.
5. **"PNG/SVG outputs parse" as a property** (ADR 0027 section 6).
   `tests/scene_properties.rs::exercise` now decodes every PNG
   (`tiny_skia::Pixmap::decode_png`) and parses every SVG (balanced XML
   tags) instead of discarding the output; the two properties
   (`bounded_random_models_build_scenes_without_panicking`,
   `random_geometry_rows_map_to_paths_without_panicking`) exercise both.

Written reason for the golden pass: `tests/golden.rs` pins 11 fixture
goldens plus `blue-box` with the ADR 0027 metric, `tests/svg_snapshot.rs`
pins the SVG text, and `tests/oracle_render.rs` compares 3 corpus pages
against the libvisio oracle. The boundary cases above are structural, not
visual; adding goldens for them would pin our own caps, not rendering
fidelity. The metric itself is unit-tested (`compare_images` tests), so
no new golden work was needed.

## `larrygram-cli` (ADR 0026)

Untested paths found:

1. **`--page` bounds.** `u32::MAX` parses, one past it does not;
   negative, empty and space-prefixed values error; a leading `+` is
   accepted by the integer parser (pinned deliberately). Added
   `src/cli/parse.rs::page_numbers_stay_in_their_bounds`.
2. **`--scale` bounds.** Only finite and positive: zero, negatives,
   `inf`, `1e400` and `nan` spellings error with the numeric message;
   unparsable text errors with the parse message; tiny positive values
   parse. Added `src/cli/parse.rs::scale_values_stay_in_their_bounds`.
3. **Case sensitivity and colour spellings.** `--format PNG` and
   `--background NONE` are errors (the keywords are exact), `#ABCDEF`
   and `abcdef` are colours, `##123456` and the empty string are errors.
   Added `src/cli/parse.rs::format_and_background_values_are_exact`.
4. **Non-UTF-8 arguments** (unix). Both the command name and an option
   value are clean usage errors. Added
   `src/cli/parse.rs::non_utf8_arguments_are_usage_errors`.
5. **Exit-code boundaries end to end.** A huge scale and a huge page are
   exit 3 with the `TooLarge` message and no output file, not a panic.
   Added `src/cli/exec.rs::a_huge_scale_is_exit_three_not_a_panic` and
   `::a_huge_page_is_exit_three_not_a_panic`.

Written reason for the rest of the CLI surface: the exit-code table, the
stdout/stderr split, the strict flag and every command's happy path are
pinned by the implementing task's 23 tests in `exec.rs`/`inspect.rs`.

## Crates with no Phase 3 code

`larrygram-shapesheet`, `larrygram-commands`, `larrygram-control`,
`larrygram-ui`, `larrygram-ui-spec`, `larrygram-vsd-binary`,
`apps/larrygram`, `apps/larrygram-web` received no Phase 3 code (the
phase touched geom, model, text, render, vsdx and the CLI). Written
reason: no new untested paths; their existing suites and thresholds
stand.

## Bug fixed

`text::layout` — `Line::width` could come out a tiny negative number.
`finalize` computed the width as `pen - x_start - shift`; for a
zero-advance line (e.g. a zero-point run) that subtracts two nearly equal
positions and cancels (observed: `-5.55e-17`, `-1.11e-16`). A width is
the sum of advances and is non-negative. Fix: `finalize` now sums the
advances it places (`crates/text/src/layout.rs`). Regression test
`src/layout.rs::a_zero_advance_line_has_a_non_negative_width` pins the
minimal input the property test found; it was verified to fail against
the pre-fix code. No other behavior changes: all 57 pre-existing layout
tests pass unmodified.

## Coverage

`cargo xtask coverage`, before and after this task (line %):

| Crate | Before | After | Threshold | Ratchet |
|---|---|---|---|---|
| larrygram | 66.7 | 66.7 | 50.0 | 66.6 |
| larrygram-cli | 94.6 | 95.0 | 50.0 | 66.6 |
| larrygram-geom | 100.0 | 100.0 | 85.0 | 100.0 |
| larrygram-model | 99.7 | 99.7 | 85.0 | 99.7 |
| larrygram-render | 94.5 | 94.4 | 75.0 | — |
| larrygram-shapesheet | 100.0 | 100.0 | 85.0 | 100.0 |
| larrygram-text | 96.8 | 97.0 | 75.0 | — |
| larrygram-ui-spec | 99.3 | 99.3 | 85.0 | 99.3 |
| larrygram-vsdx | 97.9 | 97.9 | 85.0 | 97.7 |
| xtask | 91.6 | 91.6 | — | 91.6 |

No crate is below its threshold or its ratchet. `larrygram-render` shows
94.5 -> 94.4: the scene property tests draw random documents, so a few
branch lines in `build.rs` differ run to run; the crate has no ratchet
and is far above its 75.0 threshold.

## Open issues (for the fix tasks and the audits)

- **`text::layout` output can be non-finite for finite input.** Extreme
  but finite values (`line_spacing: f64::MAX` over several lines) give
  non-finite glyph positions. Nothing panics and the backends sanitize
  (SVG writes zero, PNG has `degenerate_scene_data_never_panics`), so
  this is a hardening candidate for the formula-engine/Phase 6 pass, not
  a crash. Pinned as no-panic by `extreme_but_finite_values_never_panic`.
- **`render_svg` has no upper bound on `SvgOptions::scale`.** PNG caps
  the raster and returns `TooLarge`; SVG validates only
  finite-and-positive and its byte cap, so `--scale 1e300 --format svg`
  writes a valid document with absurd dimensions. This is the documented
  contract (ADR 0025), so the test pins it
  (`a_huge_scale_caps_the_raster_not_the_svg_text`); P3-12 may want one
  shared cap for both backends.
- **New dev-dependency:** `proptest` on `larrygram-text` (already in the
  workspace via geom, model, render, ui-spec and vsdx). `Cargo.lock`
  gains one dependency edge; the master resolves lock conflicts as
  usual.
- The corpus-render and golden tests skip without `corpus/`; the run for
  this report had the corpus linked (`LARRYGRAM_REQUIRE_CORPUS` unset),
  so all 15 corpus files rendered in `corpus_render.rs` and the
  `blue-box` golden compared.
