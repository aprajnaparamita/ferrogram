# Phase 3 clean-room and licence audit

- Task: P3-14, role `cleanroom-auditor`
- Diff audited: `git diff phase-2..main` (Phase 3, 144 files, +33,464)
- Date: 2026-10-15, branch `agent/P3-14`

Method: `cargo deny check` in the worktree; `git ls-files` scanned for
binary and Microsoft-owned file types across the whole tree; the font name
tables of every committed `.ttf` read directly (family, full name,
version, designer, copyright); every added string literal of 40
characters or more in the Phase 3 `.rs`, `.md` and `.yaml` files and
40-character windows over every added (and every deleted) prose line
compared against a text corpus built from `reference/` (all Markdown, TXT,
HTML, XML and XSD downloads); libvisio identifiers grepped outside
`crates/vsd-binary`; `ATTRIBUTION.md`, `corpus/PROVENANCE.md` and
`docs/sources.md` rows checked against the committed assets. The master's
knot-vector item was checked against [MS-VSDX] (the PDF in
`reference/ms-vsdx/`) and the Visio VBA/automation reference in
`reference/learn/`.

## Verdict

**Pass with findings: 0 blocker, 0 major, 3 minor** (clean-room and
licence rules). No Microsoft-owned content is committed, no
libvisio-derived code exists outside `crates/vsd-binary`, the visual
style data is our own work with per-entry provenance notes, the bundled
fonts are exactly Carlito and Selawik with their OFL licences, every
asset has its attribution and provenance rows, and `cargo deny check` is
green. The master's knot-vector item is settled: the rule is an
independently supported fact, not libvisio-derived (details under
"Master item"); what must change is the provenance citation, not the
code. The three minor findings are one short verbatim clause from the
ShapeSheet reference quoted in an ADR, the libvisio-first citation of the
knot rule, and an unhandled periodic-spline reading.

**Separately, the gate is RED (F4 below): the audit run surfaced a
product crash in `render_png` that the Phase 3 property test hits on
random input.** It is not a clean-room issue and it is outside this
role's lane ("you never edit code"), so it goes to a fix task.

## Checks that passed

1. **Licences.** `cargo deny check` is green (`advisories ok, bans ok,
   licenses ok, sources ok`). `deny.toml` allows MPL-2.0 only through the
   `ferrogram-vsd-binary` exception and checks our own crates, so the
   graph is MPL-confined as ADR 0003 requires. Phase 3 touches no file in
   `crates/vsd-binary`.
2. **No Microsoft-owned files.** The committed binaries are our own
   fixtures and fuzz seeds (`corpus/fixtures/render-*.vsdx`, their copies
   in `crates/vsdx/fuzz/corpus/` and `crates/render/fuzz/corpus/`, the
   three hand-written `junk-*.bin`), our own renderer output (12 golden
   PNGs, 3 `insta` snapshots) and nine font files. No Office documents,
   stencils, templates, icons, images or spec PDFs anywhere in
   `git ls-files`, added or deleted, in this phase. No corpus third-party
   file is committed.
3. **Fonts.** Name tables confirm exactly the AGENTS.md section 4 set:
   four Carlito faces (family `Carlito`, version 1.104, designer tyPoland
   Lukasz Dziedzic, copyright The Carlito Project Authors) and five
   Selawik faces (family `Selawik`, version 1.01, designer and copyright
   Microsoft Corporation with Reserved Font Name Selawik). Both are
   SIL OFL 1.1 open-source releases; the licence texts are committed as
   `crates/text/fonts/Carlito-OFL.txt` and `Selawik-OFL.txt`. No Calibri,
   Segoe UI or any other Microsoft font is in the tree. Selawik is
   Microsoft-authored but open-licensed and named explicitly in
   AGENTS.md section 4; it is not a Microsoft proprietary asset.
4. **Assets and provenance.** Every Phase 3 asset has an `ATTRIBUTION.md`
   row: the golden PNGs (with the regeneration command), the snapshots,
   the `render_document` fuzz seeds, and a row per font face plus one per
   licence text. `corpus/PROVENANCE.md` gains rows for the 11 `render-*`
   fixtures and for the `render` fuzz seed copies (byte-identical copies
   of committed fixtures, per the file). `docs/sources.md` gains rows
   9–16 (visual style enumerations, the black-box corpus census, both
   fonts, `VisDefaultColors`, Quick Style cells, documented cell
   defaults, [MS-VSDX] Quick Style slices). All rows match the files on
   disk.
