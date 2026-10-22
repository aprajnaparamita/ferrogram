//! Mapping `Text`, themes, the colour table and layer rows into the model
//! (task P3-02, ADR 0019).
//!
//! The corpus is fetched, never committed (ADR 0005); these tests skip
//! with a message when `corpus/vsdx/` is absent and fail instead when
//! `FERROGRAM_REQUIRE_CORPUS=1` is set. Every assertion below was read off
//! the corpus files themselves (the black-box evidence of the task).

use std::path::{Path, PathBuf};

use ferrogram_model::{CellLookup, CellValue, Color, Document, PageId, ShapeId, TextPart};
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

#[test]
fn sparse_geometry_rows_merge_with_the_master_rows() {
    // Acceptance (P3-15, fix 1): bgcolor.vsdx's page shape `Rectangle`
    // (Visio `ID` 1) carries only two `LineTo` rows (`IX` 2..3, every cell
    // `F='Inh'`, and only the `X` cell at that) while its master holds
    // `MoveTo` + the closing rows at `IX` 1..5. The union is the master's
    // closed rectangle, not the instance's open polyline (ADR 0019
    // amendment).
    //
    // The `Inh` rows carry this sheet's own snapshots (ADR 0054
    // amendment, ADR 0057): the instance is 6.98818897637795 in wide
    // against its master's 1.574803149606299 in, and the snapshots are
    // the geometry Visio computed at the instance's size (the P3-15 gap
    // closed). The rows the instance does not carry (`MoveTo`, the `Y`
    // cells, the closing row) still come from the master's rung.
    let doc = corpus_document!("bgcolor.vsdx");
    let shape = doc
        .shape_by_visio_id(PageId { index: 0 }, 1)
        .expect("shape `Rectangle`");
    let geometry = doc.shape_geometry(shape).expect("reads");
    assert_eq!(geometry.paths.len(), 1);
    let rows = &geometry.paths[0].rows;
    assert_eq!(rows.len(), 5, "{:?}", rows);
    assert_eq!(
        rows[0],
        ferrogram_model::GeometryRow::MoveTo { x: 0.0, y: 0.0 }
    );
    for (index, (x, y)) in [
        (1, (6.98818897637795, 0.0)),
        (2, (6.98818897637795, 1.181102362204724)),
        (3, (0.0, 1.181102362204724)),
        (4, (0.0, 0.0)),
    ] {
        assert_eq!(
            rows[index],
            ferrogram_model::GeometryRow::LineTo { x, y },
            "row {index}"
        );
    }
}

#[test]
fn a_themed_fill_resolves_to_the_theme_variant_colour() {
    // Acceptance (P3-15, fix 2): color-boxes.vsdx's page shapes
    // `Rectangle.5`..`Rectangle` (Visio `ID` 68..74) fill with a pure
    // `Themed` value (`FillForegnd` resolves to the marker through their
    // styles) and carry `QuickStyleFillColor` 200..206. Each resolves to a
    // colour of the theme's first variation colour scheme
    // (`vt:variationClrSchemeLst`).
    //
    // Reasoning: [MS-VSDX] `QuickStyleFillColor` maps 0..8 to the colour
    // scheme (`dk1`, `lt1`, `accent1`..`accent6`, background) and 100..106
    // to "the color property ... of a `CT_VariationClrScheme` element
    // specified by the `VariationColorIndex` cell"; `VisQuickStyleColors`
    // names 100..106 "Color variant 1..7". The 200..206 family has no
    // published meaning, but the shapes' own cached line colours (the same
    // slots through `QuickStyleLineColor`, `LineColor V='...' F='Inh'`)
    // match the same variation colours modulated by the line style matrix
    // (`QuickStyleLineMatrix` 6): 68 `#547495` = 0.73 x `varColor2`
    // `#759FCC`, 74 `#41719c` = 0.73 x `varColor1` `#5B9BD5`, and
    // tdf154379-QuickStyleFillMatrix.vsdx's shape 317 (variant 200, matrix
    // 2) caches `LineColor` `#5b9bd5` = `varColor1` unshaded. So 200+k
    // names `varColor{k+1}` like 100+k (ADR 0021 amendment).
    //
    // `VariationColorIndex` is absent on these shapes and on the page
    // sheet, so variation scheme 0 applies ([MS-VSDX]: 65534 defers to the
    // page sheet).
    let doc = corpus_document!("color-boxes.vsdx");
    for (visio_id, slot, rgb) in [
        (68, 201, (0x75, 0x9F, 0xCC)),
        (69, 205, (0x70, 0xAD, 0x47)),
        (70, 206, (0xFE, 0xC0, 0x00)),
        (71, 202, (0x41, 0x71, 0x9C)),
        (72, 204, (0xED, 0x7D, 0x31)),
        (73, 203, (0xBD, 0xD0, 0xE9)),
        (74, 200, (0x5B, 0x9B, 0xD5)),
    ] {
        let shape = doc
            .shape_by_visio_id(PageId { index: 0 }, visio_id)
            .expect("corpus shape");
        // The premise: the fill is the pure `Themed` marker.
        let resolved = doc
            .resolve(shape, &ferrogram_model::CellName::new("FillForegnd"))
            .expect("resolves");
        assert_eq!(
            resolved.value,
            Some(&CellValue::Str("Themed".to_owned())),
            "shape {visio_id} ({slot})"
        );
        let mut reader = ferrogram_model::CellReader::new(&doc);
        let fill = reader
            .color(
                ferrogram_model::SheetRef::Shape(shape),
                &ferrogram_model::CellName::new("FillForegnd"),
            )
            .expect("reads");
        assert_eq!(
            fill,
            Color::new(rgb.0, rgb.1, rgb.2),
            "shape {visio_id} (QuickStyleFillColor {slot})"
        );
    }
}
