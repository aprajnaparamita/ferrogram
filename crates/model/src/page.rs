//! Pages and their connect records.

use crate::id::{PageId, ShapeId};
use crate::raw::Preserved;
use crate::sheet::Sheet;

/// One page of a document.
#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    /// The list id of this page.
    pub id: PageId,
    /// Visio's `ID` attribute on the `Page` element of `pages.xml`.
    pub visio_id: i32,
    /// Visio's `Name` attribute (user-visible name), as written.
    pub name: String,
    /// Visio's `NameU` attribute (unique name), as written.
    pub name_u: String,
    /// The background page, or `None` for a plain white background.
    pub background: Option<PageId>,
    /// The top-level shapes, in file order. Group sub-shapes live in the
    /// shape arena and are reached through `Shape::children`.
    pub shapes: Vec<ShapeId>,
    /// The page's ShapeSheet content.
    pub sheet: Sheet,
    /// The page's `Connect` records, in file order.
    pub connects: Vec<Connect>,
    /// Unmapped attributes and children of the page contents element kept
    /// verbatim (ADR 0012).
    pub preserved: Preserved,
}

impl Page {
    /// Creates a page with the given list id and Visio `ID`, empty otherwise.
    pub fn new(id: PageId, visio_id: i32) -> Self {
        Self {
            id,
            visio_id,
            name: String::new(),
            name_u: String::new(),
            background: None,
            shapes: Vec::new(),
            sheet: Sheet::new(),
            connects: Vec::new(),
            preserved: Preserved::new(),
        }
    }
}

/// One `Connect` record: a glue between two shapes' cells (the `Connects`
/// section, written as a `Connect` element).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Connect {
    /// The `FromSheet` attribute: the Visio `ID` of the shape the connector
    /// starts from.
    pub from_sheet: i32,
    /// The `FromCell` attribute, e.g. `"BeginX"`.
    pub from_cell: String,
    /// The `FromPart` attribute (the shape part, see `VisFromParts`), or 0.
    pub from_part: i32,
    /// The `ToSheet` attribute: the Visio `ID` of the shape the connector
    /// ends at.
    pub to_sheet: i32,
    /// The `ToCell` attribute, e.g. `"EndX"`.
    pub to_cell: String,
    /// The `ToPart` attribute (the shape part, see `VisToParts`), or 0.
    pub to_part: i32,
}

impl Connect {
    /// Creates a connect record with all parts zero or empty.
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_page_is_empty() {
        let p = Page::new(PageId { index: 2 }, 5);
        assert_eq!(p.id, PageId { index: 2 });
        assert_eq!(p.visio_id, 5);
        assert!(p.name.is_empty());
        assert!(p.name_u.is_empty());
        assert_eq!(p.background, None);
        assert!(p.shapes.is_empty());
        assert!(p.connects.is_empty());
        assert!(p.preserved.children.is_empty());
    }

    #[test]
    fn new_connect_is_zero() {
        let c = Connect::new();
        assert_eq!(c.from_sheet, 0);
        assert_eq!(c.to_sheet, 0);
        assert!(c.from_cell.is_empty());
        assert!(c.to_cell.is_empty());
        assert_eq!(Connect::default(), Connect::new());
    }
}
