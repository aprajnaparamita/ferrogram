//! Reader edge cases fed by hand-built packages (task P2-10, focus area:
//! relationship edge cases, caps and malformed cases).
//!
//! The reader's rule table is tested next to the implementation
//! (`src/read/mod.rs`, `src/read/sheet.rs`); these tests pin the degrade
//! paths that only matter for malformed or unusual files: namespace
//! declarations on mapped elements, unmapped children of index and contents
//! parts, dangling `Rel` and `BackPage` references, wrong contents roots,
//! binary `docProps` parts and bad cached values.

mod common;

use larrygram_model::{Preserved, QName, XmlAttribute, XmlChild, XmlNode};
use larrygram_vsdx::{
    ContentType, Error, Limits, Package, Part, PartData, PartName, Relationship, RelationshipId,
    TargetMode, to_model,
};

const CT_XML: &str = "application/xml";

/// Builds a qualified name from `prefix:local` or `local`.
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

/// Builds an element from its name, attributes and child elements.
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

/// Builds an element with one text child.
fn text_element(name: &str, attrs: &[(&str, &str)], text: &str) -> XmlNode {
    let mut node = element(name, attrs, &[]);
    node.children.push(XmlChild::Text(text.to_owned()));
    node
}

/// Builds a package from (part name, root element) pairs and relationships
/// (source, id, target).
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

/// A main part with only `DocumentSettings`, plus the page and master parts
/// of a one-page, one-master drawing.
fn drawing_parts() -> Vec<(&'static str, XmlNode)> {
    let document = element(
        "VisioDocument",
        &[("xml:space", "preserve")],
        &[element("DocumentSettings", &[("TopPage", "0")], &[])],
    );
    let pages = element(
        "Pages",
        &[],
        &[element(
            "Page",
            &[("ID", "0"), ("NameU", "Page-1"), ("Name", "Page-1")],
            &[element("Rel", &[("r:id", "rId1")], &[])],
        )],
    );
    let page_contents = element("PageContents", &[], &[element("Shapes", &[], &[])]);
    let masters = element(
        "Masters",
        &[],
        &[element(
            "Master",
            &[("ID", "1"), ("NameU", "Box")],
            &[element("Rel", &[("r:id", "rId1")], &[])],
        )],
    );
    let master_contents = element("MasterContents", &[], &[element("Shapes", &[], &[])]);
    vec![
        ("/visio/document.xml", document),
        ("/visio/pages/pages.xml", pages),
        ("/visio/pages/page1.xml", page_contents),
        ("/visio/masters/masters.xml", masters),
        ("/visio/masters/master1.xml", master_contents),
    ]
}

const DRAWING_RELS: &[(&str, &str, &str)] = &[
    ("/", "rId1", "visio/document.xml"),
    ("/visio/document.xml", "rId1", "pages/pages.xml"),
    ("/visio/document.xml", "rId2", "masters/masters.xml"),
    ("/visio/pages/pages.xml", "rId1", "page1.xml"),
    ("/visio/masters/masters.xml", "rId1", "master1.xml"),
];

#[test]
fn namespace_declarations_are_never_mapped_attributes() {
    let mut parts = drawing_parts();
    for (_, root) in &mut parts {
        root.attributes.push(XmlAttribute {
            name: qname("xmlns:foo"),
            value: "urn:foo".to_owned(),
        });
    }
    // A `Page`, `Master`, `Shape`, `Row` and `Connect` carrying only
    // namespace declarations maps with no preserved attributes.
    let page = element(
        "Page",
        &[("ID", "0"), ("xmlns:foo", "urn:foo")],
        &[element("Rel", &[("r:id", "rId1")], &[])],
    );
    parts[1] = (
        "/visio/pages/pages.xml",
        element("Pages", &[("xmlns:foo", "urn:foo")], &[page]),
    );
    let shape = element(
        "Shape",
        &[("ID", "1"), ("xmlns:foo", "urn:foo")],
        &[element(
            "Section",
            &[("N", "Geometry"), ("xmlns:foo", "urn:foo")],
            &[element(
                "Row",
                &[("IX", "1"), ("xmlns:foo", "urn:foo")],
                &[element(
                    "Cell",
                    &[("N", "X"), ("V", "1"), ("xmlns:foo", "urn:foo")],
                    &[],
                )],
            )],
        )],
    );
    parts[2] = (
        "/visio/pages/page1.xml",
        element("PageContents", &[], &[element("Shapes", &[], &[shape])]),
    );
    let package = package(&parts, DRAWING_RELS);
    let document = to_model(&package).expect("reads");
    let no_declarations = |attributes: &[XmlAttribute]| {
        attributes
            .iter()
            .all(|attribute| !attribute.name.local.starts_with("xmlns"))
    };
    assert!(no_declarations(&document.preserved.attributes));
    assert!(no_declarations(&document.pages[0].preserved.attributes));
    let shape = document.shape(document.pages[0].shapes[0]).expect("shape");
    assert!(no_declarations(&shape.preserved.attributes));
    assert!(no_declarations(
        &shape.sheet.sections[0].preserved.attributes
    ));
    assert!(no_declarations(
        &shape.sheet.sections[0].rows[0].preserved.attributes
    ));
    assert!(no_declarations(
        &shape.sheet.sections[0].rows[0].cells[0]
            .preserved
            .attributes
    ));
}

