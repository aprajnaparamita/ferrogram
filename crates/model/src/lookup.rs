//! The inheritance lookup API (ADR 0013).
//!
//! Master/instance and stylesheet inheritance are resolved through this
//! lookup, never flattened on load. A lookup returns where the value came
//! from, so a caller can show provenance ("from master", "from style
//! `Normal`"). A cyclic style chain is cut and reported in `via`, never
//! looped on.

use crate::cell::CellValue;
use crate::id::{MasterId, ShapeId, StyleId};
use crate::name::CellName;

/// Where a resolved cell value came from (ADR 0013).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ValueSource {
    /// The shape's own cell.
    Local,
    /// The cell of the master shape this shape is an instance of.
    Master(MasterId),
    /// A cell of a stylesheet row in the style chain.
    Style(StyleId),
    /// The default value of the cell in the ShapeSheet table
    /// (`ui-spec/shapesheet.toml`, ADR 0008).
    SectionDefault,
    /// The hard default of the cell's type, when nothing else defines it.
    HardDefault,
}

/// The result of a lookup: the resolved value and its provenance.
#[derive(Debug, Clone, PartialEq)]
pub struct Resolved<'a> {
    /// The resolved cached value, or `None` when no source defines the cell.
    pub value: Option<&'a CellValue>,
    /// Where the value came from.
    pub source: ValueSource,
    /// The formula of the source that defines the cell, if any.
    pub formula: Option<&'a str>,
    /// Every source consulted, in resolution order (ADR 0013). The last
    /// entry is `source`. A cyclic style chain appears here as a repeated
    /// [`ValueSource`] and the chain stops there.
    pub via: Vec<ValueSource>,
}

impl<'a> Resolved<'a> {
    /// Creates a resolution with no value, no formula and no chain.
    pub fn empty(source: ValueSource) -> Self {
        Self {
            value: None,
            source,
            formula: None,
            via: Vec::new(),
        }
    }
}

/// Resolves cell values through master/instance and stylesheet inheritance
/// (ADR 0013).
pub trait CellLookup {
    /// Resolves `cell` on `shape`, following the resolution order of
    /// ADR 0013: the shape's own cell, the master's cell, the style chain
    /// (each style's own chain included), the section default, the hard
    /// default.
    fn resolve(&self, shape: ShapeId, cell: &CellName) -> Result<Resolved<'_>, crate::Error>;
}

impl CellLookup for crate::document::Document {
    fn resolve(&self, shape: ShapeId, cell: &CellName) -> Result<Resolved<'_>, crate::Error> {
        let _ = (shape, cell);
        Err(crate::Error::NotImplemented("Document::resolve"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_resolution_has_no_value() {
        let r = Resolved::empty(ValueSource::HardDefault);
        assert_eq!(r.value, None);
        assert_eq!(r.source, ValueSource::HardDefault);
        assert_eq!(r.formula, None);
        assert!(r.via.is_empty());
    }

    #[test]
    fn value_sources_carry_their_ids() {
        assert_ne!(
            ValueSource::Master(MasterId { index: 1 }),
            ValueSource::Master(MasterId { index: 2 })
        );
        assert_ne!(ValueSource::Style(StyleId { index: 1 }), ValueSource::Local);
    }

    #[test]
    fn resolve_is_a_stub() {
        let d = crate::Document::new();
        let r = d.resolve(ShapeId { index: 0 }, &CellName::from_static("PinX"));
        assert_eq!(r, Err(crate::Error::NotImplemented("Document::resolve")));
    }
}
