//! The `Connects` invariants (ADR 0039 invariant 5, ADR 0045): one glue
//! per connector endpoint, records that leave with a detached shape and
//! come back with it, and the `VisFromParts`/`VisToParts` transcription.

use larrygram_model::mutate::{
    VIS_BEGIN, VIS_CONNECTION_POINT, VIS_CONTROL_POINT, VIS_END, VIS_FROM_NONE, VIS_TO_NONE,
    VIS_WHOLE_SHAPE,
};
use larrygram_model::{Connect, Document, Error, PageId, Shape, ShapeId};

/// A document with one page and a connector with two endpoints glued to
/// two targets.
#[allow(clippy::expect_used, clippy::panic)] // test helper
fn glued() -> (Document, PageId, ShapeId, ShapeId, ShapeId) {
    let mut doc = Document::new();
    let page = doc.add_page("Page-1").expect("page");
    let connector = doc
        .insert_shape(page, Shape::new(ShapeId { index: 0 }, 0), None)
        .expect("connector");
    let target = doc
        .insert_shape(page, Shape::new(ShapeId { index: 0 }, 0), None)
        .expect("target");
    let other = doc
        .insert_shape(page, Shape::new(ShapeId { index: 0 }, 0), None)
        .expect("other");
    (doc, page, connector, target, other)
}

/// One glue record from the connector's `BeginX` to the target's pin.
#[allow(clippy::expect_used, clippy::panic)] // test helper
fn begin(connector: ShapeId, doc: &Document, target: ShapeId) -> Connect {
    let mut connect = Connect::new();
    connect.from_sheet = doc.shape(connector).expect("connector").visio_id;
    connect.from_cell = "BeginX".to_owned();
    connect.from_part = VIS_BEGIN;
    connect.to_sheet = doc.shape(target).expect("target").visio_id;
    connect.to_cell = "PinX".to_owned();
    connect.to_part = VIS_WHOLE_SHAPE;
    connect
}

/// One glue record from the connector's `EndX` to the target's pin.
#[allow(clippy::expect_used, clippy::panic)] // test helper
fn end(connector: ShapeId, doc: &Document, target: ShapeId) -> Connect {
    let mut connect = Connect::new();
    connect.from_sheet = doc.shape(connector).expect("connector").visio_id;
    connect.from_cell = "EndX".to_owned();
    connect.from_part = VIS_END;
    connect.to_sheet = doc.shape(target).expect("target").visio_id;
    connect.to_cell = "PinX".to_owned();
    connect.to_part = VIS_WHOLE_SHAPE;
    connect
}

#[test]
fn add_connect_deduplicates_by_connector_endpoint() {
    let (mut doc, page, connector, target, other) = glued();
    doc.add_connect(page, begin(connector, &doc, target))
        .expect("1");
    // A repeat for the same endpoint replaces the record.
    doc.add_connect(page, begin(connector, &doc, other))
        .expect("2");
    assert_eq!(doc.page(page).expect("page").connects.len(), 1);
    assert_eq!(
        doc.page(page).expect("page").connects[0].to_sheet,
        doc.shape(other).expect("other").visio_id
    );
    // The other endpoint gets its own record.
    doc.add_connect(page, end(connector, &doc, target))
        .expect("3");
    assert_eq!(doc.page(page).expect("page").connects.len(), 2);
    assert_eq!(
        doc.add_connect(PageId { index: 9 }, Connect::new()),
        Err(Error::UnknownPage(PageId { index: 9 }))
    );
}

#[test]
fn connects_of_sees_the_begin_and_the_end() {
    let (mut doc, page, connector, target, other) = glued();
    doc.add_connect(page, begin(connector, &doc, target))
        .expect("b");
    doc.add_connect(page, end(connector, &doc, other))
        .expect("e");
    let ends = doc.connects_of(page, connector).expect("ends");
    assert_eq!(
        ends[0].as_ref().expect("begin").to_sheet,
        doc.shape(target).expect("t").visio_id
    );
    assert_eq!(
        ends[1].as_ref().expect("end").to_sheet,
        doc.shape(other).expect("o").visio_id
    );
    // An unglued connector sees nothing.
    let loose = doc
        .insert_shape(page, Shape::new(ShapeId { index: 0 }, 0), None)
        .expect("loose");
    let ends = doc.connects_of(page, loose).expect("ends");
    assert_eq!(ends, [None, None]);
    assert_eq!(
        doc.connects_of(page, ShapeId { index: 9 }),
        Err(Error::UnknownShape(ShapeId { index: 9 }))
    );
    assert_eq!(
        doc.connects_of(PageId { index: 9 }, connector),
        Err(Error::UnknownPage(PageId { index: 9 }))
    );
}

#[test]
fn remove_connects_drops_both_directions() {
    let (mut doc, page, connector, target, other) = glued();
    doc.add_connect(page, begin(connector, &doc, target))
        .expect("b");
    doc.add_connect(page, end(connector, &doc, other))
        .expect("e");
    // Removing the target takes the record that names it.
    let removed = doc.remove_connects(page, target).expect("remove");
    assert_eq!(removed.len(), 1);
    assert_eq!(doc.page(page).expect("page").connects.len(), 1);
    // Removing the connector takes the rest.
    let removed = doc.remove_connects(page, connector).expect("remove");
    assert_eq!(removed.len(), 1);
    assert!(doc.page(page).expect("page").connects.is_empty());
    assert_eq!(
        doc.remove_connects(page, ShapeId { index: 9 }),
        Err(Error::UnknownShape(ShapeId { index: 9 }))
    );
}

#[test]
fn detach_takes_the_connects_and_restore_puts_them_back() {
    let (mut doc, page, connector, target, other) = glued();
    doc.add_connect(page, begin(connector, &doc, target))
        .expect("b");
    doc.add_connect(page, end(connector, &doc, other))
        .expect("e");
    let before = doc.page(page).expect("page").connects.clone();
    let detached = doc.detach_shape(connector).expect("detach");
    assert_eq!(detached.connects.len(), 2);
    assert!(doc.page(page).expect("page").connects.is_empty());
    let restored = doc.restore_shapes(detached).expect("restore");
    assert_eq!(restored, connector);
    assert_eq!(doc.page(page).expect("page").connects.len(), 2);
    // The records come back at the exact list positions they held
    // (task P5-11: undo is the inverse, order included).
    assert_eq!(doc.page(page).expect("page").connects, before);
    // Detaching the target takes only the record naming it, and the
    // restore puts it back at its own position, not at the end.
    let detached = doc.detach_shape(target).expect("detach");
    assert_eq!(detached.connects.len(), 1);
    assert_eq!(doc.page(page).expect("page").connects.len(), 1);
    doc.restore_shapes(detached).expect("restore");
    assert_eq!(doc.page(page).expect("page").connects, before);
}

#[test]
fn the_vis_parts_constants_match_the_transcription() {
    // `VisFromParts` (Microsoft Learn).
    assert_eq!(VIS_FROM_NONE, 0);
    assert_eq!(VIS_BEGIN, 9);
    assert_eq!(VIS_END, 12);
    assert_eq!(VIS_CONTROL_POINT, 100);
    // `VisToParts` (Microsoft Learn).
    assert_eq!(VIS_TO_NONE, 0);
    assert_eq!(VIS_WHOLE_SHAPE, 3);
    assert_eq!(VIS_CONNECTION_POINT, 100);
}
