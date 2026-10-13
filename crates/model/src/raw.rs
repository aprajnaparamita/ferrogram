//! Preserved unknown XML and the semantic XML equivalence relation
//! (ADR 0012).
//!
//! Unknown elements, attributes and text are kept as [`XmlNode`] trees so a
//! write can put them back verbatim. [`diff`] compares two trees under the
//! semantic equivalence defined in ADR 0012: namespace prefixes, attribute
//! order and self-closing versus empty elements do not matter, while element
//! order, namespace URIs and text content do.
//!
//! The rule table of ADR 0012, as implemented by [`diff`]:
//!
//! - **Element name**: equal when local name and namespace URI are equal;
//!   namespace prefixes are ignored, and namespace declarations are not
//!   attributes for comparison.
//! - **Attribute set**: compared by (namespace URI, local name); order is
//!   ignored. An omitted attribute is a presence difference, never equal to
//!   an empty one.
//! - **Attribute value**: strings compare exactly, except that a value made
//!   of a number literal with an optional unit suffix compares numerically
//!   against another such value with the same suffix (`0.5` equals `.50`).
//!   Units are never converted: `1 in` and `25.4 mm` differ.
//! - **Child elements**: compared in document order; reordering is a
//!   difference.
//! - **Text**: character data compares exactly. A whitespace-only text node
//!   is ignored when the parent's content is element-like (the parent has at
//!   least one element, comment or processing-instruction child and no
//!   non-whitespace text child); that is pretty-printing. Any other text,
//!   including whitespace inside text-bearing content, is significant.
//!   Empty text nodes carry no character data and are ignored.
//! - **Self-closing**: `<a/>` and `<a></a>` are equivalent.
//! - **Comments and PIs**: significant; compared by data (and target for
//!   PIs) exactly.

use std::fmt;

/// A qualified XML name: prefix, namespace URI and local name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QName {
    /// The namespace prefix as written, without the colon, or `None`.
    pub prefix: Option<String>,
    /// The namespace URI, or `None` for no namespace.
    pub uri: Option<String>,
    /// The local name.
    pub local: String,
}

impl QName {
    /// Creates a qualified name with a local name and no prefix or URI.
    pub fn local(local: impl Into<String>) -> Self {
        Self {
            prefix: None,
            uri: None,
            local: local.into(),
        }
    }
}

/// One attribute of an [`XmlNode`], in document order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlAttribute {
    /// The attribute name.
    pub name: QName,
    /// The attribute value as written (entity references are decoded).
    pub value: String,
}

/// One child of an [`XmlNode`], in document order.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum XmlChild {
    /// A child element.
    Element(XmlNode),
    /// Character data.
    Text(String),
    /// A comment, without the `<!--` and `-->` markers.
    Comment(String),
    /// A processing instruction, without `<?` and `?>`.
    ProcessingInstruction {
        /// The instruction target.
        target: String,
        /// The instruction data.
        data: String,
    },
}

/// A preserved XML element.
#[derive(Debug, Clone, PartialEq)]
pub struct XmlNode {
    /// The element name.
    pub name: QName,
    /// The attributes, in document order, namespace declarations included.
    pub attributes: Vec<XmlAttribute>,
    /// The children, in document order.
    pub children: Vec<XmlChild>,
    /// Whether the element was written self-closing (`<a/>`) in the source.
    /// Stored for byte-faithful writes; ignored by [`diff`].
    pub self_closing: bool,
}

/// How two XML trees differ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DifferenceKind {
    /// The element names differ.
    ElementName,
    /// The namespace URIs differ.
    NamespaceUri,
    /// One tree has an attribute the other does not.
    AttributePresence,
    /// An attribute value differs.
    AttributeValue,
    /// A child exists in one tree only, or children are in a different order.
    Child,
    /// Text content differs (including comment and processing-instruction
    /// data).
    Text,
}

/// One difference found by [`diff`], with its location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlDifference {
    /// Where the difference is, as an XPath-like path. Elements carry their
    /// display name and a 1-based occurrence index among same-named element
    /// siblings (`/Pages[1]/Page[2]`), attributes are `@name` on their
    /// element (`/Pages[1]/Page[2]/@ID`), and text, comment and
    /// processing-instruction children are `text()[k]`, `comment()[k]` and
    /// `processing-instruction()[k]` on their parent.
    pub path: String,
    /// What differs.
    pub kind: DifferenceKind,
}

