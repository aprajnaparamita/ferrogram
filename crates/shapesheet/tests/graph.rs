//! Behavior of the exported dependency-graph interface (task P6-06).
//!
//! These tests pin the public contract of [`DepGraph`] and
//! [`CellAddress`] over the model types. The order, cycle and budget
//! rules are pinned next to the code in `src/graph.rs`, where the test
//! engine lives.

use larrygram_geom::Unit;
use larrygram_model::{
    Cell, CellName, Document, Page, PageId, Preserved, Shape, ShapeId, SheetRef,
};
use larrygram_shapesheet::{CellAddress, DepGraph};

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
        larrygram_model::SectionName::from_static("Geometry"),
        larrygram_model::RowTarget::at(1),
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

// ---------------------------------------------------------------------------
// Reference addressing (P6-17, P6-12 review F2 and the coverage audit)
// ---------------------------------------------------------------------------

/// A formula that reads another shape **by name** builds its edge to
/// that shape's cell (`PinX = Beta!Width`).
#[test]
fn a_cross_shape_reference_by_name_builds_its_edge() {
    let mut doc = Document::new();
    let page = doc.add_page("Page-1").expect("page");
    let mut beta = Shape::new(ShapeId { index: 0 }, 2);
    beta.name = String::from("Beta");
    beta.name_u = String::from("Beta");
    beta.sheet.cells = vec![cell("Width", Some("4"))];
    let beta = doc.insert_shape(page, beta, None).expect("insert");
    let mut alpha = Shape::new(ShapeId { index: 0 }, 3);
    alpha.sheet.cells = vec![cell("PinX", Some("Beta!Width*2"))];
    let alpha = doc.insert_shape(page, alpha, None).expect("insert");

    let graph = DepGraph::build(&doc).expect("build");
    let source = CellAddress::new(SheetRef::Shape(alpha), CellName::from_static("PinX"));
    let reads = CellAddress::new(SheetRef::Shape(beta), CellName::from_static("Width"));
    assert!(
        graph.dependencies(&source).expect("deps").contains(&reads),
        "{:?}",
        graph.dependencies(&source)
    );
    assert!(graph.dependents(&reads).expect("deps").contains(&source));
}

/// A bare name that matches a **row cell** (`A = X`, `X` sitting in a
/// geometry row) builds its edge to the row cell's address.
#[test]
fn a_bare_row_cell_reference_builds_its_edge() {
    let mut doc = Document::new();
    let mut shape = Shape::new(ShapeId { index: 0 }, 1);
    {
        let mut section =
            larrygram_model::Section::new(larrygram_model::SectionName::from_static("Geometry"));
        let mut row = larrygram_model::Row::new();
        row.ix = Some(0);
        row.cells = vec![cell("X", Some("1"))];
        section.rows.push(row);
        shape.sheet.sections.push(section);
    }
    shape.sheet.cells = vec![cell("Alpha", Some("X*3"))];
    let shape = doc.add_shape(shape);
    doc.pages.push(Page::new(PageId { index: 0 }, 1));
    if let Some(page) = doc.pages.first_mut() {
        page.shapes.push(shape);
    }

    let graph = DepGraph::build(&doc).expect("build");
    let source = CellAddress::new(SheetRef::Shape(shape), CellName::from_static("Alpha"));
    let reads = CellAddress::in_row(
        SheetRef::Shape(shape),
        larrygram_model::SectionName::from_static("Geometry"),
        larrygram_model::RowTarget {
            position: Some(0),
            ix: Some(0),
        },
        CellName::from_static("X"),
    );
    assert!(
        graph.dependencies(&source).expect("deps").contains(&reads),
        "{:?}",
        graph.dependencies(&source)
    );
}

/// A reference a master defines is edged to the **master rung's** cell
/// (P6-12 review F2): an instance with no own `Character` rows reads
/// `Char.Size` from its master, and an edit of that cell must reach the
/// dependent.
#[test]
fn a_reference_resolved_through_a_master_builds_its_edge() {
    let mut doc = Document::new();
    let page = doc.add_page("Page-1").expect("page");

    // The master: a `Character` section whose first row carries `Size`.
    let mut master_shape = Shape::new(ShapeId { index: 0 }, 10);
    {
        let mut section =
            larrygram_model::Section::new(larrygram_model::SectionName::from_static("Character"));
        let mut row = larrygram_model::Row::new();
        row.ix = Some(0);
        row.cells = vec![cell("Size", Some("5"))];
        section.rows.push(row);
        master_shape.sheet.sections.push(section);
    }
    let master_root = doc.add_shape(master_shape);
    let mut master = larrygram_model::Master::new(larrygram_model::MasterId { index: 0 }, 1);
    master.name = String::from("Badge");
    master.name_u = String::from("Badge");
    master.shapes = vec![master_root];
    let master = doc.add_master(master).expect("master");

    // The instance: no own `Character` rows, `PinX = Char.Size*2`.
    let mut instance = Shape::new(ShapeId { index: 0 }, 11);
    instance.master = Some(master);
    instance.sheet.cells = vec![cell("PinX", Some("Char.Size*2"))];
    let instance = doc.insert_shape(page, instance, None).expect("insert");

    let graph = DepGraph::build(&doc).expect("build");
    let source = CellAddress::new(SheetRef::Shape(instance), CellName::from_static("PinX"));
    let reads = CellAddress::in_row(
        SheetRef::Shape(master_root),
        larrygram_model::SectionName::from_static("Character"),
        larrygram_model::RowTarget {
            position: Some(0),
            ix: Some(0),
        },
        CellName::from_static("Size"),
    );
    assert!(
        graph.dependencies(&source).expect("deps").contains(&reads),
        "{:?}",
        graph.dependencies(&source)
    );
    assert!(graph.dependents(&reads).expect("deps").contains(&source));
}
