//! Text block layout: wrapping, alignment, spacing, bullets and tabs
//! (ADR 0024).
//!
//! The input is plain runs with formatting and a box; the output is
//! positioned glyphs in inches in the block's local space (origin
//! top-left, y down). Coordinates are canonical units (ADR 0011); sizes
//! are points (1/72 inch). The renderer maps the output through the shape
//! transform (ADR 0020). Missing fonts and glyphs are warnings on the
//! [`Layout`], never errors (ADR 0021).
//!
//! # Layout rules
//!
//! - Wrapping is greedy at UAX #14 opportunities (`unicode-linebreak`)
//!   when [`Block::wrap`] is true; `\n` in a run hard-breaks the line. A
//!   word wider than the box overflows its line instead of breaking
//!   mid-word.
//! - Trailing spaces at a line end are not rendered and not counted in
//!   the line width.
//! - [`Align::Left`], [`Align::Center`] and [`Align::Right`] shift the
//!   line in the space right of its indent; [`Align::Justify`] stretches
//!   the spaces of wrapped lines (never a paragraph's last line) to fill
//!   that space.
//! - Line pitch is [`Paragraph::line_spacing`] times the line's single
//!   height (ascent + descent + line gap, the largest over its glyphs).
//! - [`Paragraph::space_before`] and [`Paragraph::space_after`] (points)
//!   always apply. Indents are inches from the text-area left edge
//!   (`margins.left` plus the block content).
//! - A bullet renders as its own glyph run at `indent_left +
//!   bullet.indent`, on the paragraph's first line; those glyphs carry
//!   [`BULLET_RUN`] as their run index. Paragraph text starts at
//!   `indent_first` as usual.
//! - Tab stops are measured from the text-area left edge: the pen
//!   advances to the next stop in [`Paragraph::tabs`], then to the next
//!   multiple of 0.5 inches (ADR 0024).
//! - [`Block::rotation`] rotates the glyph positions about the box
//!   centre. Glyph outlines stay in font orientation; the backend rotates
//!   each outline by the same angle about its glyph origin (ADR 0024
//!   amendment).

use std::ops::Range;

use ferrogram_geom::Angle;
use harfrust::{Buffer, ShapeOptions, ShaperFont, shape};
use unicode_linebreak::{BreakOpportunity, linebreaks};

use crate::error::Error;
use crate::font::{Font, FontStack, GlyphId};

/// The run index of bullet glyphs ([`Bullet`] has no input run).
pub const BULLET_RUN: usize = usize::MAX;

/// Font size (points) for empty paragraphs and sizeless bullets.
const DEFAULT_SIZE_PT: f64 = 12.0;

/// The overflow tab step, in inches (ADR 0024).
const TAB_OVERFLOW: f64 = 0.5;

/// Comparison tolerance for widths and heights, in inches.
const EPS: f64 = 1e-9;

/// Default line metrics (em units) when no font is available at all.
const FALLBACK_ASCENT_EM: f64 = 0.8;
const FALLBACK_DESCENT_EM: f64 = 0.2;
const FALLBACK_GAP_EM: f64 = 0.0;

/// Face index for runs whose family resolves to no font at all: the
/// text structure is laid out, but no glyph is emitted.
const NO_FACE: usize = usize::MAX;

/// Horizontal alignment of a paragraph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Align {
    /// Flush left.
    #[default]
    Left,
    /// Centred.
    Center,
    /// Flush right.
    Right,
    /// Justified (the alpha stretches word spacing).
    Justify,
}

/// Vertical alignment of the block in its box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum VAlign {
    /// At the top.
    #[default]
    Top,
    /// Centred.
    Middle,
    /// At the bottom.
    Bottom,
}

/// The margins of a text block, in inches.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Insets {
    /// Left margin.
    pub left: f64,
    /// Right margin.
    pub right: f64,
    /// Top margin.
    pub top: f64,
    /// Bottom margin.
    pub bottom: f64,
}

/// A paragraph bullet (ADR 0024).
#[derive(Debug, Clone, PartialEq)]
pub struct Bullet {
    /// The bullet text, e.g. `"•"`.
    pub text: String,
    /// The font family for the bullet text.
    pub family: String,
    /// The bullet size, in points.
    pub size: f64,
    /// The extra indent of the bullet, in inches.
    pub indent: f64,
}

/// The formatting of one run (from the `Character` rows).
#[derive(Debug, Clone, PartialEq)]
pub struct Format {
    /// The font family name.
    pub family: String,
    /// The font size, in points.
    pub size: f64,
    /// Bold.
    pub bold: bool,
    /// Italic.
    pub italic: bool,
    /// Underlined (the renderer draws it; layout does not).
    pub underline: bool,
    /// Struck through (the renderer draws it; layout does not).
    pub strikethrough: bool,
}

/// One run of text with uniform formatting.
#[derive(Debug, Clone, PartialEq)]
pub struct Run<'a> {
    /// The text; `\n` breaks the line.
    pub text: &'a str,
    /// The run's formatting.
    pub format: Format,
}

/// One paragraph (from one `Paragraph` row's run).
#[derive(Debug, Clone, PartialEq)]
pub struct Paragraph<'a> {
    /// The runs, in order.
    pub runs: Vec<Run<'a>>,
    /// Horizontal alignment.
    pub align: Align,
    /// Line spacing as a multiple of single spacing.
    pub line_spacing: f64,
    /// Space before the paragraph, in points.
    pub space_before: f64,
    /// Space after the paragraph, in points.
    pub space_after: f64,
    /// First-line indent, in inches.
    pub indent_first: f64,
    /// Left indent, in inches.
    pub indent_left: f64,
    /// The bullet, when the paragraph has one.
    pub bullet: Option<Bullet>,
    /// Tab stops from the `Tabs` row, in inches.
    pub tabs: Vec<f64>,
}

/// The text block input (one shape's text).
#[derive(Debug, Clone, PartialEq)]
pub struct Block<'a> {
    /// The paragraphs, in order.
    pub paragraphs: Vec<Paragraph<'a>>,
    /// The box width, in inches.
    pub width: f64,
    /// The box height, in inches.
    pub height: f64,
    /// Vertical alignment in the box.
    pub valign: VAlign,
    /// The text block margins.
    pub margins: Insets,
    /// The text block rotation.
    pub rotation: Angle,
    /// Whether text wraps at the box width.
    pub wrap: bool,
}

/// One positioned glyph of the output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlyphPos {
    /// The glyph in the font.
    pub glyph: GlyphId,
    /// The index into the caller's font list for this run.
    pub font: usize,
    /// The x offset from the block origin, in inches.
    pub x: f64,
    /// The y offset from the block origin, in inches.
    pub y: f64,
    /// The index of the input run this glyph belongs to ([`BULLET_RUN`]
    /// for bullet glyphs).
    pub run: usize,
    /// The font size of this glyph, in points (the bullet size for bullet
    /// glyphs).
    ///
    /// The outline from `Font::outline` is in em units (size 1), so
    /// backends scale it by `size / 72` inches per point (ADR 0024).
    pub size: f64,
}

/// One laid-out line.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    /// The glyphs, in order.
    pub glyphs: Vec<GlyphPos>,
    /// The baseline y offset from the block origin, in inches, before
    /// block rotation.
    pub baseline: f64,
    /// The line width, in inches (advance width, without trailing
    /// spaces).
    pub width: f64,
    /// The index of the input paragraph this line belongs to.
    pub paragraph: usize,
}

