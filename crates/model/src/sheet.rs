//! The ShapeSheet content common to shapes, page sheets and styles.

use crate::cell::Cell;
use crate::section::Section;

/// The ShapeSheet content of one sheet: shape, page sheet or style.
///
/// `cells` holds the direct cells (the "object" sections such as Shape
/// Transform, Character and Paragraph), which `.vsdx` writes as direct `Cell`
/// children. `sections` holds the repeated-row sections (Geometry, Connection
/// Points, ...). Unmapped attributes and children of the owning element are
/// kept in its `preserved` field (ADR 0012).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Sheet {
    /// The direct cells, in file order.
    pub cells: Vec<Cell>,
    /// The sections, in file order.
    pub sections: Vec<Section>,
}

impl Sheet {
    /// Creates an empty sheet.
    pub fn new() -> Self {
        Self::default()
    }
}
