//! Preserved unknown XML and the semantic XML equivalence relation
//! (ADR 0012).
//!
//! Unknown elements, attributes and text are kept as [`XmlNode`] trees so a
//! write can put them back verbatim. [`diff`] compares two trees under the
//! semantic equivalence defined in ADR 0012: namespace prefixes, attribute
//! order and self-closing versus empty elements do not matter, while element
//! order, namespace URIs and text content do.

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
    /// Text content differs.
    Text,
}

/// One difference found by [`diff`], with its location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlDifference {
    /// Where the difference is, as an XPath-like path, e.g.
    /// `/Pages[1]/Page[2]/@ID`.
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
pub fn diff(a: &XmlNode, b: &XmlNode) -> Result<Vec<XmlDifference>, crate::Error> {
    let _ = (a, b);
    Err(crate::Error::NotImplemented("raw::diff"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_element(local: &str) -> XmlNode {
        XmlNode {
            name: QName::local(local),
            attributes: Vec::new(),
            children: Vec::new(),
            self_closing: true,
        }
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
            attributes: vec![XmlAttribute {
                name: QName::local("ID"),
                value: "1".to_owned(),
            }],
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
    fn diff_is_a_stub() {
        let a = empty_element("Cell");
        assert_eq!(diff(&a, &a), Err(crate::Error::NotImplemented("raw::diff")));
    }

    #[test]
    fn difference_displays_its_path_and_kind() {
        let d = XmlDifference {
            path: "/Cell[1]".to_owned(),
            kind: DifferenceKind::AttributeValue,
        };
        assert_eq!(d.to_string(), "/Cell[1]: AttributeValue");
    }
}
