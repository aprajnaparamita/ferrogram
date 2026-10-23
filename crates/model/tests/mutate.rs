//! The mutation API invariants (ADR 0039, task P5-02): stable ids across
//! detach/restore, no cycles, clamped indices and id allocation.

use larrygram_geom::{Point, Size, Unit};
use larrygram_model::mutate::{CellWrite, DetachedShapes};
use larrygram_model::{
    Cell, CellName, CellValue, Connect, Document, Error, Master, MasterId, PageId, Row, RowTarget,
    Section, SectionName, Shape, ShapeId, SheetRef, TextContent,
};

/// A document with one page.
#[allow(clippy::expect_used, clippy::panic)] // test helper
fn document() -> (Document, PageId) {
    let mut doc = Document::new();
    let page = doc.add_page("Page-1").expect("page");
    (doc, page)
}

/// A rectangle shape with `Width`/`Height` transform cells.
#[allow(clippy::expect_used, clippy::panic)] // test helper
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
#[allow(clippy::expect_used, clippy::panic)] // test helper
fn number_cell(name: &str, value: f64) -> Cell {
    let mut cell = Cell::new(CellName::new(name));
    cell.value = Some(CellValue::Number(value));
    cell.unit = Unit::Inches;
    cell
}

/// The cached number of one of the shape's own cells.
#[allow(clippy::expect_used, clippy::panic)] // test helper
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
#[allow(clippy::expect_used, clippy::panic)] // test helper
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
        .push(larrygram_model::TextPart::Text("kept".to_owned()));
    doc.set_shape_text(shape, text.clone()).expect("text");
    let detached = doc.detach_shape(shape).expect("detach");
    let restored = doc.restore_shapes(detached).expect("restore");
    assert_eq!(doc.shape(restored).expect("s").text, Some(text));
}

// ---------------------------------------------------------------------------
// Error arms and edge cases (ADR 0039, task P5-15)
// ---------------------------------------------------------------------------

/// A shape with explicit transform cells, so a test can build the frames
/// the bounding-box and write rules are checked against.
#[allow(clippy::expect_used, clippy::panic)] // test helper
fn framed_shape(
    pinx: f64,
    piny: f64,
    locpinx: f64,
    locpiny: f64,
    angle: f64,
    width: f64,
    height: f64,
) -> Shape {
    let mut shape = Shape::new(ShapeId { index: 0 }, 0);
    shape.sheet.cells = vec![
        number_cell("Width", width),
        number_cell("Height", height),
        number_cell("PinX", pinx),
        number_cell("PinY", piny),
        number_cell("LocPinX", locpinx),
        number_cell("LocPinY", locpiny),
        number_cell("Angle", angle),
        number_cell("FlipX", 0.0),
        number_cell("FlipY", 0.0),
    ];
    shape
}

/// A `Geometry` section with one `MoveTo` row.
#[allow(clippy::expect_used, clippy::panic)] // test helper
fn move_to_section(x: f64, y: f64) -> Section {
    let mut section = Section::new(SectionName::from_static("Geometry"));
    let mut row = Row::new();
    row.row_type = Some("MoveTo".to_owned());
    row.ix = Some(0);
    row.cells = vec![
        Cell {
            name: CellName::new("X"),
            formula: Some(x.to_string()),
            value: Some(CellValue::Number(x)),
            unit: Unit::Inches,
            preserved: Default::default(),
        },
        Cell {
            name: CellName::new("Y"),
            formula: Some(y.to_string()),
            value: Some(CellValue::Number(y)),
            unit: Unit::Inches,
            preserved: Default::default(),
        },
    ];
    section.rows.push(row);
    section
}

#[test]
fn restore_rejects_a_page_that_is_gone() {
    let (mut doc, page) = document();
    let root = doc
        .insert_shape(page, box_shape(1.0, 1.0), None)
        .expect("r");
    let mut detached = doc.detach_shape(root).expect("detach");
    detached.page = PageId { index: 9 };
    assert_eq!(
        doc.restore_shapes(detached),
        Err(Error::UnknownPage(PageId { index: 9 }))
    );
}

