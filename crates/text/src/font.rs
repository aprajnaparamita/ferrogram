//! Fonts: parsing, glyph outlines and the fallback stack (ADR 0024).
//!
//! Fonts are data: Carlito (for Calibri) and Selawik (for Segoe UI) are
//! bundled with their OFL licences (AGENTS.md section 4). A [`FontStack`]
//! resolves a family name to a font and falls back to its first font; a
//! missing font or glyph is a warning on the [`Layout`](crate::Layout),
//! never an error (ADR 0021).

use ferrogram_geom::Path;

use crate::error::Error;

/// A glyph index in one font.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlyphId(pub u16);

/// One parsed font face.
///
/// The font keeps its bytes and parses faces on demand; outlines come out
/// in em units (size 1), so callers scale by their font size (ADR 0024).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Font {
    bytes: Vec<u8>,
    family: String,
}

impl Font {
    /// Parses a TrueType or OpenType font from its bytes.
    ///
    /// Returns [`Error::NotImplemented`] until the font work lands (P3-04).
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Font, Error> {
        let _ = bytes;
        Err(Error::NotImplemented)
    }

    /// The family name of the font.
    pub fn family(&self) -> &str {
        &self.family
    }

    /// The outline of one glyph in em units (size 1).
    ///
    /// Returns [`Error::NotImplemented`] until the font work lands (P3-04).
    pub fn outline(&self, glyph: GlyphId) -> Result<Path, Error> {
        let _ = glyph;
        Err(Error::NotImplemented)
    }
}

/// The fonts available to one layout, in fallback order (ADR 0024).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FontStack {
    fonts: Vec<Font>,
}

impl FontStack {
    /// An empty stack.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends a font to the fallback chain.
    pub fn push(&mut self, font: Font) {
        self.fonts.push(font);
    }

    /// The font for `family`: the first font with that family name, else
    /// the first font in the stack, else `None`.
    pub fn resolve(&self, family: &str) -> Option<&Font> {
        self.fonts
            .iter()
            .find(|f| f.family() == family)
            .or_else(|| self.fonts.first())
    }

    /// The fonts, in fallback order.
    pub fn fonts(&self) -> &[Font] {
        &self.fonts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn font(family: &str) -> Font {
        Font {
            bytes: Vec::new(),
            family: family.to_owned(),
        }
    }

    #[test]
    fn family_accessor_returns_the_name() {
        assert_eq!(font("Carlito").family(), "Carlito");
    }

    #[test]
    fn from_bytes_is_not_implemented_yet() {
        assert_eq!(Font::from_bytes(Vec::new()), Err(Error::NotImplemented));
    }

    #[test]
    fn outline_is_not_implemented_yet() {
        assert_eq!(
            font("Carlito").outline(GlyphId(3)),
            Err(Error::NotImplemented)
        );
    }

    #[test]
    fn stack_resolves_by_family_then_first() {
        let mut stack = FontStack::new();
        assert_eq!(stack.resolve("Carlito"), None);
        stack.push(font("Selawik"));
        stack.push(font("Carlito"));
        assert_eq!(stack.resolve("Carlito").map(Font::family), Some("Carlito"));
        assert_eq!(stack.resolve("Missing").map(Font::family), Some("Selawik"));
        assert_eq!(stack.fonts().len(), 2);
    }
}
