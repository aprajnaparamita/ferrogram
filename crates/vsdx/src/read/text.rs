//! Mapping the `Text` element of a shape (task P3-02, ADR 0019).
//!
//! A shape's text is mixed content: literal text, `cp`/`pp`/`tp` markers
//! that open a character, paragraph or tabs run, and `fld` field
//! insertions. A marker names the `IX` of the `Character`, `Paragraph` or
//! `Tabs` row that formats the run (the ShapeSheet reference); a marker
//! without an `IX` opens row 0. A field's cached display text is the
//! literal text that follows its marker (its child text in the files we
//! have); field evaluation is Phase 6. Text nodes keep their boundaries:
//! every literal run is its own [`TextPart::Text`].
//!
//! The `Text` element itself stays in the shape's preserved content as
//! before (ADR 0012); this mapping is a typed copy (ADR 0019 decision 1).

use ferrogram_model::{TextContent, TextPart, XmlNode};

use crate::Error;
use crate::read::node::attribute;

/// The bound on nested elements inside a `Text` element: malformed input
/// must not recurse without a limit (AGENTS.md section 6).
const MAX_TEXT_DEPTH: usize = 32;

/// Maps the mixed content of a `Text` element into [`TextContent`].
///
/// Comments and processing instructions carry no text and are skipped. A
/// marker whose `IX` does not parse is [`Error::InvalidPackage`] naming
/// the part and the attribute (the same policy as every other scalar).
pub(crate) fn map_text(node: &XmlNode, part: &str) -> Result<TextContent, Error> {
    let mut content = TextContent::new();
    walk(node, part, &mut content, 0)?;
    Ok(content)
}

/// Appends the mixed content of `node` to `content`, bounded by `depth`.
fn walk(node: &XmlNode, part: &str, content: &mut TextContent, depth: usize) -> Result<(), Error> {
    if depth >= MAX_TEXT_DEPTH {
        return Ok(());
    }
    for child in &node.children {
        match child {
            ferrogram_model::XmlChild::Text(text) => push_text(content, text),
            ferrogram_model::XmlChild::Element(element) => match element.name.local.as_str() {
                "cp" => content.parts.push(TextPart::CharRun {
                    ix: marker_ix(element, part)?,
                }),
                "pp" => content.parts.push(TextPart::ParaRun {
                    ix: marker_ix(element, part)?,
                }),
                "tp" => content.parts.push(TextPart::TabsRun {
                    ix: marker_ix(element, part)?,
                }),
                "fld" => {
                    content.parts.push(TextPart::Field {
                        ix: marker_ix(element, part)?,
                    });
                    walk(element, part, content, depth + 1)?;
                }
                // An element we do not model keeps its literal text.
                _ => walk(element, part, content, depth + 1)?,
            },
            // Comments and processing instructions hold no text.
            _ => {}
        }
    }
    Ok(())
}

/// Appends one literal text part. Text nodes keep their boundaries (a
/// field's cached display text is the part that follows its marker); only
/// empty text is dropped.
fn push_text(content: &mut TextContent, text: &str) {
    if text.is_empty() {
        return;
    }
    content.parts.push(TextPart::Text(text.to_owned()));
}

/// The `IX` attribute of a run marker: row 0 when the attribute is
/// missing, [`Error::InvalidPackage`] when it does not parse.
fn marker_ix(node: &XmlNode, part: &str) -> Result<u32, Error> {
    match attribute(node, "IX") {
        None => Ok(0),
        Some(value) => value.trim().parse::<u32>().map_err(|_| {
            Error::InvalidPackage(format!("{part}: attribute `IX`: not an unsigned integer"))
        }),
    }
}

#[cfg(test)]
mod tests {
    use ferrogram_model::{QName, XmlAttribute, XmlChild};

    use super::*;

    /// Builds a qualified name from `prefix:local` or `local`.
    fn qname(name: &str) -> QName {
        match name.split_once(':') {
            Some((prefix, local)) => QName {
                prefix: Some(prefix.to_owned()),
                uri: None,
                local: local.to_owned(),
            },
            None => QName {
                prefix: None,
                uri: None,
                local: name.to_owned(),
            },
        }
    }