#[test]
fn unmapped_children_of_index_and_contents_parts_stay_preserved() {
    let mut parts = drawing_parts();
    let junk = || element("Junk", &[("x", "1")], &[]);
    let pages = element(
        "Pages",
        &[],
        &[
            junk(),
            element(
                "Page",
                &[("ID", "0")],
                &[element("Rel", &[("r:id", "rId1")], &[])],
            ),
            junk(),
        ],
    );
    parts[1] = ("/visio/pages/pages.xml", pages);
    // A contents root keeps what is not `Shapes` or `Connects`; a `Shapes`
    // wrapper keeps its own junk and a group's wrapper theirs.
    let group_junk = element("Junk", &[("where", "group")], &[]);
    let group = element(
        "Shape",
        &[("ID", "2")],
        &[element(
            "Shapes",
            &[],
            &[element("Shape", &[("ID", "3")], &[]), group_junk],
        )],
    );
    let wrapper_junk = element("Junk", &[("where", "wrapper")], &[]);
    let page_contents = element(
        "PageContents",
        &[],
        &[
            junk(),
            element("Shapes", &[], &[group, wrapper_junk]),
            junk(),
        ],
    );
    parts[2] = ("/visio/pages/page1.xml", page_contents);
    let package = package(&parts, DRAWING_RELS);
    let document = to_model(&package).expect("reads");

    let junk_children = |preserved: &Preserved| {
        preserved
            .children
            .iter()
            .filter(|child| child.name.local == "Junk")
            .count()
    };
    // The index part's junk stays with the document; the contents part's
    // junk follows the page.
    assert_eq!(junk_children(&document.preserved), 2);
    assert_eq!(junk_children(&document.pages[0].preserved), 3);
    let group = document.shape(document.pages[0].shapes[0]).expect("group");
    assert_eq!(junk_children(&group.preserved), 1);
}

#[test]
fn a_page_or_master_without_a_working_rel_keeps_no_shapes() {
    // The `Rel` names an id nothing targets; the contents part exists but is
    // unreachable.
    let mut rels: Vec<(&str, &str, &str)> = DRAWING_RELS
        .iter()
        .filter(|(_, id, target)| !(*id == "rId1" && *target == "page1.xml"))
        .copied()
        .collect();
    rels.push(("/visio/pages/pages.xml", "rId9", "page1.xml"));
    let mut parts = drawing_parts();
    let page = element(
        "Page",
        &[("ID", "0")],
        &[element("Rel", &[("r:id", "rId1")], &[])],
    );
    parts[1] = ("/visio/pages/pages.xml", element("Pages", &[], &[page]));
    // The master has no `Rel` at all.
    parts[3] = (
        "/visio/masters/masters.xml",
        element("Masters", &[], &[element("Master", &[("ID", "1")], &[])]),
    );
    let package = package(&parts, &rels);
    let document = to_model(&package).expect("reads");
    assert!(document.pages[0].shapes.is_empty());
    assert!(document.masters[0].shapes.is_empty());
}

#[test]
fn a_contents_part_with_the_wrong_root_name_is_ignored() {
    let mut parts = drawing_parts();
    // A recursion cycle in the corpus points a master at an index part.
    parts[4] = (
        "/visio/masters/master1.xml",
        element("Pages", &[], &[element("Junk", &[], &[])]),
    );
    parts[2] = (
        "/visio/pages/page1.xml",
        element("Masters", &[], &[element("Junk", &[], &[])]),
    );
    let package = package(&parts, DRAWING_RELS);
    let document = to_model(&package).expect("reads");
    assert!(document.pages[0].shapes.is_empty());
    assert!(document.masters[0].shapes.is_empty());
    // The wrong part is not mapped into the entity either.
    let junk_count = document
        .preserved
        .children
        .iter()
        .filter(|child| child.name.local == "Junk")
        .count();
    assert_eq!(junk_count, 0);
}