#[test]
fn reparent_rejects_a_stale_owner_and_a_cross_page_move() {
    let (mut doc, page) = document();
    let shape = doc
        .insert_shape(page, box_shape(1.0, 1.0), None)
        .expect("s");
    let stale = ShapeId { index: 999 };
    assert_eq!(
        doc.reparent_shape(shape, Some(stale), 0),
        Err(Error::UnknownShape(stale))
    );
    let other = doc.add_page("Page-2").expect("p2");
    let owner = doc
        .insert_shape(other, box_shape(1.0, 1.0), None)
        .expect("owner");
    assert_eq!(
        doc.reparent_shape(shape, Some(owner), 0),
        Err(Error::NotOnPage(owner, page))
    );
    // The shape is untouched by both refusals.
    assert_eq!(doc.shape(shape).expect("s").parent, None);
    assert_eq!(doc.page(page).expect("page").shapes, vec![shape]);
}

#[test]
fn write_cell_writes_a_page_sheet() {
    let (mut doc, page) = document();
    doc.write_cell(
        SheetRef::Page(page),
        &CellName::new("PageWidth"),
        &CellWrite::new("11", CellValue::Number(11.0)),
    )
    .expect("write");
    let cells = &doc.page(page).expect("page").sheet.cells;
    let cell = cells
        .iter()
        .find(|cell| cell.name.as_str() == "PageWidth")
        .expect("PageWidth");
    assert_eq!(cell.formula.as_deref(), Some("11"));
    assert_eq!(cell.value, Some(CellValue::Number(11.0)));
    assert_eq!(cell.unit, Unit::Inches);
    // A stale page id is an error, not a panic.
    assert_eq!(
        doc.write_cell(
            SheetRef::Page(PageId { index: 9 }),
            &CellName::new("PageWidth"),
            &CellWrite::new("11", CellValue::Number(11.0)),
        ),
        Err(Error::UnknownPage(PageId { index: 9 }))
    );
}
#[test]
fn delete_page_follows_the_background_references() {
    let mut doc = Document::new();
    let first = doc.add_page("A").expect("a");
    let second = doc.add_page("B").expect("b");
    let third = doc.add_page("C").expect("c");
    // B draws on A; C draws on B.
    doc.pages[1].background = Some(first);
    doc.pages[2].background = Some(second);
    assert_eq!(doc.pages[2].id, third);
    let removed = doc.delete_page(first).expect("delete");
    assert_eq!(removed.name, "A");
    // B's background named the deleted page: it is cleared. Page ids are
    // list positions, so the survivors are renumbered.
    assert_eq!(doc.pages[0].name, "B");
    assert_eq!(doc.pages[0].id, PageId { index: 0 });
    assert_eq!(doc.pages[0].background, None);
    // C's background follows its page down one position.
    assert_eq!(doc.pages[1].name, "C");
    assert_eq!(doc.pages[1].id, PageId { index: 1 });
    assert_eq!(doc.pages[1].background, Some(PageId { index: 0 }));
}

#[test]
fn move_page_at_the_same_index_is_a_no_op() {
    let mut doc = Document::new();
    let first = doc.add_page("A").expect("a");
    let second = doc.add_page("B").expect("b");
    doc.pages[1].background = Some(first);
    doc.move_page(1, 1).expect("move");
    assert_eq!(
        doc.pages
            .iter()
            .map(|page| page.name.as_str())
            .collect::<Vec<_>>(),
        vec!["A", "B"]
    );
    assert_eq!(doc.pages[1].id, second);
    assert_eq!(doc.pages[1].background, Some(first));
}

