//! Provenance (`Resolved::via`) and arena accessor edges (task P2-10,
//! ADR 0013 and ADR 0011).
//!
//! The lookup's rule table is tested next to the implementation
//! (`src/lookup.rs`); these tests pin the edges that only show through the
//! public API: an instance root that lives inside a group (the position rule
//! reads the parent's children), a resolution that walks every rung of the
//! chain, and stale or overflowing arena ids.

use larrygram_model::{
    Cell, CellLookup, CellName, CellValue, Document, Master, MasterId, Page, PageId, Shape,
    ShapeId, Style, StyleId, ValueSource,
};

/// A number cell.
fn number_cell(name: &str, value: f64) -> Cell {
    let mut cell = Cell::new(CellName::new(name));
    cell.value = Some(CellValue::Number(value));
    cell
}

/// A cell whose formula is the `Inh` inherit marker (ADR 0013 amendment).
fn inherit_cell(name: &str) -> Cell {
    let mut cell = Cell::new(CellName::new(name));
    cell.formula = Some("Inh".to_owned());
    cell.value = Some(CellValue::Number(99.0));
    cell
}

#[test]
fn an_instance_inside_a_group_matches_the_master_shape_at_its_sibling_position() {
    let mut doc = Document::new();
    // A master with three top-level shapes: the instance's sibling position
    // picks the shape (P2-06b decision 2).
    let mut master = Master::new(MasterId { index: 0 }, 1);
    for value in [10.0, 20.0, 30.0] {
        let content = doc.add_shape(Shape::new(ShapeId { index: 0 }, 1));
        doc.shape_mut(content)
            .expect("content exists")
            .sheet
            .cells
            .push(number_cell("PinX", value));
        master.shapes.push(content);
    }
    doc.masters.push(master);

    // The group holds two instances; each resolves against its own position.
    let group = doc.add_shape(Shape::new(ShapeId { index: 0 }, 1));
    let first = doc.add_shape(Shape::new(ShapeId { index: 0 }, 1));
    let second = doc.add_shape(Shape::new(ShapeId { index: 0 }, 1));
    for id in [first, second] {
        doc.shape_mut(id).expect("exists").master = Some(MasterId { index: 0 });
        doc.shape_mut(id).expect("exists").parent = Some(group);
        doc.shape_mut(group).expect("exists").children.push(id);
    }
    let mut page = Page::new(PageId { index: 0 }, 1);
    page.shapes.push(group);
    doc.pages.push(page);

    let resolve = |id: ShapeId| {
        doc.resolve(id, &CellName::from_static("PinX"))
            .expect("resolves")
    };
    let first_resolved = resolve(first);
    assert_eq!(first_resolved.value, Some(&CellValue::Number(10.0)));
    assert_eq!(
        first_resolved.via,
        vec![
            ValueSource::Local,
            ValueSource::Master(MasterId { index: 0 })
        ]
    );
    let second_resolved = resolve(second);
    assert_eq!(second_resolved.value, Some(&CellValue::Number(20.0)));
}

