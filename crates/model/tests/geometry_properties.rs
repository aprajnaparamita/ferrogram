//! Property tests over the geometry row typing (task P3-11, ADR 0027
//! section 6): random `Geometry` rows with random cells, formulas,
//! `IX` values and master rungs always type into [`GeometryRow`]s without
//! panicking. The row *mapping* is pinned by the unit tests in
//! `src/geometry.rs`; these properties stress the merge and the decoders
//! (`NurbsTo` formula text, `PolylineTo` lists) with adversarial values.

use larrygram_model::{
    Cell, CellName, CellValue, Document, Master, MasterId, Page, PageId, QName, Row, Section,
    SectionName, Shape, ShapeGeometry, ShapeId, XmlAttribute,
};
use proptest::prelude::*;

/// The row types of ADR 0022, a case variant and junk the mapper must
/// reject into `Unknown`.
const ROW_TYPES: &[&str] = &[
    "MoveTo",
    "LineTo",
    "ArcTo",
    "EllipticalArcTo",
    "NURBSTo",
    "NurbsTo",
    "PolylineTo",
    "SplineStart",
    "SplineKnot",
    "Ellipse",
    "InfiniteLine",
    "RelMoveTo",
    "RelLineTo",
    "RelQuadBezTo",
    "RelCubBezTo",
    "RelEllipticalArcTo",
    "nurbsto",
    "",
    "BogusRow",
];

/// The cell names a geometry row may carry.
const ROW_CELLS: &[&str] = &["X", "Y", "A", "B", "C", "D", "E", "Bow", "Zz", ""];

/// The section-level flags of a `Geometry` section.
const SECTION_CELLS: &[&str] = &["NoFill", "NoLine", "NoShow"];

/// Numbers a cell may cache: bounded values plus the extremes a file can
/// spell (`1e999` reads as infinity).
fn arb_number() -> impl Strategy<Value = f64> {
    prop_oneof![
        -1.0e6f64..1.0e6,
        Just(0.0),
        Just(-0.0),
        Just(1.0),
        Just(f64::MAX),
        Just(f64::MIN_POSITIVE),
        Just(f64::INFINITY),
        Just(f64::NEG_INFINITY),
        Just(f64::NAN),
    ]
}

/// Formula text a cell may carry: the documented `NURBS()` and
/// `POLYLINE()` spellings, near-misses, bare point lists and junk.
fn arb_formula() -> impl Strategy<Value = String> {
    prop::sample::select(vec![
        "Inh".to_owned(),
        "=NURBS(2, 3, 0, 0, 0.5, 0.5, 0.25, 2, 1, 1, 0.75, 1)".to_owned(),
        "= nurbs(\"1\", 3, 1, 1, \"0\", \"0\", \"0\", \"1\")".to_owned(),
        "NURBS(1, 3, 0, 0, 0, 0, 0, 1".to_owned(),
        "NURBS(1, 3, 0, 0, 0, 0, 0, 1))".to_owned(),
        "NURBS(1, (3), 0, 0, 0, 0, 0, 1)".to_owned(),
        "NURBS(1, 4294967296, 0, 0, 0, 0, 0, 1)".to_owned(),
        "NURBS(1e999, 3, 0, 0, 0, 0, 0, 1)".to_owned(),
        "NURBS()".to_owned(),
        "NURBS".to_owned(),
        "POLYLINE(1, 1, 0, 0, 1, 1)".to_owned(),
        "POLYLINE(1, 1)".to_owned(),
        "0 0;1 1;2 2".to_owned(),
        "1e999 0".to_owned(),
        "0 0 0".to_owned(),
        "".to_owned(),
        "junk".to_owned(),
        "1 0; 2 x".to_owned(),
    ])
}