#[test]
fn instantiate_master_rejects_a_stale_master_or_page_and_a_bad_size() {
    let (mut doc, page) = document();
    let content = doc.add_shape(box_shape(2.0, 4.0));
    let mut master = Master::new(MasterId { index: 0 }, 1);
    master.name_u = "Box".to_owned();
    master.shapes = vec![content];
    let master = doc.add_master(master).expect("master");
    assert_eq!(
        doc.instantiate_master(MasterId { index: 5 }, page, Point::new(0.0, 0.0), None),
        Err(Error::UnknownMaster(MasterId { index: 5 }))
    );
    assert_eq!(
        doc.instantiate_master(master, PageId { index: 9 }, Point::new(0.0, 0.0), None),
        Err(Error::UnknownPage(PageId { index: 9 }))
    );
    assert_eq!(
        doc.instantiate_master(
            master,
            page,
            Point::new(0.0, 0.0),
            Some(Size::new(-1.0, 1.0))
        ),
        Err(Error::BadCell("negative or non-finite size"))
    );
}

#[test]
fn group_shapes_rejects_a_stale_member() {
    let (mut doc, page) = document();
    let shape = doc
        .insert_shape(page, box_shape(1.0, 1.0), None)
        .expect("s");
    let stale = ShapeId { index: 999 };
    assert_eq!(
        doc.group_shapes(page, &[shape, stale]),
        Err(Error::UnknownShape(stale))
    );
}

#[test]
fn group_shapes_rejects_a_non_finite_bounding_box() {
    let (mut doc, page) = document();
    // The corners leave the finite plane: `LocPinX` is subtracted first.
    let shape = doc
        .insert_shape(
            page,
            framed_shape(1.0e308, 0.0, -1.0e308, 0.5, 0.0, 1.0, 1.0),
            None,
        )
        .expect("s");
    assert_eq!(
        doc.group_shapes(page, &[shape]),
        Err(Error::BadCell("non-finite bounding box"))
    );
}

#[test]
fn group_shapes_reports_a_non_finite_member_pin() {
    // The bounding box of the two members is finite but wider than the
    // finite plane allows: the extent overflows to `inf` and the check
    // runs before the first mutation (ADR 0039 amendment: a failed
    // mutation changes nothing, task P5-19 review F2).
    let (mut doc, page) = document();
    let first = doc
        .insert_shape(
            page,
            framed_shape(1.7e308, 0.0, 0.5, 0.5, 0.0, 1.0, 1.0),
            None,
        )
        .expect("first");
    let second = doc
        .insert_shape(
            page,
            framed_shape(-1.7e308, 0.0, 0.5, 0.5, 0.0, 1.0, 1.0),
            None,
        )
        .expect("second");
    let before = doc.clone();
    assert_eq!(
        doc.group_shapes(page, &[first, second]),
        Err(Error::BadCell("non-finite group extent"))
    );
    assert_eq!(doc, before, "the document is unchanged");

    let (mut doc, page) = document();
    let first = doc
        .insert_shape(
            page,
            framed_shape(0.0, 1.7e308, 0.5, 0.5, 0.0, 1.0, 1.0),
            None,
        )
        .expect("first");
    let second = doc
        .insert_shape(
            page,
            framed_shape(0.0, -1.7e308, 0.5, 0.5, 0.0, 1.0, 1.0),
            None,
        )
        .expect("second");
    let before = doc.clone();
    assert_eq!(
        doc.group_shapes(page, &[first, second]),
        Err(Error::BadCell("non-finite group extent"))
    );
    assert_eq!(doc, before, "the document is unchanged");
}

