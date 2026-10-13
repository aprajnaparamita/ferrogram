//! Integration tests for the inheritance lookup ([`CellLookup`], ADR 0013)
//! through the public API of `ferrogram-model`.

use ferrogram_model::{
    Cell, CellLookup, CellName, CellValue, Document, Error, Master, MasterId, Page, PageId, Shape,
    ShapeId, Style, StyleId, ValueSource,
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
    doc.shape_mut(id).expect("shape exists").line_style = Some("Red".to_owned());
    let mut red = Style::new(StyleId { index: 0 });
    red.name = "Red".to_owned();
    red.base_style = Some("Normal".to_owned());
    doc.stylesheet.styles.push(red);
    let mut normal = Style::new(StyleId { index: 1 });
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
    doc.shape_mut(id).expect("shape exists").line_style = Some("A".to_owned());
    let mut a = Style::new(StyleId { index: 0 });
    a.name = "A".to_owned();
    a.base_style = Some("B".to_owned());
    doc.stylesheet.styles.push(a);
    let mut b = Style::new(StyleId { index: 1 });
    b.name = "B".to_owned();
    b.base_style = Some("A".to_owned());
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
