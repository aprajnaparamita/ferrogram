//! The model mutation API (ADR 0039): creating, deleting and re-parenting
//! shapes, writing cells, adding and re-ordering pages, master instances
//! and `Connects`.
//!
//! Every operation is a method on [`Document`] that returns
//! `Result<_, Error>`; the model never panics (AGENTS.md section 6). The
//! invariants each operation keeps are recorded in ADR 0039; the short
//! version:
//!
//! - Arena ids are stable: an operation never moves or removes a
//!   [`Shape`] of `Document::shapes`, so ids held by an undo step stay
//!   valid across undo. A deleted shape is **detached** (removed from its
//!   page or parent) and comes back through [`Document::restore_shapes`].
//! - Every page and parent list holds live shapes in draw order (z-order
//!   for `Page::shapes`); the arena may hold detached shapes that no page
//!   reaches.
//! - Input-derived counts are capped and arithmetic is checked
//!   (AGENTS.md section 6).
//!
//! Task P5-02 implements the bodies. The bodies are
//! `Err(Error::NotImplemented)` stubs until then.

use crate::cell::CellValue;
use crate::document::Document;
use crate::error::Error;
use crate::id::{MasterId, PageId, ShapeId};
use crate::master::Master;
use crate::name::{CellName, SectionName};
use crate::page::{Connect, Page};
use crate::shape::Shape;
use crate::text::TextContent;
use crate::values::{RowTarget, SheetRef};

use ferrogram_geom::{Point, Size};

/// One cell write (ADR 0038): the formula to store and the cached value
/// that goes with it.
///
/// An edit writes **both**: a literal formula (or the documented simple
/// formula, e.g. `Width*0.5` for `LocPinX`) and the cached `V` value, so a
/// file we save reads back the same and a later formula engine (Phase 6)
/// recalculates to the same numbers. The formula is written as spelled;
/// `F='Inh'` is expressed by *not* writing the cell.
#[derive(Debug, Clone, PartialEq)]
pub struct CellWrite {
    /// The formula text, e.g. `"2.5"` or `"Width*0.5"`.
    pub formula: String,
    /// The cached value to store alongside the formula.
    pub value: CellValue,
}

impl CellWrite {
    /// A write of one formula and its cached value.
    pub fn new(formula: impl Into<String>, value: CellValue) -> Self {
        Self {
            formula: formula.into(),
            value,
        }
    }
}

/// A detached shape subtree, kept so [`Document::restore_shapes`] can put
/// it back (ADR 0039: delete is detach, ids stay stable).
#[derive(Debug, Clone, PartialEq)]
pub struct DetachedShapes {
    /// The page the subtree was detached from.
    pub page: PageId,
    /// The parent the root was detached from, or `None` for a top-level
    /// shape.
    pub parent: Option<ShapeId>,
    /// The index the root held in its page or parent list.
    pub index: usize,
    /// The detached root first, then its descendants (arena order).
    pub shapes: Vec<Shape>,
    /// The `Connects` records removed with the subtree.
    pub connects: Vec<Connect>,
}

impl Document {
    /// Inserts `shape` as a top-level shape of `page`, at z-order `index`
    /// (`None` appends on top).
    ///
    /// Keeps: the new shape gets a fresh arena id and the page's next free
    /// Visio `ID`; `index` past the end clamps; a stale `page` is
    /// [`Error::UnknownPage`].
    pub fn insert_shape(
        &mut self,
        page: PageId,
        shape: Shape,
        index: Option<usize>,
    ) -> Result<ShapeId, Error> {
        let _ = (page, shape, index);
        Err(Error::NotImplemented)
    }

    /// Inserts `shape` as a sub-shape of `parent`, at child index `index`
    /// (`None` appends last).
    ///
    /// Keeps: `Shape::parent` and `Shape::children` agree; inserting under
    /// a descendant of `shape` would cycle and is an `Err`.
    pub fn insert_subshape(
        &mut self,
        parent: ShapeId,
        shape: Shape,
        index: Option<usize>,
    ) -> Result<ShapeId, Error> {
        let _ = (parent, shape, index);
        Err(Error::NotImplemented)
    }