#[test]
fn ungroup_reports_a_non_finite_pin_or_angle() {
    // The dissolved group's transform sends the child's pin to `inf` in
    // x: the member transforms are computed and checked before the
    // first write (task P5-19 review F2).
    let (mut doc, page) = document();
    let child = doc.add_shape(framed_shape(1.7e308, 0.0, 0.5, 0.5, 0.0, 1.0, 1.0));
    let mut group = framed_shape(1.7e308, 0.0, -1.7e308, 0.5, 0.0, 1.0, 1.0);
    group.children = vec![child];
    let group = doc.insert_shape(page, group, None).expect("group");
    let before = doc.clone();
    assert_eq!(
        doc.ungroup_shape(group),
        Err(Error::BadCell("non-finite member transform"))
    );
    assert_eq!(doc, before, "the document is unchanged");

    // The same overflow in y.
    let (mut doc, page) = document();
    let child = doc.add_shape(framed_shape(0.0, 1.7e308, 0.5, 0.5, 0.0, 1.0, 1.0));
    let mut group = framed_shape(0.0, 1.7e308, 0.5, -1.7e308, 0.0, 1.0, 1.0);
    group.children = vec![child];
    let group = doc.insert_shape(page, group, None).expect("group");
    let before = doc.clone();
    assert_eq!(
        doc.ungroup_shape(group),
        Err(Error::BadCell("non-finite member transform"))
    );
    assert_eq!(doc, before, "the document is unchanged");

    // Finite pins but an absorbed rotation that sums to `inf`: the
    // angle is checked with the pins.
    let (mut doc, page) = document();
    let child = doc.add_shape(framed_shape(1.0, 1.0, 0.5, 0.5, 1.0e308, 1.0, 1.0));
    let mut group = framed_shape(0.0, 0.0, 0.5, 0.5, 1.0e308, 1.0, 1.0);
    group.children = vec![child];
    let group = doc.insert_shape(page, group, None).expect("group");
    let before = doc.clone();
    assert_eq!(
        doc.ungroup_shape(group),
        Err(Error::BadCell("non-finite member transform"))
    );
    assert_eq!(doc, before, "the document is unchanged");
}

#[test]
fn write_connector_path_rejects_a_too_long_path() {
    let (mut doc, page) = document();
    let shape = doc
        .insert_shape(page, box_shape(1.0, 1.0), None)
        .expect("s");
    let path = vec![Point::new(0.0, 0.0); (1 << 16) + 1];
    assert_eq!(
        doc.write_connector_path(shape, &path),
        Err(Error::BadCell("connector path too long"))
    );
}

#[test]
fn write_connector_path_replaces_the_extra_geometry_sections() {
    let (mut doc, page) = document();
    let mut shape = box_shape(1.0, 1.0);
    shape.sheet.sections = vec![move_to_section(0.0, 0.0), move_to_section(2.0, 2.0), {
        let mut section = Section::new(SectionName::from_static("Character"));
        section.cells.push(number_cell("Size", 12.0));
        section
    }];
    let shape = doc.insert_shape(page, shape, None).expect("s");
    doc.write_connector_path(shape, &[Point::new(0.0, 0.0), Point::new(1.0, 1.0)])
        .expect("path");
    let sections = &doc.shape(shape).expect("s").sheet.sections;
    let names: Vec<&str> = sections
        .iter()
        .map(|section| section.name.as_str())
        .collect();
    // One `Geometry` section survives (the first slot keeps its place) and
    // the non-geometry section is untouched.
    assert_eq!(names, vec!["Geometry", "Character"]);
    let rows = &sections[0].rows;
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].row_type.as_deref(), Some("MoveTo"));
    assert_eq!(rows[1].row_type.as_deref(), Some("LineTo"));
    assert_eq!(sections[1].cells.len(), 1);
}

#[test]
fn next_visio_id_walks_a_repeated_child_once_and_stops_at_the_cap() {
    let (mut doc, page) = document();
    let root = doc
        .insert_shape(page, box_shape(1.0, 1.0), None)
        .expect("root");
    let child = doc
        .insert_subshape(root, box_shape(1.0, 1.0), None)
        .expect("child");
    // The same child listed twice is walked once; a `children` entry the
    // arena does not hold is walked past.
    doc.shape_mut(root).expect("root").children.push(child);
    doc.shape_mut(root)
        .expect("root")
        .children
        .push(ShapeId { index: 999 });
    assert_eq!(doc.next_visio_id(page).expect("id"), 3);
    // The largest `ID` in use leaves no room for the next one.
    doc.shape_mut(child).expect("child").visio_id = i32::MAX;
    assert_eq!(doc.next_visio_id(page), Err(Error::TooManyShapes));
}

