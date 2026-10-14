//! Mapping `Text`, themes, the colour table and layer rows into the model
//! (task P3-02, ADR 0019).
//!
//! The corpus is fetched, never committed (ADR 0005); these tests skip
//! with a message when `corpus/vsdx/` is absent and fail instead when
//! `FERROGRAM_REQUIRE_CORPUS=1` is set. Every assertion below was read off
//! the corpus files themselves (the black-box evidence of the task).

use std::path::{Path, PathBuf};

use ferrogram_model::{CellValue, Color, Document, PageId, ShapeId, TextPart};
use ferrogram_vsdx::{Limits, read_document};

/// The corpus directory with the real `.vsdx` packages.
fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("corpus")
        .join("vsdx")
}

/// Reads one corpus file into the model: `None` when the file is absent
/// (the run may skip), `Some(Err(_))` when it does not read.
fn read_corpus_document(name: &str) -> Option<Result<Document, ferrogram_vsdx::Error>> {
    let path = corpus_dir().join(name);
    let bytes = std::fs::read(&path).ok()?;
    Some(read_document(&bytes, &Limits::DEFAULT))
}

/// Returns from the test with a skip when the corpus is absent. The
/// expansion sits in the `#[test]` function, where a panic reports the
/// failure (clippy.toml exempts tests only).
macro_rules! corpus_document {
    ($name:expr) => {
        match read_corpus_document($name) {
            Some(Ok(document)) => document,
            Some(Err(why)) => panic!("{}: {why}", $name),
            None => {
                assert!(
                    std::env::var_os("FERROGRAM_REQUIRE_CORPUS").is_none(),
                    "corpus/vsdx is missing and FERROGRAM_REQUIRE_CORPUS=1 is set"
                );
                eprintln!("skipping: corpus/vsdx is absent; run `cargo xtask corpus`");
                return;
            }
        }
    };
}

/// The first shape whose text contains `needle`.
fn shape_with_text<'a>(document: &'a Document, needle: &str) -> Option<&'a ferrogram_model::Shape> {
    document.shapes.iter().find(|shape| {
        shape.text.as_ref().is_some_and(|text| {
            text.parts.iter().any(|part| match part {
                TextPart::Text(literal) => literal.contains(needle),
                _ => false,
            })
        })
    })
}

/// The first shape whose sheet holds a cell named `name` with `value`.
fn shape_with_cell(document: &Document, name: &str, value: &CellValue) -> Option<ShapeId> {
    document
        .shapes
        .iter()
        .find(|shape| {
            shape
                .sheet
                .cells
                .iter()
                .any(|cell| cell.name.as_str() == name && cell.value.as_ref() == Some(value))
        })
        .map(|shape| shape.id)
}

#[test]
fn text_maps_run_markers_and_literal_text() {
    let document = corpus_document!("tdf136564-WhiteTextBackground.vsdx");
    let shape = shape_with_text(&document, "First triangle without background").expect("shape");
    assert_eq!(
        shape.text.as_ref().expect("text").parts,
        vec![
            TextPart::CharRun { ix: 0 },
            TextPart::ParaRun { ix: 0 },
            TextPart::Text("First triangle without background\r\n".to_owned()),
        ]
    );
    // The `Text` element also stays preserved (ADR 0012).
    assert!(
        shape
            .preserved
            .children
            .iter()
            .any(|child| child.name.local == "Text"),
        "the Text element must stay preserved"
    );
}

#[test]
fn a_field_keeps_its_cached_display_text() {
    let document = corpus_document!("tdf154379-QuickStyleFillMatrix.vsdx");
    let found = document.shapes.iter().any(|shape| {
        shape.text.as_ref().is_some_and(|text| {
            let parts = &text.parts;
            (0..parts.len()).any(|position| {
                parts[position] == TextPart::Field { ix: 0 }
                    && matches!(parts.get(position + 1), Some(TextPart::Text(literal))
                        if literal.starts_with("06.07.2015 - 13.07.2015"))
            })
        })
    });
    assert!(found, "no shape carries the dated field");
}