/// The preserved, unmapped part of one modeled XML element (ADR 0012).
///
/// Every modeled element keeps the attributes it does not model and the child
/// elements the model does not map, so a write can put them back verbatim.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Preserved {
    /// Attributes not mapped to any field, in document order.
    pub attributes: Vec<XmlAttribute>,
    /// Child elements not mapped to any field, in document order.
    pub children: Vec<XmlNode>,
}

impl Preserved {
    /// Creates an empty preserved set.
    pub fn new() -> Self {
        Self::default()
    }
}

impl fmt::Display for XmlDifference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {:?}", self.path, self.kind)
    }
}

/// Compares two XML trees under the semantic equivalence of ADR 0012 and
/// returns every difference, in document order.
///
/// Empty output means the trees are semantically equivalent: same element
/// names and namespace URIs, same attributes ignoring order and prefix, same
/// children in order, same text. Self-closing and empty elements are
/// equivalent, and attribute values compare as strings except that numeric
/// spellings compare numerically (`0.5` equals `.50`).
///
/// The call always succeeds; the `Result` keeps the signature stable for
/// callers that compare whole packages (ADR 0012).
pub fn diff(a: &XmlNode, b: &XmlNode) -> Result<Vec<XmlDifference>, crate::Error> {
    let mut differences = Vec::new();
    let path = format!("/{}[1]", display_name(&a.name));
    compare_element(a, b, &path, &mut differences);
    Ok(differences)
}

/// The name as written, `prefix:local` or `local`.
fn display_name(name: &QName) -> String {
    match &name.prefix {
        Some(prefix) => format!("{prefix}:{}", name.local),
        None => name.local.clone(),
    }
}

/// The namespace URI, with `None` (no namespace) read as the empty URI.
fn uri_of(name: &QName) -> &str {
    name.uri.as_deref().unwrap_or("")
}

/// Whether the string is empty or made only of whitespace.
fn blank(text: &str) -> bool {
    text.chars().all(char::is_whitespace)
}

/// Whether the attribute is a namespace declaration (`xmlns="..."` or
/// `xmlns:prefix="..."`). Declarations are not attributes for comparison
/// (ADR 0012).
fn is_namespace_declaration(attr: &XmlAttribute) -> bool {
    match attr.name.prefix.as_deref() {
        Some(prefix) => prefix == "xmlns",
        None => attr.name.local == "xmlns",
    }
}

/// Whether the parent's content is element-like: it has at least one
/// non-text child (element, comment or processing instruction) and no
/// non-whitespace text child. Whitespace-only text inside such content is
/// pretty-printing and is ignored by [`diff`].
fn element_like_content(node: &XmlNode) -> bool {
    let mut has_markup = false;
    let mut has_text = false;
    for child in &node.children {
        match child {
            XmlChild::Text(text) => has_text |= !blank(text),
            _ => has_markup = true,
        }
    }
    has_markup && !has_text
}

/// The children that take part in the comparison, in document order:
/// every child except ignorable pretty-printing whitespace (and empty text
/// nodes, which carry no character data).
fn significant_children(node: &XmlNode) -> Vec<&XmlChild> {
    let element_like = element_like_content(node);
    node.children
        .iter()
        .filter(|child| match child {
            XmlChild::Text(text) => !(text.is_empty() || (blank(text) && element_like)),
            _ => true,
        })
        .collect()
}

/// The 1-based occurrence of the element child at `index` among its
/// same-named element siblings (namespace URI and local name), for the path.
fn element_occurrence(children: &[&XmlChild], index: usize) -> usize {
    let wanted = match children.get(index) {
        Some(XmlChild::Element(node)) => node,
        _ => return 1,
    };
    let mut seen = 0;
    for (i, child) in children.iter().enumerate() {
        if let XmlChild::Element(node) = child
            && node.name.local == wanted.name.local
            && uri_of(&node.name) == uri_of(&wanted.name)
        {
            seen += 1;
            if i == index {
                break;
            }
        }
    }
    seen.max(1)
}

/// The 1-based occurrence of the child at `index` among its same-kind
/// siblings, for the path of non-element children.
fn child_occurrence(children: &[&XmlChild], index: usize) -> usize {
    let wanted = match children.get(index) {
        Some(child) => child,
        None => return 1,
    };
    let mut seen = 0;
    for (i, child) in children.iter().enumerate() {
        if std::mem::discriminant(*child) == std::mem::discriminant(wanted) {
            seen += 1;
            if i == index {
                break;
            }
        }
    }
    seen.max(1)
}

