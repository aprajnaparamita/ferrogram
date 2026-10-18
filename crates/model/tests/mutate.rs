//! The mutation API invariants (ADR 0039, task P5-02): stable ids across
//! detach/restore, no cycles, clamped indices, id allocation and the
//! `scale_geometry` row-union rule (ADR 0038 rule 3).

use ferrogram_geom::{Point, Size, Unit};
use ferrogram_model::mutate::{CellWrite, DetachedShapes};
use ferrogram_model::stencils;
use ferrogram_model::{
    Cell, CellName, CellValue, Document, Error, GeometryRow, Master, MasterId, PageId, Row,
    Section, SectionName, Shape, ShapeId, SheetRef, TextContent,
};

/// A document with one page.
#[allow(clippy::expect_used)] // test helper
fn document() -> (Document, PageId) {
    let mut doc = Document::new();
    let page = doc.add_page("Page-1").expect("page");
    (doc, page)
}

/// A rectangle shape with `Width`/`Height` transform cells.
fn box_shape(width: f64, height: f64) -> Shape {
    let mut shape = Shape::new(ShapeId { index: 0 }, 0);
    shape.sheet.cells = vec![
        number_cell("Width", width),
        number_cell("Height", height),
        number_cell("PinX", width * 0.5),
        number_cell("PinY", height * 0.5),
    ];
    shape
}

/// A cell with a cached number.
fn number_cell(name: &str, value: f64) -> Cell {
    let mut cell = Cell::new(CellName::new(name));
    cell.value = Some(CellValue::Number(value));
    cell.unit = Unit::Inches;
    cell
}

/// The cached number of one of the shape's own cells.
#[allow(clippy::expect_used)] // test helper
fn cell_number(doc: &Document, shape: ShapeId, name: &str) -> f64 {
    match doc
        .shape(shape)
        .expect("shape")
        .sheet
        .cells
        .iter()
        .find(|cell| cell.name.as_str() == name)
        .and_then(|cell| cell.value.as_ref())
    {
        Some(CellValue::Number(value)) => *value,
        other => panic!("no numeric {name}: {other:?}"),
    }
}

/// The first `Geometry` section of the shape's own sheet.
#[allow(clippy::expect_used)] // test helper
fn geometry(doc: &Document, shape: ShapeId) -> Section {
    doc.shape(shape)
        .expect("shape")
        .sheet
        .sections
        .iter()
        .find(|section| section.name.as_str() == "Geometry")
        .cloned()
        .expect("Geometry section")
}

#[test]
fn arena_ids_stay_stable_across_detach_and_restore() {
    let (mut doc, page) = document();
    let root = doc
        .insert_shape(page, box_shape(1.0, 1.0), None)
        .expect("r");
    let child = doc
        .insert_subshape(root, box_shape(1.0, 1.0), None)
        .expect("c");
    let grandchild = doc
        .insert_subshape(child, box_shape(1.0, 1.0), None)
        .expect("g");
    let detached = doc.detach_shape(child).expect("detach");
    assert_eq!(
        detached.shapes.iter().map(|s| s.id).collect::<Vec<_>>(),
        vec![child, grandchild]
    );
    // A new shape lands at the end of the arena: the detached ids keep
    // their slots.
    let fresh = doc
        .insert_shape(page, box_shape(1.0, 1.0), None)
        .expect("f");
    assert_eq!(fresh, ShapeId { index: 3 });
    assert_eq!(doc.shape(child).expect("child").id, child);
    assert_eq!(doc.shape(grandchild).expect("g").id, grandchild);
    let restored = doc.restore_shapes(detached).expect("restore");
    assert_eq!(restored, child);
    assert_eq!(doc.shape(root).expect("root").children, vec![child]);
    assert_eq!(doc.shape(child).expect("child").children, vec![grandchild]);
    assert_eq!(doc.shape(grandchild).expect("g").parent, Some(child));
}

