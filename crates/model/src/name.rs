//! Typed names for ShapeSheet sections and cells.

use std::borrow::Cow;

/// A ShapeSheet cell name, e.g. `"PinX"` or `"LineWeight"`.
///
/// The name is the Visio cell name as written in `.vsdx` (`Cell/@N`) and in
/// the ShapeSheet table (`ui-spec/shapesheet.toml`, ADR 0008). Cells the
/// table does not know still carry their file name, so nothing is lost.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CellName(Cow<'static, str>);

impl CellName {
    /// Creates a cell name from any string.
    pub fn new(name: impl Into<String>) -> Self {
        Self(Cow::Owned(name.into()))
    }

    /// Creates a cell name from a `'static` string without allocating.
    pub const fn from_static(name: &'static str) -> Self {
        Self(Cow::Borrowed(name))
    }

    /// The name as written in the file.
    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }
}

/// A ShapeSheet section name, e.g. `"Geometry"` or `"Shape Transform"`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SectionName(Cow<'static, str>);

impl SectionName {
    /// Creates a section name from any string.
    pub fn new(name: impl Into<String>) -> Self {
        Self(Cow::Owned(name.into()))
    }

    /// Creates a section name from a `'static` string without allocating.
    pub const fn from_static(name: &'static str) -> Self {
        Self(Cow::Borrowed(name))
    }

    /// The name as written in the file.
    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_name_from_static_borrows() {
        let n = CellName::from_static("PinX");
        assert_eq!(n.as_str(), "PinX");
    }

    #[test]
    fn cell_name_from_string_owns() {
        let owned = String::from("LineWeight");
        let n = CellName::new(owned);
        assert_eq!(n.as_str(), "LineWeight");
    }

    #[test]
    fn cell_names_compare_by_value() {
        assert_eq!(CellName::new("PinX"), CellName::from_static("PinX"));
        assert_ne!(CellName::new("PinX"), CellName::new("PinY"));
    }

    #[test]
    fn section_name_round_trips() {
        assert_eq!(SectionName::from_static("Geometry").as_str(), "Geometry");
        assert_eq!(
            SectionName::new("Shape Transform".to_owned()).as_str(),
            "Shape Transform"
        );
    }
}
