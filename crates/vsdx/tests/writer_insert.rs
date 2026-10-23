//! The insert direction of the patch writer (task P5-20, ADR 0018,
//! ADR 0048).
//!
//! The delete direction is covered by `roundtrip_edit.rs`; these tests
//! edit a document whose parts came from a *source tree*, so the patcher
//! must add children the source part lacks (the `sync_children` build
//! closures of `write/patch.rs`): a shape onto a read page, a cell the
//! read shape does not define, a row, a section, a `Connect`, a style, a
//! page and a master. Each case saves, reopens and checks the ADR 0048
//! equality, and the second and third saves are byte-identical (the
//! writer is deterministic).
//!
//! # The ADR 0048 model equality
//!
//! The comparison rules are copied from `apps/larrygram/tests/flowchart.rs`
//! (a test binary cannot import from another test crate): [`normalised`]
//! drops preserved `Rel` children and preserved cell `U` spellings and
//! sorts preserved attributes (they are not content), [`canonical`]
//! renames arena [`ShapeId`]s into document order (the arena is
//! addressing, not content) and [`model_equal`] is the two together.

mod common;

use std::collections::HashMap;

use common::{Fixture, build};
use larrygram_model::mutate::{CellWrite, VIS_BEGIN, VIS_END, VIS_WHOLE_SHAPE};
use larrygram_model::{
    Cell, CellName, CellValue, Connect, Document, Master, MasterId, PageId, Preserved, QName, Row,
    Section, SectionName, Shape, ShapeId, Sheet, Style, StyleId, XmlAttribute, XmlChild, XmlNode,
};
use larrygram_vsdx::{
    ContentType, ContentTypeDecl, ContentTypeKind, Limits, Package, Part, PartData, PartName,
    Relationship, RelationshipId, TargetMode, read_package, to_model, write_document,
};

// ---------------------------------------------------------------------------
// The ADR 0048 comparison (copied from `apps/larrygram/tests/flowchart.rs`)
// ---------------------------------------------------------------------------

/// Sorts the attributes of every preserved element (attribute order is
/// not semantic, ADR 0012).
fn sort_node(node: &mut XmlNode) {
    node.attributes
        .sort_by(|a, b| (&a.name.uri, &a.name.local).cmp(&(&b.name.uri, &b.name.local)));
    for child in &mut node.children {
        if let XmlChild::Element(element) = child {
            sort_node(element);
        }
    }
}

/// Sorts the preserved fragment and drops its `Rel` children (the
/// relationship plumbing; see the module docs).
fn sort_preserved(preserved: &mut Preserved) {
    preserved
        .attributes
        .sort_by(|a, b| (&a.name.uri, &a.name.local).cmp(&(&b.name.uri, &b.name.local)));
    preserved.children.retain(|child| child.name.local != "Rel");
    for child in &mut preserved.children {
        sort_node(child);
    }
}

/// The preserved fragment of one cell: [`sort_preserved`] plus the
/// `U` kind/unit spelling dropped (writer-managed; module docs).
fn sort_cell_preserved(preserved: &mut Preserved) {
    preserved
        .attributes
        .retain(|attribute| attribute.name.local != "U");
    sort_preserved(preserved);
}

/// Sorts the preserved fragments of one sheet.
fn sort_sheet(sheet: &mut Sheet) {
    for cell in &mut sheet.cells {
        sort_cell_preserved(&mut cell.preserved);
    }
    for section in &mut sheet.sections {
        sort_preserved(&mut section.preserved);
        for cell in &mut section.cells {
            sort_cell_preserved(&mut cell.preserved);
        }
        for row in &mut section.rows {
            sort_preserved(&mut row.preserved);
            for cell in &mut row.cells {
                sort_cell_preserved(&mut cell.preserved);
            }
        }
    }
}

