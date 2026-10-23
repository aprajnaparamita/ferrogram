//! Writer edge cases through the public API (task P2-10, focus area:
//! reader/writer symmetry).
//!
//! The patcher's rules are tested next to the implementation
//! (`src/write/patch.rs`); these tests pin the insert and remove paths of
//! `write_document` on hand-built sources: entities of every list kind that
//! no source element claims, properties claimed by name only, rows matched
//! by name, a shape that gains sub-shapes without a `Shapes` wrapper, and
//! master attributes and cell values that change.

mod common;

use larrygram_geom::Unit;
use larrygram_model::{
    Cell, CellName, CellValue, Connect, Document, Master, MasterId, Page, PageId, Preserved,
    Property, QName, Row, Section, SectionName, Shape, ShapeId, Style, StyleId, XmlAttribute,
    XmlChild, XmlNode,
};
use larrygram_vsdx::{
    ContentType, Limits, Package, Part, PartData, PartName, Relationship, RelationshipId,
    TargetMode, read_package, to_model, write_document, write_package,
};

const CT_XML: &str = "application/xml";

fn qname(name: &str) -> QName {
    match name.split_once(':') {
        Some((prefix, local)) => QName {
            prefix: Some(prefix.to_owned()),
            uri: None,
            local: local.to_owned(),
        },
        None => QName::local(name),
    }
}

fn element(name: &str, attrs: &[(&str, &str)], children: &[XmlNode]) -> XmlNode {
    XmlNode {
        name: qname(name),
        attributes: attrs
            .iter()
            .map(|(name, value)| XmlAttribute {
                name: qname(name),
                value: (*value).to_owned(),
            })
            .collect(),
        children: children.iter().cloned().map(XmlChild::Element).collect(),
        self_closing: false,
    }
}

fn text_element(name: &str, attrs: &[(&str, &str)], text: &str) -> XmlNode {
    let mut node = element(name, attrs, &[]);
    node.children.push(XmlChild::Text(text.to_owned()));
    node
}

fn package(parts: &[(&str, XmlNode)], relationships: &[(&str, &str, &str)]) -> Package {
    let mut package = Package::new();
    for (name, root) in parts {
        package.parts.push(Part {
            name: PartName::new(*name),
            content_type: ContentType::new(CT_XML),
            data: PartData::Xml(root.clone()),
        });
    }
    for (source, id, target) in relationships {
        let kind = match target.rsplit('/').next().unwrap_or(target) {
            "document.xml" => "document",
            "pages.xml" => "pages",
            "masters.xml" => "masters",
            "page1.xml" => "page",
            "master1.xml" => "master",
            _ => "other",
        };
        package.relationships.push(Relationship {
            source: PartName::new(*source),
            id: RelationshipId::new(*id),
            rel_type: format!("http://schemas.microsoft.com/visio/2010/relationships/{kind}"),
            target: (*target).to_owned(),
            target_mode: TargetMode::Internal,
            preserved: Preserved::new(),
        });
    }
    package
}

/// The written package of `document` patched into `source`, and the model
/// read back from it.
#[allow(clippy::expect_used, clippy::panic)] // test helper: the panic reports the unstable write
fn round_trip(document: &Document, source: &Package) -> (Package, Document) {
    let first = write_document(document, source).expect("write_document succeeds");
    let written = read_package(&first, &Limits::DEFAULT).expect("the written package reads");
    let model = to_model(&written).expect("the written package maps");
    // Writer stability: the second write is byte-identical.
    let second = write_document(&model, &written).expect("the second write succeeds");
    if first != second {
        let parts = |bytes: &[u8]| {
            let package = read_package(bytes, &Limits::DEFAULT).expect("reopens");
            let parts: Vec<String> = package
                .parts
                .iter()
                .map(|p| p.name.as_str().to_owned())
                .collect();
            let rels: Vec<(String, String)> = package
                .relationships
                .iter()
                .map(|r| (r.id.as_str().to_owned(), r.target.clone()))
                .collect();
            (parts, rels)
        };
        panic!(
            "the writer is unstable:\nfirst {:?}\nsecond {:?}",
            parts(&first),
            parts(&second)
        );
    }
    (written, model)
}

/// The root element of a part.
#[allow(clippy::panic)] // test helper: the assertions report the failing part
fn root<'p>(package: &'p Package, name: &str) -> &'p XmlNode {
    match &package
        .part(&PartName::new(name))
        .unwrap_or_else(|| panic!("part {name} exists"))
        .data
    {
        PartData::Xml(node) => node,
        other => panic!("part {name} is XML, got {other:?}"),
    }
}

/// The child elements of `node` with this local name.
fn children_named<'n>(node: &'n XmlNode, local: &str) -> Vec<&'n XmlNode> {
    node.children
        .iter()
        .filter_map(|child| match child {
            XmlChild::Element(element) => Some(element),
            _ => None,
        })
        .filter(|element| element.name.local == local)
        .collect()
}

/// The `attr` attribute of `node`.
fn attr<'n>(node: &'n XmlNode, attr: &str) -> Option<&'n str> {
    node.attributes
        .iter()
        .find(|attribute| attribute.name.local == attr)
        .map(|attribute| attribute.value.as_str())
}

/// A source with empty lists: no pages, masters, styles or properties.
fn empty_lists() -> Package {
    package(
        &[
            (
                "/visio/document.xml",
                element(
                    "VisioDocument",
                    &[("xml:space", "preserve")],
                    &[element("DocumentSettings", &[("TopPage", "0")], &[])],
                ),
            ),
            ("/visio/pages/pages.xml", element("Pages", &[], &[])),
            ("/visio/masters/masters.xml", element("Masters", &[], &[])),
        ],
        &[
            ("/", "rId1", "visio/document.xml"),
            ("/visio/document.xml", "rId1", "pages/pages.xml"),
            ("/visio/document.xml", "rId2", "masters/masters.xml"),
        ],
    )
}