#[test]
fn ungroup_into_a_parent_group_reinserts_the_children() {
    let (mut doc, page) = document();
    let first = doc
        .insert_shape(page, box_shape(1.0, 1.0), None)
        .expect("first");
    let second = doc
        .insert_shape(page, box_shape(2.0, 1.0), None)
        .expect("second");
    let inner = doc.group_shapes(page, &[first, second]).expect("inner");
    let outer = doc.group_shapes(page, &[inner]).expect("outer");
    assert_eq!(doc.shape(outer).expect("outer").children, vec![inner]);
    // Dissolving the inner group hands its children to the outer group:
    // they keep their order and their group-local pins.
    let children = doc.ungroup_shape(inner).expect("ungroup");
    assert_eq!(children, vec![first, second]);
    assert_eq!(
        doc.shape(outer).expect("outer").children,
        vec![first, second]
    );
    assert_eq!(doc.shape(first).expect("first").parent, Some(outer));
    assert_eq!(doc.shape(second).expect("second").parent, Some(outer));
    assert_eq!(doc.page(page).expect("page").shapes, vec![outer]);
    // Dissolving the outer group puts them back on the page.
    let children = doc.ungroup_shape(outer).expect("ungroup");
    assert_eq!(children, vec![first, second]);
    assert_eq!(doc.shape(first).expect("first").parent, None);
    assert_eq!(doc.shape(second).expect("second").parent, None);
    assert_eq!(doc.page(page).expect("page").shapes, vec![first, second]);
}

#[test]
fn shape_by_visio_id_walks_a_broken_children_list_once() {
    let (mut doc, page) = document();
    let root = doc
        .insert_shape(page, box_shape(1.0, 1.0), None)
        .expect("root");
    let child = doc
        .insert_subshape(root, box_shape(1.0, 1.0), None)
        .expect("child");
    // A `children` list naming a shape the arena does not hold is walked
    // past, never indexed (AGENTS.md section 6: no panics).
    doc.shape_mut(root)
        .expect("root")
        .children
        .push(ShapeId { index: 999 });
    assert_eq!(doc.shape_by_visio_id(page, 1), Some(root));
    assert_eq!(doc.shape_by_visio_id(page, 2), Some(child));
    assert_eq!(doc.shape_by_visio_id(page, 9), None);
}

#[test]
fn detach_ignores_a_child_the_arena_does_not_hold() {
    let (mut doc, page) = document();
    let root = doc
        .insert_shape(page, box_shape(1.0, 1.0), None)
        .expect("root");
    let child = doc
        .insert_subshape(root, box_shape(1.0, 1.0), None)
        .expect("child");
    doc.shape_mut(root)
        .expect("root")
        .children
        .push(ShapeId { index: 999 });
    let detached = doc.detach_shape(root).expect("detach");
    let ids: Vec<ShapeId> = detached.shapes.iter().map(|shape| shape.id).collect();
    assert_eq!(ids, vec![root, child]);
    assert_eq!(doc.shape(root).expect("root").parent, None);
}

#[test]
fn delete_page_keeps_a_background_of_an_earlier_page() {
    let mut doc = Document::new();
    let first = doc.add_page("A").expect("a");
    let second = doc.add_page("B").expect("b");
    let third = doc.add_page("C").expect("c");
    // A draws on B; deleting C must not move that reference.
    doc.pages[0].background = Some(second);
    doc.delete_page(third).expect("delete");
    assert_eq!(doc.pages.len(), 2);
    assert_eq!(doc.pages[0].id, first);
    assert_eq!(doc.pages[0].background, Some(second));
    assert_eq!(doc.pages[1].id, PageId { index: 1 });
}