#[test]
fn restore_rejects_a_page_that_is_gone_and_an_empty_snapshot() {
    let (mut doc, page) = document();
    let root = doc
        .insert_shape(page, box_shape(1.0, 1.0), None)
        .expect("r");
    let detached = doc.detach_shape(root).expect("detach");
    let empty = DetachedShapes {
        page,
        parent: None,
        index: 0,
        shapes: Vec::new(),
        connects: Vec::new(),
    };
    assert_eq!(
        doc.restore_shapes(empty),
        Err(Error::BadCell("no detached root"))
    );
    let second = doc.add_page("Page-2").expect("p2");
    doc.delete_page(second).expect("delete");
    // The detached snapshot names page 0, which still exists.
    assert!(doc.restore_shapes(detached).is_ok());
}

#[test]
fn reparent_never_cycles() {
    let (mut doc, page) = document();
    let outer = doc
        .insert_shape(page, box_shape(1.0, 1.0), None)
        .expect("o");
    let middle = doc
        .insert_subshape(outer, box_shape(1.0, 1.0), None)
        .expect("m");
    let inner = doc
        .insert_subshape(middle, box_shape(1.0, 1.0), None)
        .expect("i");
    assert_eq!(
        doc.reparent_shape(outer, Some(inner), 0),
        Err(Error::WouldCycle)
    );
    assert_eq!(
        doc.reparent_shape(outer, Some(middle), 0),
        Err(Error::WouldCycle)
    );
    // Reparenting the middle shape to the top level is fine.
    doc.reparent_shape(middle, None, 0).expect("move");
    assert_eq!(doc.shape(middle).expect("m").parent, None);
    assert_eq!(doc.shape(inner).expect("i").parent, Some(middle));
}

#[test]
fn indices_clamp_and_stale_ids_error() {
    let (mut doc, page) = document();
    let first = doc
        .insert_shape(page, box_shape(1.0, 1.0), Some(0))
        .expect("1");
    let second = doc
        .insert_shape(page, box_shape(1.0, 1.0), Some(usize::MAX))
        .expect("2");
    assert_eq!(doc.page(page).expect("page").shapes, vec![first, second]);
    doc.set_z_order(page, first, usize::MAX).expect("z");
    assert_eq!(doc.page(page).expect("page").shapes, vec![second, first]);
    let detached = doc.detach_shape(first).expect("detach");
    let restored = doc.restore_shapes(detached).expect("restore");
    assert_eq!(doc.page(page).expect("page").shapes, vec![second, restored]);
    let stale = ShapeId { index: 9 };
    assert_eq!(doc.detach_shape(stale), Err(Error::UnknownShape(stale)));
    assert_eq!(
        doc.reparent_shape(stale, None, 0),
        Err(Error::UnknownShape(stale))
    );
    assert_eq!(
        doc.set_z_order(page, stale, 0),
        Err(Error::NotOnPage(stale, page))
    );
}

#[test]
fn ids_are_allocated_not_guessed() {
    let (mut doc, page) = document();
    let first = doc
        .insert_shape(page, box_shape(1.0, 1.0), None)
        .expect("1");
    let group = doc
        .insert_subshape(first, box_shape(1.0, 1.0), None)
        .expect("sub");
    assert_eq!(doc.shape(first).expect("s").visio_id, 1);
    assert_eq!(doc.shape(group).expect("s").visio_id, 2);
    assert_eq!(doc.next_visio_id(page).expect("id"), 3);
    let second = doc.add_page("Page").expect("p");
    let third = doc.add_page("Page").expect("p");
    assert_eq!(doc.page(second).expect("p").name_u, "Page");
    assert_eq!(doc.page(third).expect("p").name_u, "Page (2)");
    assert_eq!(doc.page(third).expect("p").visio_id, 3);
    let content = doc
        .insert_shape(page, box_shape(1.0, 1.0), None)
        .expect("c");
    let mut master = Master::new(MasterId { index: 0 }, 0);
    master.name_u = "Box".to_owned();
    master.shapes.push(content);
    let first_master = doc.add_master(master.clone()).expect("m");
    let second_master = doc.add_master(master).expect("m");
    assert_eq!(doc.master(first_master).expect("m").visio_id, 1);
    assert_eq!(doc.master(second_master).expect("m").visio_id, 2);
    assert_eq!(doc.master(second_master).expect("m").name_u, "Box (2)");
}

