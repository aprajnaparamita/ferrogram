//! Property tests for the package and model round-trips (task P2-10,
//! ADR 0017 section 2).
//!
//! Three properties, generated with bounded trees and documents (the caps of
//! `vsdx::Limits` are what production enforces):
//!
//! 1. A random [`Package`] survives `write_package` -> `read_package` with
//!    semantic equivalence (XML parts under [`diff`], binary parts
//!    byte-equal, same content types and relationships) and the second write
//!    is byte-identical to the first.
//! 2. A random model [`Document`] survives `from_model` -> `to_model`.
//!
//! The third property of ADR 0017 (random `XmlNode` trees survive parse ->
//! serialize -> parse with `diff` empty) lives next to the XML reader and
//! writer it tests, in `src/package/xml.rs`.
//!
//! Generator bounds: the trees and documents are small (a handful of parts,
//! pages, masters and shapes), values are drawn from safe alphabets (no
//! carriage returns: XML normalizes line endings) and references only name
//! entities that exist (a dangling `Master` or `BackPage` is preserved
//! verbatim by design, so it does not round-trip as a reference).

use ferrogram_geom::Unit;
use ferrogram_model::{
    Cell, CellName, CellValue, Color, Connect, Document, DocumentKind, Master, MasterId, Page,
    PageId, Preserved, Property, QName, Row, Section, SectionName, Shape, ShapeId, Style, StyleId,
    XmlAttribute, XmlChild, XmlNode, diff,
};
use ferrogram_vsdx::{
    ContentType, ContentTypeDecl, ContentTypeKind, Limits, Package, Part, PartData, PartName,
    Relationship, RelationshipId, TargetMode, from_model, read_document, read_package, to_model,
    write_document, write_package,
};
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Shared XML tree generation (the preserved tree of ADR 0012)
// ---------------------------------------------------------------------------

const ELEMENT_NAMES: &[&str] = &["a", "b", "Cell", "X9", "_k"];
const ATTRIBUTE_NAMES: &[&str] = &["id", "N", "V", "n2"];
const TEXTS: &[&str] = &["x", "a & b < c>", "0.5", " ", "", "0.5 in", "'q'"];
const COMMENTS: &[&str] = &["note", "c-1", "x & y"];
const PI_TARGETS: &[&str] = &["p1", "t"];
const PI_DATA: &[&str] = &["d", "data x"];

fn arb_qname() -> impl Strategy<Value = QName> {
    proptest::prop_oneof![
        proptest::sample::select(ELEMENT_NAMES.to_vec())
            .prop_map(|local| QName::local(local.to_owned())),
        proptest::sample::select(ELEMENT_NAMES.to_vec()).prop_map(|local| QName {
            prefix: Some("p".to_owned()),
            uri: Some("urn:x".to_owned()),
            local: local.to_owned(),
        }),
    ]
}

fn arb_attributes() -> impl Strategy<Value = Vec<XmlAttribute>> {
    proptest::collection::vec(
        (
            proptest::sample::select(ATTRIBUTE_NAMES.to_vec()),
            proptest::sample::select(TEXTS.to_vec()).prop_map(str::to_owned),
        ),
        0..4,
    )
    .prop_map(|pairs| {
        let mut out: Vec<XmlAttribute> = Vec::new();
        for (name, value) in pairs {
            if out.iter().any(|attribute| attribute.name.local == name) {
                continue;
            }
            out.push(XmlAttribute {
                name: QName::local(name.to_owned()),
                value,
            });
        }
        out
    })
}

fn arb_leaf_child() -> impl Strategy<Value = XmlChild> {
    proptest::prop_oneof![
        proptest::sample::select(TEXTS.to_vec()).prop_map(|text| XmlChild::Text(text.to_owned())),
        proptest::sample::select(COMMENTS.to_vec())
            .prop_map(|comment| XmlChild::Comment(comment.to_owned())),
        (
            proptest::sample::select(PI_TARGETS.to_vec()),
            proptest::sample::select(PI_DATA.to_vec()),
        )
            .prop_map(|(target, data)| XmlChild::ProcessingInstruction {
                target: target.to_owned(),
                data: data.to_owned(),
            }),
    ]
}

