//! Property tests over the scene builder and the geometry-row mapping
//! (task P3-10, ADR 0027 section 6): bounded random models never panic.
//!
//! The generators are bounded (shapes, nesting, row counts, value
//! ranges) so the tests stay fast; the bounds are below what production
//! code caps (AGENTS.md section 6). Every run builds every page with the
//! background on and pushes the scenes through both backends: a panic
//! fails the test, and an `Err` for a capped raster is a valid outcome.

mod common;

use ferrogram_model::{
    Cell, CellName, CellValue, Document, Master, MasterId, Page, PageId, Row, Section, SectionName,
    Shape, ShapeId, TextContent, TextPart,
};
use ferrogram_render::{BuildOptions, PngOptions, SvgOptions, build_scene, render_png, render_svg};
use proptest::prelude::*;

/// The value of one generated cell.
#[derive(Debug, Clone)]
enum Val {
    /// A cached number.
    Num(f64),
    /// A cached string (some of which are markers like `Themed`).
    Text(String),
}

/// One generated shape: cells, optional geometry rows, optional text,
/// sub-shapes and a master link (possibly dangling).
#[derive(Debug, Clone)]
struct ShapeSpec {
    /// The sheet cells.
    cells: Vec<(String, Val)>,
    /// The geometry rows, when the shape has a `Geometry` section.
    rows: Option<Vec<RowSpec>>,
    /// The text parts, when the shape has text.
    text: Option<Vec<TextPart>>,
    /// The sub-shapes.
    children: Vec<ShapeSpec>,
    /// The master index the shape instances (may dangle).
    master: Option<u32>,
}

/// One generated geometry row.
#[derive(Debug, Clone)]
struct RowSpec {
    /// The row type (`MoveTo`, `RelLineTo`, ..., or one we do not map).
    row_type: String,
    /// The row cells.
    cells: Vec<(String, Val)>,
}

/// One generated page.
#[derive(Debug, Clone)]
struct PageSpec {
    /// The page sheet cells (including `PageWidth`/`PageHeight`).
    cells: Vec<(String, Val)>,
    /// The `Layer` section rows: `(name, visible)`.
    layers: Vec<(String, bool)>,
    /// The background page index (may dangle).
    background: Option<u32>,
    /// The page's top-level shapes.
    shapes: Vec<ShapeSpec>,
}

/// One generated master: its own shapes (which may instance other
/// masters, giving chained rungs and cycles).
#[derive(Debug, Clone)]
struct MasterSpec {
    /// The master's shapes.
    shapes: Vec<ShapeSpec>,
}

/// The whole generated document.
#[derive(Debug, Clone)]
struct DocSpec {
    /// The pages.
    pages: Vec<PageSpec>,
    /// The masters.
    masters: Vec<MasterSpec>,
}

/// A number a cell may cache: bounded, plus a few values that stress
/// overflow paths.
fn arb_number() -> impl Strategy<Value = f64> {
    prop_oneof![
        -1.0e6f64..1.0e6,
        Just(0.0),
        Just(-1.0),
        Just(1.0),
        Just(1.0e30),
        Just(-1.0e30),
    ]
}

/// A string a cell may cache: markers, colours, formula text and junk.
fn arb_text() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(String::new()),
        Just("Themed".to_owned()),
        Just("#123456".to_owned()),
        Just("#zz".to_owned()),
        Just("0;1".to_owned()),
        Just("Calibri".to_owned()),
        Just("NURBS(2, 1, 1, 1, 2, 7.5, 0.5, 1)".to_owned()),
        Just("NURBS(broken".to_owned()),
        Just("POLYLINE(0, 0, 1, 1)".to_owned()),
        Just("1 0; 2 1".to_owned()),
    ]
}

/// One cell value.
fn arb_value() -> impl Strategy<Value = Val> {
    prop_oneof![arb_number().prop_map(Val::Num), arb_text().prop_map(Val::Text)]
}

/// One cell with a name from `names`.
fn arb_cell(names: &'static [&'static str]) -> impl Strategy<Value = (String, Val)> {
    (
        prop::sample::select(names).prop_map(str::to_owned),
        arb_value(),
    )
}

/// The cell names of a geometry row.
const ROW_CELLS: &[&str] = &["X", "Y", "A", "B", "C", "D", "E", "Bow"];

/// Every geometry row type of ADR 0022, plus one we do not map.
const ROW_TYPES: &[&str] = &[
    "MoveTo",
    "LineTo",
    "ArcTo",
    "EllipticalArcTo",
    "NURBSTo",
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
    "BogusRow",
];

