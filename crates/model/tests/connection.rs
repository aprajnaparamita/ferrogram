//! Connection points from the shape's own `Connection` section and from
//! its master chain (task P5-16, ADR 0013, ADR 0043).

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use larrygram_geom::Point;
use larrygram_model::{
    Cell, CellName, CellValue, CellWrite, Document, Master, MasterId, Page, PageId, Preserved, Row,
    Section, SectionName, Shape, ShapeId, SheetRef, stencils,
};

fn cell(name: &str, value: f64) -> Cell {
    Cell {
        name: CellName::new(name),
        formula: Some(format!("{value}")),
        value: Some(CellValue::Number(value)),
        unit: larrygram_geom::Unit::Inches,
        preserved: Preserved::new(),
    }
}

fn row(x: Option<f64>, y: Option<f64>) -> Row {
    let mut row = Row::new();
    row.row_type = Some("Connection".to_owned());
    if let Some(x) = x {
        row.cells.push(cell("X", x));
    }
    if let Some(y) = y {
        row.cells.push(cell("Y", y));
    }
    row
}

fn connection_section(rows: Vec<Row>) -> Section {
    let mut section = Section::new(SectionName::from_static("Connection"));
    section.rows = rows;
    section
}

fn shape(cells: &[(&str, f64)], sections: Vec<Section>) -> Shape {
    let mut shape = Shape::new(ShapeId { index: 0 }, 1);
    shape.sheet.cells = cells
        .iter()
        .map(|(name, value)| cell(name, *value))
        .collect();
    shape.sheet.sections = sections;
    shape
}

fn on_a_page(document: &mut Document, shape: Shape) -> ShapeId {
    let id = document.add_shape(shape);
    let mut page = Page::new(PageId { index: 0 }, 1);
    page.shapes.push(id);
    document.pages.push(page);
    id
}

fn rectangle() -> stencils::MasterDef {
    stencils::basic_shapes()
        .masters
        .into_iter()
        .find(|master| master.name_u == "Rectangle")
        .expect("the Basic Shapes rectangle")
}

#[test]
fn a_resized_builtin_master_instance_scales_its_points() {
    let mut document = Document::new();
    let instance = stencils::instantiate(&mut document, &rectangle());
    document
        .write_cell(
            SheetRef::Shape(instance),
            &CellName::new("Width"),
            &CellWrite::new("3", CellValue::Number(3.0)),
        )
        .expect("resize width");
    document
        .write_cell(
            SheetRef::Shape(instance),
            &CellName::new("Height"),
            &CellWrite::new("2", CellValue::Number(2.0)),
        )
        .expect("resize height");

    assert_eq!(
        document.connection_points(instance),
        vec![
            Point::new(1.5, 0.0),
            Point::new(3.0, 1.0),
            Point::new(1.5, 2.0),
            Point::new(0.0, 1.0),
        ]
    );
}

#[test]
fn own_rows_win_and_bad_values_are_skipped() {
    let mut document = Document::new();
    let master_shape = document.add_shape(shape(
        &[("Width", 2.0), ("Height", 2.0)],
        vec![connection_section(vec![row(Some(9.0), Some(9.0))])],
    ));
    let mut master = Master::new(MasterId { index: 0 }, 1);
    master.shapes.push(master_shape);
    document.masters.push(master);
    let id = on_a_page(&mut document, {
        let mut shape = shape(
            &[("Width", 4.0), ("Height", 4.0)],
            vec![connection_section(vec![
                row(Some(1.0), Some(2.0)),
                row(Some(f64::NAN), Some(3.0)),
                row(Some(4.0), None),
            ])],
        );
        shape.master = Some(MasterId { index: 0 });
        shape
    });
    assert_eq!(
        document.connection_points(id),
        vec![Point::new(1.0, 2.0)],
        "own rows are unscaled and the master's rows are ignored"
    );
}

#[test]
fn a_missing_shape_or_a_masterless_shape_has_no_points() {
    let document = Document::new();
    assert!(document.connection_points(ShapeId { index: 99 }).is_empty());

    let mut document = Document::new();
    let id = on_a_page(&mut document, shape(&[], Vec::new()));
    assert!(document.connection_points(id).is_empty());
}

