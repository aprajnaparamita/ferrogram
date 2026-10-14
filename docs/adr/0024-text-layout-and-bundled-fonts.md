# 0024: Text layout and bundled fonts

- Status: accepted
- Date: 2026-10-14
- Task: P3-01

## Context

Phase 3 renders "text blocks with basic formatting" (KICKOFF). Text layout
lives in `ferrogram-text` (L1) and **cannot see `model`** (ADR 0002): the
renderer passes it plain runs, formatting and a box. The crate must shape
text, break lines, align and indent paragraphs, and hand glyph outlines to
the backends. It needs fonts, and AGENTS.md section 4 fixes them: Carlito
for Calibri, Selawik for Segoe UI, with their OFL licences — Microsoft's
own fonts are never committed.

## Decision

### Crates

Pure Rust, permissive, wasm-compatible (checked with `cargo deny check` and
the wasm build, ADR 0025):

| Crate | Role | License |
|---|---|---|
| `skrifa` | font parsing, metrics, glyph outlines | Apache-2.0 OR MIT |
| `harfrust` | OpenType shaping (kerning, features) | Apache-2.0 OR MIT |
| `unicode-linebreak` | UAX #14 line-break opportunities | Unicode-3.0 / Apache-2.0 |

We write the layout ourselves (wrapping, alignment, spacing, bullets, tabs,
vertical alignment, block margins and rotation are Visio semantics, not UAX
semantics), so `cosmic-text`, `parley` and `swash` (full layout engines)
are not needed: they would bring their own layout model that we would fight
when matching Visio. Revisit if shaping needs exceed `harfrust`.

The obvious first picks were `ttf-parser` and `rustybuzz`; `cargo deny`
rejects both as unmaintained (RUSTSEC-2026-0192, RUSTSEC-2026-0206) and
names `skrifa` (Google Fonts `fontations`) and `harfrust` (the harfbuzz
organisation's Rust port) as the maintained alternatives. We take the
maintained pair.

### API (`ferrogram_text`)

Input is plain runs and formatting; output is positioned glyphs:

```rust
pub struct Format {
    pub family: String, pub size: f64,      // size in points
    pub bold: bool, pub italic: bool,
    pub underline: bool, pub strikethrough: bool,
}
pub struct Run<'a> { pub text: &'a str, pub format: Format }
pub struct Paragraph<'a> {
    pub runs: Vec<Run<'a>>,
    pub align: Align,                        // Left, Center, Right, Justify
    pub line_spacing: f64,                   // multiples of single spacing
    pub space_before: f64, pub space_after: f64,   // points
    pub indent_first: f64, pub indent_left: f64,   // inches
    pub bullet: Option<Bullet>,
    pub tabs: Vec<f64>,                      // inches
}
pub struct Block<'a> {
    pub paragraphs: Vec<Paragraph<'a>>,
    pub width: f64, pub height: f64,         // box, inches
    pub valign: VAlign,                      // Top, Middle, Bottom
    pub margins: Insets,                     // inches
    pub rotation: Angle,
    pub wrap: bool,
}
pub fn layout(block: &Block<'_>, fonts: &FontStack) -> Result<Layout, Error>;

pub struct GlyphPos { pub glyph: GlyphId, pub font: usize, pub x: f64, pub y: f64,
                      pub run: usize }
pub struct Line { pub glyphs: Vec<GlyphPos>, pub baseline: f64, pub width: f64,
                  pub paragraph: usize }
pub struct Layout {
    pub lines: Vec<Line>, pub width: f64, pub height: f64,
    pub overflow: bool, pub warnings: Vec<Warning>,
}
```

Coordinates are inches in the block's local space (origin top-left, y down,
matching text layout convention); the scene builder maps them through the
shape transform and the text rotation. `Font::outline(glyph) ->
ferrogram_geom::Path` returns the glyph outline in **em units** (size 1),
so backends scale by `size / 72` inches per point.

Line breaking: UAX #14 opportunities from `unicode-linebreak`, soft wrap
when `wrap` is true; a paragraph hard-break is `\n` in the run text.
Bullets render as their own glyph run at `indent_first`; tab stops advance
to the next `tabs` entry, then to a multiple of 0.5 inches (our own
extension rule for overflow tabs).

### Fonts

- **Bundled**: Carlito Regular/Bold/Italic/BoldItalic (metric-compatible
  with Calibri) and Selawik Regular/Bold/Italic/BoldItalic (metric-
  compatible with Segoe UI), committed under `crates/text/fonts/` with
  their OFL licence files, `ATTRIBUTION.md` rows and `docs/sources.md`
  rows (P3-04 adds all of them in the same commit).
- **Fallback**: the run's `family` (from its `Character` row `Font` cell)
  is matched by family name against the `FontStack`; on no match, fall back
  to Carlito, then to the first font in the stack, with a
  `Warning::MissingFont { family }` on the layout. A glyph missing from a
  font falls back to the next font in the stack, then to `.notdef` with
  `Warning::MissingGlyph`.
- Theme fonts (ADR 0019) are used by the scene builder when a run's `Font`
  cell is empty: "major latin" resolves to the theme's major latin family
  name, passed to `layout` as the run family.

