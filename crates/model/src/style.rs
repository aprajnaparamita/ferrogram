//! The stylesheet: the named styles shapes can reference.

use crate::id::StyleId;
use crate::raw::Preserved;
use crate::sheet::Sheet;

/// The document stylesheet (`Style Sheet` section of `document.xml`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Stylesheet {
    /// The styles, in file order. The lookup API (ADR 0013) resolves the
    /// `Style` cell of each style into a chain.
    pub styles: Vec<Style>,
}

impl Stylesheet {
    /// Creates an empty stylesheet.
    pub fn new() -> Self {
        Self::default()
    }
}

/// One style: a named row of the stylesheet.
#[derive(Debug, Clone, PartialEq)]
pub struct Style {
    /// The stylesheet row id.
    pub id: StyleId,
    /// Visio's `Name` attribute (user-visible name), as written.
    pub name: String,
    /// Visio's `NameU` attribute (unique name), as written.
    pub name_u: String,
    /// The base style name (the style's own `Style` cell), or `None`. The
    /// lookup resolves it; it is never flattened on load (ADR 0013).
    pub base_style: Option<String>,
    /// The style's ShapeSheet content.
    pub sheet: Sheet,
    /// Unmapped attributes and children of the style row element kept
    /// verbatim (ADR 0012).
    pub preserved: Preserved,
}

impl Style {
    /// Creates a style with the given stylesheet row id, empty otherwise.
    pub fn new(id: StyleId) -> Self {
        Self {
            id,
            name: String::new(),
            name_u: String::new(),
            base_style: None,
            sheet: Sheet::new(),
            preserved: Preserved::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_stylesheet_is_empty() {
        let s = Stylesheet::new();
        assert!(s.styles.is_empty());
    }

    #[test]
    fn new_style_is_empty() {
        let s = Style::new(StyleId { index: 4 });
        assert_eq!(s.id, StyleId { index: 4 });
        assert!(s.name.is_empty());
        assert!(s.name_u.is_empty());
        assert_eq!(s.base_style, None);
        assert!(s.sheet.cells.is_empty());
        assert!(s.preserved.children.is_empty());
    }

    #[test]
    fn stylesheet_holds_styles() {
        let mut sh = Stylesheet::new();
        sh.styles.push(Style::new(StyleId { index: 0 }));
        assert_eq!(sh.styles.len(), 1);
    }
}