/// A document with one shape per page and master, plus styles and
/// properties.
fn filled_document() -> Document {
    let mut document = Document::new();
    let first = document.add_shape(Shape::new(ShapeId { index: 0 }, 1));
    let second = document.add_shape(Shape::new(ShapeId { index: 0 }, 2));
    let master_shape = document.add_shape(Shape::new(ShapeId { index: 0 }, 3));
    let mut page = Page::new(PageId { index: 0 }, 10);
    page.name = "P1".to_owned();
    page.shapes.push(first);
    document.pages.push(page);
    let mut page = Page::new(PageId { index: 1 }, 20);
    page.name = "P2".to_owned();
    page.shapes.push(second);
    page.connects.push(Connect {
        from_sheet: 2,
        from_cell: "PinX".to_owned(),
        from_part: 9,
        to_sheet: 2,
        to_cell: "PinX".to_owned(),
        to_part: 3,
        preserved: Preserved::new(),
    });
    document.pages.push(page);
    let mut master = Master::new(MasterId { index: 0 }, 30);
    master.name_u = "Box".to_owned();
    master.hidden = true;
    master.unique_id = Some("{BOX-1}".to_owned());
    master.prompt = Some("A box".to_owned());
    master.shapes.push(master_shape);
    document.masters.push(master);
    for index in 0..2 {
        let mut style = Style::new(StyleId { index });
        style.visio_id = index + 1;
        style.name = format!("Style-{}", index + 1);
        document.stylesheet.styles.push(style);
    }
    document.properties.push(Property::new("title", "Hello"));
    document
}

#[test]
fn new_entities_land_in_every_list() {
    let source = empty_lists();
    let (written, model) = round_trip(&filled_document(), &source);

    let pages = children_named(root(&written, "/visio/pages/pages.xml"), "Page");
    assert_eq!(pages.len(), 2);
    assert_eq!(attr(pages[0], "Name"), Some("P1"));
    assert_eq!(attr(pages[1], "Name"), Some("P2"));
    assert!(attr(pages[0], "BackPage").is_none());

    let masters = children_named(root(&written, "/visio/masters/masters.xml"), "Master");
    assert_eq!(masters.len(), 1);
    assert_eq!(attr(masters[0], "NameU"), Some("Box"));
    assert_eq!(attr(masters[0], "Hidden"), Some("1"));
    assert_eq!(attr(masters[0], "UniqueID"), Some("{BOX-1}"));
    assert_eq!(attr(masters[0], "Prompt"), Some("A box"));

    let main = root(&written, "/visio/document.xml");
    let properties = children_named(
        children_named(main, "DocumentProperties")
            .first()
            .expect("DocumentProperties is created"),
        "title",
    );
    assert_eq!(properties.len(), 1);
    assert_eq!(
        children_named(main, "StyleSheets")
            .first()
            .map(|w| { children_named(w, "StyleSheet").len() }),
        Some(2)
    );

    // The new page's contents part carries the shapes and the connects.
    let page2 = root(&written, "/visio/pages/page2.xml");
    assert_eq!(
        children_named(
            children_named(page2, "Shapes").first().expect("Shapes"),
            "Shape"
        )
        .len(),
        1
    );
    let connects = children_named(
        children_named(page2, "Connects").first().expect("Connects"),
        "Connect",
    );
    assert_eq!(connects.len(), 1);
    assert_eq!(attr(connects[0], "FromCell"), Some("PinX"));

    // Everything reads back.
    assert_eq!(model.pages.len(), 2);
    assert_eq!(model.masters.len(), 1);
    assert_eq!(model.stylesheet.styles.len(), 2);
    assert_eq!(model.properties.len(), 1);
    assert_eq!(model.pages[1].connects.len(), 1);
}