/// A layout warning (ADR 0021).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Warning {
    /// No font in the stack has the family and style; a fallback was
    /// used.
    MissingFont {
        /// The requested family.
        family: String,
    },
    /// No font in the stack has the glyph; `.notdef` was used.
    MissingGlyph {
        /// The family that was used.
        font: String,
        /// The `.notdef` glyph that was used instead.
        glyph: GlyphId,
        /// The character no font could draw.
        char: char,
    },
}

/// The layout of one text block (ADR 0024).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Layout {
    /// The lines, in order.
    pub lines: Vec<Line>,
    /// The laid-out width, in inches: the widest line extent measured
    /// from the text-area left edge (`margins.left` in), before block
    /// rotation.
    pub width: f64,
    /// The laid-out height, in inches: the content height (paragraph
    /// spacing included), before block rotation.
    pub height: f64,
    /// Whether content overflowed the box.
    pub overflow: bool,
    /// The block rotation that was baked into the glyph positions.
    ///
    /// The positions are already rotated; backends rotate each glyph
    /// outline by this angle about its glyph origin (ADR 0024 amendment).
    pub rotation: Angle,
    /// The warnings recorded while laying out (ADR 0021).
    pub warnings: Vec<Warning>,
}

/// One shaped glyph before placement.
#[derive(Debug, Clone)]
struct Sg {
    glyph: GlyphId,
    font: usize,
    run: usize,
    /// Advance width, in inches.
    advance: f64,
    /// Shaping offsets, in inches.
    x_off: f64,
    y_off: f64,
    /// Whether the glyph is a space (justification handle).
    is_space: bool,
    /// Line metrics of the face at this size, in inches.
    ascent: f64,
    descent: f64,
    gap: f64,
    /// Font size of this glyph, in points.
    size: f64,
    /// Placement: x from the block origin, in inches, before rotation.
    x: f64,
}

/// One fill unit: a shaped piece of one run and face, or a tab.
enum Item {
    Piece(Vec<Sg>),
    Tab,
}

/// One fill unit with its break flags.
struct LineItem {
    item: Item,
    /// A soft wrap is allowed after this item.
    break_after: bool,
    /// The line ends after this item (`\n` or a UAX #14 mandatory
    /// break).
    mandatory_after: bool,
}

/// A finished line before vertical placement.
struct Draft {
    glyphs: Vec<Sg>,
    /// Advance width from the line start, in inches.
    width: f64,
    /// Line extent from the text-area left edge, in inches.
    extent: f64,
    ascent: f64,
    descent: f64,
    gap: f64,
    paragraph: usize,
}

impl Draft {
    fn single_height(&self) -> f64 {
        self.ascent + self.descent + self.gap
    }
}

/// Layout state: the font stack and the collected warnings.
struct Ctx<'a> {
    fonts: &'a FontStack,
    warnings: Vec<Warning>,
}

impl<'a> Ctx<'a> {
    fn new(fonts: &'a FontStack) -> Self {
        Self {
            fonts,
            warnings: Vec::new(),
        }
    }

    fn warn(&mut self, warning: Warning) {
        if !self.warnings.contains(&warning) {
            self.warnings.push(warning);
        }
    }

    /// The face for a run, with fallback warnings (ADR 0024).
    fn resolve_run(&mut self, format: &Format) -> Option<(usize, &'a Font)> {
        let Some((idx, font)) =
            self.fonts
                .resolve_styled(&format.family, format.bold, format.italic)
        else {
            self.warn(Warning::MissingFont {
                family: format.family.clone(),
            });
            return None;
        };
        let exact = font.matches_family(&format.family)
            && font.italic() == format.italic
            && font.bold() == format.bold;
        if !exact {
            self.warn(Warning::MissingFont {
                family: format.family.clone(),
            });
        }
        Some((idx, font))
    }

    /// The face that draws `ch`, falling back down the stack; `.notdef`
    /// in the base face when no font has the character.
    fn face_for_char(&mut self, base: usize, ch: char) -> usize {
        let fonts = self.fonts.fonts();
        if fonts.get(base).is_some_and(|f| f.has_char(ch)) {
            return base;
        }
        if let Some((idx, _)) = fonts
            .iter()
            .enumerate()
            .find(|(i, f)| *i != base && f.has_char(ch))
        {
            return idx;
        }
        let font = fonts
            .get(base)
            .map(|f| f.family().to_owned())
            .unwrap_or_default();
        self.warn(Warning::MissingGlyph {
            font,
            glyph: GlyphId(0),
            char: ch,
        });
        base
    }

    /// Shapes one hard segment into fill items.
    ///
    /// Pieces split at UAX #14 break opportunities, run/face changes and
    /// tabs; each piece is shaped as one `harfrust` run.
    fn build_items(&mut self, seg: &[SegChar], items: &mut Vec<LineItem>) {
        let text: String = seg.iter().map(|c| c.ch).collect();
        let breaks: Vec<(usize, BreakOpportunity)> = linebreaks(&text).collect();
        let mut pending: Option<Pending> = None;
        let mut byte = 0usize;
        for sc in seg {
            if let Some(kind) = breaks.iter().find(|(b, _)| *b == byte).map(|(_, k)| *k) {
                if let Some(p) = pending.take() {
                    items.push(p.into_item(self));
                }
                if let Some(last) = items.last_mut() {
                    match kind {
                        BreakOpportunity::Mandatory => last.mandatory_after = true,
                        BreakOpportunity::Allowed => last.break_after = true,
                    }
                }
            }
            if sc.ch == '\t' {
                if let Some(p) = pending.take() {
                    items.push(p.into_item(self));
                }
                items.push(LineItem {
                    item: Item::Tab,
                    break_after: false,
                    mandatory_after: false,
                });
                byte += sc.ch.len_utf8();
                continue;
            }
            let face = if sc.base == NO_FACE {
                NO_FACE
            } else {
                self.face_for_char(sc.base, sc.ch)
            };
            match pending.as_mut() {
                Some(p) if p.run == sc.run && p.face == face && p.size_pt == sc.size_pt => {
                    p.text.push(sc.ch);
                }
                _ => {
                    if let Some(p) = pending.take() {
                        items.push(p.into_item(self));
                    }
                    pending = Some(Pending {
                        text: sc.ch.to_string(),
                        run: sc.run,
                        face,
                        size_pt: sc.size_pt,
                    });
                }
            }
            byte += sc.ch.len_utf8();
        }
        if let Some(p) = pending.take() {
            items.push(p.into_item(self));
        }
    }

    /// Shapes one uniform piece with a known face index.
    fn shape_uniform(&mut self, text: &str, run: usize, size_pt: f64, face: usize) -> Vec<Sg> {
        let Some(font) = self.fonts.fonts().get(face) else {
            return Vec::new();
        };
        self.shape_with(face, font, text, run, size_pt)
    }

    /// Shapes the bullet text with its own format.
    fn shape_bullet(&mut self, text: &str, size_pt: f64, family: &str) -> Vec<Sg> {
        let format = Format {
            family: family.to_owned(),
            size: size_pt,
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
        };
        let Some((face, font)) = self.resolve_run(&format) else {
            return Vec::new();
        };
        self.shape_with(face, font, text, BULLET_RUN, size_pt)
    }

