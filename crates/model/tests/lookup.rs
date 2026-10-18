//! Integration tests for the inheritance lookup ([`CellLookup`], ADR 0013)
//! through the public API of `ferrogram-model`.

use ferrogram_model::{
    Cell, CellLookup, CellName, CellValue, Document, Error, Master, MasterId, Page, PageId, Row,
    Section, SectionName, Shape, ShapeId, SheetRef, Style, StyleId, ValueSource,
};

/// Builds a document with one page and one top-level shape.
fn document_with_shape() -> (Document, ShapeId) {
    let mut doc = Document::new();
    let id = doc.add_shape(Shape::new(ShapeId { index: 0 }, 1));
    doc.pages.push(Page::new(PageId { index: 0 }, 1));
    doc.pages[0].shapes.push(id);
    (doc, id)
}

fn number_cell(name: &str, value: f64) -> Cell {
    let mut cell = Cell::new(CellName::new(name));
    cell.value = Some(CellValue::Number(value));
    cell
}

#[test]
fn an_instance_resolves_against_its_master() {
    let (mut doc, id) = document_with_shape();
    let mut master = Master::new(MasterId { index: 0 }, 1);
    master.name_u = "Box".to_owned();
    let content = doc.add_shape(Shape::new(ShapeId { index: 0 }, 1));
    doc.shape_mut(content)
        .expect("content exists")
        .sheet
        .cells
        .push(number_cell("PinX", 7.0));
    master.shapes = vec![content];
    doc.masters.push(master);
    doc.shape_mut(id).expect("shape exists").master = Some(MasterId { index: 0 });

    let r = doc
        .resolve(id, &CellName::from_static("PinX"))
        .expect("resolves");
    assert_eq!(r.value, Some(&CellValue::Number(7.0)));
    assert_eq!(r.source, ValueSource::Master(MasterId { index: 0 }));
    assert_eq!(r.formula, None);
    assert_eq!(
        r.via,
        vec![
            ValueSource::Local,
            ValueSource::Master(MasterId { index: 0 })
        ]
    );
}

#[test]
fn a_local_edit_overrides_the_master() {
    let (mut doc, id) = document_with_shape();
    let mut master = Master::new(MasterId { index: 0 }, 1);
    let content = doc.add_shape(Shape::new(ShapeId { index: 0 }, 1));
    doc.shape_mut(content)
        .expect("content exists")
        .sheet
        .cells
        .push(number_cell("PinX", 7.0));
    master.shapes = vec![content];
    doc.masters.push(master);
    doc.shape_mut(id).expect("shape exists").master = Some(MasterId { index: 0 });
    doc.shape_mut(id)
        .expect("shape exists")
        .sheet
        .cells
        .push(number_cell("PinX", 1.0));

    let r = doc
        .resolve(id, &CellName::from_static("PinX"))
        .expect("resolves");
    assert_eq!(r.value, Some(&CellValue::Number(1.0)));
    assert_eq!(r.source, ValueSource::Local);
    assert_eq!(r.via, vec![ValueSource::Local]);
}

#[test]
fn a_style_chain_is_reported_with_provenance() {
    let (mut doc, id) = document_with_shape();
    doc.shape_mut(id).expect("shape exists").line_style = Some(1);
    let mut red = Style::new(StyleId { index: 0 });
    red.visio_id = 1;
    red.name = "Red".to_owned();
    red.line_style = Some(0);
    doc.stylesheet.styles.push(red);
    let mut normal = Style::new(StyleId { index: 1 });
    normal.visio_id = 0;
    normal.name = "Normal".to_owned();
    normal.sheet.cells.push(number_cell("LineWeight", 0.05));
    doc.stylesheet.styles.push(normal);

    let r = doc
        .resolve(id, &CellName::from_static("LineWeight"))
        .expect("resolves");
    assert_eq!(r.value, Some(&CellValue::Number(0.05)));
    assert_eq!(r.source, ValueSource::Style(StyleId { index: 1 }));
    assert_eq!(
        r.via,
        vec![
            ValueSource::Local,
            ValueSource::Style(StyleId { index: 0 }),
            ValueSource::Style(StyleId { index: 1 })
        ]
    );
}

#[test]
fn a_cyclic_style_chain_terminates_at_the_defaults() {
    let (mut doc, id) = document_with_shape();
    doc.shape_mut(id).expect("shape exists").line_style = Some(0);
    let mut a = Style::new(StyleId { index: 0 });
    a.visio_id = 0;
    a.name = "A".to_owned();
    a.line_style = Some(1);
    doc.stylesheet.styles.push(a);
    let mut b = Style::new(StyleId { index: 1 });
    b.visio_id = 1;
    b.name = "B".to_owned();
    b.line_style = Some(0);
    doc.stylesheet.styles.push(b);

    let r = doc
        .resolve(id, &CellName::from_static("LineWeight"))
        .expect("resolves");
    assert_eq!(r.source, ValueSource::HardDefault);
    assert_eq!(r.value, Some(&CellValue::Number(0.0)));
    assert_eq!(
        r.via,
        vec![
            ValueSource::Local,
            ValueSource::Style(StyleId { index: 0 }),
            ValueSource::Style(StyleId { index: 1 }),
            ValueSource::Style(StyleId { index: 0 }),
            ValueSource::SectionDefault,
            ValueSource::HardDefault
        ]
    );
}