#[test]
fn connects_of_keeps_the_first_record_of_each_end() {
    let (mut doc, page) = document();
    let connector = doc
        .insert_shape(page, box_shape(1.0, 1.0), None)
        .expect("connector");
    let target = doc
        .insert_shape(page, box_shape(1.0, 1.0), None)
        .expect("target");
    let from = doc.shape(connector).expect("connector").visio_id;
    let to = doc.shape(target).expect("target").visio_id;
    let mut begin = Connect::new();
    begin.from_sheet = from;
    begin.from_cell = "BeginX".to_owned();
    begin.to_sheet = to;
    let mut begin_again = Connect::new();
    begin_again.from_sheet = from;
    begin_again.from_cell = "BeginY".to_owned();
    begin_again.to_sheet = to;
    let mut end = Connect::new();
    end.from_sheet = from;
    end.from_cell = "EndX".to_owned();
    end.to_sheet = to;
    for connect in [&begin, &begin_again, &end] {
        doc.add_connect(page, connect.clone()).expect("connect");
    }
    let ends = doc.connects_of(page, connector).expect("ends");
    assert_eq!(ends[0].as_ref().expect("begin").from_cell, begin.from_cell);
    assert_eq!(ends[1].as_ref().expect("end").from_cell, end.from_cell);
}

#[test]
fn add_connect_reports_a_stale_page() {
    let (mut doc, _page) = document();
    assert_eq!(
        doc.add_connect(PageId { index: 9 }, Connect::new()),
        Err(Error::UnknownPage(PageId { index: 9 }))
    );
}

#[test]
fn instantiate_master_needs_a_master_shape() {
    let (mut doc, page) = document();
    let master = doc
        .add_master(Master::new(MasterId { index: 0 }, 1))
        .expect("master");
    // `size: None` takes the master's own size, which needs a shape.
    assert_eq!(
        doc.instantiate_master(master, page, Point::new(0.0, 0.0), None),
        Err(Error::BadCell("master has no shapes"))
    );
}

#[test]
fn id_allocation_stops_at_the_visio_id_cap() {
    let mut doc = Document::new();
    let page = doc.add_page("Page-1").expect("p");
    // A sub-shape on a page whose largest `ID` leaves no room for the
    // next one.
    let root = doc
        .insert_shape(page, box_shape(1.0, 1.0), None)
        .expect("root");
    doc.shape_mut(root).expect("root").visio_id = i32::MAX;
    assert_eq!(
        doc.insert_subshape(root, box_shape(1.0, 1.0), None),
        Err(Error::TooManyShapes)
    );
    // The same for a page and for a master.
    doc.pages[0].visio_id = i32::MAX;
    assert_eq!(doc.add_page("Page-2"), Err(Error::TooManyShapes));
    doc.masters
        .push(Master::new(MasterId { index: 0 }, i32::MAX));
    assert_eq!(
        doc.add_master(Master::new(MasterId { index: 1 }, 0)),
        Err(Error::TooManyShapes)
    );
}

/// P6-11: `write_section_cell` writes a section-level cell (the
/// `NoFill` family of `Geometry`, ADR 0019): an existing entry takes
/// the formula, the value and the ShapeSheet table's unit, and a
/// missing one is created.
#[test]
fn write_section_cell_writes_and_creates_a_section_level_cell() {
    let (mut doc, page) = document();
    let mut shape = box_shape(1.0, 1.0);
    let mut section = Section::new(SectionName::from_static("Geometry"));
    section.cells.push(number_cell("NoFill", 1.0));
    shape.sheet.sections.push(section);
    let shape = doc.insert_shape(page, shape, None).expect("shape");
    let section = SectionName::from_static("Geometry");
    doc.write_section_cell(
        SheetRef::Shape(shape),
        &section,
        &CellName::from_static("NoFill"),
        &CellWrite::new("1", CellValue::Number(2.0)),
    )
    .expect("write");
    doc.write_section_cell(
        SheetRef::Shape(shape),
        &section,
        &CellName::from_static("NoShow"),
        &CellWrite::new("0", CellValue::Number(0.0)),
    )
    .expect("write");
    let written = geometry(&doc, shape);
    let entry = written
        .cells
        .iter()
        .find(|cell| cell.name.as_str() == "NoFill")
        .expect("NoFill");
    assert_eq!(entry.formula.as_deref(), Some("1"));
    assert_eq!(entry.value, Some(CellValue::Number(2.0)));
    assert_eq!(entry.unit, Unit::Unitsless);
    let created = written
        .cells
        .iter()
        .find(|cell| cell.name.as_str() == "NoShow")
        .expect("NoShow");
    assert_eq!(created.formula.as_deref(), Some("0"));
    assert_eq!(created.value, Some(CellValue::Number(0.0)));
}