#[test]
fn a_dangling_back_page_reference_stays_preserved() {
    let mut parts = drawing_parts();
    let page = element(
        "Page",
        &[("ID", "0"), ("BackPage", "42")],
        &[element("Rel", &[("r:id", "rId1")], &[])],
    );
    parts[1] = ("/visio/pages/pages.xml", element("Pages", &[], &[page]));
    let package = package(&parts, DRAWING_RELS);
    let document = to_model(&package).expect("reads");
    assert_eq!(document.pages[0].background, None);
    let kept = document.pages[0]
        .preserved
        .attributes
        .iter()
        .find(|attribute| attribute.name.local == "BackPage")
        .expect("the reference is preserved");
    assert_eq!(kept.value, "42");
}

#[test]
fn a_page_child_that_is_not_a_sheet_or_rel_stays_preserved() {
    let mut parts = drawing_parts();
    let page = element(
        "Page",
        &[("ID", "0")],
        &[
            element("Rel", &[("r:id", "rId1")], &[]),
            element("Icon", &[("w", "1")], &[]),
        ],
    );
    parts[1] = ("/visio/pages/pages.xml", element("Pages", &[], &[page]));
    let package = package(&parts, DRAWING_RELS);
    let document = to_model(&package).expect("reads");
    assert!(
        document.pages[0]
            .preserved
            .children
            .iter()
            .any(|child| child.name.local == "Icon")
    );
}

#[test]
fn binary_doc_props_parts_are_ignored_and_properties_keep_their_text() {
    let mut package = package(&drawing_parts(), DRAWING_RELS);
    package.parts.push(Part {
        name: PartName::new("/docProps/core.xml"),
        content_type: ContentType::new(CT_XML),
        data: PartData::Binary(b"\x00\x01".to_vec()),
    });
    let core = element(
        "coreProperties",
        &[],
        &[
            text_element("title", &[], "Hello"),
            element("HeadingPairs", &[], &[text_element("Pages", &[], "1")]),
        ],
    );
    package.parts.push(Part {
        name: PartName::new("/docProps/app.xml"),
        content_type: ContentType::new(CT_XML),
        data: PartData::Xml(core),
    });
    let document = to_model(&package).expect("reads");
    assert!(
        document
            .properties
            .iter()
            .any(|p| p.name == "title" && p.value == "Hello")
    );
    // A property element with element children is kept verbatim, and its
    // text is not a property.
    assert!(!document.properties.iter().any(|p| p.name == "HeadingPairs"));
    assert!(
        document
            .preserved
            .children
            .iter()
            .any(|child| child.name.local == "HeadingPairs")
    );
}

#[test]
fn a_property_element_keeps_comment_text_out_of_its_value() {
    let mut package = package(&drawing_parts(), DRAWING_RELS);
    let mut note = text_element("note", &[], "text");
    note.children.push(XmlChild::Comment("hidden".to_owned()));
    let core = element("coreProperties", &[], &[note]);
    package.parts.push(Part {
        name: PartName::new("/docProps/core.xml"),
        content_type: ContentType::new(CT_XML),
        data: PartData::Xml(core),
    });
    let document = to_model(&package).expect("reads");
    assert!(
        document
            .properties
            .iter()
            .any(|p| p.name == "note" && p.value == "text")
    );
}

#[test]
fn a_stylesheet_child_that_is_not_a_style_stays_preserved() {
    let mut parts = drawing_parts();
    let styles = element(
        "StyleSheets",
        &[],
        &[
            element("Junk", &[], &[]),
            element("StyleSheet", &[("ID", "0"), ("Name", "Normal")], &[]),
        ],
    );
    parts[0] = (
        "/visio/document.xml",
        element("VisioDocument", &[], &[styles]),
    );
    let package = package(&parts, DRAWING_RELS);
    let document = to_model(&package).expect("reads");
    assert_eq!(document.stylesheet.styles.len(), 1);
    assert!(
        document
            .preserved
            .children
            .iter()
            .any(|child| child.name.local == "Junk")
    );
}

