//! Shape text: the `Text` element and its run markers (ADR 0019).
//!
//! A shape's text is mixed content: literal text, `cp`/`pp`/`tp` markers
//! that open a character, paragraph or tabs run, and `fld` field
//! insertions. A run marker names the `IX` of the `Character`, `Paragraph`
//! or `Tabs` row that formats the run, which lasts to the end of the text
//! or the next marker (the ShapeSheet reference). Field evaluation is
//! Phase 6; the cached display text of a field is the literal text that
//! follows its marker.

/// One part of a shape's text (ADR 0019).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TextPart {
    /// Literal text; may contain `\n` line breaks.
    Text(String),
    /// A `cp` marker: the character run of `Character` row `ix` starts
    /// here.
    CharRun {
        /// The `IX` of the `Character` row.
        ix: u32,
    },
    /// A `pp` marker: the paragraph run of `Paragraph` row `ix` starts
    /// here.
    ParaRun {
        /// The `IX` of the `Paragraph` row.
        ix: u32,
    },
    /// A `tp` marker: the tabs run of `Tabs` row `ix` starts here.
    TabsRun {
        /// The `IX` of the `Tabs` row.
        ix: u32,
    },
    /// A `fld` marker: the field named by `ix`; its cached display text
    /// follows as literal text.
    Field {
        /// The `IX` of the field.
        ix: u32,
    },
}

/// The text of a shape (ADR 0019).
///
/// The `Text` element stays in `Shape::preserved` as well (ADR 0012);
/// Phase 3 maps it read-side only (ADR 0019).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TextContent {
    /// The parts, in document order.
    pub parts: Vec<TextPart>,
}

impl TextContent {
    /// An empty text.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the text has no parts.
    pub fn is_empty(&self) -> bool {
        self.parts.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_text_is_empty() {
        let t = TextContent::new();
        assert!(t.is_empty());
        assert!(t.parts.is_empty());
    }

    #[test]
    fn parts_hold_their_data() {
        let mut t = TextContent::new();
        t.parts.push(TextPart::CharRun { ix: 0 });
        t.parts.push(TextPart::ParaRun { ix: 0 });
        t.parts.push(TextPart::TabsRun { ix: 1 });
        t.parts.push(TextPart::Field { ix: 2 });
        t.parts.push(TextPart::Text("hello".to_owned()));
        assert!(!t.is_empty());
        assert_eq!(t.parts.len(), 5);
        assert_eq!(t.parts[4], TextPart::Text("hello".to_owned()));
    }
}