/// P6-11: `write_section_cell` reports a stale sheet, a missing
/// section and a non-finite number; a page sheet with no such section
/// is the same error as a shape without one.
#[test]
fn write_section_cell_reports_a_stale_sheet_a_missing_section_and_a_bad_number() {
    let (mut doc, page) = document();
    let stale = ShapeId { index: 999 };
    let section = SectionName::from_static("Geometry");
    assert_eq!(
        doc.write_section_cell(
            SheetRef::Shape(stale),
            &section,
            &CellName::from_static("NoFill"),
            &CellWrite::new("1", CellValue::Number(1.0)),
        ),
        Err(Error::UnknownShape(stale))
    );
    assert_eq!(
        doc.write_section_cell(
            SheetRef::Page(PageId { index: 999 }),
            &section,
            &CellName::from_static("NoFill"),
            &CellWrite::new("1", CellValue::Number(1.0)),
        ),
        Err(Error::UnknownPage(PageId { index: 999 }))
    );
    assert_eq!(
        doc.write_section_cell(
            SheetRef::Page(page),
            &section,
            &CellName::from_static("NoFill"),
            &CellWrite::new("1", CellValue::Number(1.0)),
        ),
        Err(Error::BadCell("no such section"))
    );
    let mut shape = box_shape(1.0, 1.0);
    let mut empty = Section::new(SectionName::from_static("Geometry"));
    empty.cells.push(number_cell("NoFill", 1.0));
    shape.sheet.sections.push(empty);
    let shape = doc.insert_shape(page, shape, None).expect("shape");
    assert_eq!(
        doc.write_section_cell(
            SheetRef::Shape(shape),
            &section,
            &CellName::from_static("NoFill"),
            &CellWrite::new("nan", CellValue::Number(f64::NAN)),
        ),
        Err(Error::BadCell("non-finite number"))
    );
}

#[test]
fn write_row_cell_reports_a_stale_sheet_and_a_bad_number() {
    let (mut doc, _page) = document();
    let stale = ShapeId { index: 999 };
    let section = SectionName::from_static("Geometry");
    assert_eq!(
        doc.write_row_cell(
            SheetRef::Shape(stale),
            &section,
            RowTarget::at(0),
            &CellName::new("X"),
            &CellWrite::new("1", CellValue::Number(1.0)),
        ),
        Err(Error::UnknownShape(stale))
    );
    let (mut doc, page) = document();
    let mut shape = box_shape(1.0, 1.0);
    let mut geometry = Section::new(SectionName::from_static("Geometry"));
    let mut row = Row::new();
    row.ix = Some(0);
    geometry.rows.push(row);
    shape.sheet.sections.push(geometry);
    let shape = doc.insert_shape(page, shape, None).expect("shape");
    assert_eq!(
        doc.write_row_cell(
            SheetRef::Shape(shape),
            &section,
            RowTarget::at(0),
            &CellName::new("X"),
            &CellWrite::new("nan", CellValue::Number(f64::NAN)),
        ),
        Err(Error::BadCell("non-finite number"))
    );
}