/// The children a writer can produce: no two adjacent text nodes (the parser
/// merges them into one).
fn children_from(items: Vec<XmlChild>) -> Vec<XmlChild> {
    let mut out: Vec<XmlChild> = Vec::new();
    for item in items {
        if matches!(item, XmlChild::Text(_)) && matches!(out.last(), Some(XmlChild::Text(_))) {
            continue;
        }
        out.push(item);
    }
    out
}

/// A tree whose root declares the `p` prefix the prefixed names use.
fn arb_tree() -> impl Strategy<Value = XmlNode> {
    let leaf = (
        arb_qname(),
        arb_attributes(),
        proptest::collection::vec(arb_leaf_child(), 0..4),
    )
        .prop_map(|(name, attributes, items)| XmlNode {
            name,
            attributes,
            children: children_from(items),
            self_closing: false,
        });
    leaf.prop_recursive(3, 24, 3, |inner| {
        (
            arb_qname(),
            arb_attributes(),
            proptest::collection::vec(
                proptest::prop_oneof![inner.prop_map(XmlChild::Element), arb_leaf_child()],
                0..5,
            ),
        )
            .prop_map(|(name, attributes, items)| XmlNode {
                name,
                attributes,
                children: children_from(items),
                self_closing: false,
            })
    })
    .prop_map(|mut node| {
        node.attributes.insert(
            0,
            XmlAttribute {
                name: QName {
                    prefix: Some("xmlns".to_owned()),
                    uri: None,
                    local: "p".to_owned(),
                },
                value: "urn:x".to_owned(),
            },
        );
        node
    })
}

// ---------------------------------------------------------------------------
// Property 1: random packages round-trip
// ---------------------------------------------------------------------------

/// The parts a generated package may hold, in zip order.
const PART_NAMES: &[&str] = &[
    "/visio/document.xml",
    "/visio/pages/pages.xml",
    "/visio/pages/page1.xml",
    "/docProps/core.xml",
    "/custom/other.xml",
    "/media/image1.bin",
];

/// The content type of a generated part.
fn part_content_type(name: &str) -> &'static str {
    match name {
        "/visio/document.xml" => "application/vnd.ms-visio.drawing.main+xml",
        "/media/image1.bin" => "application/octet-stream",
        _ => "application/xml",
    }
}

fn arb_parts() -> impl Strategy<Value = Vec<Part>> {
    (
        proptest::sample::subsequence(PART_NAMES, 1..5),
        proptest::collection::vec(arb_tree(), 1..5),
    )
        .prop_map(|(names, trees)| {
            names
                .into_iter()
                .zip(trees.iter().cycle())
                .map(|(name, tree)| Part {
                    name: PartName::new(name),
                    content_type: ContentType::new(part_content_type(name)),
                    data: if name.ends_with(".bin") {
                        PartData::Binary(b"PNG\x89bytes".to_vec())
                    } else {
                        PartData::Xml(tree.clone())
                    },
                })
                .collect()
        })
}

/// The relationship sources, one rels file each.
const REL_SOURCES: &[&str] = &["/", "/visio/document.xml", "/visio/pages/pages.xml"];
const REL_TYPES: &[&str] = &[
    "http://schemas.microsoft.com/office/2006/relationships/document",
    "http://schemas.microsoft.com/office/2006/relationships/pages",
    "http://schemas.microsoft.com/office/2006/relationships/page",
    "http://example.test/relationships/thing",
];
const REL_TARGETS: &[&str] = &["visio/document.xml", "pages/pages.xml", "page1.xml", ""];

