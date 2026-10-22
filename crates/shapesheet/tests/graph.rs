//! Behavior of the exported dependency-graph interface (task P6-06).
//!
//! These tests pin the public contract of [`DepGraph`] and
//! [`CellAddress`] over the model types. The order, cycle and budget
//! rules are pinned next to the code in `src/graph.rs`, where the test
//! engine lives.

use ferrogram_geom::Unit;
use ferrogram_model::{
    Cell, CellName, Document, Page, PageId, Preserved, Shape, ShapeId, SheetRef,
};
use ferrogram_shapesheet::{CellAddress, DepGraph};

fn cell(name: &str, formula: Option<&str>) -> Cell {
    Cell {
        name: CellName::new(name),
        formula: formula.map(str::to_owned),
        value: None,
        unit: Unit::Unitsless,
        preserved: Preserved::new(),
    }
}

fn shape_with_formulas(formulas: &[(&str, &str)]) -> Document {
    let mut doc = Document::new();
    let mut shape = Shape::new(ShapeId { index: 0 }, 1);
    shape.sheet.cells = formulas
        .iter()
        .map(|(name, formula)| cell(name, Some(formula)))
        .collect();
    let shape = doc.add_shape(shape);
    doc.pages.push(Page::new(PageId { index: 0 }, 1));
    if let Some(page) = doc.pages.first_mut() {
        page.shapes.push(shape);
    }
    doc
}

fn address(name: &str) -> CellAddress {
    CellAddress::new(SheetRef::Shape(ShapeId { index: 0 }), CellName::new(name))
}

#[test]
fn an_address_holds_its_parts() {
    let plain = address("PinX");
    assert_eq!(plain.cell.as_str(), "PinX");
    assert_eq!(plain.section, None);
    assert_eq!(plain.row, None);
    let row_cell = CellAddress::in_row(
        SheetRef::Page(PageId { index: 0 }),
        ferrogram_model::SectionName::from_static("Geometry"),
        ferrogram_model::RowTarget::at(1),
        CellName::from_static("X"),
    );
    assert_eq!(row_cell.cell.as_str(), "X");
    assert!(row_cell.row.is_some());
}

#[test]
fn an_empty_graph_answers_empty_lists() {
    let graph = DepGraph::build(&Document::new()).expect("build");
    let address = address("PinX");
    assert_eq!(graph.dependencies(&address).expect("deps"), Vec::new());
    assert_eq!(graph.dependents(&address).expect("deps"), Vec::new());
    assert_eq!(
        graph
            .evaluation_order(std::slice::from_ref(&address))
            .expect("order"),
        vec![address]
    );
}

#[test]
fn a_build_over_formulas_never_fails_the_document() {
    let doc = shape_with_formulas(&[("Alpha", "Beta+1"), ("Beta", "0")]);
    let graph = DepGraph::build(&doc).expect("build");
    // The edges come from the parser (task P6-02): while it is a stub
    // every formula contributes none, and the build still succeeds.
    assert!(graph.dependencies(&address("Alpha")).is_ok());
    assert!(graph.dependents(&address("Beta")).is_ok());
}

#[test]
fn evaluation_order_covers_the_dirty_set_once() {
    let doc = shape_with_formulas(&[("Alpha", "0")]);
    let graph = DepGraph::build(&doc).expect("build");
    let dirty = [address("Alpha")];
    let order = graph.evaluation_order(&dirty).expect("order");
    assert_eq!(order.len(), 1);
    assert_eq!(order.first(), dirty.first());
}