    /// Shapes `text` with one face into glyphs.
    fn shape_with(
        &mut self,
        face: usize,
        font: &Font,
        text: &str,
        run: usize,
        size_pt: f64,
    ) -> Vec<Sg> {
        let scale = size_pt / 72.0 / font.units_per_em();
        let ascent = font.ascent_em() * size_pt / 72.0;
        let descent = font.descent_em() * size_pt / 72.0;
        let gap = font.line_gap_em() * size_pt / 72.0;
        let mut out = Vec::new();
        for (glyph, advance, x_off, y_off, byte) in shape_pairs(font, text) {
            let is_space = text.get(byte..).is_some_and(|t| t.starts_with(' '));
            out.push(Sg {
                glyph,
                font: face,
                run,
                advance: advance * scale,
                x_off: x_off * scale,
                y_off: y_off * scale,
                is_space,
                ascent,
                descent,
                gap,
                size: size_pt,
                x: 0.0,
            });
        }
        out
    }
}

/// One input character of a hard segment, with its run and base face.
struct SegChar {
    ch: char,
    run: usize,
    size_pt: f64,
    base: usize,
}

/// A pending shaped piece while a hard segment is scanned.
struct Pending {
    text: String,
    run: usize,
    face: usize,
    size_pt: f64,
}

impl Pending {
    fn into_item(self, ctx: &mut Ctx<'_>) -> LineItem {
        let glyphs = ctx.shape_uniform(&self.text, self.run, self.size_pt, self.face);
        LineItem {
            item: Item::Piece(glyphs),
            break_after: false,
            mandatory_after: false,
        }
    }
}

/// Lays out `block` with fonts from `fonts` (ADR 0024).
///
/// Missing fonts and glyphs are [`Warning`]s on the result, never errors
/// (ADR 0021). [`Error::InvalidBlock`] reports a non-finite or negative
/// dimension or size in the input.
pub fn layout(block: &Block<'_>, fonts: &FontStack) -> Result<Layout, Error> {
    validate(block)?;
    let mut ctx = Ctx::new(fonts);
    let text_width = (block.width - block.margins.left - block.margins.right).max(0.0);
    let box_height = (block.height - block.margins.top - block.margins.bottom).max(0.0);

    // Line building: items, greedy fill, placement.
    let mut drafts: Vec<Draft> = Vec::new();
    for (pi, paragraph) in block.paragraphs.iter().enumerate() {
        // The default line metrics for empty lines: em units scaled by
        // the paragraph's default size.
        let dsize = paragraph
            .runs
            .first()
            .map(|r| r.format.size)
            .unwrap_or(DEFAULT_SIZE_PT);
        let (a, d, g) = fallback_metrics(paragraph, fonts);
        let default_metrics = (a * dsize / 72.0, d * dsize / 72.0, g * dsize / 72.0);
        let items = items_of(paragraph, &mut ctx);
        let x_first = paragraph.indent_left + paragraph.indent_first;
        let x_next = paragraph.indent_left;
        let fills = fill(
            &items,
            block.wrap,
            x_first,
            x_next,
            text_width,
            &paragraph.tabs,
        );
        for (line_in_para, (range, wrapped)) in fills.into_iter().enumerate() {
            let x_start = x_start_of(line_in_para, x_first, x_next);
            let empty: Vec<LineItem> = Vec::new();
            let slice = items.get(range).unwrap_or(&empty);
            let spec = LineSpec {
                x_start,
                tabs: &paragraph.tabs,
                align: paragraph.align,
                text_width,
                paragraph: pi,
                wrapped,
                default_metrics,
            };
            drafts.push(finalize(slice, &spec));
        }
        if let (Some(bullet), Some(first)) = (
            &paragraph.bullet,
            drafts.iter_mut().find(|d| d.paragraph == pi),
        ) {
            let size = if bullet.size > 0.0 {
                bullet.size
            } else {
                DEFAULT_SIZE_PT
            };
            let mut glyphs = ctx.shape_bullet(&bullet.text, size, &bullet.family);
            let bx = paragraph.indent_left + bullet.indent;
            for g in &mut glyphs {
                g.x = bx + g.x_off;
                first.ascent = first.ascent.max(g.ascent);
                first.descent = first.descent.max(g.descent);
                first.gap = first.gap.max(g.gap);
            }
            first.glyphs.splice(0..0, glyphs);
        }
    }

    // Vertical placement: paragraph spacing, line pitch, vertical align.
    let content_height = drafts
        .iter()
        .fold((0.0_f64, None::<usize>), |(y, last), draft| {
            let mut y = y;
            let mut last = last;
            if last != Some(draft.paragraph) {
                if let Some(lp) = last {
                    y += space_of(block, lp, |p| p.space_after) / 72.0;
                }
                y += space_of(block, draft.paragraph, |p| p.space_before) / 72.0;
                last = Some(draft.paragraph);
            }
            y += draft.single_height() * line_spacing_of(block, draft.paragraph);
            (y, last)
        })
        .0;
    // The trailing space-after of the last paragraph.
    let content_height = content_height
        + drafts
            .last()
            .map(|d| space_of(block, d.paragraph, |p| p.space_after) / 72.0)
            .unwrap_or(0.0);

    let shift_v = match block.valign {
        VAlign::Top => 0.0,
        VAlign::Middle => ((box_height - content_height) / 2.0).max(0.0),
        VAlign::Bottom => (box_height - content_height).max(0.0),
    };
    let top0 = block.margins.top + shift_v;

    let mut lines = Vec::new();
    let mut y = top0;
    let mut last_para: Option<usize> = None;
    let mut width_max = 0.0_f64;
    let mut overflow = content_height > box_height + EPS;
    for draft in &drafts {
        if last_para != Some(draft.paragraph) {
            if let Some(lp) = last_para {
                y += space_of(block, lp, |p| p.space_after) / 72.0;
            }
            y += space_of(block, draft.paragraph, |p| p.space_before) / 72.0;
            last_para = Some(draft.paragraph);
        }
        let baseline = y + draft.ascent;
        y += draft.single_height() * line_spacing_of(block, draft.paragraph);
        width_max = width_max.max(draft.extent);
        if draft.extent > text_width + EPS {
            overflow = true;
        }
        lines.push(Line {
            glyphs: draft
                .glyphs
                .iter()
                .map(|g| GlyphPos {
                    glyph: g.glyph,
                    font: g.font,
                    x: block.margins.left + g.x,
                    y: baseline - g.y_off,
                    run: g.run,
                    size: g.size,
                })
                .collect(),
            baseline,
            width: draft.width,
            paragraph: draft.paragraph,
        });
    }

    // Block rotation: rigid rotation of glyph positions about the box
    // centre (ADR 0024 amendment).
    if block.rotation.radians() != 0.0 {
        let (cx, cy) = (block.width / 2.0, block.height / 2.0);
        let (sin, cos) = block.rotation.radians().sin_cos();
        for line in &mut lines {
            for g in &mut line.glyphs {
                let (dx, dy) = (g.x - cx, g.y - cy);
                g.x = cx + dx * cos - dy * sin;
                g.y = cy + dx * sin + dy * cos;
            }
        }
    }

    Ok(Layout {
        lines,
        width: width_max,
        height: content_height,
        overflow,
        rotation: block.rotation,
        warnings: std::mem::take(&mut ctx.warnings),
    })
}