/// The value a cell may cache, with its kind varying like the reader's
/// kinds do (`Bool`, `List` and `Error` included).
fn arb_value() -> impl Strategy<Value = CellValue> {
    prop_oneof![
        arb_number().prop_map(CellValue::Number),
        Just(CellValue::Str("Themed".to_owned())),
        Just(CellValue::Str("NURBS(1, 3, 0, 0, 0, 0, 0, 1)".to_owned())),
        Just(CellValue::Str("0 0;1 1".to_owned())),
        any::<bool>().prop_map(CellValue::Bool),
        Just(CellValue::List("0;1".to_owned())),
        Just(CellValue::Error("#VALUE!".to_owned())),
    ]
}

/// One generated cell: name, optional formula and optional cached value.
#[derive(Debug, Clone)]
struct CellSpec {
    /// The cell name.
    name: String,
    /// The formula source text, when the cell has one.
    formula: Option<String>,
    /// The cached value, when the cell has one.
    value: Option<CellValue>,
}

/// One generated geometry row.
#[derive(Debug, Clone)]
struct RowSpec {
    /// The row type (`Row/@T`), when it has one.
    row_type: Option<String>,
    /// The row index (`Row/@IX`), when it has one.
    ix: Option<i32>,
    /// Whether the row is marked deleted (`Del`).
    deleted: bool,
    /// The row's cells.
    cells: Vec<CellSpec>,
}

/// One generated `Geometry` section: rows plus section-level flags.
#[derive(Debug, Clone)]
struct SectionSpec {
    /// The rows.
    rows: Vec<RowSpec>,
    /// The section-level flag cells.
    flags: Vec<CellSpec>,
}

/// One cell.
fn arb_cell(names: &'static [&'static str]) -> impl Strategy<Value = CellSpec> {
    (
        prop::sample::select(names).prop_map(str::to_owned),
        proptest::option::of(arb_formula()),
        proptest::option::of(arb_value()),
    )
        .prop_map(|(name, formula, value)| CellSpec {
            name,
            formula,
            value,
        })
}

/// One geometry row, with a random `IX` (duplicates and negatives
/// included) and occasional `Del` marks.
fn arb_row() -> impl Strategy<Value = RowSpec> {
    (
        proptest::option::of(prop::sample::select(ROW_TYPES).prop_map(str::to_owned)),
        proptest::option::of(any::<i32>()),
        proptest::bool::weighted(0.1),
        proptest::collection::vec(arb_cell(ROW_CELLS), 0..6),
    )
        .prop_map(|(row_type, ix, deleted, cells)| RowSpec {
            row_type,
            ix,
            deleted,
            cells,
        })
}

/// One geometry section.
fn arb_section() -> impl Strategy<Value = SectionSpec> {
    (
        proptest::collection::vec(arb_row(), 0..8),
        proptest::collection::vec(arb_cell(SECTION_CELLS), 0..3),
    )
        .prop_map(|(rows, flags)| SectionSpec { rows, flags })
}

/// Materializes one generated cell.
fn material_cell(spec: &CellSpec) -> Cell {
    let mut cell = Cell::new(CellName::new(spec.name.clone()));
    cell.formula = spec.formula.clone();
    cell.value = spec.value.clone();
    cell
}

/// Materializes one generated row.
fn material_row(spec: &RowSpec) -> Row {
    let mut row = Row::new();
    row.row_type = spec.row_type.clone();
    row.ix = spec.ix;
    row.cells = spec.cells.iter().map(material_cell).collect();
    if spec.deleted {
        row.preserved.attributes.push(attribute("Del", "1"));
    }
    row
}

/// A preserved attribute of a row or section.
fn attribute(local: &str, value: &str) -> XmlAttribute {
    XmlAttribute {
        name: QName {
            prefix: None,
            uri: None,
            local: local.to_owned(),
        },
        value: value.to_owned(),
    }
}

/// Materializes one generated section, with an optional `IX` identity
/// (a preserved attribute, like the reader keeps it).
fn material_section(spec: &SectionSpec, ix: Option<i32>) -> Section {
    let mut section = Section::new(SectionName::new("Geometry"));
    if let Some(ix) = ix {
        section
            .preserved
            .attributes
            .push(attribute("IX", &ix.to_string()));
    }
    section.rows = spec.rows.iter().map(material_row).collect();
    for flag in &spec.flags {
        section.cells.push(material_cell(flag));
    }
    section
}

