//! Minimal XML text to [`XmlNode`] and back, for preserved part data.
//!
//! The parser keeps namespace prefixes, attribute order and whitespace text
//! nodes so the serializer reproduces the source tree (ADR 0012). Namespace
//! URIs are resolved from `xmlns` declarations, which are themselves kept as
//! attributes. The XML declaration is not representable in [`XmlNode`] and is
//! dropped on write; comments outside the root element and character data
//! outside the root element (whitespace only in well-formed documents) are
//! dropped for the same reason.
//!
//! Malformed input returns an [`Error`], never a panic (AGENTS.md section 6);
//! messages never carry file content.

use ferrogram_model::{QName, XmlAttribute, XmlChild, XmlNode};
use quick_xml::XmlVersion;
use quick_xml::events::Event;
use quick_xml::name::PrefixDeclaration;
use quick_xml::reader::Reader;

use super::Limits;
use crate::Error;

/// The namespace URI of the predefined `xml` prefix.
const XML_NAMESPACE_URI: &str = "http://www.w3.org/XML/1998/namespace";

/// A namespace scope: prefix (`None` for the default namespace) to URI.
type Scope = Vec<(Option<String>, String)>;

/// Parses XML text into a preserved tree, enforcing the XML caps of `limits`.
///
/// Whitespace text nodes are kept exactly as written. Entity and character
/// references are decoded into the surrounding text node (or attribute value).
pub(crate) fn parse_xml(text: &str, limits: &Limits) -> Result<XmlNode, Error> {
    let mut reader = Reader::from_str(text);
    let decoder = reader.decoder();
    let mut scopes: Vec<Scope> = Vec::new();
    let mut stack: Vec<XmlNode> = Vec::new();
    let mut root: Option<XmlNode> = None;
    let mut text_buffer = String::new();

    loop {
        let event = reader.read_event().map_err(|_| invalid("malformed XML"))?;
        match event {
            Event::Start(element) => {
                flush_text(&mut text_buffer, &mut stack);
                let depth = stack.len().saturating_add(1);
                check_depth(depth, limits)?;
                let (name, attributes, scope) = parse_element(&element, &scopes, limits, decoder)?;
                scopes.push(scope);
                stack.push(XmlNode {
                    name,
                    attributes,
                    children: Vec::new(),
                    self_closing: false,
                });
            }
            Event::Empty(element) => {
                flush_text(&mut text_buffer, &mut stack);
                let depth = stack.len().saturating_add(1);
                check_depth(depth, limits)?;
                let (name, attributes, _scope) = parse_element(&element, &scopes, limits, decoder)?;
                push_child(
                    &mut stack,
                    &mut root,
                    XmlNode {
                        name,
                        attributes,
                        children: Vec::new(),
                        self_closing: true,
                    },
                )?;
            }
            Event::End(_) => {
                flush_text(&mut text_buffer, &mut stack);
                let node = stack.pop().ok_or_else(|| invalid("unexpected end tag"))?;
                scopes.pop();
                push_child(&mut stack, &mut root, node)?;
            }
            Event::Text(text_event) => {
                let decoded = text_event
                    .decode()
                    .map_err(|_| invalid("malformed XML text"))?;
                text_buffer.push_str(&decoded);
            }
            Event::CData(cdata) => {
                let decoded = cdata.decode().map_err(|_| invalid("malformed XML text"))?;
                text_buffer.push_str(&decoded);
            }
            Event::GeneralRef(reference) => {
                let name = reference
                    .decode()
                    .map_err(|_| invalid("malformed entity reference"))?;
                text_buffer.push_str(&resolve_reference(&name)?);
            }
            Event::Comment(comment) => {
                flush_text(&mut text_buffer, &mut stack);
                let decoded = comment
                    .decode()
                    .map_err(|_| invalid("malformed XML comment"))?;
                // Comments outside the root element are dropped: `XmlNode` has
                // no slot for them.
                if let Some(parent) = stack.last_mut() {
                    parent
                        .children
                        .push(XmlChild::Comment(decoded.into_owned()));
                }
            }
            Event::PI(instruction) => {
                flush_text(&mut text_buffer, &mut stack);
                let target = String::from_utf8_lossy(instruction.target()).into_owned();
                let data = String::from_utf8_lossy(instruction.content()).into_owned();
                let data = data.trim_start().to_owned();
                if let Some(parent) = stack.last_mut() {
                    parent
                        .children
                        .push(XmlChild::ProcessingInstruction { target, data });
                }
            }
            Event::Decl(_) => {
                // The XML declaration has no slot in `XmlNode`; dropped.
            }
            Event::DocType(_) => return Err(invalid("doctype declarations are not supported")),
            Event::Eof => break,
        }
    }

    flush_text(&mut text_buffer, &mut stack);
    if !stack.is_empty() {
        return Err(invalid("unclosed element at end of input"));
    }
    root.ok_or_else(|| invalid("no root element"))
}

