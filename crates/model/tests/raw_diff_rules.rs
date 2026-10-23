//! Rule-table edges of the semantic XML equivalence, through the public
//! [`diff`] (task P2-10, ADR 0012).
//!
//! The rule table is tested next to the implementation (`src/raw.rs`); these
//! tests pin the two edges the implementation's task called out by name:
//! a namespace-only difference and mixed content.

use larrygram_model::{QName, XmlAttribute, XmlChild, XmlNode, diff};

fn qname(prefix: Option<&str>, uri: Option<&str>, local: &str) -> QName {
    QName {
        prefix: prefix.map(str::to_owned),
        uri: uri.map(str::to_owned),
        local: local.to_owned(),
    }
}

fn element(name: QName, children: Vec<XmlChild>) -> XmlNode {
    XmlNode {
        name,
        attributes: Vec::new(),
        children,
        self_closing: false,
    }
}

#[test]
fn a_namespace_only_difference_is_a_uri_difference_not_a_prefix_one() {
    // Same local name and URI, different prefix: equal (the prefix is not
    // part of the name).
    let a = element(qname(Some("v"), Some("urn:x"), "Cell"), Vec::new());
    let b = element(qname(None, Some("urn:x"), "Cell"), Vec::new());
    assert_eq!(diff(&a, &b).expect("diff never fails"), Vec::new());

    // Same local name and prefix, different URI: a `NamespaceUri`
    // difference at the root.
    let c = element(qname(Some("v"), Some("urn:y"), "Cell"), Vec::new());
    let differences = diff(&a, &c).expect("diff never fails");
    assert_eq!(differences.len(), 1);
    assert_eq!(differences[0].path, "/v:Cell[1]");
    assert_eq!(
        differences[0].kind,
        larrygram_model::DifferenceKind::NamespaceUri
    );

    // The prefix alone changing is not a difference at any depth.
    let nested_a = element(qname(None, None, "a"), vec![XmlChild::Element(a.clone())]);
    let nested_b = element(qname(None, None, "a"), vec![XmlChild::Element(b.clone())]);
    assert_eq!(
        diff(&nested_a, &nested_b).expect("diff never fails"),
        Vec::new()
    );
}

#[test]
fn whitespace_only_text_in_mixed_content_is_significant() {
    // Mixed content (text-bearing, `docProps`-like): a whitespace-only text
    // node between two text nodes is character data and counts.
    let text = |value: &str| XmlChild::Text(value.to_owned());
    let with_gap = element(
        qname(None, None, "title"),
        vec![text("a"), text(" "), text("b")],
    );
    let without_gap = element(qname(None, None, "title"), vec![text("a"), text("b")]);
    assert!(
        !diff(&with_gap, &without_gap)
            .expect("diff never fails")
            .is_empty()
    );

    // The same whitespace node among element children is pretty-printing.
    let leaf = || element(qname(None, None, "Cell"), Vec::new());
    let pretty = element(
        qname(None, None, "Sheet"),
        vec![
            XmlChild::Element(leaf()),
            text(" "),
            XmlChild::Element(leaf()),
        ],
    );
    let tight = element(
        qname(None, None, "Sheet"),
        vec![XmlChild::Element(leaf()), XmlChild::Element(leaf())],
    );
    assert_eq!(diff(&pretty, &tight).expect("diff never fails"), Vec::new());
}

#[test]
fn attribute_values_compare_numerically_only_with_the_same_suffix() {
    let with_attr = |value: &str| {
        let mut node = element(qname(None, None, "Cell"), Vec::new());
        node.attributes.push(XmlAttribute {
            name: QName::local("V"),
            value: value.to_owned(),
        });
        node
    };
    assert_eq!(
        diff(&with_attr("0.5"), &with_attr(".50")).expect("diff never fails"),
        Vec::new()
    );
    assert_eq!(
        diff(&with_attr("1 in"), &with_attr("1.0 in")).expect("diff never fails"),
        Vec::new()
    );
    // A suffix difference is a value difference, not a numeric one.
    let differences = diff(&with_attr("1 in"), &with_attr("1 ft")).expect("diff never fails");
    assert_eq!(differences.len(), 1);
    assert_eq!(
        differences[0].kind,
        larrygram_model::DifferenceKind::AttributeValue
    );
}