#[test]
fn scale_geometry_materialises_the_row_union_of_a_p4_06_master() {
    // One of our own P4-06 masters (ADR 0034): the "Process" flowchart
    // master, instantiated and resized (the master note in P5-02).
    let mut doc = Document::new();
    let process = stencils::basic_flowchart()
        .masters
        .into_iter()
        .find(|master| master.name_u == "Process")
        .expect("Process master");
    let instance = stencils::instantiate(&mut doc, &process);
    let page = PageId { index: 0 };
    assert_eq!(doc.page_of(instance), Some(page));

    let before = doc.shape_geometry(instance).expect("geometry");
    assert!(!before.paths.is_empty());
    let rows_before = before.paths[0].rows.len();

    // The instance starts at the master's own size; resize it to double.
    doc.scale_geometry(
        instance,
        Size::new(process.width, process.height),
        Size::new(process.width * 2.0, process.height * 2.0),
    )
    .expect("scale");

    // The row union is materialised: the instance's own `Geometry`
    // section carries every row, literal, with the coordinates scaled.
    let own = geometry(&doc, instance);
    assert_eq!(own.rows.len(), rows_before);
    assert!(own.rows.iter().all(|row| row.ix.is_some()));
    let after = doc.shape_geometry(instance).expect("geometry");
    assert_eq!(after.paths[0].rows.len(), rows_before);

    // The instance draws at its new size: the absolute coordinates that
    // came from the master are scaled per axis.
    fn coords(row: &GeometryRow) -> Option<(f64, f64)> {
        match row {
            GeometryRow::MoveTo { x, y } | GeometryRow::LineTo { x, y } => Some((*x, *y)),
            _ => None,
        }
    }
    let mut scaled = 0;
    for (before_row, after_row) in before.paths[0].rows.iter().zip(after.paths[0].rows.iter()) {
        if let (Some((x0, y0)), Some((x1, y1))) = (coords(before_row), coords(after_row)) {
            assert!((x1 - x0 * 2.0).abs() < 1e-9, "{x0} -> {x1}");
            assert!((y1 - y0 * 2.0).abs() < 1e-9, "{y0} -> {y1}");
            scaled += 1;
        }
    }
    assert!(scaled > 0, "the master's rows must include absolute ones");

    // The master's own rows are untouched.
    let master_shape = doc
        .master(MasterId { index: 0 })
        .expect("master")
        .shapes
        .first()
        .copied()
        .expect("master shape");
    let master_geometry = geometry(&doc, master_shape);
    assert_eq!(master_geometry.rows.len(), rows_before.max(1));
}

#[test]
fn scale_geometry_materialises_master_rows_with_formulas() {
    // A master whose row coordinates use the documented-style formula
    // `Width*0.5` (the master note in P5-02): a resized instance draws at
    // its new size because the materialised rows are literal.
    let (mut doc, page) = document();
    let mut master_shape = Shape::new(ShapeId { index: 0 }, 1);
    master_shape.sheet.cells = vec![
        number_cell("Width", 2.0),
        number_cell("Height", 2.0),
        number_cell("PinX", 1.0),
        number_cell("PinY", 1.0),
    ];
    let mut section = Section::new(SectionName::from_static("Geometry"));
    let mut row = Row::new();
    row.row_type = Some("LineTo".to_owned());
    row.ix = Some(0);
    let mut x = Cell::new(CellName::from_static("X"));
    x.formula = Some("Width*0.5".to_owned());
    x.value = Some(CellValue::Number(1.0));
    x.unit = Unit::Inches;
    let mut y = Cell::new(CellName::from_static("Y"));
    y.formula = Some("Height*0.5".to_owned());
    y.value = Some(CellValue::Number(1.0));
    y.unit = Unit::Inches;
    row.cells = vec![x, y];
    section.rows.push(row);
    master_shape.sheet.sections.push(section);
    let content = doc.add_shape(master_shape);
    let mut master = Master::new(MasterId { index: 0 }, 1);
    master.name_u = "Formula".to_owned();
    master.shapes.push(content);
    let master = doc.add_master(master).expect("master");

    let instance = doc
        .instantiate_master(
            master,
            page,
            Point::new(0.0, 0.0),
            Some(Size::new(2.0, 2.0)),
        )
        .expect("instance");
    doc.scale_geometry(instance, Size::new(2.0, 2.0), Size::new(4.0, 4.0))
        .expect("scale");

    // The row union is materialised as literal cells: the instance's own
    // row no longer carries the master's `Width*0.5` formula.
    let own = geometry(&doc, instance);
    let row = own.rows.first().expect("row");
    assert_eq!(row.cells[0].formula.as_deref(), Some("2"));
    assert_eq!(row.cells[0].value, Some(CellValue::Number(2.0)));
    assert_eq!(row.cells[1].value, Some(CellValue::Number(2.0)));
    // Reading through the lookup sees the instance's own (scaled) rows.
    let geometry = doc.shape_geometry(instance).expect("geometry");
    match geometry.paths[0].rows.first() {
        Some(GeometryRow::LineTo { x, y }) => {
            assert_eq!(*x, 2.0);
            assert_eq!(*y, 2.0);
        }
        other => panic!("expected LineTo, got {other:?}"),
    }
}