    /// Detaches the shape's subtree from its page or parent (delete,
    /// ADR 0039) and returns it with the `Connects` records that named
    /// any detached shape.
    ///
    /// Keeps: the arena ids survive; the page or parent list no longer
    /// holds the root. A stale `shape` is [`Error::UnknownShape`].
    pub fn detach_shape(&mut self, shape: ShapeId) -> Result<DetachedShapes, Error> {
        let _ = shape;
        Err(Error::NotImplemented)
    }

    /// Puts a detached subtree back at its recorded page, parent and
    /// index, and restores its `Connects`. Returns the root's new (old)
    /// arena id.
    ///
    /// Keeps: the same invariants as [`Document::insert_shape`]; an index
    /// past the current list length clamps.
    pub fn restore_shapes(&mut self, detached: DetachedShapes) -> Result<ShapeId, Error> {
        let _ = detached;
        Err(Error::NotImplemented)
    }

    /// Moves `shape` to child index `index` of `new_parent` (`None`:
    /// top-level of its page), keeping z-order semantics of the target
    /// list.
    ///
    /// Keeps: no cycles (a shape cannot become its own ancestor); the
    /// source list loses the shape at its old position.
    pub fn reparent_shape(
        &mut self,
        shape: ShapeId,
        new_parent: Option<ShapeId>,
        index: usize,
    ) -> Result<(), Error> {
        let _ = (shape, new_parent, index);
        Err(Error::NotImplemented)
    }

    /// Moves `shape` to z-order `index` of its page (bring forward, send
    /// to back, ...). Indices clamp.
    pub fn set_z_order(&mut self, page: PageId, shape: ShapeId, index: usize) -> Result<(), Error> {
        let _ = (page, shape, index);
        Err(Error::NotImplemented)
    }

    /// Replaces the shape's text (`Text` element and `Shape::text`, ADR
    /// 0019) with `text`.
    pub fn set_shape_text(&mut self, shape: ShapeId, text: TextContent) -> Result<(), Error> {
        let _ = (shape, text);
        Err(Error::NotImplemented)
    }

    /// Writes one cell of a shape or page sheet (ADR 0038: formula and
    /// cached value together).
    ///
    /// Keeps: a sheet the id does not name is an `Err`; the cell's unit
    /// follows the ShapeSheet table's kind.
    pub fn write_cell(
        &mut self,
        sheet: SheetRef,
        cell: &CellName,
        write: &CellWrite,
    ) -> Result<(), Error> {
        let _ = (sheet, cell, write);
        Err(Error::NotImplemented)
    }

    /// Writes one cell of one row of a repeated section (a `Geometry` or
    /// `Character` row), named by [`RowTarget`] (ADR 0021 amendment).
    pub fn write_row_cell(
        &mut self,
        sheet: SheetRef,
        section: &SectionName,
        row: RowTarget,
        cell: &CellName,
        write: &CellWrite,
    ) -> Result<(), Error> {
        let _ = (sheet, section, row, cell, write);
        Err(Error::NotImplemented)
    }

    /// Scales the shape's own geometry rows from the `from` box to the
    /// `to` box (resize without a formula engine, ADR 0038): `Rel*` rows
    /// keep their values, absolute rows scale linearly per axis, and rows
    /// inherited from a master are materialised as the shape's own rows
    /// first (the ADR 0019 row union).
    ///
    /// A zero or non-finite dimension in `from` is an `Err`; `to` may be
    /// zero (a collapsed shape).
    pub fn scale_geometry(&mut self, shape: ShapeId, from: Size, to: Size) -> Result<(), Error> {
        let _ = (shape, from, to);
        Err(Error::NotImplemented)
    }

    /// Adds a new empty page named `name` on top of the page list and
    /// returns its id.
    ///
    /// Keeps: `NameU` is unique in the document; the new page's sheet gets
    /// the documented page defaults (ADR 0021 table).
    pub fn add_page(&mut self, name: &str) -> Result<PageId, Error> {
        let _ = name;
        Err(Error::NotImplemented)
    }

    /// Removes the page and returns it (with its shapes and connects).
    ///
    /// Keeps: `Page::background` references to the removed page are
    /// cleared; the last page may be removed (a document with no pages is
    /// legal in the model; the writer refuses to save it).
    pub fn delete_page(&mut self, page: PageId) -> Result<Page, Error> {
        let _ = page;
        Err(Error::NotImplemented)
    }