#[test]
fn theme_parts_map_colors_and_fonts() {
    let document = corpus_document!("qs-box.vsdx");
    assert_eq!(document.themes.len(), 1);
    let theme = &document.themes[0];
    assert_eq!(theme.name, "Office 2016");
    let colors = theme.colors.as_ref().expect("color scheme");
    assert_eq!(colors.name, "Office 2016");
    assert_eq!(colors.dk1, Color::new(0x00, 0x00, 0x00));
    assert_eq!(colors.lt1, Color::new(0xFE, 0xFF, 0xFF));
    assert_eq!(colors.dk2, Color::new(0x44, 0x54, 0x6A));
    assert_eq!(colors.lt2, Color::new(0xE7, 0xE6, 0xE6));
    assert_eq!(colors.accents[0], Color::new(0x5B, 0x9B, 0xD5));
    assert_eq!(colors.accents[5], Color::new(0x70, 0xAD, 0x47));
    assert_eq!(colors.hlink, Color::new(0x05, 0x63, 0xC1));
    assert_eq!(colors.fol_hlink, Color::new(0x95, 0x4F, 0x72));
    let fonts = theme.fonts.as_ref().expect("font scheme");
    assert_eq!(fonts.name, "Office 2016");
    assert_eq!(fonts.major_latin, "Calibri");
    assert_eq!(fonts.minor_latin, "Calibri");
    // The effects scheme stays preserved with the part (ADR 0019).
    assert!(
        theme
            .preserved
            .children
            .iter()
            .any(|child| child.name.local == "fmtScheme")
    );
}

#[test]
fn the_color_table_maps_entries_from_index_24() {
    let document = corpus_document!("qs-box.vsdx");
    assert_eq!(document.color_table.len(), 43);
    let first = &document.color_table[0];
    assert_eq!(first.index, 24);
    assert_eq!(first.color, Color::new(0x7F, 0x7F, 0x7F));
    let last = document.color_table.last().expect("one entry");
    assert_eq!(last.index, 66);
    assert_eq!(last.color, Color::new(0xAE, 0x5A, 0x21));
    // The `Colors` element itself stays preserved (ADR 0012).
    assert!(
        document
            .preserved
            .children
            .iter()
            .any(|child| child.name.local == "Colors")
    );
}

#[test]
fn face_names_stay_preserved() {
    let document = corpus_document!("qs-box.vsdx");
    assert!(
        document
            .preserved
            .children
            .iter()
            .any(|child| child.name.local == "FaceNames"),
        "FaceNames is preserved-only and must stay verbatim"
    );
}

#[test]
fn layer_rows_and_membership_map_into_the_model() {
    let document = corpus_document!("blue-box.vsdx");
    let layers = document.page_layers(PageId { index: 0 }).expect("layers");
    assert_eq!(layers.len(), 2);
    assert_eq!(layers[0].ix, 0);
    assert_eq!(layers[0].name, "Connector");
    assert!(layers[0].visible);
    assert_eq!(layers[1].ix, 1);
    assert_eq!(layers[1].name, "Flowchart");
    assert!(layers[1].visible);
    // The shapes belong to the first layer.
    let memberships: Vec<Vec<i32>> = document
        .shapes
        .iter()
        .map(|shape| document.shape_layers(shape.id).expect("layers"))
        .filter(|membership| !membership.is_empty())
        .collect();
    assert!(!memberships.is_empty(), "no shape has a LayerMember cell");
    for membership in memberships {
        assert_eq!(membership, vec![0]);
    }
}

#[test]
fn themed_cached_values_stay_strings() {
    // The master's `LineColor` caches the `Themed` marker; the reader keeps
    // it as the string and `CellReader` resolves it from the theme (ADR
    // 0021 amendment).
    let document = corpus_document!("tdf154379-QuickStyleFillMatrix.vsdx");
    let themed = CellValue::Str("Themed".to_owned());
    assert!(
        shape_with_cell(&document, "LineColor", &themed).is_some(),
        "no shape caches the Themed marker"
    );
}
