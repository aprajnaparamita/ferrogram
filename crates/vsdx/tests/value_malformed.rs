//! Malformed cached values (`V`, `U`, `E`) of a `Cell` element (task
//! P6-11, the malformed-input suite for P6-08's value reader).
//!
//! The contract (ADR 0052, `src/read/value.rs`): a numeric or boolean
//! literal that does not parse is [`Error::InvalidPackage`] naming the
//! part and the cell (never the value), every other bad or extreme
//! spelling degrades to a value of another kind, and **nothing panics**.
//! Huge numeric literals are the interesting corner: they parse to the
//! IEEE infinity (the doc's "does not parse" rule sees them as parsed),
//! and the suite pins that outcome so a future hardening (rejecting
//! non-finite literals as the writer does) shows up as a test change.

// The never-crash lints apply to non-test code only (clippy.toml).
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use ferrogram_model::{CellValue, Preserved, QName, XmlAttribute, XmlChild, XmlNode};
use ferrogram_vsdx::{
    ContentType, Error, Package, Part, PartData, PartName, Relationship, RelationshipId,
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

/// Builds a package from (part name, root element) pairs and
/// relationships (source, id, target).
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

/// A main part with only `DocumentSettings`, plus the page part of a
/// one-page drawing whose one shape carries `attrs` on its `Cell`.
fn drawing_with_cell(attrs: &[(&str, &str)]) -> Vec<(&'static str, XmlNode)> {
    let document = element(
        "VisioDocument",
        &[("xml:space", "preserve")],
        &[element("DocumentSettings", &[("TopPage", "0")], &[])],
    );
    let shape = element(
        "Shape",
        &[("ID", "1")],
        &[element("Cell", attrs, &[])],
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
    let page_contents = element(
        "PageContents",
        &[],
        &[element("Shapes", &[], &[shape])],
    );
    vec![
        ("/visio/document.xml", document),
        ("/visio/pages/pages.xml", pages),
        ("/visio/pages/page1.xml", page_contents),
    ]
}

const DRAWING_RELS: &[(&str, &str, &str)] = &[
    ("/", "rId1", "visio/document.xml"),
    ("/visio/document.xml", "rId1", "pages/pages.xml"),
    ("/visio/pages/pages.xml", "rId1", "page1.xml"),
];

/// Reads the one shape cell of `attrs`' package.
fn read_cell(attrs: &[(&str, &str)]) -> Result<ferrogram_model::Cell, Error> {
    let document = to_model(&package(&drawing_with_cell(attrs), DRAWING_RELS))?;
    let page = document.pages.first().expect("page");
    let shape = document
        .shape(*page.shapes.first().expect("shape"))
        .expect("shape entity");
    Ok(shape.sheet.cells.first().expect("cell").clone())
}

/// The `InvalidPackage` message of a rejected value.
fn message(attrs: &[(&str, &str)]) -> String {
    match read_cell(attrs) {
        Err(Error::InvalidPackage(message)) => message,
        Err(other) => panic!("expected InvalidPackage, got {other:?}"),
        Ok(cell) => panic!("expected an error, got {cell:?}"),
    }
}

/// P6-11: every spelling that does not parse as a number under a
/// numeric `U` code is one `InvalidPackage` naming the part and the
/// cell, and never the value itself.
#[test]
fn a_numeric_literal_that_does_not_parse_is_an_error_naming_part_and_cell() {
    for v in ["word", "1.2.3", "0x10", "--1", "1,5"] {
        let message = message(&[("N", "PinX"), ("U", "MM"), ("V", v)]);
        assert!(message.contains("page1.xml"), "{message}");
        assert!(message.contains("PinX"), "{message}");
        assert!(!message.contains(v), "the value is never quoted: {message}");
    }
}

/// P6-11: a boolean `V` that is neither `0` nor `1` (in any spelling)
/// is one `InvalidPackage`.
#[test]
fn a_boolean_that_is_neither_zero_nor_one_is_an_error() {
    for v in ["maybe", "2", "", "0.5"] {
        assert!(
            message(&[("N", "PinX"), ("U", "BOOL"), ("V", v)]).contains("boolean"),
            "V={v:?}"
        );
    }
}

/// P6-11: a huge numeric literal parses to the IEEE infinity and the
/// reader keeps it as a number (see the module docs: this is the pinned
/// outcome of today's parser, not a recommendation).
#[test]
fn a_huge_numeric_literal_keeps_the_ieee_infinity() {
    let mut huge = String::from("9");
    huge.push_str(&"0".repeat(400));
    for v in ["1e999999", "-1e999999", huge.as_str()] {
        let cell = read_cell(&[("N", "Width"), ("U", "IN"), ("V", v)]).expect("reads");
        match cell.value {
            Some(CellValue::Number(number)) => assert!(number.is_infinite(), "V={v:?} -> {number}"),
            other => panic!("expected a number for V={v:?}, got {other:?}"),
        }
    }
}

/// P6-11: `nan` and `inf` parse under a numeric code too: the value is
/// kept as the parsed float, whatever it is, and nothing panics.
#[test]
fn a_non_finite_spelling_keeps_the_parsed_float() {
    for v in ["nan", "inf", "-inf", "NaN", "INF"] {
        let cell = read_cell(&[("N", "Width"), ("U", "IN"), ("V", v)]).expect("reads");
        assert!(
            matches!(cell.value, Some(CellValue::Number(number)) if !number.is_finite()),
            "V={v:?} -> {:?}",
            cell.value
        );
    }
}

/// P6-11: an error cell keeps its `E` text verbatim and preserves the
/// `V` and `U` it was written with (task P2-06), whatever the text is.
#[test]
fn an_error_cell_keeps_its_text_and_preserves_its_value() {
    let cell = read_cell(&[
        ("N", "PinX"),
        ("U", "IN"),
        ("V", "junk"),
        ("E", "#Nonsense!"),
    ])
    .expect("reads");
    assert_eq!(
        cell.value,
        Some(CellValue::Error(String::from("#Nonsense!")))
    );
    let kept: Vec<&str> = cell
        .preserved
        .attributes
        .iter()
        .map(|attribute| attribute.value.as_str())
        .collect();
    assert!(kept.contains(&"junk"), "{:?}", cell.preserved);
    assert!(kept.contains(&"IN"), "{:?}", cell.preserved);
}

/// P6-11: the error spelling overrides the declared kind: a bad `V`
/// beside an `E` is never parsed and never fails the read.
#[test]
fn an_error_cell_never_parses_its_value() {
    let cell = read_cell(&[
        ("N", "PinX"),
        ("U", "BOOL"),
        ("V", "not a boolean"),
        ("E", "#VALUE!"),
    ])
    .expect("reads");
    assert_eq!(cell.value, Some(CellValue::Error(String::from("#VALUE!"))));
}

/// P6-11: a `LIST` spelling keeps its raw text whatever it holds, so a
/// malformed list never fails the read (the raw text is what a write
/// emits back).
#[test]
fn a_list_value_keeps_its_raw_text() {
    let mut list = String::from(";;");
    for index in 0..1_000 {
        list.push_str(&format!("entry {index};;;"));
    }
    let cell = read_cell(&[("N", "Format"), ("U", "LIST"), ("V", list.as_str())]).expect("reads");
    assert_eq!(cell.value, Some(CellValue::List(list)));
}

/// P6-11: a `Cell` element with no `N` names no cell and is dropped by
/// the reader: no panic, no error.
#[test]
fn a_cell_with_no_name_is_dropped() {
    let document = to_model(&package(
        &drawing_with_cell(&[("U", "MM"), ("V", "x")]),
        DRAWING_RELS,
    ))
    .expect("reads");
    let page = document.pages.first().expect("page");
    let shape = document
        .shape(*page.shapes.first().expect("shape"))
        .expect("shape entity");
    assert!(shape.sheet.cells.is_empty(), "{:?}", shape.sheet.cells);
}