/// Serializes a preserved tree back to XML text.
///
/// The output re-parses to the same tree: prefixes, attribute order,
/// whitespace and self-closing spelling are kept exactly.
pub(crate) fn write_xml(node: &XmlNode) -> String {
    let mut out = String::new();
    write_node(node, &mut out);
    out
}

/// Appends an escaped attribute value to `out`.
pub(crate) fn escape_attribute(value: &str, out: &mut String) {
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '"' => out.push_str("&quot;"),
            '\t' => out.push_str("&#x9;"),
            '\n' => out.push_str("&#xA;"),
            '\r' => out.push_str("&#xD;"),
            other => out.push(other),
        }
    }
}

/// Appends escaped character data to `out`.
fn escape_text(text: &str, out: &mut String) {
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            other => out.push(other),
        }
    }
}

/// Builds an [`Error::InvalidPackage`] with a fixed, content-free message.
fn invalid(why: &str) -> Error {
    Error::InvalidPackage(why.to_owned())
}

/// Enforces `max_xml_depth` for one element at `depth`.
fn check_depth(depth: usize, limits: &Limits) -> Result<(), Error> {
    let value = depth as u64;
    let max = u64::from(limits.max_xml_depth);
    if value > max {
        return Err(Error::LimitExceeded {
            limit: "max_xml_depth",
            value,
            max,
        });
    }
    Ok(())
}

/// Parses one start or empty element: name, attributes and new namespace scope.
fn parse_element(
    element: &quick_xml::events::BytesStart<'_>,
    outer_scopes: &[Scope],
    limits: &Limits,
    decoder: quick_xml::encoding::Decoder,
) -> Result<(QName, Vec<XmlAttribute>, Scope), Error> {
    struct RawAttribute {
        prefix: Option<String>,
        local: String,
        value: String,
        declaration: Option<(Option<String>, String)>,
    }

    let mut raw: Vec<RawAttribute> = Vec::new();
    for attribute in element.attributes() {
        let attribute = attribute.map_err(|_| invalid("malformed attribute"))?;
        let count = raw.len().saturating_add(1);
        if count > limits.max_attributes_per_element as usize {
            return Err(Error::LimitExceeded {
                limit: "max_attributes_per_element",
                value: count as u64,
                max: u64::from(limits.max_attributes_per_element),
            });
        }
        let (local, prefix) = attribute.key.decompose();
        let prefix = prefix.map(|p| String::from_utf8_lossy(p.as_ref()).into_owned());
        let local = String::from_utf8_lossy(local.as_ref()).into_owned();
        let value = attribute
            .decoded_and_normalized_value(XmlVersion::Implicit1_0, decoder)
            .map_err(|_| invalid("malformed attribute value"))?
            .into_owned();
        let declaration = match attribute.key.as_namespace_binding() {
            Some(PrefixDeclaration::Default) => Some((None, value.clone())),
            Some(PrefixDeclaration::Named(name)) => Some((
                Some(String::from_utf8_lossy(name).into_owned()),
                value.clone(),
            )),
            None => None,
        };
        raw.push(RawAttribute {
            prefix,
            local,
            value,
            declaration,
        });
    }

    let scope: Scope = raw.iter().filter_map(|a| a.declaration.clone()).collect();

    // The element's own declarations apply to the element name itself.
    let mut inner: Vec<&Scope> = outer_scopes.iter().collect();
    inner.push(&scope);

    let (local, prefix) = element.name().decompose();
    let prefix = prefix.map(|p| String::from_utf8_lossy(p.as_ref()).into_owned());
    let local = String::from_utf8_lossy(local.as_ref()).into_owned();
    let uri = resolve_prefix(prefix.as_deref(), &inner);

    let attributes = raw
        .into_iter()
        .map(|a| {
            let uri = if a.declaration.is_some() {
                None
            } else {
                resolve_prefix(a.prefix.as_deref(), &inner)
            };
            XmlAttribute {
                name: QName {
                    prefix: a.prefix,
                    uri,
                    local: a.local,
                },
                value: a.value,
            }
        })
        .collect();

    Ok((QName { prefix, uri, local }, attributes, scope))
}