/// Relationships grouped by source (the writer's rels files), so the read
/// back order is the generated order.
fn arb_relationships() -> impl Strategy<Value = Vec<Relationship>> {
    proptest::sample::subsequence(REL_SOURCES, 1..4)
        .prop_flat_map(|sources| {
            let group = |source: &'static str| {
                proptest::collection::vec(
                    (
                        proptest::sample::select(REL_TYPES.to_vec()),
                        proptest::sample::select(REL_TARGETS.to_vec()),
                        any::<bool>(),
                    ),
                    1..3,
                )
                .prop_map(move |entries| {
                    entries
                        .into_iter()
                        .enumerate()
                        .map(|(index, (rel_type, target, external))| Relationship {
                            source: PartName::new(source),
                            id: RelationshipId::new(format!("rId{}", index + 1)),
                            rel_type: rel_type.to_owned(),
                            target: target.to_owned(),
                            target_mode: if external {
                                TargetMode::External
                            } else {
                                TargetMode::Internal
                            },
                            preserved: Preserved::new(),
                        })
                        .collect::<Vec<Relationship>>()
                })
            };
            let groups: Vec<_> = sources.into_iter().map(group).collect();
            groups
        })
        .prop_map(|groups: Vec<Vec<Relationship>>| groups.into_iter().flatten().collect())
}

fn arb_package() -> impl Strategy<Value = Package> {
    (arb_parts(), arb_relationships()).prop_map(|(parts, relationships)| {
        let content_types = parts
            .iter()
            .map(|part| ContentTypeDecl {
                kind: ContentTypeKind::Override,
                name: part.name.as_str().to_owned(),
                content_type: part.content_type.as_str().to_owned(),
            })
            .collect();
        Package {
            parts,
            relationships,
            content_types,
        }
    })
}

/// Every difference between a generated package and the package read back
/// from its written bytes, empty when they are semantically equivalent.
fn package_differences(want: &Package, got: &Package) -> Vec<String> {
    let mut out = Vec::new();
    if want.content_types != got.content_types {
        out.push(format!(
            "content types differ: want {:?}, got {:?}",
            want.content_types, got.content_types
        ));
    }
    if want.relationships != got.relationships {
        out.push(format!(
            "relationships differ: want {:?}, got {:?}",
            want.relationships, got.relationships
        ));
    }
    let want_names: Vec<&str> = want.parts.iter().map(|p| p.name.as_str()).collect();
    let got_names: Vec<&str> = got.parts.iter().map(|p| p.name.as_str()).collect();
    if want_names != got_names {
        out.push(format!(
            "part lists differ: want {want_names:?}, got {got_names:?}"
        ));
        return out;
    }
    for (index, want_part) in want.parts.iter().enumerate() {
        let Some(got_part) = got.parts.get(index) else {
            out.push(format!("{} is missing", want_part.name.as_str()));
            continue;
        };
        if want_part.content_type != got_part.content_type {
            out.push(format!(
                "{}: content type differs: want {:?}, got {:?}",
                want_part.name.as_str(),
                want_part.content_type,
                got_part.content_type
            ));
        }
        match (&want_part.data, &got_part.data) {
            (PartData::Xml(a), PartData::Xml(b)) => {
                let differences = diff(a, b).unwrap_or_default();
                for difference in differences {
                    out.push(format!("{}: {difference}", want_part.name.as_str()));
                }
            }
            (PartData::Binary(a), PartData::Binary(b)) => {
                if a != b {
                    out.push(format!("{}: binary bytes differ", want_part.name.as_str()));
                }
            }
            _ => out.push(format!("{}: part kinds differ", want_part.name.as_str())),
        }
    }
    out
}

proptest::proptest! {
    /// ADR 0017: a random `Package` round-trips through `write_package` ->
    /// `read_package` -> `write_package` unchanged (semantic equivalence of
    /// the parts, and byte-identical writer stability).
    #[test]
    fn random_packages_round_trip(package in arb_package()) {
        let bytes = write_package(&package).expect("write_package succeeds");
        let read_back = read_package(&bytes, &Limits::DEFAULT).expect("read_package succeeds");
        let differences = package_differences(&package, &read_back);
        proptest::prop_assert!(
            differences.is_empty(),
            "package round-trip differs: {:#?}",
            differences
        );
        let second = write_package(&read_back).expect("write_package succeeds");
        proptest::prop_assert_eq!(bytes, second);
    }
}