#[test]
fn ungroup_reports_a_group_no_page_reaches() {
    let (mut doc, page) = document();
    let member = doc
        .insert_shape(page, box_shape(1.0, 1.0), None)
        .expect("member");
    let group = doc.group_shapes(page, &[member]).expect("group");
    // A parent chain that ends outside the arena: the detach cannot name
    // the group's page, so the dissolve is an error, not a panic.
    doc.shape_mut(group).expect("group").parent = Some(ShapeId { index: 999 });
    assert_eq!(doc.ungroup_shape(group), Err(Error::UnknownShape(group)));
}

#[test]
fn restore_rejects_a_shape_the_arena_does_not_hold() {
    let (mut doc, page) = document();
    let mut detached = DetachedShapes {
        page,
        parent: None,
        index: 0,
        shapes: Vec::new(),
        connects: Vec::new(),
    };
    detached.shapes.push(Shape::new(ShapeId { index: 999 }, 0));
    assert_eq!(
        doc.restore_shapes(detached),
        Err(Error::UnknownShape(ShapeId { index: 999 }))
    );
}

#[test]
fn reparent_reports_a_parent_chain_outside_the_arena() {
    let (mut doc, page) = document();
    let shape = doc
        .insert_shape(page, box_shape(1.0, 1.0), None)
        .expect("shape");
    doc.shape_mut(shape).expect("shape").parent = Some(ShapeId { index: 999 });
    assert_eq!(
        doc.reparent_shape(shape, None, 0),
        Err(Error::UnknownShape(shape))
    );
}

#[test]
fn ungroup_reports_a_child_the_arena_does_not_hold() {
    let (mut doc, page) = document();
    let group = doc
        .insert_shape(page, box_shape(1.0, 1.0), None)
        .expect("group");
    doc.shape_mut(group)
        .expect("group")
        .children
        .push(ShapeId { index: 999 });
    assert_eq!(
        doc.ungroup_shape(group),
        Err(Error::UnknownShape(ShapeId { index: 999 }))
    );
}

#[test]
fn master_size_reports_a_master_shape_the_arena_does_not_hold() {
    let (mut doc, page) = document();
    // A master list built by hand can name a shape the arena lost.
    doc.masters.push(Master::new(MasterId { index: 0 }, 1));
    doc.masters[0].shapes = vec![ShapeId { index: 999 }];
    assert_eq!(
        doc.instantiate_master(MasterId { index: 0 }, page, Point::new(0.0, 0.0), None),
        Err(Error::UnknownShape(ShapeId { index: 999 }))
    );
}

#[test]
fn ungroup_drops_the_connects_of_the_dissolved_group() {
    let (mut doc, page) = document();
    let member = doc
        .insert_shape(page, box_shape(1.0, 1.0), None)
        .expect("member");
    let target = doc
        .insert_shape(page, box_shape(1.0, 1.0), None)
        .expect("target");
    let group = doc.group_shapes(page, &[member]).expect("group");
    let group_vis = doc.shape(group).expect("group").visio_id;
    let member_vis = doc.shape(member).expect("member").visio_id;
    let target_vis = doc.shape(target).expect("target").visio_id;
    let mut for_group = Connect::new();
    for_group.from_sheet = group_vis;
    for_group.from_cell = "BeginX".to_owned();
    for_group.to_sheet = target_vis;
    let mut for_member = Connect::new();
    for_member.from_sheet = member_vis;
    for_member.from_cell = "BeginX".to_owned();
    for_member.to_sheet = target_vis;
    doc.add_connect(page, for_group).expect("connect");
    doc.add_connect(page, for_member.clone()).expect("connect");
    let children = doc.ungroup_shape(group).expect("ungroup");
    assert_eq!(children, vec![member]);
    // The record naming the dissolved group goes with it; the member's
    // record stays on the page (ADR 0039 invariant 5).
    assert_eq!(doc.page(page).expect("page").connects, vec![for_member]);
}