/// Resolves a namespace prefix against the visible scopes.
///
/// Unprefixed names take the default namespace (attributes call this only with
/// a prefix, so they never pick up the default namespace). An unbound prefix
/// resolves to no namespace.
fn resolve_prefix(prefix: Option<&str>, scopes: &[&Scope]) -> Option<String> {
    let prefix = prefix?;
    if prefix == "xml" {
        return Some(XML_NAMESPACE_URI.to_owned());
    }
    let wanted = Some(prefix.to_owned());
    for scope in scopes.iter().rev() {
        if let Some((_, uri)) = scope.iter().find(|(p, _)| *p == wanted) {
            return Some(uri.clone());
        }
    }
    None
}

/// Appends a completed element to its parent, or records it as the root.
fn push_child(
    stack: &mut [XmlNode],
    root: &mut Option<XmlNode>,
    node: XmlNode,
) -> Result<(), Error> {
    if let Some(parent) = stack.last_mut() {
        parent.children.push(XmlChild::Element(node));
        Ok(())
    } else if root.is_none() {
        *root = Some(node);
        Ok(())
    } else {
        Err(invalid("more than one root element"))
    }
}

/// Moves buffered character data into the open element as one text node.
fn flush_text(buffer: &mut String, stack: &mut [XmlNode]) {
    if buffer.is_empty() {
        return;
    }
    let text = std::mem::take(buffer);
    if let Some(parent) = stack.last_mut() {
        parent.children.push(XmlChild::Text(text));
    }
}

/// Decodes one entity or character reference into its replacement text.
fn resolve_reference(name: &str) -> Result<String, Error> {
    let replacement = match name {
        "amp" => "&",
        "lt" => "<",
        "gt" => ">",
        "quot" => "\"",
        "apos" => "'",
        _ => {
            let (radix, digits) = if let Some(hex) = name.strip_prefix("#x") {
                (16, hex)
            } else if let Some(dec) = name.strip_prefix('#') {
                (10, dec)
            } else {
                return Err(invalid("unresolved entity reference"));
            };
            let code = u32::from_str_radix(digits, radix)
                .map_err(|_| invalid("malformed character reference"))?;
            let ch =
                char::from_u32(code).ok_or_else(|| invalid("malformed character reference"))?;
            return Ok(ch.to_string());
        }
    };
    Ok(replacement.to_owned())
}