/// One geometry row.
fn arb_row() -> impl Strategy<Value = RowSpec> {
    (
        prop::sample::select(ROW_TYPES),
        proptest::collection::vec(arb_cell(ROW_CELLS), 0..6),
    )
        .prop_map(|(row_type, cells)| RowSpec {
            row_type: row_type.to_owned(),
            cells,
        })
}

/// The cell names a shape sheet may carry.
const SHAPE_CELLS: &[&str] = &[
    "PinX",
    "PinY",
    "Width",
    "Height",
    "LocPinX",
    "LocPinY",
    "Angle",
    "FlipX",
    "FlipY",
    "FillPattern",
    "FillForegnd",
    "FillBkgnd",
    "FillForegndTrans",
    "FillGradientEnabled",
    "FillGradientDir",
    "FillGradientAngle",
    "LinePattern",
    "LineWeight",
    "LineColor",
    "LineCap",
    "Rounding",
    "BeginArrow",
    "BeginArrowSize",
    "EndArrow",
    "EndArrowSize",
    "ShapeShdwShow",
    "ShdwPattern",
    "ShdwForegnd",
    "ShapeShdwOffsetX",
    "ShapeShdwOffsetY",
    "ShapeShdwBlur",
    "LayerMember",
    "QuickStyleFillColor",
    "QuickStyleLineColor",
    "VariationColorIndex",
    "Visible",
    "PageWidth",
    "PageHeight",
];

/// One text part: literal text or a run marker.
fn arb_text_part() -> impl Strategy<Value = TextPart> {
    prop_oneof![
        Just(TextPart::Text("hi".to_owned())),
        Just(TextPart::Text("two\r\nlines".to_owned())),
        (0..4u32).prop_map(|ix| TextPart::CharRun { ix }),
        (0..4u32).prop_map(|ix| TextPart::ParaRun { ix }),
        (0..4u32).prop_map(|ix| TextPart::TabsRun { ix }),
        (0..4u32).prop_map(|ix| TextPart::Field { ix }),
    ]
}

/// One shape, recursively bounded.
fn arb_shape(depth: u32) -> impl Strategy<Value = ShapeSpec> {
    let cells = proptest::collection::vec(arb_cell(SHAPE_CELLS), 0..12);
    let rows = proptest::option::of(proptest::collection::vec(arb_row(), 0..8));
    let text = proptest::option::of(proptest::collection::vec(arb_text_part(), 0..6));
    let master = proptest::option::of(0u32..4);
    if depth == 0 {
        (cells, rows, text, master)
            .prop_map(move |(cells, rows, text, master)| ShapeSpec {
                cells,
                rows,
                text,
                children: Vec::new(),
                master,
            })
            .boxed()
    } else {
        (
            cells,
            rows,
            text,
            master,
            proptest::collection::vec(arb_shape(depth - 1), 0..3),
        )
            .prop_map(move |(cells, rows, text, master, children)| ShapeSpec {
                cells,
                rows,
                text,
                children,
                master,
            })
            .boxed()
    }
}

/// One page.
fn arb_page() -> impl Strategy<Value = PageSpec> {
    (
        proptest::collection::vec(arb_cell(SHAPE_CELLS), 0..10),
        proptest::collection::vec((proptest::string::string_regex("[A-Za-z]{0,4}").unwrap(), any::<bool>()), 0..3),
        proptest::option::of(0u32..4),
        proptest::collection::vec(arb_shape(1), 0..4),
    )
        .prop_map(|(cells, layers, background, shapes)| PageSpec {
            cells,
            layers,
            background,
            shapes,
        })
}

/// One master.
fn arb_master() -> impl Strategy<Value = MasterSpec> {
    proptest::collection::vec(arb_shape(1), 0..3).prop_map(|shapes| MasterSpec { shapes })
}

/// The bounded random document.
fn arb_document() -> impl Strategy<Value = DocSpec> {
    (
        proptest::collection::vec(arb_page(), 1..3),
        proptest::collection::vec(arb_master(), 0..3),
    )
        .prop_map(|(pages, masters)| DocSpec { pages, masters })
}

/// One cell from a generated pair.
fn material_cell(name: &str, value: &Val) -> Cell {
    let mut cell = Cell::new(CellName::new(name.to_owned()));
    cell.value = Some(match value {
        Val::Num(number) => CellValue::Number(*number),
        Val::Text(text) => CellValue::Str(text.clone()),
    });
    cell
}