// ---------------------------------------------------------------------------
// Property 2: random model documents round-trip through from_model -> to_model
// ---------------------------------------------------------------------------

const NAMES: &[&str] = &["", "Shape-1", "Box.2", "x"];
const CELL_NAMES: &[&str] = &["PinX", "Width", "LineWeight", "Color"];
const SECTION_NAMES: &[&str] = &["Geometry", "Connection Points", "Character"];
const ROW_NAMES: &[&str] = &["Row_1", "Row_2"];
const ROW_TYPES: &[&str] = &["RelCubBezTo", "InX"];
const FORMULAS: &[&str] = &["2", "0.5 in", "Inh", ""];
const STRINGS: &[&str] = &["Themed", "a b", "not-a-number", "", "2", "0.5", "#4d4d4d"];
const ERRORS: &[&str] = &["#VALUE!", "#REF!"];
const PROPERTY_NAMES: &[&str] = &["title", "subject", "creator"];
const UNITS: &[Unit] = &[
    Unit::Inches,
    Unit::Feet,
    Unit::Millimeters,
    Unit::Points,
    Unit::Degrees,
    Unit::Radians,
    Unit::Percent,
    Unit::Unitsless,
];

/// A cell value with a unit the value can round-trip with: a number carries
/// any unit, every other kind carries [`Unit::Unitsless`] (a `U` code
/// declares the kind, and a written `V` that contradicts it does not read
/// back). `List` is excluded: no `U` code maps back to a list (the formula
/// engine owns it).
fn arb_cell_value() -> impl Strategy<Value = (CellValue, Unit)> {
    proptest::prop_oneof![
        (
            proptest::num::f64::NORMAL,
            proptest::sample::select(UNITS.to_vec())
        )
            .prop_map(|(value, unit)| (CellValue::Number(value), unit)),
        proptest::sample::select(STRINGS.to_vec())
            .prop_map(|text| { (CellValue::Str(text.to_owned()), Unit::Unitsless) }),
        any::<bool>().prop_map(|flag| (CellValue::Bool(flag), Unit::Unitsless)),
        proptest::sample::select(ERRORS.to_vec())
            .prop_map(|text| { (CellValue::Error(text.to_owned()), Unit::Unitsless) }),
        (any::<u8>(), any::<u8>(), any::<u8>())
            .prop_map(|(r, g, b)| (CellValue::Color(Color::new(r, g, b)), Unit::Unitsless)),
    ]
}

fn arb_cell() -> impl Strategy<Value = Cell> {
    (
        proptest::sample::select(CELL_NAMES.to_vec()),
        proptest::sample::select(FORMULAS.to_vec()).prop_map(|text| Some(text.to_owned())),
        proptest::option::of(arb_cell_value()),
    )
        .prop_map(|(name, formula, value)| {
            let mut cell = Cell::new(CellName::new(name.to_owned()));
            cell.formula = formula;
            if let Some((value, unit)) = value {
                // A `U` with no `Unit` spelling lives in the reader's
                // preserved attributes, so a model that a read produced
                // looks exactly like this (`U="BOOL"`, `U="STR"`).
                let code = match &value {
                    CellValue::Bool(_) => Some("BOOL"),
                    CellValue::Str(_) => Some("STR"),
                    _ => None,
                };
                if let Some(code) = code {
                    cell.preserved.attributes.push(XmlAttribute {
                        name: QName::local("U"),
                        value: code.to_owned(),
                    });
                }
                cell.value = Some(value);
                cell.unit = unit;
            }
            cell
        })
}