/// Rejects non-finite and negative input (AGENTS.md section 6: errors,
/// never panics).
fn validate(block: &Block<'_>) -> Result<(), Error> {
    let finite = f64::is_finite;
    let ok = finite(block.width)
        && finite(block.height)
        && block.width >= 0.0
        && block.height >= 0.0
        && finite(block.rotation.radians())
        && [
            block.margins.left,
            block.margins.right,
            block.margins.top,
            block.margins.bottom,
        ]
        .iter()
        .all(|v| finite(*v) && *v >= 0.0);
    if !ok {
        return Err(Error::InvalidBlock);
    }
    for paragraph in &block.paragraphs {
        let ok = finite(paragraph.line_spacing)
            && finite(paragraph.space_before)
            && finite(paragraph.space_after)
            && paragraph.line_spacing >= 0.0
            && paragraph.space_before >= 0.0
            && paragraph.space_after >= 0.0
            && finite(paragraph.indent_first)
            && finite(paragraph.indent_left)
            && paragraph.tabs.iter().all(|t| finite(*t));
        if !ok {
            return Err(Error::InvalidBlock);
        }
        for run in &paragraph.runs {
            if !finite(run.format.size) || run.format.size < 0.0 {
                return Err(Error::InvalidBlock);
            }
        }
        if let Some(bullet) = &paragraph.bullet
            && (!finite(bullet.size) || bullet.size < 0.0 || !finite(bullet.indent))
        {
            return Err(Error::InvalidBlock);
        }
    }
    Ok(())
}

/// Splits the paragraph into hard segments and builds their fill items.
/// A run whose face cannot be resolved still contributes its line
/// structure (empty segments and breaks), but no glyphs.
fn items_of(paragraph: &Paragraph<'_>, ctx: &mut Ctx<'_>) -> Vec<LineItem> {
    let mut items: Vec<LineItem> = Vec::new();
    let mut segment: Vec<SegChar> = Vec::new();
    for (ri, run) in paragraph.runs.iter().enumerate() {
        let base = ctx.resolve_run(&run.format).map_or(NO_FACE, |(b, _)| b);
        for (si, part) in run.text.split('\n').enumerate() {
            if si > 0 {
                // A hard break: the pending segment ends the line.
                finish_segment(&mut segment, &mut items, ctx);
            }
            segment.extend(part.chars().map(|ch| SegChar {
                ch,
                run: ri,
                size_pt: run.format.size,
                base,
            }));
        }
    }
    finish_segment(&mut segment, &mut items, ctx);
    items
}

/// Shapes the pending hard segment into items and ends its line. An
/// empty segment still produces one empty line.
fn finish_segment(segment: &mut Vec<SegChar>, items: &mut Vec<LineItem>, ctx: &mut Ctx<'_>) {
    let before = items.len();
    let seg = std::mem::take(segment);
    ctx.build_items(&seg, items);
    if items.len() == before {
        items.push(LineItem {
            item: Item::Piece(Vec::new()),
            break_after: false,
            mandatory_after: true,
        });
    } else if let Some(last) = items.last_mut() {
        last.mandatory_after = true;
    }
}

/// The paragraph's default line metrics (em units), for empty lines.
fn fallback_metrics(paragraph: &Paragraph<'_>, fonts: &FontStack) -> (f64, f64, f64) {
    for run in &paragraph.runs {
        if let Some(font) = fonts.resolve(&run.format.family) {
            return (font.ascent_em(), font.descent_em(), font.line_gap_em());
        }
    }
    match fonts.fonts().first() {
        Some(font) => (font.ascent_em(), font.descent_em(), font.line_gap_em()),
        None => (FALLBACK_ASCENT_EM, FALLBACK_DESCENT_EM, FALLBACK_GAP_EM),
    }
}

fn space_of(block: &Block<'_>, pi: usize, pick: fn(&Paragraph<'_>) -> f64) -> f64 {
    block.paragraphs.get(pi).map(pick).unwrap_or(0.0)
}

fn line_spacing_of(block: &Block<'_>, pi: usize) -> f64 {
    block
        .paragraphs
        .get(pi)
        .map(|p| p.line_spacing)
        .unwrap_or(1.0)
}

fn x_start_of(line_in_para: usize, x_first: f64, x_next: f64) -> f64 {
    if line_in_para == 0 { x_first } else { x_next }
}

/// Greedy line fill. Returns the item ranges of the lines and whether
/// each ended at a soft wrap.
fn fill(
    items: &[LineItem],
    wrap: bool,
    x_first: f64,
    x_next: f64,
    text_width: f64,
    tabs: &[f64],
) -> Vec<(Range<usize>, bool)> {
    let mut lines = Vec::new();
    let mut start = 0usize;
    let mut count = 0usize;
    let mut width = 0.0;
    let mut last_allowed: Option<usize> = None;
    let mut i = 0usize;
    while i < items.len() {
        let x_start = x_start_of(lines.len(), x_first, x_next);
        let item_width = advance_of(&items[i], x_start + width, tabs);
        let over = x_start + width + item_width > text_width + EPS;
        if over
            && count > 0
            && wrap
            && let Some(k) = last_allowed
        {
            lines.push((start..start + k, true));
            i = start + k;
            start = i;
            count = 0;
            width = 0.0;
            last_allowed = None;
            continue;
        }
        width += item_width;
        count += 1;
        if items[i].mandatory_after {
            i += 1;
            lines.push((start..start + count, false));
            start = i;
            count = 0;
            width = 0.0;
            last_allowed = None;
            continue;
        }
        if items[i].break_after {
            last_allowed = Some(count);
        }
        i += 1;
    }
    if count > 0 {
        lines.push((start..start + count, false));
    }
    lines
}

fn advance_of(item: &LineItem, pen: f64, tabs: &[f64]) -> f64 {
    match &item.item {
        Item::Piece(glyphs) => glyphs.iter().map(|g| g.advance).sum(),
        Item::Tab => tab_advance(pen, tabs),
    }
}

/// The pen advance of a tab from `x` to the next stop (ADR 0024): the
/// next stop in `tabs`, else the next multiple of 0.5 inches.
fn tab_advance(x: f64, tabs: &[f64]) -> f64 {
    let next = tabs
        .iter()
        .copied()
        .filter(|t| *t > x + EPS)
        .min_by(f64::total_cmp)
        .unwrap_or_else(|| {
            let steps = (x / TAB_OVERFLOW).floor() + 1.0;
            steps.max(0.0) * TAB_OVERFLOW
        });
    (next - x).max(0.0)
}

/// One line's placement parameters.
struct LineSpec<'a> {
    /// Line start from the text-area left edge, in inches.
    x_start: f64,
    /// The paragraph's tab stops, in inches.
    tabs: &'a [f64],
    align: Align,
    /// The text-area width, in inches.
    text_width: f64,
    paragraph: usize,
    /// The line ended at a soft wrap.
    wrapped: bool,
    /// Line metrics for empty lines, in inches.
    default_metrics: (f64, f64, f64),
}