5. **Visual style data (ADR 0023).** `crates/render/data/visual-styles.yaml`
   is our own work: a provenance header naming what is derived (index
   semantics and ranges from the [MS-VSDX] per-cell value tables and the
   ShapeSheet reference; fill/shadow pattern names from the
   `VisCellVals` enumeration; dash ratios *measured from* the spec's table
   figures and re-expressed in our own numbers — the figures themselves
   are not committed) and what is ours (every dash rhythm, arrowhead
   path, fill tile and shadow value). All 76 entries carry exactly one
   `note` describing intent. Nothing is traced or extracted; the notes
   and the corpus census paragraph are our own words.
6. **No copied long-form text.** The scan found one verbatim match in the
   whole Phase 3 diff (F1). Everything else that matches `reference/` is
   URLs, licence texts (which must be verbatim), identifier names and
   divider comments. CLI help strings, warnings and the YAML notes are
   our own wording.
7. **libvisio confinement.** No libvisio identifiers, comments, struct
   names or constant tables outside `crates/vsd-binary` (which Phase 3
   does not touch). The only mentions are documentation (F2) and the
   corpus manifest handling in `xtask/src/corpus.rs` (source names and
   URLs). `crates/render/tests/oracle_render.rs` shells out to the
   oracle's `vsd2xhtml` and consumes its stdout; it holds no libvisio
   code.
8. **`reference/` untracked.** `git ls-files reference` is empty.
9. **Sources.** Every source in the new `docs/sources.md` rows is in the
   AGENTS.md section 4 allowed list: [MS-VSDX], Microsoft Learn (VBA
   reference and enumerations, ShapeSheet reference), black-box
   observation of corpus file data, and the two font releases (assets
   with OFL licences, named in AGENTS.md section 4). No spec text is
   committed.
10. **New dependencies** (all permissive, `cargo deny` green): `skrifa`,
    `read-fonts`, `font-types`, `harfrust`, `unicode-linebreak`,
    `tiny-skia`, `tiny-skia-path`, `png`, `insta`, `console`, `similar`,
    `lexopt`, `bytemuck`, `miniz_oxide`, `fdeflate`, `arrayvec`,
    `arrayref`, `strict-num`, `yoke`, `zerofrom`, `stable_deref_trait`,
    `synstructure` and their derive macros. No MPL crate was added.

## Findings

### F1 (minor) — one verbatim clause from the ShapeSheet reference

`docs/adr/0021-cached-values-defaults-and-warnings.md:180` quotes, in
quotation marks and with attribution, a seven-word clause of the
ShapeSheet reference's custom-color remark (the remark repeated on the
`LineColor`/`FillForegnd`/`Color` cell pages in
`reference/ms-vsdx/office-developer-client-docs/`). It is short, factual
and cited, so it is not "long-form text", but the audit rule asks that
every sentence-length verbatim match be flagged, and AGENTS.md section 4
asks for our own summaries. Fix: rephrase in our own words (for example:
a custom RGB colour indexes at 24 and above when used numerically).
Docs-only; any fix task or the master can do it.

### F2 (minor) — the knot-vector rule cites libvisio first

See the master item below: the rule is independently supported, but
`crates/geom/src/curve.rs` ("Knot vectors"), `docs/adr/0022` (the P3-05
amendment) and the P3-03 flag present libvisio's "fill in end knots"
reading as the matching support. Provenance should rest on the allowed
sources. What must change (docs only; no code change, no move to
`crates/vsd-binary`):

1. Cite [MS-VSDX] 2.4.2.15 (`SplineStart`: `B` = first knot, `A` = second
   knot, `C` = last knot, `D` = degree 0..25), 2.4.2.14 (`SplineKnot`:
   `A` = a knot) and 2.4.2.7 (`NURBSTo`: `C` = first knot with `D` = its
   weight, `A` = the knot of the endpoint with `B` = its weight, `E` = the
   `NURBS()` formula) for the stored `n + 1` knot list.
2. Cite the Visio VBA/automation reference (`Page.DrawNURBS` /
   `Shape.DrawNURBS` parameter remarks) for the end padding: those
   remarks state that the knot array holds `n + 1` knots for `n` control
   points and that the application itself supplies the missing repeated
   knots at the tail of the conventional `n + degree + 1` sequence, with a
   worked example:
   the degree-2 stored list `0, 0, 0, 2, 5, 8` is read as
   `0, 0, 0, 2, 5, 8, 8, 8`. That is exactly the rule `curve.rs`
   implements (append `degree` copies of the last knot, run the curve over
   `knots[d] ..= knot_last`).