fn arb_row() -> impl Strategy<Value = Row> {
    (
        proptest::option::of(proptest::sample::select(ROW_NAMES.to_vec())),
        proptest::option::of(proptest::sample::select(ROW_TYPES.to_vec())),
        proptest::option::of(0..5i32),
        proptest::collection::vec(arb_cell(), 0..3),
    )
        .prop_map(|(name, row_type, ix, cells)| {
            let mut row = Row::new();
            row.name = name.map(str::to_owned);
            row.row_type = row_type.map(str::to_owned);
            row.ix = ix;
            row.cells = cells;
            row
        })
}

fn arb_section() -> impl Strategy<Value = Section> {
    (
        proptest::sample::select(SECTION_NAMES.to_vec()),
        proptest::collection::vec(arb_cell(), 0..2),
        proptest::collection::vec(arb_row(), 0..3),
    )
        .prop_map(|(name, cells, rows)| {
            let mut section = Section::new(SectionName::new(name.to_owned()));
            section.cells = cells;
            section.rows = rows;
            section
        })
}

fn arb_sheet() -> impl Strategy<Value = ferrogram_model::Sheet> {
    (
        proptest::collection::vec(arb_cell(), 0..3),
        proptest::collection::vec(arb_section(), 0..2),
    )
        .prop_map(|(cells, sections)| ferrogram_model::Sheet { cells, sections })
}

/// A shape plan: the shape plus the master index it references (bounded by
/// the document plan at assembly time).
#[derive(Debug)]
struct ShapePlan {
    visio_id: i32,
    name: String,
    name_u: String,
    master: Option<usize>,
    line_style: Option<u32>,
    fill_style: Option<u32>,
    text_style: Option<u32>,
    sheet: ferrogram_model::Sheet,
}

fn arb_shape_plan(masters: usize) -> impl Strategy<Value = ShapePlan> {
    (
        0..100i32,
        proptest::sample::select(NAMES.to_vec()),
        proptest::sample::select(NAMES.to_vec()),
        proptest::option::of(0..masters.max(1)),
        proptest::option::of(0..10u32),
        proptest::option::of(0..10u32),
        proptest::option::of(0..10u32),
        arb_sheet(),
    )
        .prop_map(
            move |(visio_id, name, name_u, master, line_style, fill_style, text_style, sheet)| {
                ShapePlan {
                    visio_id,
                    name: name.to_owned(),
                    name_u: name_u.to_owned(),
                    master: if masters == 0 { None } else { master },
                    line_style,
                    fill_style,
                    text_style,
                    sheet,
                }
            },
        )
}

#[derive(Debug)]
struct MasterPlan {
    visio_id: i32,
    name: String,
    name_u: String,
    unique_id: Option<String>,
    prompt: Option<String>,
    hidden: bool,
    shapes: Vec<ShapePlan>,
}

#[derive(Debug)]
struct PagePlan {
    visio_id: i32,
    name: String,
    name_u: String,
    background: Option<usize>,
    sheet: ferrogram_model::Sheet,
    connects: Vec<Connect>,
    shapes: Vec<ShapePlan>,
}

#[derive(Debug)]
struct StylePlan {
    visio_id: u32,
    name: String,
    name_u: String,
    line_style: Option<u32>,
    fill_style: Option<u32>,
    text_style: Option<u32>,
    sheet: ferrogram_model::Sheet,
}

#[derive(Debug)]
struct DocumentPlan {
    kind: DocumentKind,
    masters: Vec<MasterPlan>,
    pages: Vec<PagePlan>,
    styles: Vec<StylePlan>,
    properties: Vec<Property>,
}

fn arb_connect() -> impl Strategy<Value = Connect> {
    (
        0..20i32,
        proptest::sample::select(CELL_NAMES.to_vec()),
        0..4i32,
        0..20i32,
        proptest::sample::select(CELL_NAMES.to_vec()),
        0..4i32,
    )
        .prop_map(
            |(from_sheet, from_cell, from_part, to_sheet, to_cell, to_part)| Connect {
                from_sheet,
                from_cell: from_cell.to_owned(),
                from_part,
                to_sheet,
                to_cell: to_cell.to_owned(),
                to_part,
                preserved: Preserved::new(),
            },
        )
}