#[test]
fn a_resolution_reports_every_rung_it_consulted() {
    let mut doc = Document::new();
    let id = doc.add_shape(Shape::new(ShapeId { index: 0 }, 1));
    let mut page = Page::new(PageId { index: 0 }, 1);
    page.shapes.push(id);
    doc.pages.push(page);
    // The local cell is the inherit marker: it defines nothing (ADR 0013
    // amendment), and its snapshot value is not a fallback.
    doc.shape_mut(id)
        .expect("exists")
        .sheet
        .cells
        .push(inherit_cell("LineWeight"));

    // The master defines nothing either: its cell is `Inh` too.
    let mut master = Master::new(MasterId { index: 0 }, 1);
    let content = doc.add_shape(Shape::new(ShapeId { index: 0 }, 1));
    doc.shape_mut(content)
        .expect("content exists")
        .sheet
        .cells
        .push(inherit_cell("LineWeight"));
    master.shapes = vec![content];
    doc.masters.push(master);
    doc.shape_mut(id).expect("exists").master = Some(MasterId { index: 0 });

    // The style chain resolves `LineWeight` (`Line Format` follows
    // `line_style`).
    doc.shape_mut(id).expect("exists").line_style = Some(1);
    let mut red = Style::new(StyleId { index: 0 });
    red.visio_id = 1;
    red.line_style = Some(0);
    doc.stylesheet.styles.push(red);
    let mut normal = Style::new(StyleId { index: 1 });
    normal.visio_id = 0;
    normal.sheet.cells.push(number_cell("LineWeight", 0.05));
    doc.stylesheet.styles.push(normal);

    let resolved = doc
        .resolve(id, &CellName::from_static("LineWeight"))
        .expect("resolves");
    assert_eq!(resolved.value, Some(&CellValue::Number(0.05)));
    assert_eq!(resolved.source, ValueSource::Style(StyleId { index: 1 }));
    assert_eq!(resolved.formula, None);
    assert_eq!(
        resolved.via,
        vec![
            ValueSource::Local,
            ValueSource::Master(MasterId { index: 0 }),
            ValueSource::Style(StyleId { index: 0 }),
            ValueSource::Style(StyleId { index: 1 }),
        ]
    );
}

#[test]
fn the_hard_default_run_gives_the_type_default() {
    let mut doc = Document::new();
    let id = doc.add_shape(Shape::new(ShapeId { index: 0 }, 1));
    let resolved = doc
        .resolve(id, &CellName::from_static("LineWeight"))
        .expect("resolves");
    assert_eq!(resolved.value, Some(&CellValue::Number(0.0)));
    assert_eq!(resolved.source, ValueSource::HardDefault);
    assert_eq!(
        resolved.via,
        vec![
            ValueSource::Local,
            ValueSource::SectionDefault,
            ValueSource::HardDefault
        ]
    );
}

#[test]
fn stale_and_overflowing_arena_ids_find_nothing() {
    let mut doc = Document::new();
    let id = doc.add_shape(Shape::new(ShapeId { index: 0 }, 1));
    doc.pages.push(Page::new(PageId { index: 0 }, 1));
    doc.masters.push(Master::new(MasterId { index: 0 }, 1));
    doc.stylesheet.styles.push(Style::new(StyleId { index: 0 }));

    let stale = ShapeId { index: u32::MAX };
    assert!(doc.shape(stale).is_none());
    assert!(doc.shape_mut(stale).is_none());
    assert!(doc.page(PageId { index: u32::MAX }).is_none());
    assert!(doc.master(MasterId { index: u32::MAX }).is_none());
    assert!(doc.style(StyleId { index: u32::MAX }).is_none());
    assert_eq!(doc.shape_by_visio_id(PageId { index: 0 }, 42), None);
    let _ = id;
}

#[test]
fn lookups_by_file_identity_take_the_first_match() {
    let mut doc = Document::new();
    let first = doc.add_shape(Shape::new(ShapeId { index: 0 }, 1));
    let second = doc.add_shape(Shape::new(ShapeId { index: 0 }, 1));
    let mut page = Page::new(PageId { index: 0 }, 1);
    page.shapes.push(first);
    page.shapes.push(second);
    doc.pages.push(page);
    for _ in 0..2 {
        let mut master = Master::new(MasterId { index: 0 }, 1);
        master.name_u = "Box.2".to_owned();
        doc.masters.push(master);
        let mut style = Style::new(StyleId { index: 0 });
        style.name = "Normal".to_owned();
        style.visio_id = 7;
        doc.stylesheet.styles.push(style);
    }

    // Duplicated `ID`s are malformed but readable: the first wins.
    assert_eq!(doc.shape_by_visio_id(PageId { index: 0 }, 1), Some(first));
    assert_eq!(doc.master_by_name_u("Box.2"), Some(MasterId { index: 0 }));
    assert_eq!(doc.style_by_name("Normal"), Some(StyleId { index: 0 }));
    assert_eq!(
        doc.style_by_visio_id(7).map(|style| style.id),
        Some(StyleId { index: 0 })
    );
}