/// A document with one master carrying `master` and one instance of it
/// carrying `own` (its own section identity `ix`), on one page.
fn document(
    own: &SectionSpec,
    master: Option<&SectionSpec>,
    ix: Option<i32>,
) -> (Document, ShapeId) {
    let mut doc = Document::new();
    if let Some(master_spec) = master {
        let master_shape = doc.add_shape(Shape::new(ShapeId { index: 0 }, 1));
        if let Some(entity) = doc.shape_mut(master_shape) {
            entity
                .sheet
                .sections
                .push(material_section(master_spec, ix));
        }
        let mut entity = Master::new(MasterId { index: 0 }, 1);
        entity.shapes.push(master_shape);
        doc.masters.push(entity);
    }
    let mut shape = Shape::new(ShapeId { index: 0 }, 2);
    shape.master = master.map(|_| MasterId { index: 0 });
    shape.sheet.sections.push(material_section(own, ix));
    let shape = doc.add_shape(shape);
    let page = PageId { index: 0 };
    let mut entity = Page::new(page, 1);
    entity.shapes.push(shape);
    doc.pages.push(entity);
    (doc, shape)
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 64, ..ProptestConfig::default() })]

    /// Random rows with random cells type without panicking: one path
    /// per shape with its own `Geometry` section, its rows a subset of
    /// the union of the rungs' rows.
    #[test]
    fn random_geometry_rows_type_without_panicking(
        own in arb_section(),
        master in proptest::option::of(arb_section()),
        ix in proptest::option::of(any::<i32>()),
    ) {
        let (doc, shape) = document(&own, master.as_ref(), ix);
        let geometry: ShapeGeometry = doc.shape_geometry(shape).expect("types");
        let rows_max = own.rows.len() + master.as_ref().map(|m| m.rows.len()).unwrap_or(0);
        assert!(geometry.paths.len() <= 1, "{:?}", geometry.paths);
        for path in &geometry.paths {
            assert!(path.rows.len() <= rows_max, "{:?}", path.rows);
        }
    }

    /// The same rows under a chained master (a master that instances
    /// another master) also type without panicking.
    #[test]
    fn random_rows_through_a_master_chain_type_without_panicking(
        outer in arb_section(),
        middle in arb_section(),
        own in arb_section(),
    ) {
        let mut doc = Document::new();
        let outer_shape = doc.add_shape(Shape::new(ShapeId { index: 0 }, 1));
        if let Some(entity) = doc.shape_mut(outer_shape) {
            entity.sheet.sections.push(material_section(&outer, None));
        }
        let mut outer_master = Master::new(MasterId { index: 1 }, 1);
        outer_master.shapes.push(outer_shape);
        doc.masters.push(outer_master);

        let mut middle_shape = Shape::new(ShapeId { index: 0 }, 2);
        middle_shape.master = Some(MasterId { index: 1 });
        middle_shape.sheet.sections.push(material_section(&middle, None));
        let middle_shape = doc.add_shape(middle_shape);
        let mut middle_master = Master::new(MasterId { index: 0 }, 2);
        middle_master.shapes.push(middle_shape);
        doc.masters.push(middle_master);

        let mut shape = Shape::new(ShapeId { index: 0 }, 3);
        shape.master = Some(MasterId { index: 0 });
        shape.sheet.sections.push(material_section(&own, None));
        let shape = doc.add_shape(shape);
        let page = PageId { index: 0 };
        let mut entity = Page::new(page, 1);
        entity.shapes.push(shape);
        doc.pages.push(entity);

        let rows_max = outer.rows.len() + middle.rows.len() + own.rows.len();
        let geometry = doc.shape_geometry(shape).expect("types");
        for path in &geometry.paths {
            assert!(path.rows.len() <= rows_max, "{:?}", path.rows);
        }
    }
}
