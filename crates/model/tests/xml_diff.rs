//! Integration tests for the semantic XML equivalence relation
//! ([`diff`], ADR 0012) through the public API of `ferrogram-model`.

use ferrogram_model::{
    DifferenceKind, QName, XmlAttribute, XmlChild, XmlDifference, XmlNode, diff,
};

fn element(local: &str, attributes: Vec<XmlAttribute>, children: Vec<XmlChild>) -> XmlNode {
    XmlNode {
        name: QName::local(local),
        attributes,
        children,
        self_closing: false,
    }
}

fn attr(local: &str, value: &str) -> XmlAttribute {
    XmlAttribute {
        name: QName::local(local),
        value: value.to_owned(),
    }
}

#[test]
fn a_rewritten_part_is_equivalent() {
    // What a writer may change: prefix spelling, attribute order,
    // self-closing spelling and pretty-print whitespace.
    let source = XmlNode {
        name: QName {
            prefix: Some("v".to_owned()),
            uri: Some("urn:visio".to_owned()),
            local: "Shape".to_owned(),
        },
        attributes: vec![
            XmlAttribute {
                name: QName {
                    prefix: Some("v".to_owned()),
                    uri: Some("urn:visio".to_owned()),
                    local: "ID".to_owned(),
                },
                value: "1".to_owned(),
            },
            attr("N", "PinX"),
        ],
        children: vec![
            XmlChild::Text("\n  ".to_owned()),
            XmlChild::Element(XmlNode {
                name: QName {
                    prefix: Some("v".to_owned()),
                    uri: Some("urn:visio".to_owned()),
                    local: "Cell".to_owned(),
                },
                attributes: vec![attr("V", "0.5")],
                children: Vec::new(),
                self_closing: true,
            }),
            XmlChild::Text("\n".to_owned()),
        ],
        self_closing: false,
    };
    let written = element(
        "Shape",
        vec![attr("N", "PinX"), attr("ID", "1")],
        vec![XmlChild::Element(element(
            "Cell",
            vec![attr("V", ".50")],
            Vec::new(),
        ))],
    );
    // Same URIs, but no prefixes in `written`: compare a URI-carrying copy.
    let mut written = written;
    written.name.uri = Some("urn:visio".to_owned());
    for a in &mut written.attributes {
        if a.name.local == "ID" {
            a.name.uri = Some("urn:visio".to_owned());
        }
    }
    if let Some(XmlChild::Element(cell)) = written.children.first_mut() {
        cell.name.uri = Some("urn:visio".to_owned());
    }
    assert_eq!(diff(&source, &written), Ok(Vec::new()));
}

#[test]
fn a_lost_child_is_reported_with_its_path() {
    let source = element(
        "Pages",
        Vec::new(),
        vec![
            XmlChild::Element(element("Page", vec![attr("ID", "1")], Vec::new())),
            XmlChild::Element(element("Page", vec![attr("ID", "2")], Vec::new())),
        ],
    );
    let written = element(
        "Pages",
        Vec::new(),
        vec![XmlChild::Element(element(
            "Page",
            vec![attr("ID", "1")],
            Vec::new(),
        ))],
    );
    assert_eq!(
        diff(&source, &written),
        Ok(vec![XmlDifference {
            path: "/Pages[1]/Page[2]".to_owned(),
            kind: DifferenceKind::Child,
        }])
    );
}

#[test]
fn comments_and_instructions_survive_a_rewrite() {
    let source = element(
        "r",
        Vec::new(),
        vec![
            XmlChild::Text("\n".to_owned()),
            XmlChild::Comment("kept".to_owned()),
            XmlChild::Text("\n".to_owned()),
            XmlChild::ProcessingInstruction {
                target: "pi".to_owned(),
                data: "d".to_owned(),
            },
        ],
    );
    let written = element(
        "r",
        Vec::new(),
        vec![
            XmlChild::Comment("kept".to_owned()),
            XmlChild::ProcessingInstruction {
                target: "pi".to_owned(),
                data: "d".to_owned(),
            },
        ],
    );
    assert_eq!(diff(&source, &written), Ok(Vec::new()));

    let lost = element("r", Vec::new(), vec![XmlChild::Comment("kept".to_owned())]);
    assert_eq!(
        diff(&source, &lost),
        Ok(vec![XmlDifference {
            path: "/r[1]/processing-instruction()[1]".to_owned(),
            kind: DifferenceKind::Child,
        }])
    );
}