    /// Renames the page (`Name` and `NameU`).
    pub fn rename_page(&mut self, page: PageId, name: &str) -> Result<(), Error> {
        let _ = (page, name);
        Err(Error::NotImplemented)
    }

    /// Moves the page at list index `from` to index `to` (reorder). Both
    /// indices clamp; a stale index is an `Err`.
    pub fn move_page(&mut self, from: usize, to: usize) -> Result<(), Error> {
        let _ = (from, to);
        Err(Error::NotImplemented)
    }

    /// Adds a master to the master list and returns its id.
    ///
    /// Keeps: the master's shapes must already be in the arena and are
    /// linked as the master's top-level shapes; `visio_id` is assigned
    /// when the given one is 0.
    pub fn add_master(&mut self, master: Master) -> Result<MasterId, Error> {
        let _ = master;
        Err(Error::NotImplemented)
    }

    /// Adds an instance of `master` to `page` at pin position `at` (page
    /// inches) with the given size (`None`: the master's own size, the
    /// P4-06 deferred item) and returns the new shape.
    ///
    /// Keeps: the instance's `Master` link, `PinX`/`PinY`/`Width`/
    /// `Height`/`LocPinX`/`LocPinY` cells (literal values plus the
    /// documented `LocPinX = Width*0.5` formulas, ADR 0038), and the
    /// master-instance cells [MS-VSDX] requires. The instance geometry
    /// stays the master's rows (row union, ADR 0019) until resized.
    pub fn instantiate_master(
        &mut self,
        master: MasterId,
        page: PageId,
        at: Point,
        size: Option<Size>,
    ) -> Result<ShapeId, Error> {
        let _ = (master, page, at, size);
        Err(Error::NotImplemented)
    }

    /// Records one `Connect` (glue) on the page (ADR 0045).
    ///
    /// Keeps: one record per (connector endpoint, target) pair; a repeat
    /// replaces the old record instead of adding a second.
    pub fn add_connect(&mut self, page: PageId, connect: Connect) -> Result<(), Error> {
        let _ = (page, connect);
        Err(Error::NotImplemented)
    }

    /// Removes every `Connect` record naming `shape` and returns them.
    pub fn remove_connects(&mut self, page: PageId, shape: ShapeId) -> Result<Vec<Connect>, Error> {
        let _ = (page, shape);
        Err(Error::NotImplemented)
    }

    /// The connector's glued endpoints: `(begin target, end target)` as
    /// `Connect` records, `None` for an unglued end.
    pub fn connects_of(
        &self,
        page: PageId,
        connector: ShapeId,
    ) -> Result<[Option<Connect>; 2], Error> {
        let _ = (page, connector);
        Err(Error::NotImplemented)
    }

    /// Groups the shapes into a new group shape on their page (the
    /// ADR 0019 group semantics) and returns the group's id.
    ///
    /// Keeps: z-order of the members relative to each other; the group
    /// takes the members' bounding box as its `Width`/`Height`; an empty
    /// `shapes` slice or a mixed-page list is an `Err`.
    pub fn group_shapes(&mut self, page: PageId, shapes: &[ShapeId]) -> Result<ShapeId, Error> {
        let _ = (page, shapes);
        Err(Error::NotImplemented)
    }

    /// Dissolves the group: its children become top-level shapes (or
    /// children of the grandparent group) in their old order, and the
    /// group shape is detached. Returns the former children's ids.
    pub fn ungroup_shape(&mut self, group: ShapeId) -> Result<Vec<ShapeId>, Error> {
        let _ = group;
        Err(Error::NotImplemented)
    }

    /// Writes the connector's geometry rows as one path (the routed
    /// points, in page inches), as literal rows (ADR 0045: the app
    /// re-routes; a formula engine may replace this later without a
    /// migration, ADR 0038).
    ///
    /// Keeps: the row union rules of ADR 0019 (the path lands in the
    /// shape's own `Geometry` section).
    pub fn write_connector_path(
        &mut self,
        connector: ShapeId,
        path: &[Point],
    ) -> Result<(), Error> {
        let _ = (connector, path);
        Err(Error::NotImplemented)
    }