#[test]
fn scale_geometry_materialises_rows_and_deletes_the_master_leftovers() {
    // The instance deletes one master row and overrides another: after
    // `scale_geometry` the union is the shape's own rows, and the master's
    // rows no longer merge in a second time.
    let (mut doc, page) = document();
    let mut master_shape = Shape::new(ShapeId { index: 0 }, 1);
    master_shape.sheet.cells = vec![
        number_cell("Width", 1.0),
        number_cell("Height", 1.0),
        number_cell("PinX", 0.5),
        number_cell("PinY", 0.5),
    ];
    let mut section = Section::new(SectionName::from_static("Geometry"));
    for (index, (x, y)) in [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)].iter().enumerate() {
        let mut row = Row::new();
        row.row_type = Some("LineTo".to_owned());
        row.ix = Some(i32::try_from(index).expect("index"));
        row.cells = vec![number_cell("X", *x), number_cell("Y", *y)];
        section.rows.push(row);
    }
    master_shape.sheet.sections.push(section);
    let content = doc.add_shape(master_shape);
    let mut master = Master::new(MasterId { index: 0 }, 1);
    master.name_u = "Three".to_owned();
    master.shapes.push(content);
    let master = doc.add_master(master).expect("master");
    let instance = doc
        .instantiate_master(master, page, Point::new(0.0, 0.0), None)
        .expect("instance");

    // The instance deletes master row 1 (`Del`) and keeps the rest.
    let mut own = Section::new(SectionName::from_static("Geometry"));
    let mut deleted = Row::new();
    deleted.ix = Some(1);
    deleted
        .preserved
        .attributes
        .push(ferrogram_model::XmlAttribute {
            name: ferrogram_model::QName::local("Del"),
            value: "1".to_owned(),
        });
    own.rows.push(deleted);
    if let Some(entity) = doc.shape_mut(instance) {
        entity.sheet.sections.push(own);
    }
    let before = doc.shape_geometry(instance).expect("geometry");
    assert_eq!(before.paths[0].rows.len(), 2);

    doc.scale_geometry(instance, Size::new(1.0, 1.0), Size::new(1.0, 1.0))
        .expect("scale");

    // The union is materialised into the shape's own rows: two literal
    // rows plus the `Del` row that neutralises the master's third.
    let own = geometry(&doc, instance);
    assert_eq!(own.rows.len(), 3, "two rows plus the Del marker");
    let del_rows = own
        .rows
        .iter()
        .filter(|row| {
            row.preserved
                .attributes
                .iter()
                .any(|a| a.name.local == "Del")
        })
        .count();
    assert_eq!(
        del_rows, 1,
        "the master's dropped row is neutralised by a Del row"
    );
    let after = doc.shape_geometry(instance).expect("geometry");
    assert_eq!(after.paths[0].rows.len(), 2);
    match (before.paths[0].rows.first(), after.paths[0].rows.first()) {
        (Some(before_row), Some(after_row)) => assert_eq!(before_row, after_row),
        other => panic!("no rows: {other:?}"),
    }
}

