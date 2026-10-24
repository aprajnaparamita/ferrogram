//! The patcher's attribute arms (task P5-20, `write/patch.rs`
//! `sync_uint` and `bool_matches`).
//!
//! Patching an existing attribute of a source part must keep the file's
//! spelling when it already spells the value and rewrite it when it does
//! not. Two source spellings pin both directions:
//!
//! - a spelled-out boolean (`Hidden="TRUE"` / `Hidden="FALSE"`) and a
//!   zero-padded unsigned (`LineStyle="0003"`, `StyleSheet ID="0007"`)
//!   survive a patch unchanged (`bool_matches`' spelled-out arms,
//!   `sync_uint`'s "already equal" arm);
//! - unparseable text (`Hidden="junk"`, `LineStyle="junk"`) is rewritten
//!   to the canonical spelling (`bool_matches`' catch-all).
//!
//! The unparseable case cannot come from [`to_model`]: the reader
//! rejects a mapped attribute it cannot parse
//! ([`larrygram_vsdx::Error::InvalidPackage`]), so its model is read
//! from a well-spelled twin package and only the source *tree* carries
//! the foreign spelling. That is the patcher's real input: ADR 0018
//! patches whatever a foreign writer left behind.

use larrygram_model::Document;
use larrygram_model::{Preserved, QName, XmlAttribute, XmlChild, XmlNode};
use larrygram_vsdx::{
    ContentType, ContentTypeDecl, ContentTypeKind, Limits, Package, Part, PartData, PartName,
    Relationship, RelationshipId, TargetMode, read_package, to_model, write_document,
};

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

/// The `value` of the attribute named `name` of `element`, if present.
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

/// A one-page, two-master package whose mapped attributes carry the
/// given spellings: `hidden` and `hidden2` for the masters' `Hidden`,
/// `line_style` for the shape's `LineStyle` (the stylesheet's `ID` is
/// the zero-padded `"0007"` in every spelling).
fn attr_source(hidden: &str, hidden2: &str, line_style: &str) -> Package {
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
                    &[("ID", "0007"), ("Name", "Base"), ("NameU", "Base")],
                    Vec::new(),
                )],
            ),
        ],
    );
    let pages = element(
        "Pages",
        &[("xmlns", "urn:visio")],
        vec![element(
            "Page",
            &[("ID", "0"), ("NameU", "Page-1"), ("Name", "Page-1")],
            vec![element("Rel", &[("id", "rId1")], Vec::new())],
        )],
    );
    let page1 = element(
        "PageContents",
        &[("xmlns", "urn:visio")],
        vec![element(
            "Shapes",
            &[],
            vec![element(
                "Shape",
                &[("ID", "1"), ("Type", "Shape"), ("LineStyle", line_style)],
                vec![element("Cell", &[("N", "Width"), ("V", "2")], Vec::new())],
            )],
        )],
    );
    let masters = element(
        "Masters",
        &[("xmlns", "urn:visio")],
        vec![
            element(
                "Master",
                &[
                    ("ID", "1"),
                    ("NameU", "Master.1"),
                    ("Name", "Master 1"),
                    ("Hidden", hidden),
                ],
                Vec::new(),
            ),
            element(
                "Master",
                &[
                    ("ID", "2"),
                    ("NameU", "Master.2"),
                    ("Name", "Master 2"),
                    ("Hidden", hidden2),
                ],
                Vec::new(),
            ),
        ],
    );
    for (name, root) in [
        ("/visio/document.xml", document),
        ("/visio/pages/pages.xml", pages),
        ("/visio/pages/page1.xml", page1),
        ("/visio/masters/masters.xml", masters),
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
        (
            "/visio/document.xml",
            "rId2",
            "masters",
            "masters/masters.xml",
        ),
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

/// The model of the well-spelled twin (`hidden` = `true`/`false`, the
/// shape's `LineStyle` = 3, one style with Visio `ID` 7).
#[allow(clippy::expect_used)] // test helper
fn model() -> Document {
    let package = attr_source("TRUE", "FALSE", "0003");
    to_model(&package).expect("the well-spelled twin reads")
}

/// Writes `document` over `package` and answers the written package.
#[allow(clippy::expect_used)] // test helper
fn written(document: &Document, package: &Package) -> Package {
    let bytes = write_document(document, package).expect("write");
    read_package(&bytes, &Limits::DEFAULT).expect("reread")
}

/// The mapped attributes of the written masters and shape.
#[allow(clippy::panic, clippy::expect_used)] // test helper
fn written_spelling(package: &Package) -> (String, String, String, String) {
    let masters = root(package, "/visio/masters/masters.xml");
    let masters = children_named(masters, "Master");
    let first = masters.first().expect("the first master");
    let second = masters.get(1).expect("the second master");
    let contents = root(package, "/visio/pages/page1.xml");
    let wrapper = children_named(contents, "Shapes")
        .into_iter()
        .next()
        .expect("Shapes wrapper");
    let shape = children_named(wrapper, "Shape")
        .into_iter()
        .next()
        .expect("shape");
    let main = root(package, "/visio/document.xml");
    let sheets = children_named(main, "StyleSheets")
        .into_iter()
        .next()
        .expect("StyleSheets wrapper");
    let style = children_named(sheets, "StyleSheet")
        .into_iter()
        .next()
        .expect("style");
    (
        attr(first, "Hidden").expect("Hidden"),
        attr(second, "Hidden").expect("Hidden"),
        attr(shape, "LineStyle").expect("LineStyle"),
        attr(style, "ID").expect("ID"),
    )
}

#[test]
fn a_spelled_out_boolean_and_matching_spellings_survive_the_patch() {
    let package = attr_source("TRUE", "FALSE", "0003");
    let document = to_model(&package).expect("the spellings read");
    let masters = &document.masters;
    assert!(masters[0].hidden, "TRUE reads as true");
    assert!(!masters[1].hidden, "FALSE reads as false");
    let out = written(&document, &package);
    // The spellings the file already had are kept: they spell the value
    // (`bool_matches`' "TRUE"/"FALSE" arms, `sync_uint`'s and
    // `sync_opt_uint`'s "already equal" arms).
    assert_eq!(
        written_spelling(&out),
        (
            "TRUE".to_owned(),
            "FALSE".to_owned(),
            "0003".to_owned(),
            "0007".to_owned()
        )
    );
}

#[test]
fn an_unparseable_spelling_is_rewritten_by_the_patch() {
    // The model reads from the well-spelled twin; only the source tree
    // carries the foreign spellings (module docs).
    let document = model();
    assert!(document.masters[0].hidden);
    assert!(!document.masters[1].hidden);
    assert_eq!(
        document.shapes[0].line_style,
        Some(3),
        "the twin's LineStyle is 3"
    );
    let package = attr_source("junk", "maybe", "junk");
    let out = written(&document, &package);
    // `bool_matches`' catch-all: neither spelling matches, so the
    // attribute is rewritten to the canonical one (`sync_bool`'s
    // overwrite arm), and the unparseable `LineStyle` follows its
    // `sync_opt_uint` overwrite arm.
    assert_eq!(
        written_spelling(&out),
        (
            "1".to_owned(),
            "0".to_owned(),
            "3".to_owned(),
            "0007".to_owned()
        )
    );
}
