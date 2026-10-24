# Phase 3 code review (P3-12)

Task P3-12, reviewer, 2026-10-15, branch `agent/P3-12`. Review of
`git diff phase-2..HEAD` against AGENTS.md section 6, the layering (ADR
0002) and ADRs 0019–0027. The reviewer role is read-only: this task
changed no code.

Scope covered: `larrygram-geom` (curve, path), `larrygram-model`
(geometry, lookup, values, layer, theme, text, defaults, document),
`larrygram-text` (font, layout), `larrygram-render` (scene, build,
styles, png, svg), `larrygram-vsdx` (read/text, read/theme and the read
plumbing they touch) and `apps/larrygram-cli` (cli, parse, exec, inspect).

## Verdict

**Approve with fixes: 0 blocker, 4 major, 8 minor.** The phase's hard
rules hold: no panic path was found on any input-derived route, the caps
are real, the row-aware lookup follows ADR 0019 and its amendment, curve
math is deterministic and bounded, and layering is green (`cargo xtask
layers`: 15 crates). The four majors are one policy violation (silent
substitution), one latent lookup gap and two API decisions the master
left to this review.

## Checks that passed

- **Panic safety.** `cargo clippy --workspace --all-targets --all-features
  -- -D warnings` is clean, so the never-crash lints hold everywhere
  outside tests. Parser crates (`vsdx`, `vsd-binary`, `shapesheet`) deny
  `clippy::indexing_slicing` at the crate root; every crate forbids
  `unsafe`. Manual walk of the input-derived paths (geometry decoding,
  theme/text XML decoding, font parsing, layout, both backends, the CLI)
  found no unchecked index, no unbounded recursion and no input-derived
  allocation: `geom` caps segments (4096), span depth (10) and degree
  (25); the builder, both backends and the text reader cap recursion at
  128 (8 in one place, see minor 6); PNG caps the raster at 16384 px per
  side and 256 MiB with `checked_mul`; SVG caps the document at 64 MiB
  and nesting at 128; `text::layout` validates its block and errors
  `InvalidBlock`; `vsdx` parsing goes through `Limits`.
- **Row-aware lookup (ADR 0019).** `resolve_repeated_with_table`
  (`crates/model/src/lookup.rs:168`) has the right rungs in the right
  order (local row, master chain, no style rung, section default, hard
  default), cuts master and style cycles with visited sets, and treats
  `Inh` as "does not define" (`defines`, `lookup.rs:715`). The row union
  of `shape_geometry` (`crates/model/src/geometry.rs:257`) matches rows
  by `IX` when both sides have one and by position otherwise, in the same
  way `matched_row_cell` (`lookup.rs:669`) resolves cells, so the union
  and the cell lookup agree on row identity. `Del` removal, `T` takeover
  and `IX`-order insertion match the ADR 0019 amendment.
- **Warning policy (ADR 0021).** `CellReader` substitutes the documented
  or hard default and warns on every branch it covers (missing, error
  value, kind mismatch, unresolved theme); a warning never fails a
  render. The one silent substitution found is major 1. Themed colours,
  palette/document-table indices and the boolean/list coercions behave as
  the ADR 0021 amendments specify.
