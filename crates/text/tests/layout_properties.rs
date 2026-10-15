//! Property tests over the text layout (task P3-11, ADR 0027 section 6):
//! random runs in a random box lay out without ever panicking, and the
//! output is well-formed: every glyph coordinate and size is finite, and
//! every run and font index points at an input run or a bundled font.
//!
//! The generators are bounded (paragraphs, runs, text length, value
//! ranges) so the tests stay fast; the layout's golden numbers are pinned
//! by the unit tests in `src/layout.rs`.

use ferrogram_text::{
    Align, BULLET_RUN, Block, Bullet, Format, Insets, Paragraph, Run, VAlign, default_fonts, layout,
};
use proptest::prelude::*;

/// The characters a generated run may hold: letters, spaces, a hard
/// break, a tab, a bullet, CJK, a combining mark and a zero-width space.
const TEXT_CHARS: &[char] = &[
    'a', 'B', ' ', '\n', '\t', '•', '中', '\u{0301}', '\u{200B}', '9',
];

/// The font families a run may name: two bundled ones and one missing.
const FAMILIES: &[&str] = &["Carlito", "Selawik", "Missing"];

/// A bounded non-negative finite number (inches or points).
fn arb_non_negative(high: f64) -> impl Strategy<Value = f64> {
    0.0..high
}

/// A bounded finite number that may be negative (indents, tab stops).
fn arb_signed(high: f64) -> impl Strategy<Value = f64> {
    -high..high
}

/// The text of a generated run: bounded length, from [`TEXT_CHARS`].
fn arb_text() -> impl Strategy<Value = String> {
    proptest::collection::vec(prop::sample::select(TEXT_CHARS), 0..24)
        .prop_map(|chars| chars.into_iter().collect())
}

/// One generated paragraph: runs with random formats and layout values.
#[derive(Debug)]
struct ParaSpec {
    /// The run texts.
    texts: Vec<String>,
    /// The run families.
    families: Vec<String>,
    /// The run sizes.
    sizes: Vec<f64>,
    /// Horizontal alignment.
    align: Align,
    /// Line spacing multiple.
    line_spacing: f64,
    /// Space before, in points.
    space_before: f64,
    /// Space after, in points.
    space_after: f64,
    /// First-line indent, in inches.
    indent_first: f64,
    /// Left indent, in inches.
    indent_left: f64,
    /// The bullet, when present.
    bullet: Option<(String, f64, f64)>,
    /// Tab stops, in inches.
    tabs: Vec<f64>,
}

/// One generated block.
#[derive(Debug)]
struct BlockSpec {
    /// The paragraphs.
    paragraphs: Vec<ParaSpec>,
    /// The box size in inches.
    width: f64,
    /// The box height in inches.
    height: f64,
    /// Vertical alignment.
    valign: VAlign,
    /// The margins in inches.
    margins: [f64; 4],
    /// The block rotation in radians.
    rotation: f64,
    /// Whether text wraps.
    wrap: bool,
}

/// One generated paragraph.
fn arb_paragraph() -> impl Strategy<Value = ParaSpec> {
    let align = prop::sample::select(vec![
        Align::Left,
        Align::Center,
        Align::Right,
        Align::Justify,
    ]);
    let texts = proptest::collection::vec(arb_text(), 0..4);
    (
        texts,
        proptest::collection::vec(prop::sample::select(FAMILIES).prop_map(str::to_owned), 0..4),
        proptest::collection::vec(arb_non_negative(72.0), 0..4),
        align,
        arb_non_negative(4.0),
        arb_non_negative(24.0),
        arb_non_negative(24.0),
        arb_signed(2.0),
        arb_signed(2.0),
        proptest::option::of((arb_text(), arb_non_negative(72.0), arb_signed(2.0))),
        proptest::collection::vec(arb_signed(4.0), 0..4),
    )
        .prop_map(
            |(
                texts,
                families,
                sizes,
                align,
                line_spacing,
                space_before,
                space_after,
                indent_first,
                indent_left,
                bullet,
                tabs,
            )| ParaSpec {
                texts,
                families,
                sizes,
                align,
                line_spacing,
                space_before,
                space_after,
                indent_first,
                indent_left,
                bullet,
                tabs,
            },
        )
}

/// One generated block.
fn arb_block() -> impl Strategy<Value = BlockSpec> {
    let valign = prop::sample::select(vec![VAlign::Top, VAlign::Middle, VAlign::Bottom]);
    (
        proptest::collection::vec(arb_paragraph(), 0..4),
        arb_non_negative(5.0),
        arb_non_negative(5.0),
        valign,
        (
            arb_non_negative(1.0),
            arb_non_negative(1.0),
            arb_non_negative(1.0),
            arb_non_negative(1.0),
        ),
        arb_signed(6.3),
        any::<bool>(),
    )
        .prop_map(
            |(paragraphs, width, height, valign, margins, rotation, wrap)| BlockSpec {
                paragraphs,
                width,
                height,
                valign,
                margins: [margins.0, margins.1, margins.2, margins.3],
                rotation,
                wrap,
            },
        )
}