/// Turns one filled line into a [`Draft`]: trailing-space trim, x
/// placement, justification and metrics.
fn finalize(slice: &[LineItem], spec: &LineSpec<'_>) -> Draft {
    let LineSpec {
        x_start,
        tabs,
        align,
        text_width,
        paragraph: pi,
        wrapped,
        default_metrics,
    } = *spec;
    // Trailing spaces are not rendered and not counted (a trailing tab
    // stops the trim).
    let mut keep: Vec<usize> = slice
        .iter()
        .map(|it| match &it.item {
            Item::Piece(p) => p.len(),
            Item::Tab => 0,
        })
        .collect();
    let mut dropping = true;
    for (idx, item) in slice.iter().enumerate().rev() {
        if !dropping {
            break;
        }
        match &item.item {
            Item::Tab => dropping = false,
            Item::Piece(p) => {
                let mut k = p.len();
                while k > 0 && p[k - 1].is_space {
                    k -= 1;
                }
                keep[idx] = k;
                if k > 0 {
                    dropping = false;
                }
            }
        }
    }

    // First pass: width and space count.
    let mut width = 0.0_f64;
    let mut spaces = 0usize;
    for (idx, item) in slice.iter().enumerate() {
        match &item.item {
            Item::Piece(p) => {
                for g in p.iter().take(keep.get(idx).copied().unwrap_or(0)) {
                    width += g.advance;
                    if g.is_space {
                        spaces += 1;
                    }
                }
            }
            Item::Tab => width += tab_advance(x_start + width, tabs),
        }
    }
    let stretch = if align == Align::Justify && wrapped && spaces > 0 {
        ((text_width - x_start - width) / spaces as f64).max(0.0)
    } else {
        0.0
    };
    // Center and Right shift the line in the space right of its indent;
    // Justify fills that space with the stretch.
    let shift = match align {
        Align::Left | Align::Justify => 0.0,
        Align::Center => ((text_width - x_start - width) / 2.0).max(0.0),
        Align::Right => (text_width - x_start - width).max(0.0),
    };

    // Second pass: x placement with alignment and justification. The
    // width is the sum of the advances, not `pen - start`: subtracting
    // two nearly equal positions cancels and can go negative for a
    // zero-advance glyph (task P3-11 regression).
    let mut glyphs: Vec<Sg> = Vec::new();
    let mut pen = x_start + shift;
    let mut width = 0.0_f64;
    for (idx, item) in slice.iter().enumerate() {
        match &item.item {
            Item::Piece(p) => {
                for g in p.iter().take(keep.get(idx).copied().unwrap_or(0)) {
                    let mut g = g.clone();
                    g.x = pen + g.x_off;
                    let mut advance = g.advance;
                    if g.is_space {
                        advance += stretch;
                    }
                    pen += advance;
                    width += advance;
                    glyphs.push(g);
                }
            }
            Item::Tab => {
                let advance = tab_advance(pen, tabs);
                pen += advance;
                width += advance;
            }
        }
    }

    let (mut ascent, mut descent, mut gap) = default_metrics;
    if !glyphs.is_empty() {
        ascent = glyphs.iter().map(|g| g.ascent).fold(0.0, f64::max);
        descent = glyphs.iter().map(|g| g.descent).fold(0.0, f64::max);
        gap = glyphs.iter().map(|g| g.gap).fold(0.0, f64::max);
    }

    Draft {
        glyphs,
        width,
        extent: pen,
        ascent,
        descent,
        gap,
        paragraph: pi,
    }
}

/// Shapes `text`, returning `(glyph, advance, x_offset, y_offset,
/// cluster_byte)` in font units. Nominal `skrifa` advances when
/// `harfrust` cannot shape.
fn shape_pairs(font: &Font, text: &str) -> Vec<(GlyphId, f64, f64, f64, usize)> {
    let Some(hf) = harfrust::Font::new(font.blob(), 0) else {
        return nominal(font, text);
    };
    let shaper = ShaperFont::new(&hf);
    let mut buffer = Buffer::new();
    buffer.push_str(text);
    buffer.guess_segment_properties();
    if shape(&shaper, &mut buffer, ShapeOptions::new()).is_err() {
        return nominal(font, text);
    }
    buffer
        .glyph_infos()
        .iter()
        .zip(buffer.glyph_positions())
        .map(|(info, pos)| {
            (
                GlyphId(u16::try_from(info.glyph_id).unwrap_or(0)),
                f64::from(pos.x_advance),
                f64::from(pos.x_offset),
                f64::from(pos.y_offset),
                usize::try_from(info.cluster).unwrap_or(0),
            )
        })
        .collect()
}