#[test]
fn an_unknown_shape_is_an_error_and_a_lookup_never_mutates() {
    let (doc, id) = document_with_shape();
    let before = doc.clone();
    assert_eq!(
        doc.resolve(ShapeId { index: 5 }, &CellName::from_static("PinX")),
        Err(Error::UnknownShape(ShapeId { index: 5 }))
    );
    let _ = doc
        .resolve(id, &CellName::from_static("PinX"))
        .expect("resolves");
    assert_eq!(doc, before);
}

#[test]
fn a_style_that_does_not_define_the_cell_is_passed_over() {
    let (mut doc, id) = document_with_shape();
    doc.shape_mut(id).expect("shape exists").line_style = Some(1);
    // The first style carries the cell as `Inh`, which does not define it
    // (ADR 0013 amendment): the chain continues to the second style.
    let mut inherited = Style::new(StyleId { index: 0 });
    inherited.visio_id = 1;
    inherited.name = "Inherited".to_owned();
    inherited.line_style = Some(2);
    let mut cell = number_cell("LineWeight", 0.02);
    cell.formula = Some("Inh".to_owned());
    inherited.sheet.cells.push(cell);
    doc.stylesheet.styles.push(inherited);
    let mut normal = Style::new(StyleId { index: 1 });
    normal.visio_id = 2;
    normal.name = "Normal".to_owned();
    normal.sheet.cells.push(number_cell("LineWeight", 0.05));
    doc.stylesheet.styles.push(normal);

    let r = doc
        .resolve(id, &CellName::from_static("LineWeight"))
        .expect("resolves");
    assert_eq!(r.value, Some(&CellValue::Number(0.05)));
    assert_eq!(r.source, ValueSource::Style(StyleId { index: 1 }));
    assert_eq!(
        r.via,
        vec![
            ValueSource::Local,
            ValueSource::Style(StyleId { index: 0 }),
            ValueSource::Style(StyleId { index: 1 })
        ]
    );
}

#[test]
fn a_row_cell_resolves_from_the_named_row() {
    let (mut doc, id) = document_with_shape();
    let mut section = Section::new(SectionName::from_static("Geometry"));
    let mut row = Row::new();
    row.ix = Some(0);
    row.cells.push(number_cell("X", 2.0));
    section.rows.push(row);
    doc.shape_mut(id)
        .expect("shape exists")
        .sheet
        .sections
        .push(section);
    let r = doc
        .resolve_row_cell(SheetRef::Shape(id), "Geometry", 0, &CellName::new("X"))
        .expect("resolves");
    assert_eq!(r.value, Some(&CellValue::Number(2.0)));
    assert_eq!(r.source, ValueSource::Local);
    // A position the section does not have resolves to the defaults.
    let r = doc
        .resolve_row_cell(SheetRef::Shape(id), "Geometry", 7, &CellName::new("X"))
        .expect("resolves");
    assert_eq!(r.source, ValueSource::HardDefault);
}

#[test]
fn a_master_without_shapes_yields_no_chain_rungs() {
    let (mut doc, id) = document_with_shape();
    // A master whose `shapes` list is empty has no rung to offer: the
    // chain walk skips it instead of dereferencing a missing shape.
    doc.masters.push(Master::new(MasterId { index: 0 }, 1));
    doc.shape_mut(id).expect("shape exists").master = Some(MasterId { index: 0 });
    assert!(doc.master_chain_shapes(id, 32).is_empty());
}

#[test]
fn a_repeated_and_page_lookup_report_a_stale_sheet() {
    let (doc, _id) = document_with_shape();
    assert_eq!(
        doc.resolve_row_cell(
            SheetRef::Shape(ShapeId { index: 999 }),
            "Geometry",
            0,
            &CellName::new("X")
        ),
        Err(Error::UnknownShape(ShapeId { index: 999 }))
    );
    assert_eq!(
        doc.resolve_page_cell(PageId { index: 9 }, &CellName::new("PageWidth")),
        Err(Error::UnknownPage(PageId { index: 9 }))
    );
}

#[test]
fn a_themed_line_pattern_resolves_through_the_variation_style_matrix() {
    use ferrogram_model::{CellReader, Theme, ThemeLineKind, ThemeLineStyle, ThemeVariationStyles};

    // `QuickStyleLineMatrix` 100..103 names the `lineIdx` of the
    // first..fourth `vt:varStyle` of the variation style scheme
    // ([MS-VSDX]); the `lineIdx` values are 1-based.
    for (matrix, want) in [(100.0, 0.0), (101.0, 1.0)] {
        let (mut doc, id) = document_with_shape();
        doc.shape_mut(id).expect("shape exists").sheet.cells = vec![
            {
                let mut cell = Cell::new(CellName::new("QuickStyleLineMatrix"));
                cell.value = Some(CellValue::Number(matrix));
                cell
            },
            {
                let mut cell = Cell::new(CellName::new("LinePattern"));
                cell.value = Some(CellValue::Str("Themed".to_owned()));
                cell
            },
        ];
        let mut theme = Theme::new("Test");
        theme.line_styles = vec![
            ThemeLineStyle {
                kind: ThemeLineKind::Solid,
                alpha: Some(100_000),
                width_emu: Some(12_700),
            },
            ThemeLineStyle {
                kind: ThemeLineKind::None,
                alpha: None,
                width_emu: None,
            },
        ];
        theme.variation_styles = vec![ThemeVariationStyles {
            fill_indices: vec![1, 1],
            line_indices: vec![2, 1],
        }];
        doc.themes.push(theme);
        let mut reader = CellReader::new(&doc);
        assert_eq!(
            reader.number(SheetRef::Shape(id), &CellName::new("LinePattern")),
            Ok(want),
            "{matrix}"
        );
        assert!(reader.warnings().is_empty(), "{:?}", reader.warnings());
    }
}