#[test]
fn a_bad_cached_value_is_an_error_naming_the_part_and_the_cell() {
    // A shape cell.
    let mut parts = drawing_parts();
    let shape = element(
        "Shape",
        &[("ID", "1")],
        &[element(
            "Cell",
            &[("N", "PinX"), ("U", "MM"), ("V", "x")],
            &[],
        )],
    );
    parts[2] = (
        "/visio/pages/page1.xml",
        element("PageContents", &[], &[element("Shapes", &[], &[shape])]),
    );
    let error = to_model(&package(&parts, DRAWING_RELS)).unwrap_err();
    let message = match error {
        Error::InvalidPackage(message) => message,
        other => panic!("expected InvalidPackage, got {other:?}"),
    };
    assert!(message.contains("page1.xml"), "{message}");
    assert!(message.contains("PinX"), "{message}");

    // A style cell.
    let mut parts = drawing_parts();
    let styles = element(
        "StyleSheets",
        &[],
        &[element(
            "StyleSheet",
            &[("ID", "0")],
            &[element(
                "Cell",
                &[("N", "LineWeight"), ("U", "BOOL"), ("V", "7")],
                &[],
            )],
        )],
    );
    parts[0] = (
        "/visio/document.xml",
        element("VisioDocument", &[], &[styles]),
    );
    assert!(to_model(&package(&parts, DRAWING_RELS)).is_err());

    // A page-sheet cell.
    let mut parts = drawing_parts();
    let sheet = element(
        "PageSheet",
        &[],
        &[element(
            "Cell",
            &[("N", "PageWidth"), ("U", "PER"), ("V", "wide")],
            &[],
        )],
    );
    let page = element("Page", &[("ID", "0")], &[sheet]);
    parts[1] = ("/visio/pages/pages.xml", element("Pages", &[], &[page]));
    assert!(to_model(&package(&parts, DRAWING_RELS)).is_err());
}

#[test]
fn the_document_is_read_through_any_root_relationship_or_the_content_type() {
    // A rel type with no slash still ends in `document`.
    let mut package = package(&drawing_parts(), DRAWING_RELS);
    package.relationships.clear();
    package.relationships.push(Relationship {
        source: PartName::new("/"),
        id: RelationshipId::new("rId1"),
        rel_type: "document".to_owned(),
        target: "visio/document.xml".to_owned(),
        target_mode: TargetMode::Internal,
        preserved: Preserved::new(),
    });
    let document = to_model(&package).expect("reads through the plain rel type");
    assert_eq!(document.pages.len(), 1);

    // An external root relationship is never dereferenced; the content type
    // fallback finds the main part.
    for part in &mut package.parts {
        if part.name.as_str() == "/visio/document.xml" {
            part.content_type = ContentType::new("application/vnd.ms-visio.drawing.main+xml");
        }
    }
    package.relationships.clear();
    package.relationships.push(Relationship {
        source: PartName::new("/"),
        id: RelationshipId::new("rId1"),
        rel_type: "http://schemas.microsoft.com/visio/2010/relationships/document".to_owned(),
        target: "https://example.test/outside.vsdx".to_owned(),
        target_mode: TargetMode::External,
        preserved: Preserved::new(),
    });
    let document = to_model(&package).expect("reads through the content type");
    assert_eq!(document.pages.len(), 1);
}

#[test]
fn a_connect_with_loose_content_or_junk_siblings_stays_preserved() {
    let mut parts = drawing_parts();
    let connect = element(
        "Connect",
        &[
            ("FromSheet", "1"),
            ("FromCell", "PinX"),
            ("FromPart", "9"),
            ("ToSheet", "1"),
            ("ToCell", "PinX"),
            ("ToPart", "3"),
            ("xmlns:foo", "urn:foo"),
        ],
        &[],
    );
    let page_contents = element(
        "PageContents",
        &[],
        &[
            element("Shapes", &[], &[element("Shape", &[("ID", "1")], &[])]),
            element("Connects", &[], &[connect, element("Junk", &[], &[])]),
        ],
    );
    parts[2] = ("/visio/pages/page1.xml", page_contents);
    let package = package(&parts, DRAWING_RELS);
    let document = to_model(&package).expect("reads");
    let page = &document.pages[0];
    assert_eq!(page.connects.len(), 1);
    assert!(page.connects[0].preserved.attributes.is_empty());
    assert!(
        page.preserved
            .children
            .iter()
            .any(|child| child.name.local == "Junk")
    );
}

/// The hand-built drawing fixture of `common` reads through the package
/// layer as well as the struct API.
#[test]
fn the_drawing_fixture_reads_through_the_package_layer() {
    let bytes = common::build(common::Fixture::Drawing);
    let package = read_package_bytes(&bytes);
    let document = to_model(&package).expect("reads");
    assert_eq!(document.pages.len(), 1);
}

#[allow(clippy::expect_used)] // test helper: a failing read is the bug under test
fn read_package_bytes(bytes: &[u8]) -> Package {
    larrygram_vsdx::read_package(bytes, &Limits::DEFAULT).expect("the fixture package reads")
}