/// Normalises a document for the ADR 0048 model comparison: `Preserved`
/// attribute order is not semantic, `Rel` plumbing children and a
/// cell's `U` kind spelling are not content (module docs), and the
/// volatile document properties are not compared.
fn normalised(document: &Document) -> Document {
    let mut document = document.clone();
    sort_preserved(&mut document.preserved);
    for style in &mut document.stylesheet.styles {
        sort_preserved(&mut style.preserved);
        sort_sheet(&mut style.sheet);
    }
    for page in &mut document.pages {
        sort_preserved(&mut page.preserved);
        sort_sheet(&mut page.sheet);
        for connect in &mut page.connects {
            sort_preserved(&mut connect.preserved);
        }
    }
    for master in &mut document.masters {
        sort_preserved(&mut master.preserved);
    }
    for shape in &mut document.shapes {
        sort_preserved(&mut shape.preserved);
        sort_sheet(&mut shape.sheet);
    }
    document.properties.retain(|property| {
        !matches!(
            property.name.as_str(),
            "modified" | "lastModifiedBy" | "revision" | "lastPrinted"
        )
    });
    document
}

/// The reachable shape ids of one tree, in wrapper order (a cyclic
/// child list stops at the first repeat).
fn walk(document: &Document, id: ShapeId, order: &mut Vec<u32>, seen: &mut Vec<u32>) {
    if seen.contains(&id.index) {
        return;
    }
    seen.push(id.index);
    order.push(id.index);
    let children = document.shape(id).map(|shape| shape.children.clone());
    if let Some(children) = children {
        for child in children {
            walk(document, child, order, seen);
        }
    }
}

/// Renames every arena [`ShapeId`] into document order (masters first,
/// then pages, each shape tree in wrapper order; unreachable arena
/// entries keep their relative order at the tail), so two documents
/// that differ only in arena addressing compare equal (module docs).
fn canonical(document: &Document) -> Document {
    let mut document = document.clone();
    let mut order: Vec<u32> = Vec::new();
    let mut seen: Vec<u32> = Vec::new();
    for master in &document.masters {
        for &id in &master.shapes.clone() {
            walk(&document, id, &mut order, &mut seen);
        }
    }
    for page in &document.pages {
        for &id in &page.shapes.clone() {
            walk(&document, id, &mut order, &mut seen);
        }
    }
    for (index, _) in document.shapes.iter().enumerate() {
        let index = u32::try_from(index).unwrap_or(u32::MAX);
        if !seen.contains(&index) {
            order.push(index);
        }
    }
    let map: HashMap<u32, u32> = order
        .iter()
        .enumerate()
        .map(|(new, old)| (*old, u32::try_from(new).unwrap_or(u32::MAX)))
        .collect();
    let rename = |id: ShapeId| ShapeId {
        index: *map.get(&id.index).unwrap_or(&id.index),
    };
    for shape in &mut document.shapes {
        shape.id = rename(shape.id);
        shape.parent = shape.parent.map(rename);
        shape.children = shape.children.iter().copied().map(rename).collect();
    }
    let mut slots: Vec<Option<Shape>> = document.shapes.iter().map(|_| None).collect();
    for shape in document.shapes.drain(..) {
        if let Some(slot) = slots.get_mut(shape.id.index as usize) {
            *slot = Some(shape);
        }
    }
    document.shapes = slots.into_iter().flatten().collect();
    for page in &mut document.pages {
        page.shapes = page.shapes.iter().copied().map(rename).collect();
    }
    for master in &mut document.masters {
        master.shapes = master.shapes.iter().copied().map(rename).collect();
    }
    document
}