/// Serializes one element and its children into `out`.
fn write_node(node: &XmlNode, out: &mut String) {
    out.push('<');
    write_qualified_name(&node.name, out);
    for attribute in &node.attributes {
        out.push(' ');
        write_qualified_name(&attribute.name, out);
        out.push_str("=\"");
        escape_attribute(&attribute.value, out);
        out.push('"');
    }
    if node.self_closing && node.children.is_empty() {
        out.push_str("/>");
        return;
    }
    out.push('>');
    for child in &node.children {
        match child {
            XmlChild::Element(element) => write_node(element, out),
            XmlChild::Text(text) => escape_text(text, out),
            XmlChild::Comment(comment) => {
                out.push_str("<!--");
                out.push_str(comment);
                out.push_str("-->");
            }
            XmlChild::ProcessingInstruction { target, data } => {
                out.push_str("<?");
                out.push_str(target);
                if !data.is_empty() {
                    out.push(' ');
                    out.push_str(data);
                }
                out.push_str("?>");
            }
            // Future `XmlChild` kinds (the enum is non-exhaustive) are
            // ignored rather than guessed at.
            _ => {}
        }
    }
    out.push_str("</");
    write_qualified_name(&node.name, out);
    out.push('>');
}

/// Writes a qualified name with its prefix spelling.
pub(crate) fn write_qualified_name(name: &QName, out: &mut String) {
    if let Some(prefix) = &name.prefix {
        out.push_str(prefix);
        out.push(':');
    }
    out.push_str(&name.local);
}

#[cfg(test)]
mod tests {
    use ferrogram_model::diff;
    use proptest::prelude::*;

    use super::*;

    /// Element names the generator produces: valid XML names.
    const ELEMENT_NAMES: &[&str] = &["a", "b", "Cell", "X9", "_k"];
    /// Attribute names the generator produces (never `xmlns`).
    const ATTRIBUTE_NAMES: &[&str] = &["id", "N", "V", "n2"];
    /// Character data the generator produces (no carriage return: the XML
    /// parser normalizes line endings, so a raw `\r` cannot round-trip
    /// through text).
    const TEXTS: &[&str] = &["x", "a & b < c>", "0.5", " ", "", "0.5 in", "'q'", "\"z\""];
    /// Comment bodies (no `--`, which XML forbids inside a comment).
    const COMMENTS: &[&str] = &["note", "c-1", "x & y"];
    /// Processing-instruction targets (never `xml`) and data (no `?>`).
    const PI_TARGETS: &[&str] = &["p1", "t"];
    const PI_DATA: &[&str] = &["d", "data x"];

    fn arb_qname() -> impl Strategy<Value = QName> {
        proptest::prop_oneof![
            proptest::sample::select(ELEMENT_NAMES.to_vec())
                .prop_map(|local| QName::local(local.to_owned())),
            proptest::sample::select(ELEMENT_NAMES.to_vec()).prop_map(|local| QName {
                prefix: Some("p".to_owned()),
                uri: Some("urn:x".to_owned()),
                local: local.to_owned(),
            }),
        ]
    }

    fn arb_attribute_value() -> impl Strategy<Value = String> {
        proptest::sample::select(TEXTS.to_vec()).prop_map(str::to_owned)
    }

    /// Attributes with distinct names (a duplicate name is not well-formed
    /// XML and cannot round-trip).
    fn arb_attributes() -> impl Strategy<Value = Vec<XmlAttribute>> {
        proptest::collection::vec(
            (
                proptest::sample::select(ATTRIBUTE_NAMES.to_vec()),
                arb_attribute_value(),
            ),
            0..4,
        )
        .prop_map(|pairs| {
            let mut out: Vec<XmlAttribute> = Vec::new();
            for (name, value) in pairs {
                if out.iter().any(|attribute| attribute.name.local == name) {
                    continue;
                }
                out.push(XmlAttribute {
                    name: QName::local(name.to_owned()),
                    value,
                });
            }
            out
        })
    }

    fn arb_leaf_child() -> impl Strategy<Value = XmlChild> {
        proptest::prop_oneof![
            proptest::sample::select(TEXTS.to_vec())
                .prop_map(|text| XmlChild::Text(text.to_owned())),
            proptest::sample::select(COMMENTS.to_vec())
                .prop_map(|comment| XmlChild::Comment(comment.to_owned())),
            (
                proptest::sample::select(PI_TARGETS.to_vec()),
                proptest::sample::select(PI_DATA.to_vec()),
            )
                .prop_map(|(target, data)| XmlChild::ProcessingInstruction {
                    target: target.to_owned(),
                    data: data.to_owned(),
                }),
        ]
    }

