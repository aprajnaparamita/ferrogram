//! The stylesheet: the named styles shapes can reference.

use crate::id::StyleId;
use crate::raw::Preserved;
use crate::sheet::Sheet;

/// The document stylesheet (`Style Sheet` section of `document.xml`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Stylesheet {
    /// The styles, in file order. The lookup API (ADR 0013) follows each
    /// style's `LineStyle`/`FillStyle`/`TextStyle` references into a chain.
    pub styles: Vec<Style>,
}

impl Stylesheet {
    /// Creates an empty stylesheet.
    pub fn new() -> Self {
        Self::default()
    }
}

/// One style: a named row of the stylesheet.
///
/// A style's own inheritance chain in `.vsdx` is its `LineStyle`,
/// `FillStyle` and `TextStyle` attributes: each names another style by its
/// Visio `ID`, and the lookup follows the one for the cell's kind (ADR 0013,
/// P2-06b decision 3).
#[derive(Debug, Clone, PartialEq)]
pub struct Style {
    /// The stylesheet row id.
    pub id: StyleId,
    /// Visio's `ID` attribute: the value style references name, unique
    /// within the stylesheet.
    pub visio_id: u32,
    /// Visio's `Name` attribute (user-visible name), as written.
    pub name: String,
    /// Visio's `NameU` attribute (unique name), as written.
    pub name_u: String,
    /// The line style reference (`LineStyle` attribute): the Visio `ID` of
    /// the style this style's line cells inherit from, or `None`.
    pub line_style: Option<u32>,
    /// The fill style reference (`FillStyle` attribute): the Visio `ID` of
    /// the style this style's fill cells inherit from, or `None`.
    pub fill_style: Option<u32>,
    /// The text style reference (`TextStyle` attribute): the Visio `ID` of
    /// the style this style's text cells inherit from, or `None`.
    pub text_style: Option<u32>,
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
            visio_id: 0,
            name: String::new(),
            name_u: String::new(),
            line_style: None,
            fill_style: None,
            text_style: None,
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
        assert_eq!(s.visio_id, 0);
        assert!(s.name.is_empty());
        assert!(s.name_u.is_empty());
        assert_eq!(s.line_style, None);
        assert_eq!(s.fill_style, None);
        assert_eq!(s.text_style, None);
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