#[test]
fn the_nearest_master_rung_with_rows_defines_the_points() {
    let mut document = Document::new();
    let far_shape = document.add_shape(shape(
        &[("Width", 2.0), ("Height", 2.0)],
        vec![connection_section(vec![row(Some(1.0), Some(1.0))])],
    ));
    let mut far = Master::new(MasterId { index: 0 }, 1);
    far.shapes.push(far_shape);
    document.masters.push(far);

    let near_shape = document.add_shape({
        let mut shape = shape(&[("Width", 1.0), ("Height", 1.0)], Vec::new());
        shape.master = Some(MasterId { index: 0 });
        shape
    });
    let mut near = Master::new(MasterId { index: 1 }, 2);
    near.shapes.push(near_shape);
    document.masters.push(near);

    let instance = on_a_page(&mut document, {
        let mut shape = shape(&[("Width", 4.0), ("Height", 4.0)], Vec::new());
        shape.master = Some(MasterId { index: 1 });
        shape
    });
    assert_eq!(
        document.connection_points(instance),
        vec![Point::new(2.0, 2.0)],
        "the far rung's 2 x 2 box scales by the instance's 4 x 4 size"
    );
}

#[test]
fn a_zero_or_missing_master_size_does_not_scale_that_axis() {
    let mut document = Document::new();
    let master_shape = document.add_shape(shape(
        &[("Width", 0.0), ("Height", 1.0)],
        vec![connection_section(vec![row(Some(1.0), Some(2.0))])],
    ));
    let mut master = Master::new(MasterId { index: 0 }, 1);
    master.shapes.push(master_shape);
    document.masters.push(master);
    let instance = on_a_page(&mut document, {
        let mut shape = shape(&[("Width", 4.0), ("Height", 4.0)], Vec::new());
        shape.master = Some(MasterId { index: 0 });
        shape
    });
    assert_eq!(
        document.connection_points(instance),
        vec![Point::new(1.0, 8.0)],
        "the zero width axis stays unscaled"
    );

    let mut document = Document::new();
    let master_shape = document.add_shape(shape(
        &[],
        vec![connection_section(vec![row(Some(1.0), Some(2.0))])],
    ));
    let mut master = Master::new(MasterId { index: 0 }, 1);
    master.shapes.push(master_shape);
    document.masters.push(master);
    let instance = on_a_page(&mut document, {
        let mut shape = shape(&[("Width", 4.0), ("Height", 4.0)], Vec::new());
        shape.master = Some(MasterId { index: 0 });
        shape
    });
    assert_eq!(
        document.connection_points(instance),
        vec![Point::new(1.0, 2.0)],
        "a master without sizes is unscaled"
    );
}

#[test]
fn a_malformed_middle_row_keeps_the_indexes_of_the_good_rows() {
    let mut document = Document::new();
    let mut first = row(Some(1.0), Some(2.0));
    first.ix = Some(0);
    let mut malformed = row(Some(f64::NAN), Some(3.0));
    malformed.ix = Some(1);
    let mut last = row(Some(4.0), Some(5.0));
    last.ix = Some(2);
    let id = on_a_page(
        &mut document,
        shape(&[], vec![connection_section(vec![first, malformed, last])]),
    );
    assert_eq!(
        document.connection_points_indexed(id),
        vec![(0, Point::new(1.0, 2.0)), (2, Point::new(4.0, 5.0))],
        "the skipped row does not renumber the good ones"
    );
    assert_eq!(document.connection_points(id).len(), 2);
}

#[test]
fn rows_without_an_ix_are_named_by_position() {
    let mut document = Document::new();
    let id = on_a_page(
        &mut document,
        shape(
            &[],
            vec![connection_section(vec![
                row(Some(1.0), Some(2.0)),
                row(Some(3.0), Some(4.0)),
            ])],
        ),
    );
    assert_eq!(
        document.connection_points_indexed(id),
        vec![(0, Point::new(1.0, 2.0)), (1, Point::new(3.0, 4.0))]
    );
}