3. Add a `docs/sources.md` row for the `DrawNURBS` pages (VBA reference,
   CC-BY-4.0) recording this fact.
4. Recast the libvisio mention as corroboration only, or drop it. The
   P3-03 Result is a historical record and can stay as written.
5. The black-box check stays open: no corpus file contains `NURBSTo` or
   `SplineStart`/`SplineKnot` rows, so no black-box check supports or
   contradicts the rule yet.

### F3 (minor) — the periodic-spline reading is unhandled

The `DrawNURBS` remarks define a first knot repeated fewer than
`degree + 1` times as a *periodic* spline. `curve.rs` always reads the
stored list as a clamped curve over `knots[d] ..= knot_last`, so a
periodic input draws as an open curve that need not start at its first
control point. No corpus file exercises it (P3-03 notes the black-box
check is open). Record the divergence next to the knot rule (in the same
docs fix as F2) and revisit when a file or the screenshot pass needs it.

### F4 (blocker for the gate, product crash) — `render_png` panics on non-finite device-space points

`cargo xtask gate` is red: the `coverage` step failed on
`crates/render/tests/scene_properties.rs::bounded_random_models_build_scenes_without_panicking`
(the property test built exactly to catch this), and every rerun of
`cargo test --workspace --all-features` since then fails the same test
(the first gate run's `test` step passed on a lucky draw; the step is
flaky-to-red on `main`, independent of this task's doc-only changes).

- **What panics:** `assertion failed: points[0].is_finite()`,
  `tiny-skia-0.12.0/src/scan/hairline.rs:558` (a `debug_assert` in
  `compute_nocheck_cubic_bounds`), reached through `render_png`
  (`crates/render/src/png.rs`) — tiny-skia's hairline stroker gets cubic
  points that are not finite. In release the assert is compiled out, so
  the same input silently renders garbage; in debug (tests) it crashes.
- **Minimal failing input** (shrunk by proptest, full dump in the test
  output): a page with one shape caching `Width = 1e30` and
  `Angle = -1.0`, whose child caches `LinePattern = "Themed"` and has a
  `MoveTo` geometry row. All model values are finite; the non-finite
  points are derived — extreme-but-finite geometry mapped through the
  scene transform (or a stroked path of effectively zero device width,
  which tiny-skia draws as a hairline) overflows to `inf`/`NaN`.
- **Rule broken:** AGENTS.md section 6 "Never crash" — malformed or
  extreme input must produce errors, never panics. `crates/render/src/png.rs`
  already skips non-finite coordinates (the module docs promise it), so
  this is a gap in that sanitization: device-space (post-transform)
  points and stroked paths are not checked before they reach tiny-skia.
- **Fix sketch** (fix task, out of this role's lane): finiteness-check
  the transformed points (and the transform) before every tiny-skia draw
  call, skipping or erroring on non-finite geometry; apply the same check
  in `crates/render/src/svg.rs` for consistency; add the shrunk input
  above as a regression test; re-pin a proptest regression seed
  (`proptest-regressions`) so the case cannot silently pass again. Note
  that `ferrogram-text` has the related known hardening gap in
  `text::layout` (P3-11's open item: non-finite glyph positions for
  `line_spacing: f64::MAX`).
- **Blast radius:** `crates/render` (L3). Parallel audits P3-12 and
  P3-13 run the same gate and will hit the same red step.

## Master item: is the knot-vector rule libvisio-derived?

**It is an independently supported fact.** The two halves of the rule are
each documented by an allowed Microsoft source:

- The stored layout (`n + 1` knots for `n` control points, and which row
  cell holds which knot) is [MS-VSDX] 2.4.2.7, 2.4.2.14 and 2.4.2.15
  (quoted in substance under F2 step 1).
- The end padding (`degree` copies of the last knot) is the VBA
  automation reference's `DrawNURBS` remark, including the worked example
  above (F2 step 2) that pins the reading numerically.

libvisio reaches the same rule by a different route (its collector pads
the knot vector with copies of the last knot until it has one knot per
control point plus `degree + 1`); it is corroboration and a comparison
oracle, not the source. The de Boor evaluation is a textbook algorithm,
as the P3-03 Result says. Conclusion: **nothing must move to
`crates/vsd-binary`, and no code changes**; only the citations change
(F2). The one open semantic gap is the periodic case (F3).