- **Curve math (ADR 0022).** `to_cubics` (`crates/geom/src/curve.rs:224`)
  degrades every named degenerate case to the chord (or to nothing) with
  no error and no panic; tolerance is clamped; the arc piece count, the
  B-spline span subdivision and the builder's ellipse flattening are all
  capped. Determinism holds: no hash-ordered iteration reaches the output
  (`styles.rs` uses `BTreeMap`; `lookup.rs`'s `HashMap` is a keyed cache),
  and the backends use fixed output formatting (SVG) or pure-Rust
  rasterization (PNG, ADR 0025).
- **Layering and the registry seam (ADR 0026).** `cargo xtask layers`
  passes. The CLI dispatches only through `Command` values and library
  calls; nothing in L0..L6 gained a UI or CLI dependency; the exit-code
  table matches ADR 0026 and is tested.

## Findings

### Major

1. **Silent substitution of a non-finite cell number (ADR 0021
   violation).** `crates/render/src/build.rs:237` — `Ctx::finite`
   replaces a non-finite value with `0.0` and records nothing, although
   the module docs (`build.rs:78`) promise a `WarningKind::KindMismatch`
   warning for "a non-finite cell value". The value can be non-finite
   upstream: `CellReader::number_from` (`crates/model/src/values.rs:262`)
   accepts any `CellValue::Number` of the right kind, and the reader
   parses `V` with `f64::from_str` (`crates/vsdx/src/read/value.rs:133`),
   which accepts `inf`, `nan` and `1e999`. A malformed or hand-built
   cell therefore renders as 0 with no trace — the exact "silent
   substitution" the ADR 0021 policy forbids.
   **Suggested fix:** reject non-finite numbers in
   `CellReader::number_from` (and `number_from_or`) and warn
   `KindMismatch` there, one policy point for every caller; drop
   `Ctx::finite` or make it unreachable. Regression test: a shape whose
   `PinX` caches `V='inf'` renders at 0 *and* records one warning.

2. **Text row cells resolve by position, never by `IX`, when the local
   sheet lacks the row.** `crates/render/src/build.rs:1761`
   (`row_position`) maps a marker's `IX` to a row position and falls back
   to `ix as usize` when the shape's own section has no row with that
   `IX`; `CellReader::row_text`/`row_number`/`row_boolean`/`row_color`
   (`crates/model/src/values.rs:150–245`) then look the row up with
   `row_ix: None`, so the master rung matches the row **by position**
   (`lookup.rs:690`). The geometry path solved this exact problem with
   `RepeatedCell.row_ix` (ADR 0019 amendment rule 2: "matched by `IX`
   when both sides have one"); the text path does not use it. When a
   master's `Character`/`Paragraph`/`Tabs` row `IX` values are not the
   same as their positions (deleted rows, merged files), a run's style
   resolves from the wrong master row or from the defaults — silently,
   since the substitution is a plain default with a `Missing` warning
   that names the right cell. Latent today (corpus rows are contiguous)
   but it will misrender master-inherited text exactly like the geometry
   case the amendment documents.
   **Suggested fix:** pass the marker's `IX` through as `row_ix` (extend
   the `CellReader::row_*` family with an `ix` parameter or add
   `row_*_by_ix`), and delete the `ix as usize` fallback in
   `row_position`. Test: an instance with no `Character` section and a
   master whose rows are `IX` 0 and 5; the run marked `IX` 5 must read
   the master's row 5.

3. **`PatternPaint` says how to tile but not how to draw the tile; PNG
   and SVG disagree (the P3-07 decision).**
   `crates/render/src/scene.rs:99` — `PatternPaint` has no `filled` flag
   and no tile stroke width. PNG fills the tile path unconditionally
   (`crates/render/src/png.rs:533–545`; an open tile auto-closes), SVG
   fills only a tile with a `Close` segment and strokes anything else as
   a 1 px hairline (`crates/render/src/svg.rs:580–615`). The ADR 0023
   example tile (`"M 0 0 L 1 1"`) renders as a diagonal mark in SVG and
   as nothing in PNG. The shipped `visual-styles.yaml` tiles are all
   closed today, so the backends agree on current output — the divergence
   is latent and will surface with the next open tile and with the Phase
   4 egui backend, which would have to pick a side.
   **Suggested fix (decision):** add `filled: bool` and
   `tile_stroke_width: f64` to `PatternPaint`, set them from the
   `PatternTemplate` in `build_fill`, and make both backends honor them.
   That keeps the scene the single source of drawing intent (ADR 0020)
   and gives the third backend an unambiguous spec.

4. **Two fields carry each of the text rotation and the run size (the
   P3-06 decision).** `TextNode::rotation` (`crates/render/src/scene.rs`,
   set at `crates/render/src/build.rs:1755` from `block.rotation`) and
   `TextRun::size` duplicate `text::Layout::rotation` and
   `GlyphPos::size` (ADR 0024 amendment). The backends consume the
   layout fields (`crates/render/src/png.rs:401–430`,
   `crates/render/src/svg.rs:411–447`); `TextNode::rotation` is read only
   by the scene docs (`scene.rs:27`) and tests, `TextRun::size` only by
   tests. Three consumers already disagree about which copy is canonical.
   **Suggested fix (decision): keep one of each — the layout's.** Keep
   `Layout::rotation` and `GlyphPos::size` (they make `text::Layout`
   self-contained, which the ADR 0024 output contract wants) and remove
   `TextNode::rotation` and `TextRun::size`. `TextRun` keeps `color`,
   `underline` and `strikethrough`; underline thickness, when a backend
   draws it, takes the glyph's `size`. Update the `scene.rs` placement
   recipe to reference `Layout::rotation` only.

### Minor

5. **One scale cap for both backends; `Error::InvalidOptions` (the P3-11
   decision).** PNG caps the raster (`crates/render/src/png.rs:67–70`),
   SVG validates only finite-and-positive scale plus its byte cap
   (`crates/render/src/svg.rs:124–136`), so `--scale 1e300` writes an SVG
   with absurd dimensions and returns `TooLarge` for a *bad option* as if
   the document were too big. **Decision: share one cap.** Bound the
   device extent (`width * scale`, `height * scale` ≤ 16384 px) in both
   backends, add `Error::InvalidOptions` for a non-finite, non-positive
   or over-cap `scale` (also the P3-07 open item), and update the P3-11
   test `a_huge_scale_caps_the_raster_not_the_svg_text`, which pins the
   current asymmetry.

6. **A group's own geometry is not recorded in ADR 0020 (the P3-05
   decision).** `crates/render/src/build.rs:471–533` emits a group's own
   shape as the first child of its `Group` with an identity transform,
   the group carrying the shape transform — the only structure the ADR
   0020 types allow (`Group` has no shape slot), and drawn behind its
   sub-shapes. This is fine, but the ADR does not say so.
   **Suggested fix:** amend ADR 0020 with one sentence recording the
   rule and the draw order (own shape first, sub-shapes in file order on
   top), and put a black-box check of Visio's group z-order on the
   screenshot-pass list.