/// Appends the shapes of `specs` to `doc`, wiring parents and children.
fn material_shapes(
    doc: &mut Document,
    specs: &[ShapeSpec],
    parent: Option<ShapeId>,
    out: &mut Vec<ShapeId>,
) {
    for spec in specs {
        let mut shape = Shape::new(ShapeId { index: 0 }, 0);
        shape.parent = parent;
        shape.master = spec.master.map(|index| MasterId { index });
        shape.sheet.cells = spec
            .cells
            .iter()
            .map(|(name, value)| material_cell(name, value))
            .collect();
        if let Some(rows) = &spec.rows {
            let mut section = Section::new(SectionName::new("Geometry"));
            for (position, row) in rows.iter().enumerate() {
                let mut mapped = Row::new();
                mapped.row_type = Some(row.row_type.clone());
                mapped.ix = i32::try_from(position).ok();
                mapped.cells = row
                    .cells
                    .iter()
                    .map(|(name, value)| material_cell(name, value))
                    .collect();
                section.rows.push(mapped);
            }
            shape.sheet.sections.push(section);
        }
        if let Some(parts) = &spec.text {
            shape.text = Some(TextContent {
                parts: parts.clone(),
            });
        }
        let id = doc.add_shape(shape);
        if let Some(parent) = parent {
            if let Some(entity) = doc.shape_mut(parent) {
                entity.children.push(id);
            }
        }
        out.push(id);
        material_shapes(doc, &spec.children, Some(id), out);
    }
}

/// Builds the document of one generated spec.
fn material_document(spec: &DocSpec) -> Document {
    let mut doc = Document::new();
    for master_spec in &spec.masters {
        let mut master = Master::new(MasterId { index: 0 }, 0);
        let mut shapes = Vec::new();
        material_shapes(&mut doc, &master_spec.shapes, None, &mut shapes);
        master.shapes = shapes;
        doc.masters.push(master);
    }
    for (index, page_spec) in spec.pages.iter().enumerate() {
        let id = PageId {
            index: u32::try_from(index).unwrap_or(u32::MAX),
        };
        let mut page = Page::new(id, 0);
        // A dangling or cyclic `BackPage` is part of the stress (ADR
        // 0021: a broken reference warns, it does not panic).
        page.background = page_spec.background.map(|index| PageId { index });
        page.sheet.cells = page_spec
            .cells
            .iter()
            .map(|(name, value)| material_cell(name, value))
            .collect();
        if !page_spec.layers.is_empty() {
            let mut section = Section::new(SectionName::new("Layer"));
            for (position, (name, visible)) in page_spec.layers.iter().enumerate() {
                let mut row = Row::new();
                row.ix = i32::try_from(position).ok();
                row.cells
                    .push(material_cell("Name", &Val::Text(name.clone())));
                row.cells.push(material_cell(
                    "Visible",
                    &Val::Num(if *visible { 1.0 } else { 0.0 }),
                ));
                section.rows.push(row);
            }
            page.sheet.sections.push(section);
        }
        material_shapes(&mut doc, &page_spec.shapes, None, &mut page.shapes);
        doc.pages.push(page);
    }
    doc
}

/// Builds every page of `doc` through both backends; a panic fails.
#[allow(clippy::expect_used)] // test code; a stale page id here is a generator bug
fn exercise(doc: &Document) {
    for index in 0..doc.pages.len() {
        let page = PageId {
            index: u32::try_from(index).expect("page index"),
        };
        let Ok(scene) = build_scene(doc, page, &BuildOptions::new()) else {
            continue;
        };
        let _ = render_png(&scene, &PngOptions::new());
        let _ = render_svg(&scene, &SvgOptions::new());
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 32, ..ProptestConfig::default() })]

    /// A bounded random document builds scenes without panicking.
    #[test]
    fn bounded_random_models_build_scenes_without_panicking(spec in arb_document()) {
        let doc = material_document(&spec);
        exercise(&doc);
    }

    /// Random geometry row sequences map to paths without panicking.
    #[test]
    fn random_geometry_rows_map_to_paths_without_panicking(
        rows in proptest::collection::vec(arb_row(), 0..16)
    ) {
        let spec = DocSpec {
            masters: Vec::new(),
            pages: vec![PageSpec {
                cells: Vec::new(),
                layers: Vec::new(),
                background: None,
                shapes: vec![ShapeSpec {
                    cells: Vec::new(),
                    rows: Some(rows),
                    text: None,
                    children: Vec::new(),
                    master: None,
                }],
            }],
        };
        let doc = material_document(&spec);
        exercise(&doc);
    }
}