fn arb_master_plan() -> impl Strategy<Value = MasterPlan> {
    (
        1..40i32,
        proptest::sample::select(NAMES.to_vec()),
        proptest::sample::select(NAMES.to_vec()),
        proptest::option::of(proptest::sample::select(NAMES.to_vec())),
        proptest::option::of(proptest::sample::select(NAMES.to_vec())),
        any::<bool>(),
        proptest::collection::vec(arb_shape_plan(2), 0..3),
    )
        .prop_map(
            |(visio_id, name, name_u, unique_id, prompt, hidden, shapes)| MasterPlan {
                visio_id: visio_id * 7,
                name: name.to_owned(),
                name_u: name_u.to_owned(),
                unique_id: unique_id.map(str::to_owned),
                prompt: prompt.map(str::to_owned),
                hidden,
                shapes,
            },
        )
}

fn arb_page_plan() -> impl Strategy<Value = PagePlan> {
    (
        1..40i32,
        proptest::sample::select(NAMES.to_vec()),
        proptest::sample::select(NAMES.to_vec()),
        proptest::option::of(0..3usize),
        arb_sheet(),
        proptest::collection::vec(arb_connect(), 0..3),
        proptest::collection::vec(arb_shape_plan(2), 0..3),
    )
        .prop_map(
            |(visio_id, name, name_u, background, sheet, connects, shapes)| PagePlan {
                visio_id: visio_id * 11,
                name: name.to_owned(),
                name_u: name_u.to_owned(),
                background,
                sheet,
                connects,
                shapes,
            },
        )
}

fn arb_style_plan() -> impl Strategy<Value = StylePlan> {
    (
        1..20u32,
        proptest::sample::select(NAMES.to_vec()),
        proptest::sample::select(NAMES.to_vec()),
        proptest::option::of(0..10u32),
        proptest::option::of(0..10u32),
        proptest::option::of(0..10u32),
        arb_sheet(),
    )
        .prop_map(
            |(visio_id, name, name_u, line_style, fill_style, text_style, sheet)| StylePlan {
                visio_id: visio_id * 3,
                name: name.to_owned(),
                name_u: name_u.to_owned(),
                line_style,
                fill_style,
                text_style,
                sheet,
            },
        )
}

fn arb_document_plan() -> impl Strategy<Value = DocumentPlan> {
    (
        proptest::sample::select(vec![
            DocumentKind::Drawing,
            DocumentKind::Stencil,
            DocumentKind::Template,
        ]),
        proptest::collection::vec(arb_master_plan(), 0..3),
        proptest::collection::vec(arb_page_plan(), 0..3),
        proptest::collection::vec(arb_style_plan(), 0..3),
        proptest::collection::vec(
            (
                proptest::sample::select(PROPERTY_NAMES.to_vec()),
                proptest::sample::select(TEXTS.to_vec()),
            ),
            0..3,
        ),
    )
        .prop_map(|(kind, masters, pages, styles, properties)| {
            // A stencil writes no pages part (ADR 0016): its plan has none.
            let pages = if kind == DocumentKind::Stencil {
                Vec::new()
            } else {
                pages
            };
            DocumentPlan {
                kind,
                masters,
                pages,
                styles,
                properties: properties
                    .into_iter()
                    .map(|(name, value)| Property::new(name, value))
                    .collect(),
            }
        })
}