/// Compares two elements and their subtrees.
fn compare_element(a: &XmlNode, b: &XmlNode, path: &str, out: &mut Vec<XmlDifference>) {
    if a.name.local != b.name.local {
        out.push(XmlDifference {
            path: path.to_owned(),
            kind: DifferenceKind::ElementName,
        });
    }
    if uri_of(&a.name) != uri_of(&b.name) {
        out.push(XmlDifference {
            path: path.to_owned(),
            kind: DifferenceKind::NamespaceUri,
        });
    }
    compare_attributes(a, b, path, out);
    compare_children(a, b, path, out);
}

/// Compares the attribute sets by (namespace URI, local name), order and
/// prefix ignored.
fn compare_attributes(a: &XmlNode, b: &XmlNode, path: &str, out: &mut Vec<XmlDifference>) {
    let a_attrs: Vec<&XmlAttribute> = a
        .attributes
        .iter()
        .filter(|attr| !is_namespace_declaration(attr))
        .collect();
    let b_attrs: Vec<&XmlAttribute> = b
        .attributes
        .iter()
        .filter(|attr| !is_namespace_declaration(attr))
        .collect();
    let mut matched = vec![false; b_attrs.len()];
    for attr in &a_attrs {
        let hit = b_attrs.iter().enumerate().position(|(i, other)| {
            !matched[i]
                && other.name.local == attr.name.local
                && uri_of(&other.name) == uri_of(&attr.name)
        });
        match hit {
            Some(i) => {
                matched[i] = true;
                if !values_equal(&attr.value, &b_attrs[i].value) {
                    out.push(XmlDifference {
                        path: format!("{path}/@{}", display_name(&attr.name)),
                        kind: DifferenceKind::AttributeValue,
                    });
                }
            }
            None => out.push(XmlDifference {
                path: format!("{path}/@{}", display_name(&attr.name)),
                kind: DifferenceKind::AttributePresence,
            }),
        }
    }
    for (i, attr) in b_attrs.iter().enumerate() {
        if !matched[i] {
            out.push(XmlDifference {
                path: format!("{path}/@{}", display_name(&attr.name)),
                kind: DifferenceKind::AttributePresence,
            });
        }
    }
}

/// Compares the significant children pairwise, in document order.
fn compare_children(a: &XmlNode, b: &XmlNode, path: &str, out: &mut Vec<XmlDifference>) {
    let a_children = significant_children(a);
    let b_children = significant_children(b);
    let shared = a_children.len().min(b_children.len());
    for i in 0..shared {
        let (left, right) = (a_children[i], b_children[i]);
        let child_path = child_path(path, left, &a_children, i);
        compare_child(left, right, &child_path, out);
    }
    for i in shared..a_children.len() {
        out.push(XmlDifference {
            path: child_path(path, a_children[i], &a_children, i),
            kind: DifferenceKind::Child,
        });
    }
    for i in shared..b_children.len() {
        out.push(XmlDifference {
            path: child_path(path, b_children[i], &b_children, i),
            kind: DifferenceKind::Child,
        });
    }
}

/// The path of one child under its parent's path.
fn child_path(parent: &str, child: &XmlChild, siblings: &[&XmlChild], index: usize) -> String {
    match child {
        XmlChild::Element(node) => format!(
            "{parent}/{}[{}]",
            display_name(&node.name),
            element_occurrence(siblings, index)
        ),
        XmlChild::Text(_) => format!("{parent}/text()[{}]", child_occurrence(siblings, index)),
        XmlChild::Comment(_) => {
            format!("{parent}/comment()[{}]", child_occurrence(siblings, index))
        }
        XmlChild::ProcessingInstruction { .. } => format!(
            "{parent}/processing-instruction()[{}]",
            child_occurrence(siblings, index)
        ),
    }
}

/// Compares two children that sit at the same position.
fn compare_child(a: &XmlChild, b: &XmlChild, path: &str, out: &mut Vec<XmlDifference>) {
    match (a, b) {
        (XmlChild::Element(a), XmlChild::Element(b)) => compare_element(a, b, path, out),
        (XmlChild::Text(a), XmlChild::Text(b)) => {
            if a != b {
                out.push(XmlDifference {
                    path: path.to_owned(),
                    kind: DifferenceKind::Text,
                });
            }
        }
        (XmlChild::Comment(a), XmlChild::Comment(b)) => {
            if a != b {
                out.push(XmlDifference {
                    path: path.to_owned(),
                    kind: DifferenceKind::Text,
                });
            }
        }
        (
            XmlChild::ProcessingInstruction {
                target: ta,
                data: da,
            },
            XmlChild::ProcessingInstruction {
                target: tb,
                data: db,
            },
        ) => {
            if ta != tb || da != db {
                out.push(XmlDifference {
                    path: path.to_owned(),
                    kind: DifferenceKind::Text,
                });
            }
        }
        _ => out.push(XmlDifference {
            path: path.to_owned(),
            kind: DifferenceKind::Child,
        }),
    }
}