    /// An element with attributes and children.
    fn element(name: &str, attrs: &[(&str, &str)], children: Vec<XmlChild>) -> XmlNode {
        XmlNode {
            name: qname(name),
            attributes: attrs
                .iter()
                .map(|(local, value)| XmlAttribute {
                    name: qname(local),
                    value: (*value).to_owned(),
                })
                .collect(),
            self_closing: children.is_empty(),
            children,
        }
    }

    /// An empty element with attributes.
    fn marker(name: &str, attrs: &[(&str, &str)]) -> XmlChild {
        XmlChild::Element(element(name, attrs, Vec::new()))
    }

    /// A text node.
    fn text(value: &str) -> XmlChild {
        XmlChild::Text(value.to_owned())
    }

    #[test]
    fn mixed_content_maps_in_document_order() {
        let node = element(
            "Text",
            &[],
            vec![
                marker("cp", &[("IX", "0")]),
                marker("pp", &[("IX", "1")]),
                text("Hello "),
                marker("tp", &[("IX", "2")]),
                text("world"),
                marker("fld", &[("IX", "3")]),
                text("42"),
            ],
        );
        let content = map_text(&node, "part").expect("maps");
        assert_eq!(
            content.parts,
            vec![
                TextPart::CharRun { ix: 0 },
                TextPart::ParaRun { ix: 1 },
                TextPart::Text("Hello ".to_owned()),
                TextPart::TabsRun { ix: 2 },
                TextPart::Text("world".to_owned()),
                TextPart::Field { ix: 3 },
                TextPart::Text("42".to_owned()),
            ]
        );
    }

    #[test]
    fn a_field_keeps_its_cached_display_text() {
        let node = element(
            "Text",
            &[],
            vec![XmlChild::Element(element(
                "fld",
                &[("IX", "7")],
                vec![text("06.07.2015")],
            ))],
        );
        let content = map_text(&node, "part").expect("maps");
        assert_eq!(
            content.parts,
            vec![
                TextPart::Field { ix: 7 },
                TextPart::Text("06.07.2015".to_owned())
            ]
        );
    }

    #[test]
    fn text_parts_keep_boundaries_and_empty_text_is_dropped() {
        let node = element(
            "Text",
            &[],
            vec![
                text(""),
                text("a"),
                text("b"),
                XmlChild::Comment("c".into()),
                text("d"),
            ],
        );
        let content = map_text(&node, "part").expect("maps");
        assert_eq!(
            content.parts,
            vec![
                TextPart::Text("a".to_owned()),
                TextPart::Text("b".to_owned()),
                TextPart::Text("d".to_owned())
            ]
        );
    }

    #[test]
    fn a_marker_without_ix_opens_row_zero() {
        let node = element("Text", &[], vec![marker("cp", &[]), text("x")]);
        let content = map_text(&node, "part").expect("maps");
        assert_eq!(
            content.parts,
            vec![TextPart::CharRun { ix: 0 }, TextPart::Text("x".to_owned())]
        );
    }

    #[test]
    fn a_bad_ix_is_an_error_naming_the_part() {
        let node = element("Text", &[], vec![marker("cp", &[("IX", "x")])]);
        let error = map_text(&node, "/visio/pages/page1.xml").expect_err("fails");
        assert!(
            error.to_string().contains("/visio/pages/page1.xml"),
            "{error}"
        );
        assert!(error.to_string().contains("IX"), "{error}");
    }

    #[test]
    fn unknown_elements_keep_their_literal_text() {
        let node = element(
            "Text",
            &[],
            vec![XmlChild::Element(element("weird", &[], vec![text("kept")]))],
        );
        let content = map_text(&node, "part").expect("maps");
        assert_eq!(content.parts, vec![TextPart::Text("kept".to_owned())]);
    }

    #[test]
    fn deep_nesting_terminates() {
        // A chain deeper than the bound: the walk stops, never recurses
        // without a limit.
        let mut node = element("Text", &[], vec![text("deep")]);
        for _ in 0..100 {
            node = element("wrap", &[], vec![XmlChild::Element(node)]);
        }
        let content = map_text(&node, "part").expect("maps");
        assert!(content.parts.is_empty());
    }
}