/// Assembles the plan into a document. The arena holds the master shapes
/// first, then the page shapes, in list order — the order `to_model` builds
/// it in.
fn assemble(plan: DocumentPlan) -> Document {
    let mut document = Document::new();
    document.kind = plan.kind;
    for (index, plan) in plan.masters.into_iter().enumerate() {
        let id = MasterId {
            index: u32::try_from(index).unwrap_or(u32::MAX),
        };
        // Visio `ID`s are unique per list: `Master` and `BackPage` references
        // resolve by `ID`, and the reader takes the first match.
        let visio_id = (index as i32 + 1) * 100 + plan.visio_id.rem_euclid(100);
        let mut master = Master::new(id, visio_id);
        master.name = plan.name;
        master.name_u = plan.name_u;
        master.unique_id = plan.unique_id;
        master.prompt = plan.prompt;
        master.hidden = plan.hidden;
        for shape in plan.shapes {
            let shape_id = ShapeId {
                index: u32::try_from(document.shapes.len()).unwrap_or(u32::MAX),
            };
            document.shapes.push(build_shape(shape_id, shape));
            master.shapes.push(shape_id);
        }
        document.masters.push(master);
    }
    for (index, plan) in plan.pages.into_iter().enumerate() {
        let id = PageId {
            index: u32::try_from(index).unwrap_or(u32::MAX),
        };
        let visio_id = (index as i32 + 1) * 100 + plan.visio_id.rem_euclid(100);
        let mut page = Page::new(id, visio_id);
        page.name = plan.name;
        page.name_u = plan.name_u;
        page.background = plan.background.map(|position| PageId {
            index: u32::try_from(position).unwrap_or(u32::MAX),
        });
        page.sheet = plan.sheet;
        page.connects = plan.connects;
        for shape in plan.shapes {
            let shape_id = ShapeId {
                index: u32::try_from(document.shapes.len()).unwrap_or(u32::MAX),
            };
            document.shapes.push(build_shape(shape_id, shape));
            page.shapes.push(shape_id);
        }
        document.pages.push(page);
    }
    for (index, plan) in plan.styles.into_iter().enumerate() {
        let id = StyleId {
            index: u32::try_from(index).unwrap_or(u32::MAX),
        };
        let mut style = Style::new(id);
        style.visio_id = (index as u32 + 1) * 100 + plan.visio_id % 100;
        style.name = plan.name;
        style.name_u = plan.name_u;
        style.line_style = plan.line_style;
        style.fill_style = plan.fill_style;
        style.text_style = plan.text_style;
        style.sheet = plan.sheet;
        document.stylesheet.styles.push(style);
    }
    document.properties = plan.properties;
    // References must name an entity that exists: a dangling `Master` or
    // `BackPage` is preserved verbatim by the reader (by design), so it
    // would not round-trip as a reference.
    for shape in &mut document.shapes {
        if shape
            .master
            .is_some_and(|id| id.index as usize >= document.masters.len())
        {
            shape.master = None;
        }
    }
    let pages = document.pages.len();
    for page in &mut document.pages {
        if page.background.is_some_and(|id| id.index as usize >= pages) {
            page.background = None;
        }
    }
    document
}

fn build_shape(id: ShapeId, plan: ShapePlan) -> Shape {
    let mut shape = Shape::new(id, plan.visio_id);
    shape.name = plan.name;
    shape.name_u = plan.name_u;
    shape.master = plan.master.map(|index| MasterId {
        index: u32::try_from(index).unwrap_or(u32::MAX),
    });
    shape.line_style = plan.line_style;
    shape.fill_style = plan.fill_style;
    shape.text_style = plan.text_style;
    shape.sheet = plan.sheet;
    shape
}

/// Strips the writer's relationship plumbing from a read-back document: a
/// generated `Page` or `Master` element carries a fresh `Rel` child (it names
/// the contents relationship the writer just created), and the reader keeps
/// that element in the entity's preserved content. It is writer furniture,
/// not document content.
fn without_rel_markers(document: &mut Document) {
    for page in &mut document.pages {
        page.preserved
            .children
            .retain(|child| child.name.local != "Rel");
    }
    for master in &mut document.masters {
        master
            .preserved
            .children
            .retain(|child| child.name.local != "Rel");
    }
}