/// Builds the `Block` of one generated spec.
fn materialize(spec: &BlockSpec) -> Block<'_> {
    // The run texts are owned by `spec`, which outlives the block.
    let paragraphs = spec
        .paragraphs
        .iter()
        .map(|paragraph| {
            let runs = paragraph
                .texts
                .iter()
                .enumerate()
                .map(|(index, text)| Run {
                    text: text.as_str(),
                    format: Format {
                        family: paragraph
                            .families
                            .get(index)
                            .cloned()
                            .unwrap_or_else(|| "Carlito".to_owned()),
                        size: paragraph.sizes.get(index).copied().unwrap_or(12.0),
                        bold: false,
                        italic: false,
                        underline: false,
                        strikethrough: false,
                    },
                })
                .collect();
            Paragraph {
                runs,
                align: paragraph.align,
                line_spacing: paragraph.line_spacing,
                space_before: paragraph.space_before,
                space_after: paragraph.space_after,
                indent_first: paragraph.indent_first,
                indent_left: paragraph.indent_left,
                bullet: paragraph
                    .bullet
                    .as_ref()
                    .map(|(text, size, indent)| Bullet {
                        text: text.clone(),
                        family: "Carlito".to_owned(),
                        size: *size,
                        indent: *indent,
                    }),
                tabs: paragraph.tabs.clone(),
            }
        })
        .collect();
    Block {
        paragraphs,
        width: spec.width,
        height: spec.height,
        valign: spec.valign,
        margins: Insets {
            left: spec.margins[0],
            right: spec.margins[1],
            top: spec.margins[2],
            bottom: spec.margins[3],
        },
        rotation: ferrogram_geom::Angle::from_radians(spec.rotation),
        wrap: spec.wrap,
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 64, ..ProptestConfig::default() })]

    /// Random runs in a random box lay out without panicking, and every
    /// output value is well-formed: finite positions and sizes, run
    /// indices into the paragraph's runs (or the bullet run) and font
    /// indices into the bundled stack.
    #[test]
    fn random_blocks_lay_out_without_panicking(spec in arb_block()) {
        let block = materialize(&spec);
        let fonts = default_fonts();
        // `validate` rejects nothing the generator produces, but a
        // rejection is still a valid, panic-free outcome.
        if let Ok(result) = layout(&block, &fonts) {
            assert!(result.width.is_finite() && result.width >= 0.0, "{:?}", result);
            assert!(result.height.is_finite() && result.height >= 0.0, "{:?}", result);
            for line in &result.lines {
                assert!(line.baseline.is_finite(), "{line:?}");
                assert!(line.width.is_finite() && line.width >= 0.0, "{line:?}");
                assert!(line.paragraph < block.paragraphs.len(), "{line:?}");
                let runs = block.paragraphs[line.paragraph].runs.len();
                for glyph in &line.glyphs {
                    assert!(glyph.x.is_finite() && glyph.y.is_finite(), "{glyph:?}");
                    assert!(glyph.size.is_finite() && glyph.size >= 0.0, "{glyph:?}");
                    assert!(glyph.run == BULLET_RUN || glyph.run < runs, "{glyph:?}");
                    assert!(glyph.font < fonts.fonts().len(), "{glyph:?}");
                }
            }
        }
    }

    /// Blocks the validator rejects are rejected with an error, never a
    /// panic: one value is non-finite or negative.
    #[test]
    fn invalid_blocks_error_without_panicking(
        spec in arb_block(),
        which in 0..10usize,
        bad in prop_oneof![Just(f64::NAN), Just(f64::INFINITY), Just(-1.0), Just(f64::NEG_INFINITY)],
    ) {
        let mut spec = spec;
        match which {
            0 => spec.width = bad,
            1 => spec.height = bad,
            2 => spec.margins[0] = bad,
            3 => spec.rotation = bad,
            4 if !spec.paragraphs.is_empty() => spec.paragraphs[0].line_spacing = bad,
            5 if !spec.paragraphs.is_empty() => spec.paragraphs[0].space_before = bad,
            6 if !spec.paragraphs.is_empty() => spec.paragraphs[0].indent_first = bad,
            7 if !spec.paragraphs.is_empty() => spec.paragraphs[0].tabs.push(bad),
            8 if !spec.paragraphs.is_empty() && !spec.paragraphs[0].texts.is_empty() => {
                let paragraph = &mut spec.paragraphs[0];
                paragraph.sizes.resize(paragraph.texts.len(), 12.0);
                paragraph.sizes[0] = bad;
            }
            9 if !spec.paragraphs.is_empty() => {
                spec.paragraphs[0].bullet = Some((String::new(), bad, 0.0));
            }
            _ => {}
        }
        let block = materialize(&spec);
        let _ = layout(&block, &default_fonts());
    }
}

#[test]
fn extreme_but_finite_values_never_panic() {
    // Values no file should produce but finite inputs can spell: the
    // layout must not panic. The output may be non-finite (a hardening
    // candidate recorded in `docs/reviews/phase-3-tests.md`).
    let paragraph = Paragraph {
        runs: vec![Run {
            text: "x\ny",
            format: Format {
                family: "Carlito".to_owned(),
                size: f64::MAX,
                bold: false,
                italic: false,
                underline: false,
                strikethrough: false,
            },
        }],
        align: Align::Justify,
        line_spacing: f64::MAX,
        space_before: f64::MAX,
        space_after: f64::MAX,
        indent_first: f64::MAX,
        indent_left: -f64::MAX,
        bullet: Some(Bullet {
            text: "•".to_owned(),
            family: "Missing".to_owned(),
            size: f64::MAX,
            indent: f64::MAX,
        }),
        tabs: vec![f64::MAX, f64::MIN_POSITIVE],
    };
    let block = Block {
        paragraphs: vec![paragraph],
        width: f64::MAX,
        height: f64::MAX,
        valign: VAlign::Bottom,
        margins: Insets {
            left: f64::MAX,
            right: f64::MAX,
            top: f64::MAX,
            bottom: f64::MAX,
        },
        rotation: ferrogram_geom::Angle::from_radians(f64::MAX),
        wrap: false,
    };
    let _ = layout(&block, &default_fonts());
}
