//! Small helpers over preserved [`XmlNode`] trees.

use ferrogram_model::{XmlAttribute, XmlChild, XmlNode};

/// The child elements of `node`, in document order.
pub(crate) fn elements(node: &XmlNode) -> impl Iterator<Item = &XmlNode> {
    node.children.iter().filter_map(|child| match child {
        XmlChild::Element(element) => Some(element),
        _ => None,
    })
}

/// The value of the first attribute with this local name, whatever its
/// prefix.
pub(crate) fn attribute<'n>(node: &'n XmlNode, local: &str) -> Option<&'n str> {
    node.attributes
        .iter()
        .find(|attribute| attribute.name.local == local)
        .map(|attribute| attribute.value.as_str())
}

/// The first attribute with this local name, whatever its prefix, cloned out
/// of the tree.
pub(crate) fn attribute_node(node: &XmlNode, local: &str) -> Option<XmlAttribute> {
    node.attributes
        .iter()
        .find(|attribute| attribute.name.local == local)
        .cloned()
}

/// Whether an attribute is a namespace declaration (`xmlns="..."` or
/// `xmlns:prefix="..."`).
///
/// Declarations are kept on [`XmlNode`] but are not data: they are neither
/// mapped nor preserved, and [`diff`](ferrogram_model::diff) ignores them.
pub(crate) fn is_namespace_declaration(attribute: &XmlAttribute) -> bool {
    match attribute.name.prefix.as_deref() {
        Some(prefix) => prefix == "xmlns",
        None => attribute.name.local == "xmlns",
    }
}

/// The concatenated text of an element and its descendants.
pub(crate) fn text_content(node: &XmlNode) -> String {
    let mut text = String::new();
    let mut stack = vec![node];
    while let Some(node) = stack.pop() {
        for child in node.children.iter().rev() {
            match child {
                XmlChild::Text(value) => text.push_str(value),
                XmlChild::Element(element) => stack.push(element),
                _ => {}
            }
        }
    }
    text
}

/// Whether an element carries content that is not an element and not
/// ignorable whitespace: significant text, a comment or a processing
/// instruction.
///
/// Such content has no slot in the model, so a small element that has it is
/// kept verbatim instead of being mapped.
pub(crate) fn has_loose_content(node: &XmlNode) -> bool {
    node.children.iter().any(|child| match child {
        XmlChild::Text(text) => !text.trim().is_empty(),
        XmlChild::Element(_) => false,
        _ => true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn qname(name: &str) -> ferrogram_model::QName {
        match name.split_once(':') {
            Some((prefix, local)) => ferrogram_model::QName {
                prefix: Some(prefix.to_owned()),
                uri: None,
                local: local.to_owned(),
            },
            None => ferrogram_model::QName::local(name),
        }
    }

    fn element(name: &str, attrs: &[(&str, &str)], children: Vec<XmlChild>) -> XmlNode {
        XmlNode {
            name: qname(name),
            attributes: attrs
                .iter()
                .map(|(name, value)| XmlAttribute {
                    name: qname(name),
                    value: (*value).to_owned(),
                })
                .collect(),
            children,
            self_closing: false,
        }
    }

    #[test]
    fn elements_lists_child_elements_in_order() {
        let node = element(
            "Root",
            &[],
            vec![
                XmlChild::Text("  ".to_owned()),
                XmlChild::Element(element("A", &[], vec![])),
                XmlChild::Comment("note".to_owned()),
                XmlChild::Element(element("B", &[], vec![])),
            ],
        );
        let names: Vec<&str> = elements(&node)
            .map(|element| element.name.local.as_str())
            .collect();
        assert_eq!(names, ["A", "B"]);
        assert_eq!(elements(&node).count(), 2);
    }

    #[test]
    fn attributes_find_by_local_name() {
        let node = element("N", &[("r:id", "rId1"), ("ID", "7")], vec![]);
        assert_eq!(attribute(&node, "id"), Some("rId1"));
        assert_eq!(attribute(&node, "ID"), Some("7"));
        assert_eq!(attribute(&node, "missing"), None);
        let cloned = attribute_node(&node, "ID").expect("attribute");
        assert_eq!(cloned.value, "7");
        assert_eq!(attribute_node(&node, "missing"), None);
    }

    #[test]
    fn namespace_declarations_are_recognized() {
        let default = element("N", &[("xmlns", "urn:x")], vec![]);
        let prefixed = element("N", &[("xmlns:r", "urn:r")], vec![]);
        let plain = element("N", &[("xml:space", "preserve")], vec![]);
        assert!(is_namespace_declaration(&default.attributes[0]));
        assert!(is_namespace_declaration(&prefixed.attributes[0]));
        assert!(!is_namespace_declaration(&plain.attributes[0]));
    }

    #[test]
    fn text_content_concatenates_descendants() {
        let node = element(
            "X",
            &[],
            vec![
                XmlChild::Text("a".to_owned()),
                XmlChild::Element(element("Y", &[], vec![XmlChild::Text("b".to_owned())])),
            ],
        );
        assert_eq!(text_content(&node), "ab");
    }

    #[test]
    fn loose_content_ignores_whitespace_and_elements() {
        assert!(!has_loose_content(&element(
            "N",
            &[],
            vec![
                XmlChild::Text(" \n ".to_owned()),
                XmlChild::Element(element("A", &[], vec![])),
            ],
        )));
        assert!(has_loose_content(&element(
            "N",
            &[],
            vec![XmlChild::Text("text".to_owned())]
        )));
        assert!(has_loose_content(&element(
            "N",
            &[],
            vec![XmlChild::Comment("note".to_owned())]
        )));
    }
}