/// Nominal advances, one glyph per character, when shaping is
/// unavailable.
fn nominal(font: &Font, text: &str) -> Vec<(GlyphId, f64, f64, f64, usize)> {
    text.char_indices()
        .map(|(byte, ch)| {
            let glyph = font.glyph_of(ch).unwrap_or(GlyphId(0));
            let advance = font.advance_em(glyph) * font.units_per_em();
            (glyph, advance, 0.0, 0.0, byte)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enums_have_defaults() {
        assert_eq!(Align::default(), Align::Left);
        assert_eq!(VAlign::default(), VAlign::Top);
    }

    #[test]
    fn block_holds_its_paragraphs() {
        let block = Block {
            paragraphs: vec![Paragraph {
                runs: vec![Run {
                    text: "hello",
                    format: Format {
                        family: "Carlito".to_owned(),
                        size: 12.0,
                        bold: false,
                        italic: false,
                        underline: false,
                        strikethrough: false,
                    },
                }],
                align: Align::Center,
                line_spacing: 1.0,
                space_before: 0.0,
                space_after: 6.0,
                indent_first: 0.25,
                indent_left: 0.0,
                bullet: Some(Bullet {
                    text: "•".to_owned(),
                    family: "Carlito".to_owned(),
                    size: 12.0,
                    indent: 0.1,
                }),
                tabs: vec![0.5, 1.0],
            }],
            width: 2.0,
            height: 1.0,
            valign: VAlign::Middle,
            margins: Insets {
                left: 0.1,
                right: 0.1,
                top: 0.05,
                bottom: 0.05,
            },
            rotation: Angle::from_radians(0.0),
            wrap: true,
        };
        assert_eq!(block.paragraphs.len(), 1);
        assert_eq!(block.paragraphs[0].runs[0].text, "hello");
        assert_eq!(block.paragraphs[0].tabs, vec![0.5, 1.0]);
    }

    #[test]
    fn output_types_hold_their_data() {
        let line = Line {
            glyphs: vec![GlyphPos {
                glyph: GlyphId(7),
                font: 0,
                x: 0.1,
                y: 0.2,
                run: 0,
                size: 12.0,
            }],
            baseline: 0.25,
            width: 0.5,
            paragraph: 1,
        };
        assert_eq!(line.glyphs.len(), 1);
        let layout = Layout {
            lines: vec![line],
            width: 0.5,
            height: 0.3,
            overflow: false,
            rotation: Angle::from_degrees(0.0),
            warnings: vec![Warning::MissingFont {
                family: "Calibri".to_owned(),
            }],
        };
        assert_eq!(layout.warnings.len(), 1);
        assert_eq!(Layout::default().lines.len(), 0);
    }

    #[test]
    fn tab_advance_finds_the_next_stop() {
        assert_eq!(tab_advance(0.0, &[0.5, 1.0]), 0.5);
        assert_eq!(tab_advance(0.6, &[0.5, 1.0]), 0.4);
        assert_eq!(tab_advance(1.5, &[0.5, 1.0]), 0.5);
        assert_eq!(tab_advance(0.25, &[]), 0.25);
        assert_eq!(tab_advance(0.5, &[]), 0.5);
        assert_eq!(tab_advance(-0.1, &[]), 0.1);
    }
}

#[cfg(test)]
mod layout_tests {
    use super::*;
    use crate::font::default_fonts;

    /// Carlito line metrics (per 2048 em) and the 12-point values the
    /// golden numbers below are pinned against.
    const ASC: f64 = 1950.0 / 2048.0;
    const DESC: f64 = 550.0 / 2048.0;
    const BASE12: f64 = ASC * 12.0 / 72.0;
    const SINGLE12: f64 = (ASC + DESC) * 12.0 / 72.0;
    const TOL: f64 = 1e-6;

    // Golden layout numbers for "hi" at 12 pt Carlito (from the bundled
    // face; advances in inches).
    const H_ADV: f64 = 0.087_565_104;
    const HI_WIDTH: f64 = 0.125_813_802;
    // Golden line widths for "one two three four five six" wrapped at
    // 1.2 inches ("one two three" / "four five six").
    const WRAP_W0: f64 = 0.960_205_078;
    const WRAP_W1: f64 = 0.776_692_708;

    fn approx(a: f64, b: f64) {
        assert!((a - b).abs() < TOL, "{a} != {b}");
    }

    fn fmt(family: &str, size: f64) -> Format {
        Format {
            family: family.to_owned(),
            size,
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
        }
    }

    fn para(text: &str) -> Paragraph<'_> {
        para_runs(vec![Run {
            text,
            format: fmt("Carlito", 12.0),
        }])
    }

    fn para_runs(runs: Vec<Run<'_>>) -> Paragraph<'_> {
        Paragraph {
            runs,
            align: Align::Left,
            line_spacing: 1.0,
            space_before: 0.0,
            space_after: 0.0,
            indent_first: 0.0,
            indent_left: 0.0,
            bullet: None,
            tabs: Vec::new(),
        }
    }

    fn boxed(paras: Vec<Paragraph<'_>>) -> Block<'_> {
        boxed_in(paras, 2.0, 1.0)
    }

    fn boxed_in(paras: Vec<Paragraph<'_>>, width: f64, height: f64) -> Block<'_> {
        Block {
            paragraphs: paras,
            width,
            height,
            valign: VAlign::Top,
            margins: Insets::default(),
            rotation: Angle::from_radians(0.0),
            wrap: true,
        }
    }

    #[test]
    fn output_carries_the_glyph_sizes_and_block_rotation() {
        let fonts = default_fonts();
        let rotation = Angle::from_degrees(90.0);
        let mut block = boxed(vec![para_runs(vec![
            Run {
                text: "a",
                format: fmt("Carlito", 10.0),
            },
            Run {
                text: "b",
                format: fmt("Carlito", 20.0),
            },
        ])]);
        block.rotation = rotation;
        let out = layout(&block, &fonts).expect("lays out");
        // The backends need both to draw the glyph outlines (ADR 0024).
        assert_eq!(out.rotation, rotation);
        let sizes: Vec<f64> = out
            .lines
            .iter()
            .flat_map(|line| &line.glyphs)
            .map(|glyph| glyph.size)
            .collect();
        assert_eq!(sizes, vec![10.0, 20.0]);
    }

    #[test]
    fn rejects_invalid_blocks() {
        let fonts = default_fonts();
        let bad = boxed(vec![para("x")]);
        assert!(layout(&bad, &fonts).is_ok());
        let nan = Block {
            width: f64::NAN,
            ..boxed(vec![para("x")])
        };
        assert_eq!(layout(&nan, &fonts), Err(Error::InvalidBlock));
        let neg = Block {
            height: -1.0,
            ..boxed(vec![para("x")])
        };
        assert_eq!(layout(&neg, &fonts), Err(Error::InvalidBlock));
        let neg_margin = Block {
            margins: Insets {
                left: -0.1,
                ..Insets::default()
            },
            ..boxed(vec![para("x")])
        };
        assert_eq!(layout(&neg_margin, &fonts), Err(Error::InvalidBlock));
        let bad_rot = Block {
            rotation: Angle::from_radians(f64::NAN),
            ..boxed(vec![para("x")])
        };
        assert_eq!(layout(&bad_rot, &fonts), Err(Error::InvalidBlock));
        let bad_size = boxed(vec![Paragraph {
            runs: vec![Run {
                text: "x",
                format: fmt("Carlito", -2.0),
            }],
            ..para("x")
        }]);
        assert_eq!(layout(&bad_size, &fonts), Err(Error::InvalidBlock));
        let bad_spacing = boxed(vec![Paragraph {
            line_spacing: f64::INFINITY,
            ..para("x")
        }]);
        assert_eq!(layout(&bad_spacing, &fonts), Err(Error::InvalidBlock));
        let bad_indent = boxed(vec![Paragraph {
            indent_first: f64::NAN,
            ..para("x")
        }]);
        assert_eq!(layout(&bad_indent, &fonts), Err(Error::InvalidBlock));
        let bad_tab = boxed(vec![Paragraph {
            tabs: vec![f64::NAN],
            ..para("x")
        }]);
        assert_eq!(layout(&bad_tab, &fonts), Err(Error::InvalidBlock));
        let bad_bullet = boxed(vec![Paragraph {
            bullet: Some(Bullet {
                text: "*".to_owned(),
                family: "Carlito".to_owned(),
                size: -1.0,
                indent: 0.0,
            }),
            ..para("x")
        }]);
        assert_eq!(layout(&bad_bullet, &fonts), Err(Error::InvalidBlock));
    }

    #[test]
    fn empty_block_has_no_lines() {
        let fonts = default_fonts();
        let l = layout(&boxed(vec![]), &fonts).unwrap();
        assert!(l.lines.is_empty());
        assert_eq!(l.width, 0.0);
        assert_eq!(l.height, 0.0);
        assert!(!l.overflow);
        assert!(l.warnings.is_empty());
    }

    #[test]
    fn single_run_places_glyphs_at_the_golden_numbers() {
        let fonts = default_fonts();
        let l = layout(&boxed(vec![para("hi")]), &fonts).unwrap();
        assert_eq!(l.lines.len(), 1);
        let line = &l.lines[0];
        assert_eq!(line.glyphs.len(), 2);
        assert_eq!(line.glyphs[0].run, 0);
        assert_eq!(line.glyphs[0].font, 0);
        approx(line.glyphs[0].x, 0.0);
        approx(line.glyphs[1].x, H_ADV);
        approx(line.glyphs[0].y, BASE12);
        approx(line.baseline, BASE12);
        approx(line.width, HI_WIDTH);
        approx(l.width, HI_WIDTH);
        approx(l.height, SINGLE12);
        assert!(!l.overflow);
        assert!(l.warnings.is_empty());
    }

    #[test]
    fn hard_breaks_split_lines() {
        let fonts = default_fonts();
        let l = layout(&boxed(vec![para("a\nb")]), &fonts).unwrap();
        assert_eq!(l.lines.len(), 2);
        assert_eq!(l.lines[0].paragraph, 0);
        assert_eq!(l.lines[1].paragraph, 0);
        approx(l.lines[1].baseline - l.lines[0].baseline, SINGLE12);
        approx(l.height, 2.0 * SINGLE12);
    }

    #[test]
    fn trailing_hard_break_adds_an_empty_line() {
        let fonts = default_fonts();
        let l = layout(&boxed(vec![para("a\n")]), &fonts).unwrap();
        assert_eq!(l.lines.len(), 2);
        assert!(l.lines[1].glyphs.is_empty());
        approx(l.lines[1].width, 0.0);
        approx(l.height, 2.0 * SINGLE12);
    }

    #[test]
    fn empty_paragraph_has_one_empty_line() {
        let fonts = default_fonts();
        let l = layout(&boxed(vec![para("")]), &fonts).unwrap();
        assert_eq!(l.lines.len(), 1);
        assert!(l.lines[0].glyphs.is_empty());
        approx(l.lines[0].baseline, BASE12);
        approx(l.height, SINGLE12);
    }

    #[test]
    fn runs_flow_together_without_a_break() {
        let fonts = default_fonts();
        let together = layout(
            &boxed(vec![para_runs(vec![
                Run {
                    text: "hel",
                    format: fmt("Carlito", 12.0),
                },
                Run {
                    text: "lo",
                    format: fmt("Carlito", 12.0),
                },
            ])]),
            &fonts,
        )
        .unwrap();
        let single = layout(&boxed(vec![para("hello")]), &fonts).unwrap();
        assert_eq!(together.lines.len(), 1);
        assert_eq!(together.lines[0].glyphs.len(), single.lines[0].glyphs.len());
        approx(together.lines[0].width, single.lines[0].width);
        assert_eq!(together.lines[0].glyphs[3].run, 1);
    }

    #[test]
    fn wrapping_breaks_at_words() {
        let fonts = default_fonts();
        let l = layout(
            &boxed_in(vec![para("one two three four five six")], 1.2, 2.0),
            &fonts,
        )
        .unwrap();
        assert_eq!(l.lines.len(), 2);
        approx(l.lines[0].width, WRAP_W0);
        approx(l.lines[1].width, WRAP_W1);
        approx(l.lines[1].baseline - l.lines[0].baseline, SINGLE12);
        assert!(!l.overflow);
    }

    #[test]
    fn a_word_wider_than_the_box_overflows_its_line() {
        let fonts = default_fonts();
        let l = layout(
            &boxed_in(vec![para("supercalifragilistic")], 0.5, 2.0),
            &fonts,
        )
        .unwrap();
        assert_eq!(l.lines.len(), 1);
        assert!(l.overflow);
    }

    #[test]
    fn no_wrap_keeps_one_line_and_overflows() {
        let fonts = default_fonts();
        let b = Block {
            wrap: false,
            ..boxed_in(vec![para("one two three four five six")], 1.2, 2.0)
        };
        let l = layout(&b, &fonts).unwrap();
        assert_eq!(l.lines.len(), 1);
        assert!(l.overflow);
    }

    #[test]
    fn center_and_right_shift_the_line() {
        let fonts = default_fonts();
        let center = layout(
            &boxed(vec![Paragraph {
                align: Align::Center,
                ..para("hi")
            }]),
            &fonts,
        )
        .unwrap();
        approx(center.lines[0].glyphs[0].x, (2.0 - HI_WIDTH) / 2.0);
        let right = layout(
            &boxed(vec![Paragraph {
                align: Align::Right,
                ..para("hi")
            }]),
            &fonts,
        )
        .unwrap();
        approx(right.lines[0].glyphs[0].x, 2.0 - HI_WIDTH);
    }

    #[test]
    fn justify_stretches_spaces_but_not_the_last_line() {
        let fonts = default_fonts();
        let l = layout(
            &boxed_in(
                vec![Paragraph {
                    align: Align::Justify,
                    ..para("one two three four five six")
                }],
                1.2,
                2.0,
            ),
            &fonts,
        )
        .unwrap();
        assert_eq!(l.lines.len(), 2);
        approx(l.lines[0].width, 1.2);
        approx(l.lines[1].width, WRAP_W1);
    }

    #[test]
    fn justify_without_spaces_stays_left() {
        let fonts = default_fonts();
        let l = layout(
            &boxed_in(
                vec![Paragraph {
                    align: Align::Justify,
                    ..para("hi")
                }],
                1.2,
                2.0,
            ),
            &fonts,
        )
        .unwrap();
        assert_eq!(l.lines.len(), 1);
        approx(l.lines[0].width, HI_WIDTH);
    }

    #[test]
    fn line_spacing_scales_the_pitch() {
        let fonts = default_fonts();
        let l = layout(
            &boxed(vec![Paragraph {
                line_spacing: 2.0,
                ..para("a\nb")
            }]),
            &fonts,
        )
        .unwrap();
        assert_eq!(l.lines.len(), 2);
        approx(l.lines[1].baseline - l.lines[0].baseline, 2.0 * SINGLE12);
    }

    #[test]
    fn space_before_and_after_shift_the_content() {
        let fonts = default_fonts();
        let l = layout(
            &boxed(vec![
                Paragraph {
                    space_after: 72.0,
                    ..para("a")
                },
                Paragraph {
                    space_before: 144.0,
                    ..para("b")
                },
            ]),
            &fonts,
        )
        .unwrap();
        assert_eq!(l.lines.len(), 2);
        // 72 pt = 1 inch after the first line, 144 pt = 2 inches before
        // the second.
        approx(l.lines[1].baseline - l.lines[0].baseline, SINGLE12 + 3.0);
        approx(l.height, 2.0 * SINGLE12 + 3.0);
    }

    #[test]
    fn indents_apply_to_the_first_and_later_lines() {
        let fonts = default_fonts();
        let l = layout(
            &boxed(vec![Paragraph {
                indent_first: 0.25,
                indent_left: 0.1,
                ..para("a\nb")
            }]),
            &fonts,
        )
        .unwrap();
        approx(l.lines[0].glyphs[0].x, 0.35);
        approx(l.lines[1].glyphs[0].x, 0.1);
    }

    #[test]
    fn bullets_render_as_their_own_run() {
        let fonts = default_fonts();
        let l = layout(
            &boxed(vec![Paragraph {
                bullet: Some(Bullet {
                    text: "*".to_owned(),
                    family: "Carlito".to_owned(),
                    size: 12.0,
                    indent: 0.1,
                }),
                indent_first: 0.25,
                ..para("a")
            }]),
            &fonts,
        )
        .unwrap();
        let line = &l.lines[0];
        assert_eq!(line.glyphs.len(), 2);
        let bullet = &line.glyphs[0];
        assert_eq!(bullet.run, BULLET_RUN);
        approx(bullet.x, 0.1);
        // The text starts at the first-line indent.
        approx(line.glyphs[1].x, 0.25);
    }

    #[test]
    fn tab_stops_advance_to_the_next_stop() {
        let fonts = default_fonts();
        let l = layout(
            &boxed(vec![Paragraph {
                tabs: vec![1.0],
                ..para("a\tb")
            }]),
            &fonts,
        )
        .unwrap();
        let line = &l.lines[0];
        assert_eq!(line.glyphs.len(), 2);
        approx(line.glyphs[1].x, 1.0);
    }

    #[test]
    fn tabs_without_stops_advance_to_half_inches() {
        let fonts = default_fonts();
        let l = layout(&boxed(vec![para("a\tb")]), &fonts).unwrap();
        approx(l.lines[0].glyphs[1].x, 0.5);
    }

    #[test]
    fn valign_middle_and_bottom_shift_the_content() {
        let fonts = default_fonts();
        let middle = layout(
            &boxed_in(vec![Paragraph { ..para("hi") }], 2.0, 1.0),
            &fonts,
        )
        .unwrap();
        approx(middle.lines[0].baseline, BASE12);
        let b = Block {
            valign: VAlign::Middle,
            ..boxed_in(vec![para("hi")], 2.0, 1.0)
        };
        let l = layout(&b, &fonts).unwrap();
        approx(l.lines[0].baseline, (1.0 - SINGLE12) / 2.0 + BASE12);
        let b = Block {
            valign: VAlign::Bottom,
            ..boxed_in(vec![para("hi")], 2.0, 1.0)
        };
        let l = layout(&b, &fonts).unwrap();
        approx(l.lines[0].baseline, 1.0 - SINGLE12 + BASE12);
    }

    #[test]
    fn margins_shift_the_content() {
        let fonts = default_fonts();
        let b = Block {
            margins: Insets {
                left: 0.1,
                right: 0.1,
                top: 0.2,
                bottom: 0.05,
            },
            ..boxed_in(vec![para("hi")], 2.0, 1.0)
        };
        let l = layout(&b, &fonts).unwrap();
        approx(l.lines[0].glyphs[0].x, 0.1);
        approx(l.lines[0].baseline, 0.2 + BASE12);
    }

    #[test]
    fn rotation_moves_glyphs_about_the_box_centre() {
        let fonts = default_fonts();
        let b = Block {
            rotation: Angle::from_degrees(90.0),
            ..boxed_in(vec![para("hi")], 2.0, 1.0)
        };
        let l = layout(&b, &fonts).unwrap();
        let g = &l.lines[0].glyphs[0];
        // (0, BASE12) rotated 90 degrees about (1, 0.5).
        approx(g.x, 1.5 - BASE12);
        approx(g.y, -0.5);
    }

    #[test]
    fn missing_font_warns_and_falls_back() {
        let fonts = default_fonts();
        let l = layout(
            &boxed(vec![para_runs(vec![Run {
                text: "hi",
                format: fmt("Comic Sans MS", 12.0),
            }])]),
            &fonts,
        )
        .unwrap();
        assert!(l.warnings.contains(&Warning::MissingFont {
            family: "Comic Sans MS".to_owned()
        }));
        assert_eq!(l.lines[0].glyphs[0].font, 0);
        assert_eq!(l.lines[0].glyphs.len(), 2);
    }

    #[test]
    fn segoe_italic_falls_back_to_carlito_italic_with_a_warning() {
        let fonts = default_fonts();
        let format = Format {
            italic: true,
            ..fmt("Segoe UI", 12.0)
        };
        let l = layout(
            &boxed(vec![para_runs(vec![Run { text: "hi", format }])]),
            &fonts,
        )
        .unwrap();
        assert!(l.warnings.contains(&Warning::MissingFont {
            family: "Segoe UI".to_owned()
        }));
        // Carlito Italic is stack index 2.
        assert_eq!(l.lines[0].glyphs[0].font, 2);
    }

    #[test]
    fn missing_glyph_warns_and_uses_notdef() {
        let fonts = default_fonts();
        let l = layout(&boxed(vec![para("中")]), &fonts).unwrap();
        assert_eq!(
            l.warnings,
            vec![Warning::MissingGlyph {
                font: "Carlito".to_owned(),
                glyph: GlyphId(0),
                char: '中',
            }]
        );
        assert_eq!(l.lines[0].glyphs[0].glyph, GlyphId(0));
    }

    #[test]
    fn empty_stack_warns_and_draws_nothing() {
        let l = layout(&boxed(vec![para("hi")]), &FontStack::new()).unwrap();
        assert_eq!(l.lines.len(), 1);
        assert!(l.lines[0].glyphs.is_empty());
        assert!(l.warnings.contains(&Warning::MissingFont {
            family: "Carlito".to_owned()
        }));
        // Default em metrics at 12 pt.
        approx(l.lines[0].baseline, 0.8 * 12.0 / 72.0);
    }

    #[test]
    fn vertical_overflow_is_flagged() {
        let fonts = default_fonts();
        let l = layout(&boxed_in(vec![para("hi")], 2.0, 0.1), &fonts).unwrap();
        assert!(l.overflow);
    }

    #[test]
    fn trailing_spaces_are_not_rendered_or_counted() {
        let fonts = default_fonts();
        let with_spaces = layout(&boxed(vec![para("a  ")]), &fonts).unwrap();
        let without = layout(&boxed(vec![para("a")]), &fonts).unwrap();
        assert_eq!(with_spaces.lines[0].glyphs.len(), 1);
        approx(with_spaces.lines[0].width, without.lines[0].width);
    }

    #[test]
    fn a_zero_advance_line_has_a_non_negative_width() {
        // A zero-point run advances nothing; the line width is the sum
        // of the advances, so it is exactly zero. The old
        // `pen - x_start - shift` cancelled when the alignment shifted
        // the line at an indent and came out a tiny negative number
        // (task P3-11 regression, found by the layout property test;
        // this is its minimal input).
        let fonts = default_fonts();
        let paragraph = Paragraph {
            runs: vec![
                Run {
                    text: "",
                    format: fmt("Carlito", 0.0),
                },
                Run {
                    text: "\n\u{200b}a•a",
                    format: fmt("Carlito", 0.0),
                },
            ],
            align: Align::Center,
            line_spacing: 0.0,
            space_before: 0.0,
            space_after: 0.0,
            indent_first: 0.0,
            indent_left: 0.417_093_181_740_270_27,
            bullet: None,
            tabs: Vec::new(),
        };
        let block = Block {
            paragraphs: vec![paragraph],
            width: 2.018_373_444_809_812_5,
            height: 0.0,
            valign: VAlign::Top,
            margins: Insets {
                left: 0.0,
                right: 0.358_982_212_006_411_9,
                top: 0.0,
                bottom: 0.0,
            },
            rotation: Angle::from_radians(0.0),
            wrap: false,
        };
        let result = layout(&block, &fonts).unwrap();
        for line in &result.lines {
            assert!(line.width >= 0.0, "{line:?}");
            assert_eq!(line.width, 0.0, "{line:?}");
        }
    }

    #[test]
    fn decoration_flags_do_not_change_the_layout() {
        let fonts = default_fonts();
        let decorated = Format {
            underline: true,
            strikethrough: true,
            ..fmt("Carlito", 12.0)
        };
        let a = layout(&boxed(vec![para("hi")]), &fonts).unwrap();
        let b = layout(
            &boxed(vec![para_runs(vec![Run {
                text: "hi",
                format: decorated,
            }])]),
            &fonts,
        )
        .unwrap();
        assert_eq!(a.lines[0].width, b.lines[0].width);
        assert_eq!(a.lines[0].glyphs[0].x, b.lines[0].glyphs[0].x);
    }

    #[test]
    fn warn_dedupes_repeated_missing_fonts() {
        let fonts = default_fonts();
        let l = layout(
            &boxed(vec![para_runs(vec![
                Run {
                    text: "a",
                    format: fmt("Comic Sans MS", 12.0),
                },
                Run {
                    text: "b",
                    format: fmt("Comic Sans MS", 12.0),
                },
            ])]),
            &fonts,
        )
        .unwrap();
        assert_eq!(
            l.warnings
                .iter()
                .filter(|w| **w
                    == Warning::MissingFont {
                        family: "Comic Sans MS".to_owned()
                    })
                .count(),
            1
        );
    }
}