/// Compares two attribute or text values.
///
/// Values compare exactly, except that two values that each consist of a
/// number literal with an optional unit suffix compare numerically when
/// their suffixes are equal: `0.5` equals `.50`, and `0.5 in` equals `.50
/// in`, but `1 in` differs from `1 mm` (units are never converted).
fn values_equal(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    match (parse_number_value(a), parse_number_value(b)) {
        (Some((na, sa)), Some((nb, sb))) => sa == sb && na == nb,
        _ => false,
    }
}

/// Parses a number literal at the start of `text` and returns its value with
/// the remaining (unit) suffix, or `None` when `text` does not start with a
/// number literal.
///
/// The grammar is `[-+]? ( digits [ '.' [ digits ] ] | '.' digits )
/// ( [eE] [-+]? digits )?`; anything after it is the suffix. `inf` and
/// `NaN` are not number literals here.
pub(crate) fn parse_number_value(text: &str) -> Option<(f64, &str)> {
    let bytes = text.as_bytes();
    let mut end = 0;
    if matches!(bytes.first().copied(), Some(b'+' | b'-')) {
        end += 1;
    }
    let integer_start = end;
    while bytes.get(end).is_some_and(u8::is_ascii_digit) {
        end += 1;
    }
    let integer_digits = end - integer_start;
    let mut fraction_digits = 0;
    if bytes.get(end).copied() == Some(b'.') {
        end += 1;
        let fraction_start = end;
        while bytes.get(end).is_some_and(u8::is_ascii_digit) {
            end += 1;
        }
        fraction_digits = end - fraction_start;
    }
    if integer_digits == 0 && fraction_digits == 0 {
        return None;
    }
    if matches!(bytes.get(end).copied(), Some(b'e' | b'E')) {
        // An exponent is part of the literal only when it has digits.
        let mut probe = end + 1;
        if matches!(bytes.get(probe).copied(), Some(b'+' | b'-')) {
            probe += 1;
        }
        let exponent_start = probe;
        while bytes.get(probe).is_some_and(u8::is_ascii_digit) {
            probe += 1;
        }
        if probe > exponent_start {
            end = probe;
        }
    }
    let (head, tail) = text.split_at(end);
    let value: f64 = head.parse().ok()?;
    Some((value, tail))
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::strategy::Strategy;

    fn empty_element(local: &str) -> XmlNode {
        XmlNode {
            name: QName::local(local),
            attributes: Vec::new(),
            children: Vec::new(),
            self_closing: true,
        }
    }

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

    fn assert_diffs(a: &XmlNode, b: &XmlNode, want: &[(&str, DifferenceKind)]) {
        let got = diff(a, b).expect("diff never fails");
        let want: Vec<XmlDifference> = want
            .iter()
            .map(|(path, kind)| XmlDifference {
                path: (*path).to_owned(),
                kind: *kind,
            })
            .collect();
        assert_eq!(got, want);
    }

    #[test]
    fn qname_local_has_no_prefix_or_uri() {
        let q = QName::local("Cell");
        assert_eq!(q.local, "Cell");
        assert_eq!(q.prefix, None);
        assert_eq!(q.uri, None);
    }

    #[test]
    fn nodes_compare_by_value() {
        assert_eq!(empty_element("Cell"), empty_element("Cell"));
        assert_ne!(empty_element("Cell"), empty_element("Row"));
    }

    #[test]
    fn children_keep_document_order() {
        let n = XmlNode {
            name: QName::local("Shape"),
            attributes: vec![attr("ID", "1")],
            children: vec![
                XmlChild::Element(empty_element("Cell")),
                XmlChild::Text("text".to_owned()),
            ],
            self_closing: false,
        };
        assert_eq!(n.attributes.len(), 1);
        assert_eq!(n.children.len(), 2);
    }

    #[test]
    fn preserved_defaults_to_empty() {
        let p = Preserved::new();
        assert!(p.attributes.is_empty());
        assert!(p.children.is_empty());
        assert_eq!(p, Preserved::default());
    }

    #[test]
    fn difference_displays_its_path_and_kind() {
        let d = XmlDifference {
            path: "/Cell[1]".to_owned(),
            kind: DifferenceKind::AttributeValue,
        };
        assert_eq!(d.to_string(), "/Cell[1]: AttributeValue");
    }

    // --- ADR 0012 rule table -------------------------------------------

    #[test]
    fn identical_trees_are_equivalent() {
        let a = element(
            "Shape",
            vec![attr("ID", "1")],
            vec![
                XmlChild::Element(element("Cell", vec![attr("N", "PinX")], Vec::new())),
                XmlChild::Text("hello".to_owned()),
            ],
        );
        let b = a.clone();
        assert_eq!(diff(&a, &b), Ok(Vec::new()));
    }

    #[test]
    fn namespace_prefix_is_ignored() {
        let a = XmlNode {
            name: QName {
                prefix: Some("a".to_owned()),
                uri: Some("urn:x".to_owned()),
                local: "Cell".to_owned(),
            },
            attributes: vec![XmlAttribute {
                name: QName {
                    prefix: Some("a".to_owned()),
                    uri: Some("urn:attr".to_owned()),
                    local: "N".to_owned(),
                },
                value: "PinX".to_owned(),
            }],
            children: Vec::new(),
            self_closing: true,
        };
        let b = XmlNode {
            name: QName {
                prefix: None,
                uri: Some("urn:x".to_owned()),
                local: "Cell".to_owned(),
            },
            attributes: vec![XmlAttribute {
                name: QName {
                    prefix: None,
                    uri: Some("urn:attr".to_owned()),
                    local: "N".to_owned(),
                },
                value: "PinX".to_owned(),
            }],
            children: Vec::new(),
            self_closing: false,
        };
        assert_eq!(diff(&a, &b), Ok(Vec::new()));
    }

    #[test]
    fn namespace_declarations_are_not_attributes() {
        let a = element(
            "Shape",
            vec![
                XmlAttribute {
                    name: QName {
                        prefix: None,
                        uri: None,
                        local: "xmlns".to_owned(),
                    },
                    value: "urn:x".to_owned(),
                },
                XmlAttribute {
                    name: QName {
                        prefix: Some("xmlns".to_owned()),
                        uri: None,
                        local: "r".to_owned(),
                    },
                    value: "urn:rel".to_owned(),
                },
            ],
            Vec::new(),
        );
        let b = empty_element("Shape");
        assert_eq!(diff(&a, &b), Ok(Vec::new()));
    }

    #[test]
    fn attribute_order_is_ignored() {
        let a = element("Shape", vec![attr("ID", "1"), attr("N", "x")], Vec::new());
        let b = element("Shape", vec![attr("N", "x"), attr("ID", "1")], Vec::new());
        assert_eq!(diff(&a, &b), Ok(Vec::new()));
    }

    #[test]
    fn attribute_names_compare_by_uri_and_local_name() {
        let a = element(
            "Shape",
            vec![XmlAttribute {
                name: QName {
                    prefix: Some("p".to_owned()),
                    uri: Some("urn:x".to_owned()),
                    local: "ID".to_owned(),
                },
                value: "1".to_owned(),
            }],
            Vec::new(),
        );
        let same = element("Shape", vec![attr("ID", "1")], Vec::new());
        // Same local name, different URI: not the same attribute.
        assert_diffs(
            &a,
            &same,
            &[
                ("/Shape[1]/@p:ID", DifferenceKind::AttributePresence),
                ("/Shape[1]/@ID", DifferenceKind::AttributePresence),
            ],
        );
    }

    #[test]
    fn numeric_attribute_spellings_are_equal() {
        let a = element("Cell", vec![attr("V", "0.5")], Vec::new());
        let b = element("Cell", vec![attr("V", ".50")], Vec::new());
        assert_eq!(diff(&a, &b), Ok(Vec::new()));

        let c = element("Cell", vec![attr("V", "1e3")], Vec::new());
        let d = element("Cell", vec![attr("V", "1000")], Vec::new());
        assert_eq!(diff(&c, &d), Ok(Vec::new()));
    }

    #[test]
    fn numeric_values_with_equal_suffixes_are_equal() {
        let a = element("Cell", vec![attr("V", "0.5 in")], Vec::new());
        let b = element("Cell", vec![attr("V", ".50 in")], Vec::new());
        assert_eq!(diff(&a, &b), Ok(Vec::new()));
    }

    #[test]
    fn lengths_do_not_convert_units() {
        let a = element("Cell", vec![attr("V", "1 in")], Vec::new());
        let b = element("Cell", vec![attr("V", "25.4 mm")], Vec::new());
        assert_diffs(&a, &b, &[("/Cell[1]/@V", DifferenceKind::AttributeValue)]);

        let c = element("Cell", vec![attr("V", "1 in")], Vec::new());
        let d = element("Cell", vec![attr("V", "1 mm")], Vec::new());
        assert_diffs(&c, &d, &[("/Cell[1]/@V", DifferenceKind::AttributeValue)]);
    }

    #[test]
    fn non_numeric_strings_compare_exactly() {
        let a = element("Cell", vec![attr("N", "0.5")], Vec::new());
        let b = element("Cell", vec![attr("N", "abc")], Vec::new());
        assert_diffs(&a, &b, &[("/Cell[1]/@N", DifferenceKind::AttributeValue)]);
        // A number and a non-number are never equal.
        let c = element("Cell", vec![attr("V", "0.5")], Vec::new());
        let d = element("Cell", vec![attr("V", ".5x")], Vec::new());
        assert_diffs(&c, &d, &[("/Cell[1]/@V", DifferenceKind::AttributeValue)]);
    }

    #[test]
    fn omitted_attribute_is_not_empty_attribute() {
        let a = element("Cell", Vec::new(), Vec::new());
        let b = element("Cell", vec![attr("V", "")], Vec::new());
        assert_diffs(
            &a,
            &b,
            &[("/Cell[1]/@V", DifferenceKind::AttributePresence)],
        );
    }

    #[test]
    fn child_order_is_significant() {
        let a = element(
            "Shapes",
            Vec::new(),
            vec![
                XmlChild::Element(element("A", Vec::new(), Vec::new())),
                XmlChild::Element(element("B", Vec::new(), Vec::new())),
            ],
        );
        let b = element(
            "Shapes",
            Vec::new(),
            vec![
                XmlChild::Element(element("B", Vec::new(), Vec::new())),
                XmlChild::Element(element("A", Vec::new(), Vec::new())),
            ],
        );
        assert_diffs(
            &a,
            &b,
            &[
                ("/Shapes[1]/A[1]", DifferenceKind::ElementName),
                ("/Shapes[1]/B[1]", DifferenceKind::ElementName),
            ],
        );
    }

    #[test]
    fn missing_child_is_a_child_difference() {
        let a = element(
            "Shapes",
            Vec::new(),
            vec![
                XmlChild::Element(element("A", Vec::new(), Vec::new())),
                XmlChild::Element(element("B", Vec::new(), Vec::new())),
            ],
        );
        let b = element(
            "Shapes",
            Vec::new(),
            vec![XmlChild::Element(element("A", Vec::new(), Vec::new()))],
        );
        assert_diffs(&a, &b, &[("/Shapes[1]/B[1]", DifferenceKind::Child)]);
        assert_diffs(&b, &a, &[("/Shapes[1]/B[1]", DifferenceKind::Child)]);
    }

    #[test]
    fn whitespace_between_elements_is_ignored() {
        let a = element(
            "Shapes",
            Vec::new(),
            vec![
                XmlChild::Text("\n  ".to_owned()),
                XmlChild::Element(element("A", Vec::new(), Vec::new())),
                XmlChild::Text("\n".to_owned()),
                XmlChild::Element(element("B", Vec::new(), Vec::new())),
                XmlChild::Text("\n".to_owned()),
            ],
        );
        let b = element(
            "Shapes",
            Vec::new(),
            vec![
                XmlChild::Element(element("A", Vec::new(), Vec::new())),
                XmlChild::Element(element("B", Vec::new(), Vec::new())),
            ],
        );
        assert_eq!(diff(&a, &b), Ok(Vec::new()));
    }

    #[test]
    fn whitespace_around_comments_is_ignored() {
        let a = element(
            "Shapes",
            Vec::new(),
            vec![
                XmlChild::Text("\n".to_owned()),
                XmlChild::Comment("note".to_owned()),
                XmlChild::Text("\n".to_owned()),
            ],
        );
        let b = element(
            "Shapes",
            Vec::new(),
            vec![XmlChild::Comment("note".to_owned())],
        );
        assert_eq!(diff(&a, &b), Ok(Vec::new()));
    }

    #[test]
    fn whitespace_in_text_content_is_significant() {
        let a = element("Text", Vec::new(), vec![XmlChild::Text(" ".to_owned())]);
        let b = element("Text", Vec::new(), Vec::new());
        // The whitespace is the text content of the element, not
        // pretty-printing: its presence is a difference.
        assert_diffs(&a, &b, &[("/Text[1]/text()[1]", DifferenceKind::Child)]);

        let c = element("doc", Vec::new(), vec![XmlChild::Text(" x ".to_owned())]);
        let d = element("doc", Vec::new(), vec![XmlChild::Text("x".to_owned())]);
        assert_diffs(&c, &d, &[("/doc[1]/text()[1]", DifferenceKind::Text)]);
    }

    #[test]
    fn significant_text_is_compared() {
        let a = element("Text", Vec::new(), vec![XmlChild::Text("a".to_owned())]);
        let b = element("Text", Vec::new(), vec![XmlChild::Text("b".to_owned())]);
        assert_diffs(&a, &b, &[("/Text[1]/text()[1]", DifferenceKind::Text)]);
    }

    #[test]
    fn self_closing_equals_empty_element() {
        let a = empty_element("Cell");
        let b = element("Cell", Vec::new(), Vec::new());
        assert_eq!(diff(&a, &b), Ok(Vec::new()));
    }

    #[test]
    fn comments_and_instructions_are_significant() {
        let a = element(
            "r",
            Vec::new(),
            vec![
                XmlChild::Comment("one".to_owned()),
                XmlChild::ProcessingInstruction {
                    target: "pi".to_owned(),
                    data: "d".to_owned(),
                },
            ],
        );
        let b = element(
            "r",
            Vec::new(),
            vec![
                XmlChild::Comment("two".to_owned()),
                XmlChild::ProcessingInstruction {
                    target: "pi".to_owned(),
                    data: "d".to_owned(),
                },
            ],
        );
        assert_diffs(&a, &b, &[("/r[1]/comment()[1]", DifferenceKind::Text)]);

        let c = element("r", Vec::new(), vec![XmlChild::Comment("one".to_owned())]);
        let d = element(
            "r",
            Vec::new(),
            vec![
                XmlChild::Comment("one".to_owned()),
                XmlChild::Comment("two".to_owned()),
            ],
        );
        assert_diffs(&c, &d, &[("/r[1]/comment()[2]", DifferenceKind::Child)]);

        let e = element(
            "r",
            Vec::new(),
            vec![XmlChild::ProcessingInstruction {
                target: "pi".to_owned(),
                data: "d".to_owned(),
            }],
        );
        let f = element(
            "r",
            Vec::new(),
            vec![XmlChild::ProcessingInstruction {
                target: "other".to_owned(),
                data: "d".to_owned(),
            }],
        );
        assert_diffs(
            &e,
            &f,
            &[("/r[1]/processing-instruction()[1]", DifferenceKind::Text)],
        );
    }

    #[test]
    fn mixed_child_kinds_are_child_differences() {
        let a = element("r", Vec::new(), vec![XmlChild::Text("x".to_owned())]);
        let b = element(
            "r",
            Vec::new(),
            vec![XmlChild::Element(element("a", Vec::new(), Vec::new()))],
        );
        assert_diffs(&a, &b, &[("/r[1]/text()[1]", DifferenceKind::Child)]);
    }

    #[test]
    fn element_names_and_uris_report_separately() {
        let a = XmlNode {
            name: QName {
                prefix: Some("p".to_owned()),
                uri: Some("urn:x".to_owned()),
                local: "A".to_owned(),
            },
            attributes: Vec::new(),
            children: Vec::new(),
            self_closing: true,
        };
        let b = XmlNode {
            name: QName {
                prefix: None,
                uri: Some("urn:y".to_owned()),
                local: "B".to_owned(),
            },
            attributes: Vec::new(),
            children: Vec::new(),
            self_closing: true,
        };
        assert_diffs(
            &a,
            &b,
            &[
                ("/p:A[1]", DifferenceKind::ElementName),
                ("/p:A[1]", DifferenceKind::NamespaceUri),
            ],
        );
    }

    #[test]
    fn nested_paths_name_every_step() {
        let a = element(
            "Pages",
            Vec::new(),
            vec![
                XmlChild::Element(element("Page", vec![attr("ID", "1")], Vec::new())),
                XmlChild::Element(element("Page", vec![attr("ID", "2")], Vec::new())),
            ],
        );
        let b = element(
            "Pages",
            Vec::new(),
            vec![
                XmlChild::Element(element("Page", vec![attr("ID", "1")], Vec::new())),
                XmlChild::Element(element("Page", vec![attr("ID", "3")], Vec::new())),
            ],
        );
        assert_diffs(
            &a,
            &b,
            &[("/Pages[1]/Page[2]/@ID", DifferenceKind::AttributeValue)],
        );
    }

    // --- number parsing -------------------------------------------------

    #[test]
    fn number_literals_parse() {
        assert_eq!(parse_number_value("0.5"), Some((0.5, "")));
        assert_eq!(parse_number_value(".50"), Some((0.5, "")));
        assert_eq!(parse_number_value("-2"), Some((-2.0, "")));
        assert_eq!(parse_number_value("+2.25 in"), Some((2.25, " in")));
        assert_eq!(parse_number_value("1e3"), Some((1000.0, "")));
        assert_eq!(parse_number_value("1E-2"), Some((0.01, "")));
    }

    #[test]
    fn non_number_literals_do_not_parse() {
        assert_eq!(parse_number_value(""), None);
        assert_eq!(parse_number_value("x"), None);
        assert_eq!(parse_number_value("+"), None);
        assert_eq!(parse_number_value("."), None);
        assert_eq!(parse_number_value("nan"), None);
        assert_eq!(parse_number_value("inf"), None);
    }

    #[test]
    fn trailing_e_without_digits_is_a_suffix() {
        assert_eq!(parse_number_value("1e"), Some((1.0, "e")));
        assert_eq!(parse_number_value("1e+"), Some((1.0, "e+")));
    }

    #[test]
    fn values_equal_handles_edge_cases() {
        assert!(values_equal("", ""));
        assert!(values_equal("-0.0", "0.0"));
        assert!(!values_equal("", "0"));
        assert!(!values_equal("1 in", "1"));
        assert!(values_equal("1 in", "1.0 in"));
    }

    // --- property test --------------------------------------------------

    fn arb_name() -> impl proptest::strategy::Strategy<Value = QName> {
        proptest::sample::select(vec![
            QName::local("a"),
            QName::local("b"),
            QName {
                prefix: Some("p".to_owned()),
                uri: Some("urn:x".to_owned()),
                local: "a".to_owned(),
            },
        ])
    }

    fn arb_attribute() -> impl proptest::strategy::Strategy<Value = XmlAttribute> {
        (
            arb_name(),
            proptest::sample::select(vec![
                String::from("0.5"),
                String::from(".50"),
                String::from("1"),
                String::from("x"),
                String::from(""),
                String::from("1 in"),
                String::from("#fff2cc"),
            ]),
        )
            .prop_map(|(name, value)| XmlAttribute { name, value })
    }

    fn arb_child() -> impl proptest::strategy::Strategy<Value = XmlChild> {
        proptest::prop_oneof![
            proptest::sample::select(vec![
                String::from(""),
                String::from(" "),
                String::from("\n  "),
                String::from("text"),
                String::from("0.5"),
            ])
            .prop_map(XmlChild::Text),
            proptest::sample::select(vec![String::from(""), String::from("c")])
                .prop_map(XmlChild::Comment),
            (proptest::sample::select(vec![String::from("pi"), String::from("t")])).prop_map(
                |target| XmlChild::ProcessingInstruction {
                    target,
                    data: "d".to_owned(),
                }
            ),
        ]
    }

    fn arb_node() -> impl proptest::strategy::Strategy<Value = XmlNode> {
        let leaf = (
            arb_name(),
            proptest::collection::vec(arb_attribute(), 0..4),
            proptest::collection::vec(arb_child(), 0..3),
        )
            .prop_map(|(name, attributes, mut children)| {
                children.push(XmlChild::Element(XmlNode {
                    name: QName::local("leaf"),
                    attributes: Vec::new(),
                    children: Vec::new(),
                    self_closing: true,
                }));
                XmlNode {
                    name,
                    attributes,
                    children,
                    self_closing: false,
                }
            });
        leaf.prop_recursive(3, 12, 3, |inner| {
            (
                arb_name(),
                proptest::collection::vec(arb_attribute(), 0..3),
                proptest::collection::vec(
                    proptest::prop_oneof![inner.prop_map(XmlChild::Element), arb_child(),],
                    0..4,
                ),
            )
                .prop_map(|(name, attributes, children)| XmlNode {
                    name,
                    attributes,
                    children,
                    self_closing: false,
                })
        })
    }

    proptest::proptest! {
        /// `diff(a, a)` is empty for generated trees (ADR 0012).
        #[test]
        fn diff_of_a_tree_with_itself_is_empty(node in arb_node()) {
            proptest::prop_assert_eq!(diff(&node, &node), Ok(Vec::new()));
        }

        /// Self-closing spelling does not change equivalence.
        #[test]
        fn diff_ignores_self_closing(node in arb_node()) {
            let mut other = node.clone();
            other.self_closing = !node.self_closing;
            proptest::prop_assert_eq!(diff(&node, &other), Ok(Vec::new()));
        }
    }
}