Missing fonts never fail a layout: `layout` returns `Ok` with warnings
(ADR 0021 policy, warnings surfaced on `Layout`).

### Out of scope for the alpha

Right-to-left and bidi text, complex scripts beyond what harfrust shapes,
justification beyond word spacing, and text-on-a-path. Line spacing exact
to Visio's "at least" and "exactly" modes: the alpha uses multiples of the
font's default line height.

## Consequences

- The backends get positioned glyphs plus outlines; no text stack in
  `render` beyond layout calls.
- Fonts are assets with provenance rows (AGENTS.md section 4), reviewable
  in the repo.
- Two OFL fonts add ~4 MB to the repo; acceptable at this size, and they
  replace the Microsoft fonts we cannot ship.

## Alternatives considered

- **`cosmic-text`/`parley`.** Rejected: complete layout engines with their
  own models; Visio's paragraph semantics (bullets, tabs, block rotation)
  would be reimplemented anyway. Bigger dependency trees.
- **`swash` + own shaping.** Rejected: swash rasterizes but does not shape;
  harfrust is the maintained harfbuzz-family shaper.
- **`ab_glyph`/`fontdue` for outlines.** Rejected: `skrifa` already
  gives outlines with no rasterizer, and we need paths, not pixels.
- **Fetch fonts at build time.** Rejected: the repo must build offline and
  the fonts are small; committing them with licences is the AGENTS.md
  rule.
- **Substitute a system font at runtime (fontconfig/DirectWrite).**
  Rejected: platform code, non-deterministic goldens, and wasm has none.

## Amendment (P3-04, 2026-10-14): bundled faces, style fallback, rotation

Recorded when the fonts and the layout landed. The font facts below come
from the committed files themselves (sources 9 and 10 in
`docs/sources.md`).

### Bundled faces

Carlito ships Regular, Bold, Italic and BoldItalic as planned. The
Selawik release 1.01 ships Light, Semilight, Regular, Semibold and Bold
and **no italics**; those five faces are bundled (the release's `.ttf`
files, unmodified). `Format` has only a bold flag, so Light, Semilight
and Semibold are picked only by weight distance when nothing closer
exists.

### Family aliases

A face answers to its own family name and to substitute names
(`Font::with_alias`): Carlito answers to `Calibri`, Selawik to
`Segoe UI`. `default_fonts()` registers them, so a Visio run's `Font`
cell matches without warnings. Family names compare case-insensitively
and ignore ASCII spaces and hyphens.

### Style fallback (the italic rule)

`FontStack::resolve_styled(family, bold, italic)` picks, in order:

1. a face of the family with the requested style, closest in weight
   (400 or 700);
2. a face of the requested style from any family, closest in weight —
   this is where an italic `Segoe UI` run draws in **Carlito Italic**;
3. a face of the family with any style, closest in weight;
4. the first face in the stack.

Steps 2–4 warn `Warning::MissingFont { family }`. We chose Carlito
Italic over a synthetic slant of Selawik upright: a slanted outline
needs a per-glyph flag or an outline hook, and the fixed `GlyphPos` has
no room for one. If Phase 4 needs metric-exact italic Segoe UI, extend
the output API first.

### Warnings

`Warning::MissingGlyph` gains a `char: char` field. The fixed
`glyph: GlyphId` cannot name a character that has no glyph in any
bundled face (a missing glyph has no id); `glyph` records the `.notdef`
glyph used instead. Both warnings are deduplicated on the `Layout`.
Missing fonts and glyphs never fail a layout (unchanged).

### Rotation

`layout` **applies `Block::rotation`** to the glyph positions: a rigid
rotation about the box centre in the y-down layout space (a positive
angle turns the x axis toward positive y). The scene builder must not
rotate positions again; it rotates each glyph outline by the same angle
about the glyph origin. Outlines come out of `Font::outline` in font
orientation (y up), em units. `Line::baseline`, `Line::width`,
`Layout::width` and `Layout::height` stay in the unrotated text space.
This replaces the "scene builder maps them through the shape transform
and the text rotation" sentence above.

### Placement rules made precise

- **Bullets** render as their own glyph run at `indent_left +
  bullet.indent` (from the text-area left edge), on the paragraph's
  first line; those glyphs carry `BULLET_RUN` (`usize::MAX`) as their
  run index. Paragraph text starts at `indent_first` as usual. The
  earlier "at `indent_first`" wording put the bullet on top of the text
  for the common values (`indent_first` 0.25, `bullet.indent` 0.1).
- **Tab stops** are measured from the text-area left edge (the pen
  position includes the indents); overflow advances to the next
  multiple of 0.5 inches.
- **Trailing spaces** at a line end are not rendered and not counted in
  the line width.
- **Empty lines** take the paragraph's first run size and the line
  metrics of the first resolved face (0.8/0.2 em when no font is
  available at all).
- **Invalid input** (non-finite or negative box dimensions, sizes,
  spacing or margins) returns `Error::InvalidBlock`; indents and tab
  stops may be negative.