/// The model comparison of the ADR 0048 equality (module docs).
fn model_equal(a: &Document, b: &Document) -> bool {
    canonical(&normalised(a)) == canonical(&normalised(b))
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// A cell with a cached number and a literal formula.
#[allow(clippy::expect_used, clippy::panic)] // test helper
fn number_cell(name: &str, value: f64) -> Cell {
    let mut cell = Cell::new(CellName::new(name));
    cell.formula = Some(value.to_string());
    cell.value = Some(CellValue::Number(value));
    cell.unit = larrygram_geom::Unit::Inches;
    cell
}

/// A 2 x 1 rectangle shape sheet with transform cells and one `Geometry`
/// path (the shape to insert; its own Visio `ID` is assigned on insert).
#[allow(clippy::expect_used, clippy::panic)] // test helper
fn rectangle(width: f64, height: f64) -> Shape {
    let mut shape = Shape::new(ShapeId { index: 0 }, 0);
    shape.sheet.cells = vec![
        number_cell("Width", width),
        number_cell("Height", height),
        number_cell("PinX", width * 0.5),
        number_cell("PinY", height * 0.5),
        number_cell("LocPinX", width * 0.5),
        number_cell("LocPinY", height * 0.5),
    ];
    let mut section = Section::new(SectionName::from_static("Geometry"));
    for (index, (x, y)) in [
        (0.0, 0.0),
        (width, 0.0),
        (width, height),
        (0.0, height),
        (0.0, 0.0),
    ]
    .iter()
    .enumerate()
    {
        let mut row = Row::new();
        row.row_type = Some(if index == 0 { "MoveTo" } else { "LineTo" }.to_owned());
        row.ix = Some(i32::try_from(index).expect("index"));
        row.cells = vec![number_cell("X", *x), number_cell("Y", *y)];
        section.rows.push(row);
    }
    shape.sheet.sections.push(section);
    shape
}

/// The template fixture read into a model and its package (one page with
/// one rectangle, one master; `document.xml` has no `StyleSheets` and the
/// page has no `Connects`, so the added style and connect take the
/// create paths).
#[allow(clippy::expect_used)] // test helper
fn read_template() -> (Document, Package) {
    let bytes = build(Fixture::Template);
    let package = read_package(&bytes, &Limits::DEFAULT).expect("read");
    let document = to_model(&package).expect("model");
    (document, package)
}

/// Writes `document` into `package` and re-reads both bytes and model.
#[allow(clippy::expect_used, clippy::panic)] // test helper
fn write_and_read(document: &Document, package: &Package) -> (Vec<u8>, Package, Document) {
    let bytes = write_document(document, package).expect("write");
    let package = read_package(&bytes, &Limits::DEFAULT).expect("read");
    let document = to_model(&package).expect("model");
    (bytes, package, document)
}

// ---------------------------------------------------------------------------
// Goal 1: the insert direction over a read document
// ---------------------------------------------------------------------------

/// Every insert kind in one edit over the read template: a shape onto the
/// read page, a cell the read shape does not define, a row and a section,
/// a `Connect`, a style, a page and a master. The round trip keeps them
/// all (ADR 0048) and stays deterministic.
#[test]
fn inserting_into_a_read_document_round_trips() {
    let (mut document, package) = read_template();
    let page = PageId { index: 0 };
    let rect = document
        .shape_by_visio_id(page, 1)
        .expect("the read rectangle");

    // A shape onto a read page (the `Shapes` wrapper lacks it).
    let added = document
        .insert_shape(page, rectangle(1.0, 1.0), None)
        .expect("shape");
    let added_visio_id = document.shape(added).expect("shape").visio_id;

    // A cell the read shape does not define.
    document
        .write_cell(
            larrygram_model::SheetRef::Shape(rect),
            &CellName::from_static("Angle"),
            &CellWrite::new("0.5", CellValue::Number(0.5)),
        )
        .expect("angle");

    // A cell of a row the read row does not define.
    document
        .write_row_cell(
            larrygram_model::SheetRef::Shape(rect),
            &SectionName::from_static("Geometry"),
            larrygram_model::RowTarget::by_ix(1),
            &CellName::from_static("A"),
            &CellWrite::new("0.25", CellValue::Number(0.25)),
        )
        .expect("row cell");

    // A cell of the read section the section does not define.
    if let Some(section) = document.shape_mut(rect).and_then(|entry| {
        entry
            .sheet
            .sections
            .iter_mut()
            .find(|section| section.name.as_str() == "Geometry")
    }) {
        section.cells.push(number_cell("NoLine", 0.0));
    } else {
        panic!("the read shape has its Geometry section");
    }

    // A row the read section lacks.
    if let Some(section) = document.shape_mut(rect).and_then(|entry| {
        entry
            .sheet
            .sections
            .iter_mut()
            .find(|section| section.name.as_str() == "Geometry")
    }) {
        let mut row = Row::new();
        row.row_type = Some("LineTo".to_owned());
        row.ix = Some(6);
        row.cells = vec![number_cell("X", 0.5), number_cell("Y", 0.5)];
        section.rows.push(row);
    } else {
        panic!("the read shape has its Geometry section");
    }

    // A section the read sheet lacks (with its own row).
    if let Some(sheet) = document.shape_mut(rect).map(|entry| &mut entry.sheet) {
        let mut section = Section::new(SectionName::from_static("Character"));
        let mut row = Row::new();
        row.ix = Some(1);
        row.cells = vec![number_cell("Size", 0.12)];
        section.rows.push(row);
        sheet.sections.push(section);
    } else {
        panic!("the read shape exists");
    }

    // A `Connect` (the read page has no `Connects` wrapper at all).
    let mut begin = Connect::new();
    begin.from_sheet = document.shape(added).expect("added").visio_id;
    begin.from_cell = "BeginX".to_owned();
    begin.from_part = VIS_BEGIN;
    begin.to_sheet = document.shape(rect).expect("rect").visio_id;
    begin.to_cell = "PinX".to_owned();
    begin.to_part = VIS_WHOLE_SHAPE;
    document.add_connect(page, begin).expect("begin");
    let mut end = Connect::new();
    end.from_sheet = document.shape(added).expect("added").visio_id;
    end.from_cell = "EndX".to_owned();
    end.from_part = VIS_END;
    end.to_sheet = document.shape(rect).expect("rect").visio_id;
    end.to_cell = "PinX".to_owned();
    end.to_part = VIS_WHOLE_SHAPE;
    document.add_connect(page, end).expect("end");

    // A style (the read `document.xml` has no `StyleSheets` wrapper).
    let mut style = Style::new(StyleId { index: 0 });
    style.visio_id = 7;
    style.name = "Added".to_owned();
    style.name_u = "Added".to_owned();
    document.stylesheet.styles.push(style);

    // A page (with content, so it gets its contents part and `Rel`).
    let second = document.add_page("Page-2").expect("page");
    document
        .insert_shape(second, rectangle(1.0, 1.0), None)
        .expect("page two shape");

    // A master (one shape of its own).
    let mut master_shape = rectangle(1.0, 1.0);
    master_shape.name_u = "Box".to_owned();
    let content = document.add_shape(master_shape);
    let mut master = Master::new(MasterId { index: 1 }, 0);
    master.name = "Box".to_owned();
    master.name_u = "Box".to_owned();
    master.shapes.push(content);
    document.add_master(master).expect("master");

    // The ADR 0048 round trip over the patched source tree.
    let (first_bytes, first_package, first) = write_and_read(&document, &package);
    let (second_bytes, second_package, second) = write_and_read(&first, &first_package);
    let (third_bytes, _, third) = write_and_read(&second, &second_package);
    assert!(
        model_equal(&first, &second),
        "the first save and re-open keep the model"
    );
    assert!(
        model_equal(&second, &third),
        "a re-opened document keeps its model"
    );
    assert_eq!(second_bytes, third_bytes, "the writer is deterministic");
    assert!(!first_bytes.is_empty());

    // Everything the insertions added is in the re-read model.
    assert_eq!(first.pages.len(), 2);
    assert_eq!(first.pages[1].name, "Page-2");
    assert_eq!(first.pages[0].shapes.len(), 2, "the read shape plus one");
    assert_eq!(first.pages[1].shapes.len(), 1);
    assert_eq!(first.pages[0].connects.len(), 2);
    assert_eq!(first.masters.len(), 2);
    assert_eq!(first.stylesheet.styles.len(), 1);
    assert_eq!(first.stylesheet.styles[0].name_u, "Added");

    let rect = first
        .shape_by_visio_id(page, 1)
        .and_then(|id| first.shape(id))
        .expect("rect");
    let cell = |name: &str| {
        rect.sheet
            .cells
            .iter()
            .find(|cell| cell.name.as_str() == name)
            .and_then(|cell| cell.value.clone())
    };
    assert_eq!(cell("Angle"), Some(CellValue::Number(0.5)), "the new cell");
    let geometry = rect
        .sheet
        .sections
        .iter()
        .find(|section| section.name.as_str() == "Geometry")
        .expect("geometry");
    assert_eq!(geometry.rows.len(), 6, "the added row");
    assert!(
        geometry
            .rows
            .iter()
            .any(|row| row.ix == Some(6) && row.cells.iter().any(|cell| cell.name.as_str() == "X")),
        "the added row is written"
    );
    assert!(
        geometry
            .cells
            .iter()
            .any(|cell| cell.name.as_str() == "NoLine"),
        "the added section cell is written"
    );
    assert!(
        geometry
            .rows
            .iter()
            .any(|row| row.ix == Some(1) && row.cells.iter().any(|cell| cell.name.as_str() == "A")),
        "the added row cell is written"
    );
    let character = rect
        .sheet
        .sections
        .iter()
        .find(|section| section.name.as_str() == "Character")
        .expect("the added section");
    assert_eq!(character.rows.len(), 1, "the added section keeps its row");

    let added_shape = first
        .shape_by_visio_id(page, added_visio_id)
        .and_then(|id| first.shape(id))
        .expect("the added shape");
    assert_eq!(added_shape.visio_id, added_visio_id);
}

// ---------------------------------------------------------------------------
// Goal 1: the build closures and the insertion ranks
// ---------------------------------------------------------------------------

/// An element with these attributes and children (the `write_edits.rs`
/// helper).
fn element(local: &str, attrs: &[(&str, &str)], children: Vec<XmlNode>) -> XmlNode {
    XmlNode {
        name: QName {
            prefix: None,
            uri: Some("urn:visio".to_owned()),
            local: local.to_owned(),
        },
        attributes: attrs
            .iter()
            .map(|(name, value)| XmlAttribute {
                name: QName::local(*name),
                value: (*value).to_owned(),
            })
            .collect(),
        children: children.into_iter().map(XmlChild::Element).collect(),
        self_closing: false,
    }
}

/// A named cell element with a cached value.
fn cell_element(name: &str, value: &str) -> XmlNode {
    element("Cell", &[("N", name), ("V", value)], Vec::new())
}

/// A minimal source package whose parts hold children the model will not
/// map (the `Note` in the `Shapes` wrapper, a `Rel` that names no
/// relationship, an `Icon`) and wrappers that exist but lack the entities
/// the model adds (the `Connects` and `StyleSheets` build closures, the
/// `page_rank` insert).
fn ranked_source() -> Package {
    let mut package = Package::new();
    package.content_types.push(ContentTypeDecl {
        kind: ContentTypeKind::Default,
        name: "xml".to_owned(),
        content_type: "application/xml".to_owned(),
    });
    let document = element(
        "VisioDocument",
        &[("xmlns", "urn:visio")],
        vec![
            element("DocumentSettings", &[], Vec::new()),
            element(
                "StyleSheets",
                &[],
                vec![element(
                    "StyleSheet",
                    &[("ID", "7"), ("Name", "Base"), ("NameU", "Base")],
                    Vec::new(),
                )],
            ),
        ],
    );
    let pages = element(
        "Pages",
        &[("xmlns", "urn:visio")],
        vec![
            element(
                "Page",
                &[("ID", "0"), ("NameU", "Page-1"), ("Name", "Page-1")],
                vec![
                    element("PageSheet", &[], vec![cell_element("PageWidth", "8.5")]),
                    element("Rel", &[("id", "rId1")], Vec::new()),
                ],
            ),
            // A page whose only `Rel`-shaped child names no relationship
            // (no `id` attribute): it has no contents part and gains one
            // below, and the stray `Rel` holds the `Rel` rank of the
            // `page_rank` insert.
            element(
                "Page",
                &[("ID", "1"), ("NameU", "Page-2"), ("Name", "Page-2")],
                vec![
                    element("PageSheet", &[], vec![cell_element("PageWidth", "8.5")]),
                    element("Rel", &[], Vec::new()),
                    element("Icon", &[("IX", "0")], Vec::new()),
                ],
            ),
        ],
    );
    let shape = element(
        "Shape",
        &[("ID", "1"), ("Type", "Shape")],
        vec![
            cell_element("Width", "2"),
            element(
                "Section",
                &[("N", "Geometry")],
                vec![element(
                    "Row",
                    &[("T", "MoveTo"), ("IX", "1")],
                    vec![cell_element("X", "0"), cell_element("Y", "0")],
                )],
            ),
        ],
    );
    let page1 = element(
        "PageContents",
        &[("xmlns", "urn:visio")],
        vec![
            element(
                "Shapes",
                &[],
                vec![
                    shape,
                    // Unmapped content of the wrapper: it must survive
                    // the patch and keep its place after the shapes.
                    element("Note", &[("V", "kept")], Vec::new()),
                ],
            ),
            element(
                "Connects",
                &[],
                vec![element(
                    "Connect",
                    &[
                        ("FromSheet", "1"),
                        ("FromCell", "BeginX"),
                        ("FromPart", "9"),
                        ("ToSheet", "1"),
                        ("ToCell", "PinX"),
                        ("ToPart", "1"),
                    ],
                    Vec::new(),
                )],
            ),
        ],
    );
    for (name, root) in [
        ("/visio/document.xml", document),
        ("/visio/pages/pages.xml", pages),
        ("/visio/pages/page1.xml", page1),
    ] {
        package.parts.push(Part {
            name: PartName::new(name),
            content_type: ContentType::new("application/xml"),
            data: PartData::Xml(root),
        });
    }
    for (source, id, rel_type, target) in [
        ("/", "rId1", "document", "visio/document.xml"),
        ("/visio/document.xml", "rId1", "pages", "pages/pages.xml"),
        ("/visio/pages/pages.xml", "rId1", "page", "page1.xml"),
    ] {
        package.relationships.push(Relationship {
            source: PartName::new(source),
            id: RelationshipId::new(id),
            rel_type: format!("http://schemas.microsoft.com/visio/2010/relationships/{rel_type}"),
            target: target.to_owned(),
            target_mode: TargetMode::Internal,
            preserved: Preserved::new(),
        });
    }
    package
}

/// The `V` attribute of the cell named `name` in `element`, if present.
fn attr(element: &XmlNode, name: &str) -> Option<String> {
    element
        .attributes
        .iter()
        .find(|attribute| attribute.name.local == name)
        .map(|attribute| attribute.value.clone())
}

/// The child elements of `element` with this local name.
fn children_named<'n>(element: &'n XmlNode, local: &str) -> Vec<&'n XmlNode> {
    element
        .children
        .iter()
        .filter_map(|child| match child {
            XmlChild::Element(node) if node.name.local == local => Some(node),
            _ => None,
        })
        .collect()
}