#[test]
fn scale_geometry_keeps_rel_rows_and_accepts_a_zero_target() {
    let (mut doc, page) = document();
    let mut shape = box_shape(2.0, 2.0);
    let mut section = Section::new(SectionName::from_static("Geometry"));
    let mut rel = Row::new();
    rel.row_type = Some("RelMoveTo".to_owned());
    rel.ix = Some(0);
    rel.cells = vec![number_cell("X", 0.25), number_cell("Y", 0.75)];
    section.rows.push(rel);
    shape.sheet.sections.push(section);
    let shape = doc.insert_shape(page, shape, None).expect("s");

    doc.scale_geometry(shape, Size::new(1.0, 1.0), Size::new(0.0, 0.0))
        .expect("collapse");
    let own = geometry(&doc, shape);
    let row = own.rows.first().expect("row");
    assert_eq!(row.cells[0].value, Some(CellValue::Number(0.25)));
    assert_eq!(row.cells[1].value, Some(CellValue::Number(0.75)));
    let geometry = doc.shape_geometry(shape).expect("geometry");
    match geometry.paths[0].rows.first() {
        Some(GeometryRow::RelMoveTo { x, y }) => {
            assert_eq!(*x, 0.25);
            assert_eq!(*y, 0.75);
        }
        other => panic!("expected RelMoveTo, got {other:?}"),
    }
}

#[test]
fn group_and_ungroup_preserve_the_absolute_positions() {
    let (mut doc, page) = document();
    let first = doc
        .insert_shape(page, box_shape(2.0, 2.0), None)
        .expect("1");
    let second = doc
        .insert_shape(page, box_shape(2.0, 2.0), None)
        .expect("2");
    for (shape, x, y) in [(first, 1.0, 1.0), (second, 6.0, 3.0)] {
        doc.write_cell(
            SheetRef::Shape(shape),
            &CellName::from_static("PinX"),
            &CellWrite::new("1", CellValue::Number(x)),
        )
        .expect("pin");
        doc.write_cell(
            SheetRef::Shape(shape),
            &CellName::from_static("PinY"),
            &CellWrite::new("1", CellValue::Number(y)),
        )
        .expect("pin");
    }
    let pins = |doc: &Document, shape: ShapeId| -> (f64, f64) {
        (
            cell_number(doc, shape, "PinX"),
            cell_number(doc, shape, "PinY"),
        )
    };
    let group = doc.group_shapes(page, &[first, second]).expect("group");
    // The members move into the group's local box but draw at the same
    // page positions: group-local pin plus the box minimum.
    let group_min_x = cell_number(&doc, group, "PinX") - cell_number(&doc, group, "Width") * 0.5;
    let group_min_y = cell_number(&doc, group, "PinY") - cell_number(&doc, group, "Height") * 0.5;
    let (x, y) = pins(&doc, first);
    assert_eq!((x + group_min_x, y + group_min_y), (1.0, 1.0));
    let (x, y) = pins(&doc, second);
    assert_eq!((x + group_min_x, y + group_min_y), (6.0, 3.0));
    // Ungroup restores the page-space pins.
    doc.ungroup_shape(group).expect("ungroup");
    assert_eq!(pins(&doc, first), (1.0, 1.0));
    assert_eq!(pins(&doc, second), (6.0, 3.0));
    assert_eq!(doc.page(page).expect("page").shapes, vec![first, second]);
}

#[test]
fn set_shape_text_survives_a_detach_and_restore() {
    let (mut doc, page) = document();
    let shape = doc
        .insert_shape(page, box_shape(1.0, 1.0), None)
        .expect("s");
    let mut text = TextContent::new();
    text.parts
        .push(ferrogram_model::TextPart::Text("kept".to_owned()));
    doc.set_shape_text(shape, text.clone()).expect("text");
    let detached = doc.detach_shape(shape).expect("detach");
    let restored = doc.restore_shapes(detached).expect("restore");
    assert_eq!(doc.shape(restored).expect("s").text, Some(text));
}