7. **Scene doc drift after the relative-transform fix (the `e3d616f`
   check).** The builder and both backends are correct: node transforms
   are parent-relative (`build.rs:492–525`), the absolute transform is
   composed only for the `InfiniteLine` page clip, PNG composes
   `group.transform.then(total)` (`png.rs` `draw_nodes`) and SVG writes
   per-node `transform` attributes. One doc line is stale:
   `crates/render/src/scene.rs:27` ("The shape's `Shape::transform` then
   maps it to the page") — it maps to the enclosing group's space; the
   whole ancestor chain maps to the page. Fix the sentence.

8. **Degenerate-curve degradation is silent; ADR 0022 says "and warn".**
   `larrygram-geom` has no warning channel (`to_cubics` is always `Ok`)
   and `append_curve` (`build.rs:1054`) records nothing, so bow 0, a
   zero chord, `ratio <= 0`, bad knot lists and non-finite points
   degrade to chords with no `CellWarning`, while unknown row types and
   unparsable formulas do warn. **Suggested fix:** either detect the
   degradation in `rows_to_path` (a curved row that yields one chord or
   nothing) and warn, or amend ADR 0022 to say the degradation is
   silent. The cheaper honest option is the amendment.

9. **`ErrorValue` prose is printed as a cached error.** The builder
   stuffs a sentence into `WarningKind::ErrorValue`
   (`build.rs:1012–1020`), and the CLI prints it as
   `cached error {error:?}, using …` (`apps/larrygram-cli/src/cli/exec.rs:233`),
   so a `NURBS()` parse failure reads as `cached error "a NURBS() formula
   that does not parse; drawing a straight segment"`. Cosmetic, but the
   wording misattributes the failure to the file's cached value.
   Suggested fix: use `WarningKind::KindMismatch` for formula-parse
   degradations, or give the CLI a separate phrasing for builder
   warnings.

10. **`section_rows` walks a different chain than the cell lookup.**
    `build.rs:1328–1361` counts gradient rows by walking
    `master.shapes.first()` with a magic bound `visited.len() >= 8`,
    while the cells resolve through the ADR 0013 rungs (position and
    sub-shape rules, `MAX_DEPTH`). For a multi-shape master the row count
    and the row cells can come from different master shapes. Suggested
    fix: count rows through the same rung the cells use and share
    `MAX_DEPTH`.

11. **Un-drawable glyphs disappear silently.** `crates/render/src/svg.rs:409–421`
    (and the PNG equivalent) skips a glyph whose font index is out of
    range or whose outline fails or is empty — no warning reaches
    `Scene::warnings`. Impact is low (the layout already warns
    `MissingGlyph` for a missing character), but the skip is a silent
    substitution in the backend layer. Suggested fix: record one
    `CellWarning` at build time when a glyph cannot be drawn, or document
    the skip in the `scene.rs` placement recipe as intentional.

12. **`BuildOptions::new` and `Default` disagree.** `build.rs:141–156`:
    `new()` sets `background: true`, `default()` sets `false`. Rust
    convention is `Default == new()`; the asymmetry will surprise the
    Phase 4 caller. Suggested fix: make them agree (the CLI's
    `inspect`/`render` rely on `new()` including the background page, so
    pick `true` for both or document the difference in the type docs).

## Note on the deferred items from the master

All five are covered above: the duplicated text fields (major 4), the
pattern divergence (major 3), the shared scale cap (minor 5), the group
shape placement (minor 6) and the relative-transform check (minor 7).
The two P3-11 hardening candidates (non-finite `text::layout` output for
extreme finite input; the SVG scale bound) are addressed by major 1 (the
input side) and minor 5 respectively; the layout output hardening stays
with the Phase 6 formula-engine pass as P3-11 recorded.