proptest::proptest! {
    /// ADR 0017: a random model document round-trips through `from_model`
    /// -> `to_model`.
    #[test]
    fn random_documents_round_trip_from_model_to_model(plan in arb_document_plan()) {
        let document = assemble(plan);
        let package = from_model(&document).expect("from_model succeeds");
        let read_back = to_model(&package).expect("to_model succeeds");
        let mut normalized = read_back;
        without_rel_markers(&mut normalized);
        proptest::prop_assert_eq!(document, normalized);
    }

    /// The generated package also survives the file level: write it, read it
    /// back and the model is unchanged (the two halves compose).
    #[test]
    fn random_documents_survive_the_written_file(plan in arb_document_plan()) {
        let document = assemble(plan);
        let package = from_model(&document).expect("from_model succeeds");
        let bytes = write_package(&package).expect("write_package succeeds");
        let read_back = read_document(&bytes, &Limits::DEFAULT).expect("read_document succeeds");
        let mut normalized = read_back;
        without_rel_markers(&mut normalized);
        proptest::prop_assert_eq!(document, normalized);
    }
}

// ---------------------------------------------------------------------------
// Relationship edge cases (focus area: relationship edge cases)
// ---------------------------------------------------------------------------

/// The drawing fixture bytes, when the corpus fixtures are present.
fn drawing_package() -> Option<(Document, Package)> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("corpus")
        .join("fixtures")
        .join("minimal-drawing.vsdx");
    let bytes = std::fs::read(path).ok()?;
    let package = read_package(&bytes, &Limits::DEFAULT).ok()?;
    let document = read_document(&bytes, &Limits::DEFAULT).ok()?;
    Some((document, package))
}

#[test]
fn an_external_relationship_is_kept_and_never_dereferenced() {
    let Some((mut document, mut package)) = drawing_package() else {
        eprintln!("skipping: corpus/fixtures is absent; run `cargo xtask corpus`");
        return;
    };
    package.relationships.push(Relationship {
        source: PartName::new("/visio/pages/pages.xml"),
        id: RelationshipId::new("rId99"),
        rel_type: "http://example.test/relationships/thing".to_owned(),
        target: "https://example.test/outside".to_owned(),
        target_mode: TargetMode::External,
        preserved: Preserved::new(),
    });
    document.pages[0].name = "Renamed".to_owned();
    let bytes = write_document(&document, &package).expect("write_document succeeds");
    let written = read_package(&bytes, &Limits::DEFAULT).expect("the written package reads");
    let external = written
        .relationships
        .iter()
        .find(|relationship| relationship.id.as_str() == "rId99")
        .expect("the external relationship is kept");
    assert_eq!(external.target_mode, TargetMode::External);
    assert_eq!(external.target, "https://example.test/outside");
    let model = to_model(&written).expect("to_model succeeds");
    assert_eq!(model.pages[0].name, "Renamed");
}

#[test]
fn absolute_internal_targets_resolve_like_relative_ones() {
    let Some((model, mut package)) = drawing_package() else {
        eprintln!("skipping: corpus/fixtures is absent; run `cargo xtask corpus`");
        return;
    };
    for relationship in &mut package.relationships {
        if relationship.target_mode == TargetMode::Internal {
            relationship.target = absolute_target(relationship);
        }
    }
    let absolute = to_model(&package).expect("to_model succeeds");
    assert_eq!(model, absolute);
}

/// The absolute form of an internal relationship target, resolved against
/// the source part the way `read` resolves it (`/`-rooted, no `..`).
fn absolute_target(relationship: &Relationship) -> String {
    if relationship.target.starts_with('/') {
        return relationship.target.clone();
    }
    let source = relationship.source.as_str().trim_start_matches('/');
    match source.rsplit_once('/') {
        Some((directory, _)) => format!("/{directory}/{}", relationship.target),
        None => format!("/{}", relationship.target),
    }
}
