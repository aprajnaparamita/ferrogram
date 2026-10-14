//! Layers and layer membership (ADR 0019).
//!
//! A page sheet's `Layer` section rows name the page's layers; a shape's
//! `LayerMember` cell lists the layer indices it belongs to. The renderer
//! drops shapes on invisible layers.

use crate::document::Document;
use crate::error::Error;
use crate::id::{PageId, ShapeId};

/// One layer of a page (ADR 0019).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layer {
    /// The row index (`IX`); layer membership lists these.
    pub ix: i32,
    /// The layer name (`Name` cell).
    pub name: String,
    /// The `Visible` cell.
    pub visible: bool,
}

impl Document {
    /// The layers of a page, from its `Layer` section rows (ADR 0019).
    ///
    /// Returns [`Error::NotImplemented`] until the model work lands
    /// (P3-02).
    pub fn page_layers(&self, page: PageId) -> Result<Vec<Layer>, Error> {
        let _ = page;
        let _ = self;
        Err(Error::NotImplemented)
    }

    /// The layer indices a shape belongs to (its `LayerMember` cell).
    ///
    /// Returns [`Error::NotImplemented`] until the model work lands
    /// (P3-02).
    pub fn shape_layers(&self, shape: ShapeId) -> Result<Vec<i32>, Error> {
        let _ = shape;
        let _ = self;
        Err(Error::NotImplemented)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layer_holds_its_data() {
        let l = Layer {
            ix: 3,
            name: "Connectors".to_owned(),
            visible: true,
        };
        assert_eq!(l.ix, 3);
        assert!(l.visible);
    }

    #[test]
    fn layer_accessors_are_not_implemented_yet() {
        let doc = Document::new();
        assert_eq!(
            doc.page_layers(PageId { index: 0 }),
            Err(Error::NotImplemented)
        );
        assert_eq!(
            doc.shape_layers(ShapeId { index: 0 }),
            Err(Error::NotImplemented)
        );
    }
}