/// The root element of an XML part.
#[allow(clippy::panic, clippy::expect_used)] // test helper
fn root<'p>(package: &'p Package, name: &str) -> &'p XmlNode {
    match &package.part(&PartName::new(name)).expect("part").data {
        PartData::Xml(root) => root,
        _ => panic!("part is not XML"),
    }
}

/// The build closures that add a child to a wrapper that already exists
/// (`Connect`, `StyleSheet`, `Shape`) and the insertion ranks of a `Page`
/// child (`PageSheet`, `Rel`, `Icon`): the model adds what the source
/// parts lack and the unmapped content keeps its place.
#[test]
fn the_patcher_adds_the_children_the_source_parts_lack() {
    let package = ranked_source();
    let mut document = to_model(&package).expect("model");
    assert_eq!(document.pages.len(), 2);
    assert_eq!(document.pages[0].shapes.len(), 1);
    assert_eq!(document.pages[0].connects.len(), 1);
    assert_eq!(document.stylesheet.styles.len(), 1);

    // A second shape in the mapped page: inserted before the wrapper's
    // unmapped `Note` (`shapes_rank`: `Shape` first, everything else
    // after).
    let first = document.pages[0].shapes[0];
    let second = document
        .insert_shape(PageId { index: 0 }, rectangle(1.0, 1.0), None)
        .expect("shape");
    let second_visio_id = document.shape(second).expect("shape").visio_id;

    // A second connect in the mapped `Connects` wrapper.
    let mut connect = Connect::new();
    connect.from_sheet = second_visio_id;
    connect.from_cell = "BeginX".to_owned();
    connect.from_part = VIS_BEGIN;
    connect.to_sheet = document.shape(first).expect("first").visio_id;
    connect.to_cell = "PinX".to_owned();
    connect.to_part = VIS_WHOLE_SHAPE;
    document
        .add_connect(PageId { index: 0 }, connect)
        .expect("connect");

    // A second style in the mapped `StyleSheets` wrapper.
    let mut style = Style::new(StyleId { index: 1 });
    style.visio_id = 9;
    style.name = "Added".to_owned();
    style.name_u = "Added".to_owned();
    document.stylesheet.styles.push(style);

    // Content on the `Rel`-less second page: it gains a contents part
    // and the `Rel` child (`page_rank`: after `PageSheet` and the stale
    // `Rel`, before the `Icon`).
    document
        .insert_shape(PageId { index: 1 }, rectangle(1.0, 1.0), None)
        .expect("page two shape");

    let (_, reread, second) = write_and_read(&document, &package);

    // The unmapped wrapper child survives and stays after the shapes.
    let contents = root(&reread, "/visio/pages/page1.xml");
    let wrapper = children_named(contents, "Shapes")
        .into_iter()
        .next()
        .expect("Shapes wrapper");
    let kinds: Vec<&str> = wrapper
        .children
        .iter()
        .filter_map(|child| match child {
            XmlChild::Element(element) => Some(element.name.local.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(kinds, vec!["Shape", "Shape", "Note"], "{kinds:?}");

    // Both connects are in the mapped wrapper.
    let connects = children_named(contents, "Connects")
        .into_iter()
        .next()
        .expect("Connects wrapper");
    assert_eq!(children_named(connects, "Connect").len(), 2);

    // Both styles are in the mapped wrapper.
    let main = root(&reread, "/visio/document.xml");
    let sheets = children_named(main, "StyleSheets")
        .into_iter()
        .next()
        .expect("StyleSheets wrapper");
    let styles = children_named(sheets, "StyleSheet");
    assert_eq!(styles.len(), 2);
    assert_eq!(attr(styles[0], "ID").as_deref(), Some("7"));
    assert_eq!(attr(styles[1], "ID").as_deref(), Some("9"));

    // The `Rel`-less page gained its `Rel` at the `page_rank` position:
    // after `PageSheet` and the stray `Rel`, before the `Icon`, which
    // keeps its place at the tail.
    let pages = root(&reread, "/visio/pages/pages.xml");
    let page_elements = children_named(pages, "Page");
    let page = page_elements.get(1).expect("the second page");
    let order: Vec<&str> = page
        .children
        .iter()
        .filter_map(|child| match child {
            XmlChild::Element(element) => Some(element.name.local.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(order, vec!["PageSheet", "Rel", "Rel", "Icon"], "{order:?}");
    let rels = children_named(page, "Rel");
    assert_eq!(rels.len(), 2);
    assert!(attr(rels[0], "id").is_none(), "the stray `Rel` is kept");
    assert!(
        attr(rels[1], "id").is_some(),
        "the fresh `Rel` names the contents part"
    );
    assert!(
        reread
            .part(&PartName::new("/visio/pages/page2.xml"))
            .is_some(),
        "the page gained its contents part"
    );

    // The re-read model sees everything the patcher added.
    assert_eq!(second.pages[0].shapes.len(), 2, "the read shape plus one");
    assert_eq!(second.pages[0].connects.len(), 2);
    assert_eq!(
        second.pages[1].shapes.len(),
        1,
        "the new page keeps content"
    );
    assert_eq!(second.stylesheet.styles.len(), 2);

    // The whole thing round-trips (ADR 0048) and stays deterministic.
    let (third_bytes, third_package, third) = write_and_read(&second, &reread);
    let (fourth_bytes, _, _) = write_and_read(&third, &third_package);
    assert!(model_equal(&second, &third));
    assert_eq!(third_bytes, fourth_bytes);
}