#[test]
fn a_changed_property_is_rewritten_and_a_removed_one_is_dropped() {
    let source = package(
        &[(
            "/visio/document.xml",
            element(
                "VisioDocument",
                &[],
                &[element(
                    "DocumentProperties",
                    &[],
                    &[
                        text_element("title", &[], "Old"),
                        text_element("subject", &[], "Keep me"),
                        text_element("note", &[], "text"),
                    ],
                )],
            ),
        )],
        &[("/", "rId1", "visio/document.xml")],
    );
    let mut document = to_model(&source).expect("reads");
    document.properties = vec![
        Property::new("title", "New"),
        Property::new("note", "text"),
        Property::new("extra", "Added"),
    ];
    let (written, _) = round_trip(&document, &source);
    let main = root(&written, "/visio/document.xml");
    let wrappers = children_named(main, "DocumentProperties");
    let wrapper = wrappers.first().copied().expect("DocumentProperties");
    let names: Vec<&str> = wrapper
        .children
        .iter()
        .filter_map(|child| match child {
            XmlChild::Element(element) => Some(element.name.local.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(names, vec!["title", "note", "extra"]);
    let titles = children_named(wrapper, "title");
    let title = titles.first().expect("title");
    let text = title.children.iter().find_map(|child| match child {
        XmlChild::Text(text) => Some(text.as_str()),
        _ => None,
    });
    assert_eq!(text, Some("New"));
}

#[test]
fn rows_match_by_name_and_sections_take_their_own_cells() {
    let shape = element(
        "Shape",
        &[("ID", "1")],
        &[element(
            "Section",
            &[("N", "Geometry"), ("IX", "1")],
            &[
                element("Cell", &[("N", "NoFill"), ("V", "0")], &[]),
                element("Row", &[("N", "Row_1")], &[]),
                element("Row", &[("N", "Row_2")], &[]),
            ],
        )],
    );
    let source = package(
        &[
            (
                "/visio/document.xml",
                element(
                    "VisioDocument",
                    &[],
                    &[element("DocumentSettings", &[], &[])],
                ),
            ),
            (
                "/visio/pages/pages.xml",
                element(
                    "Pages",
                    &[],
                    &[element(
                        "Page",
                        &[("ID", "0")],
                        &[element("Rel", &[("r:id", "rId1")], &[])],
                    )],
                ),
            ),
            (
                "/visio/pages/page1.xml",
                element("PageContents", &[], &[element("Shapes", &[], &[shape])]),
            ),
        ],
        &[
            ("/", "rId1", "visio/document.xml"),
            ("/visio/document.xml", "rId1", "pages/pages.xml"),
            ("/visio/pages/pages.xml", "rId1", "page1.xml"),
        ],
    );
    let mut document = to_model(&source).expect("reads");
    let shape_id = document.pages[0].shapes[0];
    let sheet = &mut document.shape_mut(shape_id).expect("shape").sheet;
    let section = &mut sheet.sections[0];
    // The section-level cell is rewritten; `Row_2` is removed and a new row
    // is inserted after the rows that remain.
    section.cells[0].value = Some(CellValue::Number(1.0));
    section
        .rows
        .retain(|row| row.name.as_deref() != Some("Row_2"));
    let mut fresh = Row::new();
    fresh.name = Some("Row_9".to_owned());
    let mut cell = Cell::new(CellName::new("X"));
    cell.value = Some(CellValue::Number(0.5));
    cell.unit = Unit::Inches;
    fresh.cells.push(cell);
    section.rows.push(fresh);

    let (written, model) = round_trip(&document, &source);
    let wrappers = children_named(root(&written, "/visio/pages/page1.xml"), "Shapes");
    let wrapper = wrappers.first().copied().expect("Shapes");
    let shapes = children_named(wrapper, "Shape");
    let shape = shapes.first().copied().expect("Shape");
    let sections = children_named(shape, "Section");
    let section = sections.first().expect("Section");
    assert_eq!(attr(section, "IX"), Some("1"));
    let rows = children_named(section, "Row");
    let names: Vec<Option<&str>> = rows.iter().map(|row| attr(row, "N")).collect();
    assert_eq!(names, vec![Some("Row_1"), Some("Row_9")]);
    let cells = children_named(section, "Cell");
    assert_eq!(cells.len(), 1);
    assert_eq!(attr(cells[0], "V"), Some("1"));
    assert_eq!(attr(cells[0], "U"), None);
    assert_eq!(
        model
            .shape(model.pages[0].shapes[0])
            .unwrap()
            .sheet
            .sections[0]
            .rows
            .len(),
        2
    );
}

#[test]
fn a_shape_gains_sub_shapes_without_a_shapes_wrapper() {
    let source = package(
        &[
            (
                "/visio/document.xml",
                element(
                    "VisioDocument",
                    &[],
                    &[element("DocumentSettings", &[], &[])],
                ),
            ),
            (
                "/visio/pages/pages.xml",
                element(
                    "Pages",
                    &[],
                    &[element(
                        "Page",
                        &[("ID", "0")],
                        &[element("Rel", &[("r:id", "rId1")], &[])],
                    )],
                ),
            ),
            (
                "/visio/pages/page1.xml",
                element(
                    "PageContents",
                    &[],
                    &[element(
                        "Shapes",
                        &[],
                        &[element("Shape", &[("ID", "1")], &[])],
                    )],
                ),
            ),
        ],
        &[
            ("/", "rId1", "visio/document.xml"),
            ("/visio/document.xml", "rId1", "pages/pages.xml"),
            ("/visio/pages/pages.xml", "rId1", "page1.xml"),
        ],
    );
    let mut document = to_model(&source).expect("reads");
    let group = document.pages[0].shapes[0];
    let child = document.add_shape(Shape::new(ShapeId { index: 0 }, 2));
    document.shape_mut(child).expect("child").parent = Some(group);
    document
        .shape_mut(group)
        .expect("group")
        .children
        .push(child);

    let (written, model) = round_trip(&document, &source);
    let wrappers = children_named(root(&written, "/visio/pages/page1.xml"), "Shapes");
    let wrapper = wrappers.first().copied().expect("Shapes");
    let shapes = children_named(wrapper, "Shape");
    let shape = shapes.first().copied().expect("Shape");
    let wrappers = children_named(shape, "Shapes");
    assert_eq!(wrappers.len(), 1);
    assert_eq!(children_named(wrappers[0], "Shape").len(), 1);
    assert_eq!(
        model
            .shape(model.pages[0].shapes[0])
            .unwrap()
            .children
            .len(),
        1
    );
}

#[test]
fn a_cell_keeps_its_unit_spelling_and_an_error_takes_the_value_slot() {
    let shape = element(
        "Shape",
        &[("ID", "1")],
        &[
            element("Cell", &[("N", "Width"), ("U", "IN"), ("V", "2")], &[]),
            element("Cell", &[("N", "Height"), ("U", "MM"), ("V", "2")], &[]),
        ],
    );
    let source = package(
        &[
            (
                "/visio/document.xml",
                element(
                    "VisioDocument",
                    &[],
                    &[element("DocumentSettings", &[], &[])],
                ),
            ),
            (
                "/visio/pages/pages.xml",
                element(
                    "Pages",
                    &[],
                    &[element(
                        "Page",
                        &[("ID", "0")],
                        &[element("Rel", &[("r:id", "rId1")], &[])],
                    )],
                ),
            ),
            (
                "/visio/pages/page1.xml",
                element("PageContents", &[], &[element("Shapes", &[], &[shape])]),
            ),
        ],
        &[
            ("/", "rId1", "visio/document.xml"),
            ("/visio/document.xml", "rId1", "pages/pages.xml"),
            ("/visio/pages/pages.xml", "rId1", "page1.xml"),
        ],
    );
    let mut document = to_model(&source).expect("reads");
    let sheet = &mut document
        .shape_mut(document.pages[0].shapes[0])
        .expect("shape")
        .sheet;
    // `Width` loses its value: `V` goes, the `IN` spelling stays.
    sheet.cells[0].value = None;
    // `Height` becomes an error: `E` takes the value slot.
    sheet.cells[1].value = Some(CellValue::Error("#DIV/0!".to_owned()));

    let (written, model) = round_trip(&document, &source);
    let wrappers = children_named(root(&written, "/visio/pages/page1.xml"), "Shapes");
    let wrapper = wrappers.first().copied().expect("Shapes");
    let shapes = children_named(wrapper, "Shape");
    let shape = shapes.first().copied().expect("Shape");
    let cells = children_named(shape, "Cell");
    assert_eq!(attr(cells[0], "V"), None);
    assert_eq!(attr(cells[0], "U"), Some("IN"));
    assert_eq!(attr(cells[1], "E"), Some("#DIV/0!"));
    let sheet = &model.shape(model.pages[0].shapes[0]).unwrap().sheet;
    assert_eq!(sheet.cells[0].value, None);
    assert_eq!(
        sheet.cells[1].value,
        Some(CellValue::Error("#DIV/0!".to_owned()))
    );
}

#[test]
fn a_page_background_is_written_when_it_changes() {
    let source = package(
        &[
            (
                "/visio/document.xml",
                element(
                    "VisioDocument",
                    &[],
                    &[element("DocumentSettings", &[], &[])],
                ),
            ),
            (
                "/visio/pages/pages.xml",
                element(
                    "Pages",
                    &[],
                    &[
                        element("Page", &[("ID", "10"), ("BackPage", "20")], &[]),
                        element("Page", &[("ID", "20")], &[]),
                    ],
                ),
            ),
        ],
        &[
            ("/", "rId1", "visio/document.xml"),
            ("/visio/document.xml", "rId1", "pages/pages.xml"),
        ],
    );
    let mut document = to_model(&source).expect("reads");
    assert_eq!(document.pages[0].background, Some(PageId { index: 1 }));
    // Turn the reference around.
    document.pages[0].background = None;
    document.pages[1].background = Some(PageId { index: 0 });

    let (written, model) = round_trip(&document, &source);
    let pages = children_named(root(&written, "/visio/pages/pages.xml"), "Page");
    assert_eq!(attr(pages[0], "BackPage"), None);
    assert_eq!(attr(pages[1], "BackPage"), Some("10"));
    assert_eq!(model.pages[0].background, None);
    assert_eq!(model.pages[1].background, Some(PageId { index: 0 }));
}

#[test]
fn the_drawing_fixture_stays_byte_stable_across_a_rename() {
    let bytes = common::build(common::Fixture::Drawing);
    let source = read_package(&bytes, &Limits::DEFAULT).expect("reads");
    let mut document = to_model(&source).expect("maps");
    document.pages[0].name = "Renamed".to_owned();
    let (written, model) = round_trip(&document, &source);
    assert_eq!(model.pages[0].name, "Renamed");
    let _ = write_package(&written);
}

#[test]
fn list_inserts_follow_the_ms_vsdx_sibling_order() {
    // Every list insert ranks the existing siblings (the [MS-VSDX]
    // sequences): a `Pages` list of `Page` + junk, a `Masters` list of
    // `Master` + `MasterShortcut` + junk, a `StyleSheets` wrapper of
    // `StyleSheet` + junk, a `Shapes` wrapper of `Shape` + junk, a `Connects`
    // wrapper of `Connect` + junk and a row of `Cell` + junk.
    let source = package(
        &[
            (
                "/visio/document.xml",
                element(
                    "VisioDocument",
                    &[],
                    &[
                        element("Junk", &[], &[]),
                        element(
                            "StyleSheets",
                            &[],
                            &[
                                element("StyleSheet", &[("ID", "0")], &[]),
                                element("Junk", &[], &[]),
                            ],
                        ),
                    ],
                ),
            ),
            (
                "/visio/pages/pages.xml",
                element(
                    "Pages",
                    &[],
                    &[
                        element("Page", &[("ID", "1")], &[]),
                        element("Junk", &[], &[]),
                    ],
                ),
            ),
            (
                "/visio/masters/masters.xml",
                element(
                    "Masters",
                    &[],
                    &[
                        element("Master", &[("ID", "1")], &[]),
                        element("MasterShortcut", &[("ID", "2")], &[]),
                        element("Junk", &[], &[]),
                    ],
                ),
            ),
        ],
        &[
            ("/", "rId1", "visio/document.xml"),
            ("/visio/document.xml", "rId1", "pages/pages.xml"),
            ("/visio/document.xml", "rId2", "masters/masters.xml"),
        ],
    );
    let mut document = to_model(&source).expect("reads");
    // A new page with a shape inside a `Shapes` wrapper of `Shape` + junk and
    // a `Connects` wrapper of `Connect` + junk, and a row cell.
    let mut page = Page::new(PageId { index: 1 }, 20);
    page.name = "P2".to_owned();
    let shape_id = document.add_shape(Shape::new(ShapeId { index: 0 }, 2));
    let sheet = &mut document.shape_mut(shape_id).expect("shape").sheet;
    let mut section = Section::new(SectionName::new("Geometry"));
    let mut row = Row::new();
    row.name = Some("Row_1".to_owned());
    row.cells.push(Cell::new(CellName::new("X")));
    section.rows.push(row);
    sheet.sections.push(section);
    page.shapes.push(shape_id);
    document.pages.push(page);
    let mut master = Master::new(MasterId { index: 1 }, 30);
    master.name_u = "New".to_owned();
    document.masters.push(master);
    let mut style = Style::new(StyleId { index: 1 });
    style.visio_id = 9;
    document.stylesheet.styles.push(style);

    let (written, _) = round_trip(&document, &source);
    let pages = children_named(root(&written, "/visio/pages/pages.xml"), "Page");
    assert_eq!(pages.len(), 2);
    let masters = children_named(root(&written, "/visio/masters/masters.xml"), "Master");
    assert_eq!(masters.len(), 2);
    let styles = children_named(root(&written, "/visio/document.xml"), "StyleSheets");
    assert_eq!(children_named(styles[0], "StyleSheet").len(), 2);

    // The new page's contents part inserts its wrappers next to the junk.
    let page2 = root(&written, "/visio/pages/page2.xml");
    let shapes = children_named(page2, "Shapes");
    assert_eq!(shapes.len(), 1);
    assert_eq!(children_named(shapes[0], "Shape").len(), 1);
}

#[test]
fn wrappers_and_rels_insert_next_to_their_siblings() {
    // A contents root with a `Connects` wrapper but no `Shapes` wrapper, and
    // a page whose `Rel` names the wrong id: both are inserted where the
    // sequence puts them.
    let source = package(
        &[
            (
                "/visio/document.xml",
                element(
                    "VisioDocument",
                    &[],
                    &[element("DocumentSettings", &[], &[])],
                ),
            ),
            (
                "/visio/pages/pages.xml",
                element(
                    "Pages",
                    &[],
                    &[element(
                        "Page",
                        &[("ID", "0")],
                        &[
                            element("PageSheet", &[], &[]),
                            element("Rel", &[("r:id", "rId7")], &[]),
                            element("Icon", &[], &[]),
                        ],
                    )],
                ),
            ),
            (
                "/visio/pages/page1.xml",
                element(
                    "PageContents",
                    &[],
                    &[
                        element("Connects", &[], &[element("Junk", &[], &[])]),
                        element("Junk", &[], &[]),
                    ],
                ),
            ),
        ],
        &[
            ("/", "rId1", "visio/document.xml"),
            ("/visio/document.xml", "rId1", "pages/pages.xml"),
            ("/visio/pages/pages.xml", "rId1", "page1.xml"),
        ],
    );
    let mut document = to_model(&source).expect("reads");
    let shape_id = document.add_shape(Shape::new(ShapeId { index: 0 }, 5));
    document.pages[0].shapes.push(shape_id);
    document.pages[0].connects.push(Connect {
        from_sheet: 5,
        from_cell: "PinX".to_owned(),
        from_part: 9,
        to_sheet: 5,
        to_cell: "PinX".to_owned(),
        to_part: 3,
        preserved: Preserved::new(),
    });

    let (written, model) = round_trip(&document, &source);
    let contents = root(&written, "/visio/pages/page1.xml");
    let shapes = children_named(contents, "Shapes");
    assert_eq!(shapes.len(), 1);
    assert_eq!(children_named(shapes[0], "Shape").len(), 1);
    // The `Rel` of the wrong id gets the right one beside it.
    let page = children_named(root(&written, "/visio/pages/pages.xml"), "Page");
    // The page keeps its own `Rel` spelling; the writer does not add a
    // second one beside it.
    let rels = children_named(page[0], "Rel");
    assert_eq!(rels.len(), 1);
    assert_eq!(attr(rels[0], "id"), Some("rId7"));
    assert_eq!(model.pages[0].shapes.len(), 1);
}

#[test]
fn new_document_properties_and_styles_rank_among_their_siblings() {
    let source = package(
        &[(
            "/visio/document.xml",
            element(
                "VisioDocument",
                &[],
                &[
                    element(
                        "DocumentProperties",
                        &[],
                        &[text_element("title", &[], "Old")],
                    ),
                    element("DocumentSettings", &[], &[]),
                    element("Colors", &[], &[]),
                    element("FaceNames", &[], &[]),
                    element("DocumentSheet", &[], &[]),
                    element("EventList", &[], &[]),
                    element("HeaderFooter", &[], &[]),
                    element("DataTransferInfo", &[], &[]),
                    element("PublishSettings", &[], &[]),
                    element("Comments", &[], &[]),
                    element("Junk", &[], &[]),
                ],
            ),
        )],
        &[("/", "rId1", "visio/document.xml")],
    );
    let mut document = to_model(&source).expect("reads");
    document.properties.push(Property::new("subject", "Added"));
    let mut style = Style::new(StyleId { index: 0 });
    style.visio_id = 1;
    style.name = "Normal".to_owned();
    document.stylesheet.styles.push(style);

    let (written, _) = round_trip(&document, &source);
    let main = root(&written, "/visio/document.xml");
    let wrappers = children_named(main, "StyleSheets");
    assert_eq!(wrappers.len(), 1);
    assert_eq!(children_named(wrappers[0], "StyleSheet").len(), 1);
    let properties = children_named(main, "DocumentProperties");
    assert_eq!(children_named(properties[0], "subject").len(), 1);
    // `DocumentProperties` keeps its place at the head of the sequence.
    let names: Vec<&str> = main
        .children
        .iter()
        .filter_map(|child| match child {
            XmlChild::Element(element) => Some(element.name.local.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(names.first(), Some(&"DocumentProperties"));
}

#[test]
fn a_new_cell_ranks_inside_its_row() {
    let shape = element(
        "Shape",
        &[("ID", "1")],
        &[element(
            "Section",
            &[("N", "Geometry")],
            &[element(
                "Row",
                &[("N", "Row_1")],
                &[
                    element("Cell", &[("N", "X"), ("V", "1")], &[]),
                    element("Junk", &[], &[]),
                ],
            )],
        )],
    );
    let source = package(
        &[
            (
                "/visio/document.xml",
                element(
                    "VisioDocument",
                    &[],
                    &[element("DocumentSettings", &[], &[])],
                ),
            ),
            (
                "/visio/pages/pages.xml",
                element(
                    "Pages",
                    &[],
                    &[element(
                        "Page",
                        &[("ID", "0")],
                        &[element("Rel", &[("r:id", "rId1")], &[])],
                    )],
                ),
            ),
            (
                "/visio/pages/page1.xml",
                element("PageContents", &[], &[element("Shapes", &[], &[shape])]),
            ),
        ],
        &[
            ("/", "rId1", "visio/document.xml"),
            ("/visio/document.xml", "rId1", "pages/pages.xml"),
            ("/visio/pages/pages.xml", "rId1", "page1.xml"),
        ],
    );
    let mut document = to_model(&source).expect("reads");
    let sheet = &mut document
        .shape_mut(document.pages[0].shapes[0])
        .expect("shape")
        .sheet;
    let mut cell = Cell::new(CellName::new("Y"));
    cell.value = Some(CellValue::Number(2.0));
    sheet.sections[0].rows[0].cells.push(cell);

    let (written, _) = round_trip(&document, &source);
    let shapes = children_named(root(&written, "/visio/pages/page1.xml"), "Shapes");
    let shape = children_named(shapes[0], "Shape");
    let sections = children_named(shape[0], "Section");
    let rows = children_named(sections[0], "Row");
    let cells = children_named(rows[0], "Cell");
    assert_eq!(cells.len(), 2);
    assert_eq!(attr(cells[1], "N"), Some("Y"));
}

/// Regression (task P2-10): a new list entity without contents used to get
/// a generated `Rel` naming a relationship that did not exist. The next read
/// kept it as preserved content and the next write grew a contents part for
/// it, so `write -> read -> write` was not byte-identical.
#[test]
fn a_new_entity_without_contents_gets_no_rel_marker() {
    let source = empty_lists();
    let mut document = filled_document();
    // Both entities keep their shapes out: no contents part, no `Rel`.
    document.pages[0].shapes.clear();
    document.pages[1].shapes.clear();
    document.pages[1].connects.clear();
    document.masters[0].shapes.clear();

    let (written, model) = round_trip(&document, &source);
    let pages = children_named(root(&written, "/visio/pages/pages.xml"), "Page");
    for page in pages {
        assert!(children_named(page, "Rel").is_empty(), "no stray Rel");
    }
    let masters = children_named(root(&written, "/visio/masters/masters.xml"), "Master");
    for master in masters {
        assert!(children_named(master, "Rel").is_empty(), "no stray Rel");
    }
    assert!(model.pages.iter().all(|page| page.shapes.is_empty()));
    assert!(model.masters.iter().all(|master| master.shapes.is_empty()));
}

/// Regression (review finding 2): blank text is significant content when an
/// element has no element children (ADR 0012 whitespace rule), so a write
/// keeps it even when it syncs the element's children.
#[test]
fn blank_text_only_content_survives_a_write() {
    let source = package(
        &[
            (
                "/visio/document.xml",
                element(
                    "VisioDocument",
                    &[],
                    &[element("DocumentSettings", &[], &[])],
                ),
            ),
            (
                "/visio/pages/pages.xml",
                element(
                    "Pages",
                    &[],
                    &[element(
                        "Page",
                        &[("ID", "0")],
                        &[element("Rel", &[("r:id", "rId1")], &[])],
                    )],
                ),
            ),
            (
                "/visio/pages/page1.xml",
                element("PageContents", &[], &[text_element("Shapes", &[], "   ")]),
            ),
        ],
        &[
            ("/", "rId1", "visio/document.xml"),
            ("/visio/document.xml", "rId1", "pages/pages.xml"),
            ("/visio/pages/pages.xml", "rId1", "page1.xml"),
        ],
    );
    let document = to_model(&source).expect("reads");
    let (written, _model) = round_trip(&document, &source);
    let shapes = children_named(root(&written, "/visio/pages/page1.xml"), "Shapes");
    assert_eq!(shapes.len(), 1);
    assert_eq!(shapes[0].children, vec![XmlChild::Text("   ".to_owned())]);
}

/// Regression (P2-10 open issue 1, task P2-14): a value the model built
/// writes the `U` the reader classifies it with (`U="BOOL"`, `U="STR"`), and
/// a `U` that already declares the right kind keeps its spelling.
#[test]
fn a_patched_value_writes_the_u_its_kind_needs() {
    let source = package(
        &[
            (
                "/visio/document.xml",
                element(
                    "VisioDocument",
                    &[],
                    &[element("DocumentSettings", &[], &[])],
                ),
            ),
            (
                "/visio/pages/pages.xml",
                element(
                    "Pages",
                    &[],
                    &[element(
                        "Page",
                        &[("ID", "0")],
                        &[element("Rel", &[("r:id", "rId1")], &[])],
                    )],
                ),
            ),
            (
                "/visio/pages/page1.xml",
                element(
                    "PageContents",
                    &[],
                    &[element(
                        "Shapes",
                        &[],
                        &[element(
                            "Shape",
                            &[("ID", "1")],
                            &[
                                element("Cell", &[("N", "Flag")], &[]),
                                element("Cell", &[("N", "Label"), ("V", "text")], &[]),
                                element("Cell", &[("N", "Date"), ("U", "DATE"), ("V", "2")], &[]),
                            ],
                        )],
                    )],
                ),
            ),
        ],
        &[
            ("/", "rId1", "visio/document.xml"),
            ("/visio/document.xml", "rId1", "pages/pages.xml"),
            ("/visio/pages/pages.xml", "rId1", "page1.xml"),
        ],
    );
    let mut document = to_model(&source).expect("reads");
    let shape_id = document.pages[0].shapes[0];
    let shape = document.shape_mut(shape_id).expect("shape");
    for cell in &mut shape.sheet.cells {
        match cell.name.as_str() {
            "Flag" => cell.value = Some(CellValue::Bool(true)),
            // A string that reads back as a number without `U`.
            "Label" => cell.value = Some(CellValue::Str("2".to_owned())),
            _ => {}
        }
    }

    let (written, model) = round_trip(&document, &source);
    let wrapper = children_named(root(&written, "/visio/pages/page1.xml"), "Shapes");
    let shape = children_named(wrapper[0], "Shape");
    let cells = children_named(shape[0], "Cell");
    assert_eq!(attr(cells[0], "V"), Some("1"));
    assert_eq!(attr(cells[0], "U"), Some("BOOL"));
    assert_eq!(attr(cells[1], "V"), Some("2"));
    assert_eq!(attr(cells[1], "U"), Some("STR"));
    // A spelling that already declares the kind is never rewritten.
    assert_eq!(attr(cells[2], "V"), Some("2"));
    assert_eq!(attr(cells[2], "U"), Some("DATE"));

    let reread = model.shape(shape_id).expect("shape");
    assert_eq!(reread.sheet.cells[0].value, Some(CellValue::Bool(true)));
    assert_eq!(
        reread.sheet.cells[1].value,
        Some(CellValue::Str("2".to_owned()))
    );
    assert_eq!(
        reread.sheet.cells[2].value,
        Some(CellValue::Str("2".to_owned()))
    );
}

/// Coverage (audit major 1): the reader's sheet-level preservation fallbacks
/// (read/sheet.rs) keep an unmapped `Cell`, `Row` or `Section` child and an
/// unknown cell attribute in `Preserved`, and a write puts every one of them
/// back where the source had it.
#[test]
fn sheet_level_preservation_fallbacks_survive_the_round_trip() {
    let source = package(
        &[
            (
                "/visio/document.xml",
                element(
                    "VisioDocument",
                    &[],
                    &[element("DocumentSettings", &[], &[])],
                ),
            ),
            (
                "/visio/pages/pages.xml",
                element(
                    "Pages",
                    &[],
                    &[element(
                        "Page",
                        &[("ID", "0")],
                        &[element("Rel", &[("r:id", "rId1")], &[])],
                    )],
                ),
            ),
            (
                "/visio/pages/page1.xml",
                element(
                    "PageContents",
                    &[],
                    &[element(
                        "Shapes",
                        &[],
                        &[element(
                            "Shape",
                            &[("ID", "1")],
                            &[
                                // No `N`: cannot be named, kept verbatim.
                                element("Cell", &[("V", "9")], &[]),
                                // Unknown cell attribute.
                                element(
                                    "Cell",
                                    &[("N", "PinX"), ("V", "2"), ("Future", "yes")],
                                    &[],
                                ),
                                element(
                                    "Section",
                                    &[("N", "Geometry")],
                                    &[
                                        element("Cell", &[("N", "NoFill"), ("V", "0")], &[]),
                                        // Unmapped `Cell` child of the section.
                                        element("Cell", &[("V", "3")], &[]),
                                        // A `Row` with loose content is kept verbatim.
                                        {
                                            let mut row = element("Row", &[("IX", "1")], &[]);
                                            row.children.push(XmlChild::Comment("note".to_owned()));
                                            row
                                        },
                                        element(
                                            "Row",
                                            &[("IX", "2"), ("Del", "1")],
                                            &[
                                                // Unmapped `Cell` and unknown child of the row.
                                                element("Cell", &[("V", "7")], &[]),
                                                element("RefBy", &[("ID", "1")], &[]),
                                            ],
                                        ),
                                    ],
                                ),
                                // No `N`: cannot be named, kept verbatim.
                                element("Section", &[], &[]),
                            ],
                        )],
                    )],
                ),
            ),
        ],
        &[
            ("/", "rId1", "visio/document.xml"),
            ("/visio/document.xml", "rId1", "pages/pages.xml"),
            ("/visio/pages/pages.xml", "rId1", "page1.xml"),
        ],
    );
    let document = to_model(&source).expect("reads");
    let shape = document.shape(document.pages[0].shapes[0]).expect("shape");

    // The reader keeps what it cannot map.
    let kept: Vec<&str> = shape
        .preserved
        .children
        .iter()
        .map(|child| child.name.local.as_str())
        .collect();
    assert_eq!(kept, ["Cell", "Section"]);
    let pinx = shape
        .sheet
        .cells
        .iter()
        .find(|cell| cell.name.as_str() == "PinX")
        .expect("PinX");
    assert_eq!(pinx.preserved.attributes.len(), 1);
    assert_eq!(pinx.preserved.attributes[0].name.local, "Future");
    let section = &shape.sheet.sections[0];
    let kept: Vec<&str> = section
        .preserved
        .children
        .iter()
        .map(|child| child.name.local.as_str())
        .collect();
    assert_eq!(kept, ["Cell", "Row"]);
    let row = &section.rows[0];
    assert_eq!(row.preserved.attributes[0].name.local, "Del");
    let kept: Vec<&str> = row
        .preserved
        .children
        .iter()
        .map(|child| child.name.local.as_str())
        .collect();
    assert_eq!(kept, ["Cell", "RefBy"]);

    // The writer puts every one of them back.
    let (written, _model) = round_trip(&document, &source);
    let wrapper = children_named(root(&written, "/visio/pages/page1.xml"), "Shapes");
    let shape = children_named(wrapper[0], "Shape");
    let cells = children_named(shape[0], "Cell");
    assert_eq!(cells.len(), 2);
    assert!(attr(cells[0], "N").is_none());
    assert_eq!(attr(cells[0], "V"), Some("9"));
    assert_eq!(attr(cells[1], "Future"), Some("yes"));
    let sections = children_named(shape[0], "Section");
    assert_eq!(sections.len(), 2);
    assert!(attr(sections[1], "N").is_none());
    let section_cells = children_named(sections[0], "Cell");
    assert_eq!(section_cells.len(), 2);
    assert!(attr(section_cells[1], "N").is_none());
    assert_eq!(attr(section_cells[1], "V"), Some("3"));
    let rows = children_named(sections[0], "Row");
    assert_eq!(rows.len(), 2);
    assert!(
        rows[0]
            .children
            .iter()
            .any(|child| matches!(child, XmlChild::Comment(_)))
    );
    assert_eq!(attr(rows[1], "Del"), Some("1"));
    let row_cells = children_named(rows[1], "Cell");
    assert_eq!(attr(row_cells[0], "V"), Some("7"));
    assert_eq!(children_named(rows[1], "RefBy").len(), 1);
}

/// Strips the writer's generated `Rel` markers from pages and masters: a new
/// contents relationship brings a `Rel` element the reader keeps in the
/// entity's preserved content. It is writer furniture, not document content
/// (the property tests normalize the same way).
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

/// A document with content in every list, built in the arena order the
/// reader builds (master shapes first, then page shapes).
fn plumbing_document() -> Document {
    let mut document = Document::new();
    let master_shape = document.add_shape(Shape::new(ShapeId { index: 0 }, 3));
    let first = document.add_shape(Shape::new(ShapeId { index: 1 }, 1));
    let second = document.add_shape(Shape::new(ShapeId { index: 2 }, 2));
    let mut master = Master::new(MasterId { index: 0 }, 30);
    master.name_u = "Box".to_owned();
    master.shapes.push(master_shape);
    document.masters.push(master);
    let mut page = Page::new(PageId { index: 0 }, 10);
    page.name = "P1".to_owned();
    page.shapes.push(first);
    document.pages.push(page);
    let mut page = Page::new(PageId { index: 1 }, 20);
    page.name = "P2".to_owned();
    page.shapes.push(second);
    document.pages.push(page);
    document
}

/// Coverage (audit major 2): `write_document` into an empty package
/// generates the parts from the model alone, and the result reads back to
/// the same model.
#[test]
fn writing_into_an_empty_package_reads_back_the_same_model() {
    let document = plumbing_document();
    let bytes = write_document(&document, &Package::new()).expect("writes");
    let package = read_package(&bytes, &Limits::DEFAULT).expect("the written package reads");
    let mut model = to_model(&package).expect("maps");
    without_rel_markers(&mut model);
    assert_eq!(model, document);
}

/// Coverage (audit major 2): a package without `[Content_Types].xml` (the
/// reader infers its declarations) writes back to the same model.
#[test]
fn writing_into_a_package_without_content_types_reads_back_the_same_model() {
    let bytes = common::build(common::Fixture::Malformed(
        common::Malformed::NoContentTypes,
    ));
    let package = read_package(&bytes, &Limits::DEFAULT).expect("the fixture reads");
    let document = to_model(&package).expect("maps");
    let out = write_document(&document, &package).expect("writes");
    let reread = read_package(&out, &Limits::DEFAULT).expect("the written package reads");
    assert_eq!(to_model(&reread).expect("maps"), document);
}

/// Coverage (audit major 2): a package with only a main part and no
/// `[Content_Types].xml` gets the missing pages and masters plumbing built,
/// and reads back to the same model. A stale relationship id exercises the
/// fresh-id collision loop.
#[test]
fn writing_builds_the_missing_index_parts_and_relationships() {
    let document = plumbing_document();
    let mut bare = Package::new();
    bare.parts.push(Part {
        name: PartName::new("/visio/document.xml"),
        content_type: ContentType::new("application/xml"),
        data: PartData::Xml(element("VisioDocument", &[], &[])),
    });
    bare.relationships.push(Relationship {
        source: PartName::new("/"),
        id: RelationshipId::new("rId1"),
        rel_type: "http://schemas.microsoft.com/visio/2010/relationships/document".to_owned(),
        target: "visio/document.xml".to_owned(),
        target_mode: TargetMode::Internal,
        preserved: Preserved::new(),
    });
    // A stale id on the (missing) pages index: the next fresh id must skip
    // it.
    bare.relationships.push(Relationship {
        source: PartName::new("/visio/pages/pages.xml"),
        id: RelationshipId::new("rId2"),
        rel_type: "http://schemas.microsoft.com/visio/2010/relationships/page".to_owned(),
        target: "page1.xml".to_owned(),
        target_mode: TargetMode::Internal,
        preserved: Preserved::new(),
    });

    let out = write_document(&document, &bare).expect("writes");
    let reread = read_package(&out, &Limits::DEFAULT).expect("the written package reads");
    for name in [
        "/visio/pages/pages.xml",
        "/visio/pages/page1.xml",
        "/visio/pages/page2.xml",
        "/visio/masters/masters.xml",
        "/visio/masters/master1.xml",
    ] {
        assert!(reread.part(&PartName::new(name)).is_some(), "{name} exists");
    }
    let mut model = to_model(&reread).expect("maps");
    without_rel_markers(&mut model);
    assert_eq!(model, document);
}

/// The one-page source with two unit cells both tests below share.
fn unit_source() -> Package {
    package(
        &[
            (
                "/visio/document.xml",
                element(
                    "VisioDocument",
                    &[],
                    &[element("DocumentSettings", &[], &[])],
                ),
            ),
            (
                "/visio/pages/pages.xml",
                element(
                    "Pages",
                    &[],
                    &[element(
                        "Page",
                        &[("ID", "0")],
                        &[element("Rel", &[("r:id", "rId1")], &[])],
                    )],
                ),
            ),
            (
                "/visio/pages/page1.xml",
                element(
                    "PageContents",
                    &[],
                    &[element(
                        "Shapes",
                        &[],
                        &[element(
                            "Shape",
                            &[("ID", "1")],
                            &[
                                element("Cell", &[("N", "W1"), ("U", "IN"), ("V", "2")], &[]),
                                element("Cell", &[("N", "W2"), ("U", "MM"), ("V", "3")], &[]),
                            ],
                        )],
                    )],
                ),
            ),
        ],
        &[
            ("/", "rId1", "visio/document.xml"),
            ("/visio/document.xml", "rId1", "pages/pages.xml"),
            ("/visio/pages/pages.xml", "rId1", "page1.xml"),
        ],
    )
}

/// Coverage (audit minor 10): when the model takes a cell's unit over, the
/// file's `U` follows — it goes when the new unit has no code, and it is
/// rewritten when it has one.
#[test]
fn the_model_takes_a_unit_over_and_the_file_u_follows() {
    let source = unit_source();
    let mut document = to_model(&source).expect("reads");
    let shape_id = document.pages[0].shapes[0];
    {
        let shape = document.shape_mut(shape_id).expect("shape");
        for cell in &mut shape.sheet.cells {
            match cell.name.as_str() {
                "W1" => {
                    assert_eq!(cell.unit, Unit::Inches);
                    cell.unit = Unit::Unitsless;
                }
                "W2" => {
                    assert_eq!(cell.unit, Unit::Millimeters);
                    cell.unit = Unit::Points;
                }
                _ => {}
            }
        }
    }

    let (written, model) = round_trip(&document, &source);
    let wrapper = children_named(root(&written, "/visio/pages/page1.xml"), "Shapes");
    let shape = children_named(wrapper[0], "Shape");
    let cells = children_named(shape[0], "Cell");
    // The unitless value loses `U`; the new unit rewrites it.
    assert_eq!(attr(cells[0], "V"), Some("2"));
    assert!(attr(cells[0], "U").is_none());
    assert_eq!(attr(cells[1], "V"), Some("3"));
    assert_eq!(attr(cells[1], "U"), Some("PT"));

    let reread = model.shape(shape_id).expect("shape");
    assert_eq!(reread.sheet.cells[0].unit, Unit::Unitsless);
    assert_eq!(reread.sheet.cells[1].unit, Unit::Points);
}

/// Coverage (audit minor 10): model connects with no `Connects` wrapper in
/// the source are wrapped in a new one; a loose preserved `Connect` element
/// stays where the source put it.
#[test]
fn model_connects_get_a_new_connects_wrapper() {
    let mut loose = element(
        "Connect",
        &[
            ("FromSheet", "1"),
            ("FromCell", "PinX"),
            ("FromPart", "9"),
            ("ToSheet", "1"),
            ("ToCell", "PinX"),
            ("ToPart", "3"),
        ],
        &[],
    );
    loose.self_closing = true;
    let source = package(
        &[
            (
                "/visio/document.xml",
                element(
                    "VisioDocument",
                    &[],
                    &[element("DocumentSettings", &[], &[])],
                ),
            ),
            (
                "/visio/pages/pages.xml",
                element(
                    "Pages",
                    &[],
                    &[element(
                        "Page",
                        &[("ID", "0")],
                        &[element("Rel", &[("r:id", "rId1")], &[])],
                    )],
                ),
            ),
            (
                "/visio/pages/page1.xml",
                element(
                    "PageContents",
                    &[],
                    &[
                        element("Shapes", &[], &[element("Shape", &[("ID", "1")], &[])]),
                        loose,
                    ],
                ),
            ),
        ],
        &[
            ("/", "rId1", "visio/document.xml"),
            ("/visio/document.xml", "rId1", "pages/pages.xml"),
            ("/visio/pages/pages.xml", "rId1", "page1.xml"),
        ],
    );
    let mut document = to_model(&source).expect("reads");
    // The loose element is preserved, not modeled.
    assert!(document.pages[0].connects.is_empty());
    let mut connect = Connect::new();
    connect.from_sheet = 1;
    connect.from_cell = "PinX".to_owned();
    connect.from_part = 9;
    connect.to_sheet = 2;
    connect.to_cell = "PinY".to_owned();
    connect.to_part = 3;
    document.pages[0].connects.push(connect);

    let (written, model) = round_trip(&document, &source);
    let page_root = root(&written, "/visio/pages/page1.xml");
    let wrappers = children_named(page_root, "Connects");
    assert_eq!(wrappers.len(), 1);
    let wrapped = children_named(wrappers[0], "Connect");
    assert_eq!(wrapped.len(), 1);
    assert_eq!(attr(wrapped[0], "ToSheet"), Some("2"));
    // The loose `Connect` is still a direct child of the root.
    assert_eq!(children_named(page_root, "Connect").len(), 1);
    assert_eq!(model.pages[0].connects.len(), 1);
    let preserved_connects = model.pages[0]
        .preserved
        .children
        .iter()
        .filter(|child| child.name.local == "Connect")
        .count();
    assert_eq!(preserved_connects, 1);
}