    /// The next free Visio `ID` on the page (the largest in use plus one).
    pub fn next_visio_id(&self, page: PageId) -> Result<i32, Error> {
        let _ = page;
        Err(Error::NotImplemented)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_write_holds_formula_and_value() {
        let write = CellWrite::new("2.5", CellValue::Number(2.5));
        assert_eq!(write.formula, "2.5");
        assert_eq!(write.value, CellValue::Number(2.5));
    }

    #[test]
    fn detached_shapes_hold_the_subtree() {
        let detached = DetachedShapes {
            page: PageId { index: 0 },
            parent: None,
            index: 1,
            shapes: Vec::new(),
            connects: Vec::new(),
        };
        assert_eq!(detached.index, 1);
        assert!(detached.shapes.is_empty());
    }

    /// The P5-01 stubs: every mutation reports "not implemented" until
    /// task P5-02 implements them. These tests are replaced by the real
    /// behaviour tests in that task.
    #[test]
    fn the_p5_01_stubs_are_not_implemented() {
        let mut doc = Document::new();
        let page = PageId { index: 0 };
        let shape_id = ShapeId { index: 0 };
        let master_id = MasterId { index: 0 };
        let cell = CellName::new("PinX");
        let section = SectionName::new("Geometry");
        let write = CellWrite::new("1", CellValue::Number(1.0));
        let point = Point::new(0.0, 0.0);
        let size = Size::new(1.0, 1.0);

        assert_eq!(
            doc.insert_shape(page, Shape::new(shape_id, 1), None),
            Err(Error::NotImplemented)
        );
        assert_eq!(
            doc.insert_subshape(shape_id, Shape::new(shape_id, 2), None),
            Err(Error::NotImplemented)
        );
        assert_eq!(doc.detach_shape(shape_id), Err(Error::NotImplemented));
        assert_eq!(
            doc.restore_shapes(DetachedShapes {
                page,
                parent: None,
                index: 0,
                shapes: Vec::new(),
                connects: Vec::new(),
            }),
            Err(Error::NotImplemented)
        );
        assert_eq!(
            doc.reparent_shape(shape_id, None, 0),
            Err(Error::NotImplemented)
        );
        assert_eq!(
            doc.set_z_order(page, shape_id, 0),
            Err(Error::NotImplemented)
        );
        assert_eq!(
            doc.set_shape_text(shape_id, TextContent::default()),
            Err(Error::NotImplemented)
        );
        assert_eq!(
            doc.write_cell(SheetRef::Shape(shape_id), &cell, &write),
            Err(Error::NotImplemented)
        );
        assert_eq!(
            doc.write_row_cell(
                SheetRef::Shape(shape_id),
                &section,
                RowTarget::at(0),
                &cell,
                &write
            ),
            Err(Error::NotImplemented)
        );
        assert_eq!(
            doc.scale_geometry(shape_id, size, size),
            Err(Error::NotImplemented)
        );
        assert_eq!(doc.add_page("Page-2"), Err(Error::NotImplemented));
        assert_eq!(doc.delete_page(page), Err(Error::NotImplemented));
        assert_eq!(doc.rename_page(page, "Renamed"), Err(Error::NotImplemented));
        assert_eq!(doc.move_page(0, 1), Err(Error::NotImplemented));
        assert_eq!(
            doc.add_master(Master::new(master_id, 1)),
            Err(Error::NotImplemented)
        );
        assert_eq!(
            doc.instantiate_master(master_id, page, point, None),
            Err(Error::NotImplemented)
        );
        assert_eq!(
            doc.add_connect(page, Connect::new()),
            Err(Error::NotImplemented)
        );
        assert_eq!(
            doc.remove_connects(page, shape_id),
            Err(Error::NotImplemented)
        );
        assert_eq!(doc.connects_of(page, shape_id), Err(Error::NotImplemented));
        assert_eq!(
            doc.group_shapes(page, &[shape_id]),
            Err(Error::NotImplemented)
        );
        assert_eq!(doc.ungroup_shape(shape_id), Err(Error::NotImplemented));
        assert_eq!(
            doc.write_connector_path(shape_id, &[point]),
            Err(Error::NotImplemented)
        );
        assert_eq!(doc.next_visio_id(page), Err(Error::NotImplemented));
    }
}
