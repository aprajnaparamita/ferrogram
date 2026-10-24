# Phase 3 coverage audit

- Task: P3-13 (coverage-auditor), 2026-10-15
- Scope: `main` at `255d730` (after P3-11), with the Phase 3 code in
  `geom`, `model`, `render`, `text`, `vsdx` and `larrygram-cli`.
- Verdict: **PASS** — every crate with instrumented lines is above its
  threshold and above its ratchet. Findings: 0 blocker, 5 major, 10 minor.

## How measured

- `cargo xtask coverage` (which runs
  `cargo llvm-cov --workspace --all-features --json --summary-only`) in the
  P3-13 worktree with `CARGO_TARGET_DIR=/Volumes/Dara/larrygram/target/P3-13`.
  All tests green in that run (230 xtask tests plus every crate's tests).
- `cargo llvm-cov report --lcov` for exact per-line execution counts.
  Findings below list only lines with a count of 0, i.e. lines no test
  executes at all. `cargo-llvm-cov 0.9.1`.
- The corpus is present in the worktree (`corpus/vsdx`, `corpus/vsd`), so the
  corpus tests ran and their execution counts are included.
- Everything large stayed on `/Volumes/Dara/larrygram/` (worktree, target dir);
  this audit changed no code.

## Per-crate table

Line counts from the llvm-cov summary. "Ratchet" is the value currently in
`coverage.toml`, which is the Phase 2 value. "Phase 2" is the measured
percentage from `docs/progress/phase-2.md`.

| Crate | Lines | Covered | Pct | Threshold | Ratchet | Phase 2 | Pass/fail |
|---|---:|---:|---:|---:|---:|---:|---|
| larrygram | 9 | 6 | 66.7 | 50.0 | 66.6 | 66.7 | pass |
| larrygram-cli | 1322 | 1256 | 95.0 | 50.0 | 66.6 | 66.7 | pass |
| larrygram-commands | 0 | 0 | n/a | 85.0 | — | n/a | pass (empty) |
| larrygram-control | 0 | 0 | n/a | 50.0 | — | n/a | pass (empty) |
| larrygram-geom | 1677 | 1677 | 100.0 | 85.0 | 100.0 | 100.0 | pass |
| larrygram-model | 6633 | 6613 | 99.7 | 85.0 | 99.7 | 99.7 | pass |
| larrygram-render | 5327 | 5027 | 94.4 | 75.0 | — | new | pass |
| larrygram-shapesheet | 45 | 45 | 100.0 | 85.0 | 100.0 | 100.0 | pass |
| larrygram-text | 1677 | 1627 | 97.0 | 75.0 | — | new | pass |
| larrygram-ui | 0 | 0 | n/a | 50.0 | — | n/a | pass (empty) |
| larrygram-ui-spec | 1738 | 1726 | 99.3 | 85.0 | 99.3 | 99.3 | pass |
| larrygram-vsd-binary | 0 | 0 | n/a | 85.0 | — | n/a | pass (empty) |
| larrygram-vsdx | 7188 | 7039 | 97.9 | 85.0 | 97.7 | 97.8 | pass |
| larrygram-web | 2 | 0 | 0.0 | — | 0.0 | 0.0 | pass (no threshold) |
| xtask | 9232 | 8457 | 91.6 | — | 91.6 | 91.6 | pass |

No crate is below its threshold or below its ratchet, so there is no blocker.
`larrygram-geom` and `larrygram-shapesheet` stay at 100%;
`larrygram-commands`, `larrygram-control`, `larrygram-ui` and
`larrygram-vsd-binary` are still empty placeholders.

Comparison with Phase 2: `vsdx` grew (+0.1) while gaining Phase 3 code,
`model` and `ui-spec` are flat, `xtask` is flat. The apps changed shape:
`larrygram-cli` went from 9 stub lines (66.7%) to 1322 lines (95.0%), and
`larrygram-render` (94.4%) and `larrygram-text` (97.0%) are new, both well
above their 75% thresholds.

## Recommended `[ratchet]` for the end of Phase 3

The master updates `coverage.toml`; this task does not edit it. These are the
values `cargo xtask coverage --update-ratchet` would write (percent floored to
one decimal, empty crates omitted):

```toml
[ratchet]
larrygram = 66.6
larrygram-cli = 95.0
larrygram-geom = 100.0
larrygram-model = 99.6
larrygram-render = 94.3
larrygram-shapesheet = 100.0
larrygram-text = 97.0
larrygram-ui-spec = 99.3
larrygram-vsdx = 97.9
larrygram-web = 0.0
xtask = 91.6
```

Notes:

- The floored values are one tenth below the displayed table numbers where the
  measured percentage does not land on a tenth: `larrygram-model` 99.6
  (99.698% measured), `larrygram-render` 94.3 (94.368%).
- `larrygram-model` is the one crate where the floor (99.6) is below the
  current ratchet (99.7). The current gate passes only because `check` allows
  0.05 of float noise. Writing 99.6 is honest; writing 99.7 again would keep
  relying on the epsilon.
- `larrygram-render` and `larrygram-text` get their first ratchets now.

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
   untested.** `apps/larrygram-cli/src/cli/parse.rs:134-141`, `:183-190`
   (unexpected argument, unknown option — the first thing hostile command
   lines hit), `apps/larrygram-cli/src/cli/exec.rs:232-239` (warning-kind
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
`apps/larrygram-cli/src/cli/parse.rs:327-399`,
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
- `larrygram` (the app) is unchanged at 6/9 lines, one uncovered line above
  its ratchet. Its `main` (`apps/larrygram/src/main.rs:7-9`) and
  `larrygram-cli`'s `main` (`apps/larrygram-cli/src/main.rs:10-15`) are real
  process entry points that no test executes; the 66.6 ratchet for
  `larrygram` stays fragile for the same reason as in earlier phases.
- The line totals for `geom`, `model`, `render`, `text` and `vsdx` are roughly
  three times the Phase 2 numbers for `geom`/`model` because Phase 3 added
  the geometry rows, the render builder and text layout with their inline
  test modules. Coverage percentages are comparable; line counts are not a
  code-size measure on their own.

## Follow-up (P3-17)

- Task: P3-17 (test-engineer), 2026-10-15, branch `agent/P3-17` on top of
  `main` (`305626d`). Tests only: 56 new tests, and the only additions to
  non-test code are two `#[cfg(test)]` helpers
  (`larrygram_text::font::unparsable_for_test`,
  `larrygram_render::styles::parse_for_test`). No product behaviour
  changed; no new dependencies. Line numbers below are the current ones
  (several of the audit's moved in P3-16).

### The five major findings

1. **Render builder fallbacks for missing and wrong-kind cells —
   closed.** Each substituted kind now has a test in
   `crates/render/src/build.rs`:
   `reader_errors_substitute_defaults_for_every_kind` (number, bool,
   colour, row number, row text, row bool and row colour read from a
   sheet the document does not hold: defaults `0`/`false`/black/empty
   plus one `Missing` warning each),
   `wrong_kind_cells_substitute_defaults_and_warn` (a `Str` where a
   number, a boolean, a colour and a row number are wanted: `KindMismatch`
   plus the default), `missing_cells_without_a_documented_default_warn`,
   `a_dangling_sub_shape_warns_about_its_layer_membership` (`LayerMember`),
   `shape_content_of_an_unknown_shape_warns_about_its_geometry` (the
   geometry section) and `a_text_layout_failure_warns_about_the_text_cell`
   (a negative `SpBefore` makes `text::layout` reject the block; the
   builder drops the text and warns `KindMismatch` on `Text`).
2. **PNG hostile-input guards — closed.**
   `degenerate_gradients_draw_nothing` (non-finite linear endpoints, zero
   and negative radial radii, non-finite centres),
   `a_gradient_with_unusable_stops_still_rasterizes` (non-finite stop
   offsets dropped), `degenerate_pattern_tiles_fall_back_to_the_foreground_colour`
   (non-finite, sub-pixel and over-the-byte-cap tile boxes),
   `a_stroke_wider_than_the_device_bound_is_skipped` (the `fits_device`
   reach check of P3-14 F4) and `bad_text_draws_nothing_but_never_panics`
   (non-finite text origin, out-of-range font index). The raster caps
   (non-finite, sub-pixel, over-`MAX_DIMENSION`, over-`MAX_BYTES`) were
   already pinned by `png::tests::oversize_scenes_are_too_large` and
   `raster_size_is_checked_with_integer_math`.
3. **SVG size cap and segment mapping — closed.**
   `an_absurd_document_hits_the_output_cap` (`render_svg` returns
   `Error::TooLarge` on a 60k-segment path whose coordinates print as
   300-digit numbers), `the_document_assembly_enforces_the_output_cap`
   (the `finish` check, where the header pushes the document over the
   cap), `line_end_path_maps_quad_and_cubic_segments` (the quad/cubic
   mapping and the `Close` arm) and `endpoint_reads_the_first_segment_direction`
   (quad and cubic first segments, and a second `MoveTo`).
4. **PNG line ends — closed.**
   `line_ends_draw_the_start_arrowhead_and_outline_the_end` (the start
   end, a colour override on a filled head, and an outlined head in the
   stroke's own colour: filled body vs outline asserted on pixels) and
   `degenerate_line_ends_are_skipped` (zero width, empty shape).
   The end head with an override colour was already covered by
   `line_ends_draw_at_the_vertices`.
5. **CLI usage errors and the warning report — closed.**
   `render_and_export_reject_extra_arguments_and_unknown_options` and
   `an_unrecognizable_output_extension_is_a_usage_error` (parse.rs),
   `warning_lines_name_every_kind` (all four `WarningKind`s),
   `sheet_text_names_pages_shapes_and_fallbacks` and
   `value_text_names_every_value_kind` (every `CellValue` kind). The
   `inspect` variants of the two usage errors were already tested.

### The minors

- **Minor 1, the `nominal()` fallback — closed.**
  `nominal_advances_replace_shaping_when_the_font_does_not_parse` (the
  `harfrust::Font::new` failure arm) and
  `nominal_advances_come_from_the_font` (advances from the face).
- **Minor 2, font edge cases — closed except one.**
  `advance_em_of_an_unparsable_font_is_zero`,
  `path_pen_records_cubic_curves` (cubic glyph outlines; the bundled
  faces are TrueType, so `curve_to` is exercised directly) and
  `family_names_come_from_the_name_table`. The name-preference fallback
  (`pick_name` keeping the first record when none is English) stays
  uncovered: both bundled faces carry English name records and a
  synthetic face was not cheap.
- **Minor 3, style-level warnings — closed except two unreachable
  branches.** `a_rectangular_gradient_direction_warns`,
  `an_unknown_line_pattern_warns_and_draws_solid`,
  `an_unknown_line_cap_falls_back_to_butt`,
  `an_unknown_arrow_index_and_size_class_warn`,
  `an_unknown_shadow_pattern_warns`,
  `shadow_offsets_default_to_the_template`,
  `a_line_end_template_with_no_shape_draws_nothing`,
  `an_inverted_tile_swaps_the_colours`,
  `vertical_align_bottom_moves_the_lines_down`,
  `paragraph_alignment_and_line_spacing_branches` (right/justify and the
  negative and positive `SpLine` branches),
  `txt_placement_cells_override_the_shape_box` and
  `an_infinite_line_clips_through_a_flipped_page`.
- **Minor 4, `InvalidStyleData` on duplicate indices — closed.**
  `styles::duplicate_style_indices_are_invalid_data` (line ends, fill
  patterns and shadows).
- **Minor 5, PNG pattern tiles, quad/cubic dashes and a second clip —
  closed.** `a_stroked_tile_draws_its_outline`,
  `a_pattern_tile_without_a_mark_draws_its_background`,
  `quad_and_cubic_segments_stroke_and_dash`,
  `path_ends_follow_curve_and_close_segments`,
  `a_second_clip_intersects_into_the_mask` and
  `transparent_and_non_finite_shadows_are_skipped`.
- **Minor 6, the writer's `?` arms — cannot be covered.** The six
  `?` call sites in `crates/vsdx/src/write/mod.rs` (`ensure_list`,
  `contents_part`, `ensure_contents`) cannot see an error: all three
  callees are infallible today. The arms are an API decision (making
  them fallible), not a test gap; the Phase 2 major "the writer's
  error-returning paths" stays open with this note. The second half,
  `refresh_content_types` pushing a missing `Override`, is covered by
  `write::tests::refresh_content_types_adds_a_missing_override`.
- **Minor 7, `write/patch.rs` — not done.** The 57 miss arms and
  ordering fallbacks need preserved fragments with missing or reordered
  elements; each needs a hand-built tree. Not cheap; left for a task
  that owns the writer.
- **Minor 8, `write/emit.rs` — mislabelled.** The six flagged lines are
  the `_ =>` arms of `match child` in test helper closures (map an
  element's name, `"?"` for anything else). They are test code in the
  "never-hit arms" class of the audit's own last section, not product
  unit-code fallbacks.
- **Minor 9, `package/read.rs` — behaviour pinned, attribution
  disputed.** `parse_relationships_skips_non_element_children` pins the
  non-element child rule and `rels_source_maps_rels_names_to_their_sources`
  pins the root `/_rels/.rels` rule, but `cargo llvm-cov` still reports
  those two lines (`:214`, `:379`) at zero executions while counting
  their neighbours; it also reports `:378` (the element arm that the
  same test executes twice) at 1. This looks like a line-attribution
  artefact of the closure matches, not an untested path. The two arms
  that genuinely cannot fire: `:156` (the inferred content type of a
  part with no declaration — `infer_content_types` declares every entry
  of a package without `[Content_Types].xml`, so the arm is defensive)
  and `:220` (a `/_rels` directory with an empty parent; normalization
  removes the leading slashes, so the directory is `_rels` and the
  earlier branch takes it).

### Coverage before and after

`cargo llvm-cov --workspace --all-features` with the corpus present.
Per-crate line counts (instrumented `src/` lines, including inline test
code, as in the audit above):

| Crate | Before | After |
|---|---:|---:|
| larrygram-render | 5251/5542 (94.7%) | 6364/6447 (98.7%) |
| larrygram-cli | 1233/1290 (95.6%) | 1340/1375 (97.5%) |
| larrygram-vsdx | 6823/6934 (98.4%) | 6865/6971 (98.5%) |
| larrygram-text | 1596/1641 (97.3%) | 1714/1731 (99.0%) |

Nothing dropped. `xtask` moved 8143/8808 to 8141/8808 between runs
without a source change: run-to-run attribution noise in test code, and
still well over its ratchet.

### What stays uncovered in the touched files

All of these are unreachable by construction or test-code noise; each is
annotated where it sits:

- `build.rs`: the `GeometryRow` catch-all (all 16 variants are matched),
  `scale_path`'s quad/cubic arms (no committed tile template curves),
  `append_curve`'s error return and `clip_infinite_line`'s non-finite
  guard (the cached-value policy only ever delivers finite numbers),
  `first_size <= 0` in the line-spacing rule (a run's size is always
  positive after the default), `background_group`'s stale-page arms, and
  the never-hit `panic!` arms of the inline tests (the audit's "tests
  that assert nothing" class).
- `png.rs`: the `Font::outline` error arm (a `Font` only exists when its
  bytes parsed) and `pattern_transform`'s `None` arm (it is only called
  after `tile_pixels` succeeded).
- `svg.rs`: the same `Font::outline` arm, and the `_` catch-alls of the
  `Segment` match (`Segment` is non-exhaustive and has no other variant
  today).
- `cli`: the `_` catch-alls of the `WarningKind` and `CellValue` matches
  (both enums are non-exhaustive in `larrygram-model`) and `unknown_option`'s
  `Arg::Value` arm (a positional is handled before it can reach the
  helper).
