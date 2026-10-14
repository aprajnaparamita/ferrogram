//! Text block layout: wrapping, alignment, spacing, bullets and tabs
//! (ADR 0024).
//!
//! The input is plain runs with formatting and a box; the output is
//! positioned glyphs in inches in the block's local space (origin
//! top-left, y down). Coordinates are canonical units (ADR 0011); sizes
//! are points (1/72 inch). The renderer maps the output through the shape
//! transform (ADR 0020). Missing fonts and glyphs are warnings on the
//! [`Layout`], never errors (ADR 0021).

use ferrogram_geom::Angle;

use crate::error::Error;
use crate::font::{FontStack, GlyphId};

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
    /// Underlined.
    pub underline: bool,
    /// Struck through.
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
    /// The index of the input run this glyph belongs to.
    pub run: usize,
}

/// One laid-out line.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    /// The glyphs, in order.
    pub glyphs: Vec<GlyphPos>,
    /// The baseline y offset from the block origin, in inches.
    pub baseline: f64,
    /// The line width, in inches.
    pub width: f64,
    /// The index of the input paragraph this line belongs to.
    pub paragraph: usize,
}

/// A layout warning (ADR 0021).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Warning {
    /// No font in the stack has the family; a fallback was used.
    MissingFont {
        /// The requested family.
        family: String,
    },
    /// No font in the stack has the glyph; `.notdef` was used.
    MissingGlyph {
        /// The family that was used.
        font: String,
        /// The missing glyph.
        glyph: GlyphId,
    },
}

/// The layout of one text block (ADR 0024).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Layout {
    /// The lines, in order.
    pub lines: Vec<Line>,
    /// The laid-out width, in inches.
    pub width: f64,
    /// The laid-out height, in inches.
    pub height: f64,
    /// Whether content overflowed the box.
    pub overflow: bool,
    /// The warnings recorded while laying out (ADR 0021).
    pub warnings: Vec<Warning>,
}

/// Lays out `block` with fonts from `fonts` (ADR 0024).
///
/// Returns [`Error::NotImplemented`] until the layout work lands (P3-04).
pub fn layout(block: &Block<'_>, fonts: &FontStack) -> Result<Layout, Error> {
    let _ = (block, fonts);
    Err(Error::NotImplemented)
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
    fn layout_is_not_implemented_yet() {
        let block = Block {
            paragraphs: Vec::new(),
            width: 1.0,
            height: 1.0,
            valign: VAlign::Top,
            margins: Insets::default(),
            rotation: Angle::from_radians(0.0),
            wrap: true,
        };
        assert_eq!(
            layout(&block, &FontStack::new()),
            Err(Error::NotImplemented)
        );
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
            warnings: vec![Warning::MissingFont {
                family: "Calibri".to_owned(),
            }],
        };
        assert_eq!(layout.warnings.len(), 1);
        assert_eq!(Layout::default().lines.len(), 0);
    }
}