    /// The children a writer can produce: no two adjacent text nodes (the
    /// parser merges them into one).
    fn children_from(items: Vec<XmlChild>) -> Vec<XmlChild> {
        let mut out: Vec<XmlChild> = Vec::new();
        for item in items {
            if matches!(item, XmlChild::Text(_)) && matches!(out.last(), Some(XmlChild::Text(_))) {
                continue;
            }
            out.push(item);
        }
        out
    }

    fn arb_tree() -> impl Strategy<Value = XmlNode> {
        let leaf = (
            arb_qname(),
            arb_attributes(),
            proptest::collection::vec(arb_leaf_child(), 0..4),
        )
            .prop_map(|(name, attributes, items)| XmlNode {
                name,
                attributes,
                children: children_from(items),
                self_closing: false,
            });
        leaf.prop_recursive(3, 24, 3, |inner| {
            (
                arb_qname(),
                arb_attributes(),
                proptest::collection::vec(
                    proptest::prop_oneof![inner.prop_map(XmlChild::Element), arb_leaf_child(),],
                    0..5,
                ),
            )
                .prop_map(|(name, attributes, items)| XmlNode {
                    name,
                    attributes,
                    children: children_from(items),
                    self_closing: false,
                })
        })
    }

    /// A tree whose root declares the `p` prefix the generator's prefixed
    /// names use.
    fn arb_root() -> impl Strategy<Value = XmlNode> {
        arb_tree().prop_map(|mut node| {
            node.attributes.insert(
                0,
                XmlAttribute {
                    name: QName {
                        prefix: Some("xmlns".to_owned()),
                        uri: None,
                        local: "p".to_owned(),
                    },
                    value: "urn:x".to_owned(),
                },
            );
            node
        })
    }

    proptest::proptest! {
        /// ADR 0017: random `XmlNode` trees survive parse -> serialize ->
        /// parse with `diff` empty (the preservation tree is faithful).
        #[test]
        fn trees_survive_parse_serialize_parse(node in arb_root()) {
            let text = write_xml(&node);
            let parsed = parse_xml(&text, &Limits::DEFAULT).expect("the written text parses");
            proptest::prop_assert_eq!(diff(&node, &parsed).expect("diff never fails"), Vec::new());
        }
    }

    #[test]
    fn entities_and_character_references_decode() {
        let node = parse_xml(
            "<a x=\"&amp;&lt;&quot;\">&gt;&#65;&#x42;&apos;</a>",
            &Limits::DEFAULT,
        )
        .expect("parses");
        assert_eq!(node.attributes[0].value, "&<\"");
        assert_eq!(node.children[0], XmlChild::Text(">AB'".to_owned()));
    }

    #[test]
    fn an_unresolved_or_malformed_reference_is_an_error() {
        assert!(parse_xml("<a>&nope;</a>", &Limits::DEFAULT).is_err());
        assert!(parse_xml("<a>&#xzz;</a>", &Limits::DEFAULT).is_err());
        assert!(parse_xml("<a>&#1114112;</a>", &Limits::DEFAULT).is_err());
    }

    #[test]
    fn two_root_elements_are_an_error() {
        assert!(parse_xml("<a/><b/>", &Limits::DEFAULT).is_err());
    }

    #[test]
    fn the_xml_prefix_resolves_to_the_xml_namespace() {
        let node = parse_xml("<a xml:lang=\"en\"/>", &Limits::DEFAULT).expect("parses");
        let attribute = &node.attributes[0];
        assert_eq!(attribute.name.prefix.as_deref(), Some("xml"));
        assert_eq!(attribute.name.uri.as_deref(), Some(XML_NAMESPACE_URI));
    }

    #[test]
    fn the_empty_document_is_an_error() {
        assert!(parse_xml("", &Limits::DEFAULT).is_err());
        assert!(parse_xml("   ", &Limits::DEFAULT).is_err());
    }
}
