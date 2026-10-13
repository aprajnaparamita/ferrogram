//! The document: the arena of shapes plus pages, masters and the stylesheet.

use crate::id::{MasterId, PageId, ShapeId, StyleId};
use crate::master::Master;
use crate::page::Page;
use crate::raw::Preserved;
use crate::shape::Shape;
use crate::style::Stylesheet;

/// What kind of package a document was read from (`.vsdx`, `.vssx`, `.vstx`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum DocumentKind {
    /// A drawing (`.vsdx`).
    #[default]
    Drawing,
    /// A stencil (`.vssx`): a master library, usually no pages.
    Stencil,
    /// A template (`.vstx`): a starting drawing with masters and settings.
    Template,
}

/// One document property of `docProps/core.xml` or `docProps/app.xml`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Property {
    /// The property element name, e.g. `"title"`.
    pub name: String,
    /// The property value, as written.
    pub value: String,
}

impl Property {
    /// Creates a property from a name and a value.
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }
}

/// A whole document: shapes in one arena plus pages, masters, the stylesheet
/// and document properties (ADR 0011).
///
/// Shapes live in one arena (`shapes`), indexed by [`ShapeId`]. Pages and
/// masters reference arena shapes by id, and group shapes reference their
/// sub-shapes the same way. Ids are stable for the lifetime of the document,
/// so Phase 4 commands can hold them across mutations.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Document {
    /// Whether this is a drawing, stencil or template (ADR 0016).
    pub kind: DocumentKind,
    /// The shape arena. Index `id.index` is the shape.
    pub shapes: Vec<Shape>,
    /// The pages, in file order.
    pub pages: Vec<Page>,
    /// The masters, in file order.
    pub masters: Vec<Master>,
    /// The stylesheet.
    pub stylesheet: Stylesheet,
    /// The core and extended document properties.
    pub properties: Vec<Property>,
    /// Unmapped attributes and children of the root `VisioDocument` element
    /// kept verbatim (ADR 0012).
    pub preserved: Preserved,
}

impl Document {
    /// Creates an empty drawing.
    pub fn new() -> Self {
        Self {
            kind: DocumentKind::Drawing,
            ..Self::default()
        }
    }

    /// The shape with this arena id, or `None` when the id is stale.
    pub fn shape(&self, id: ShapeId) -> Option<&Shape> {
        self.shapes.get(id.index as usize)
    }

    /// The mutable shape with this arena id, or `None` when the id is stale.
    pub fn shape_mut(&mut self, id: ShapeId) -> Option<&mut Shape> {
        self.shapes.get_mut(id.index as usize)
    }

    /// The page with this list id, or `None` when the id is stale.
    pub fn page(&self, id: PageId) -> Option<&Page> {
        self.pages.get(id.index as usize)
    }

    /// The master with this list id, or `None` when the id is stale.
    pub fn master(&self, id: MasterId) -> Option<&Master> {
        self.masters.get(id.index as usize)
    }

    /// The stylesheet row with this id, or `None` when the id is stale.
    pub fn style(&self, id: StyleId) -> Option<&crate::style::Style> {
        self.stylesheet.styles.get(id.index as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_document_is_an_empty_drawing() {
        let d = Document::new();
        assert_eq!(d.kind, DocumentKind::Drawing);
        assert!(d.shapes.is_empty());
        assert!(d.pages.is_empty());
        assert!(d.masters.is_empty());
        assert!(d.stylesheet.styles.is_empty());
        assert!(d.properties.is_empty());
    }

    #[test]
    fn arena_accessors_find_their_entries() {
        let mut d = Document::new();
        d.shapes.push(Shape::new(ShapeId { index: 0 }, 1));
        d.pages.push(Page::new(PageId { index: 0 }, 1));
        d.masters.push(Master::new(MasterId { index: 0 }, 1));
        d.stylesheet
            .styles
            .push(crate::style::Style::new(StyleId { index: 0 }));

        assert!(d.shape(ShapeId { index: 0 }).is_some());
        assert!(d.shape(ShapeId { index: 1 }).is_none());
        assert!(d.shape_mut(ShapeId { index: 0 }).is_some());
        assert!(d.page(PageId { index: 0 }).is_some());
        assert!(d.page(PageId { index: 1 }).is_none());
        assert!(d.master(MasterId { index: 0 }).is_some());
        assert!(d.master(MasterId { index: 1 }).is_none());
        assert!(d.style(StyleId { index: 0 }).is_some());
        assert!(d.style(StyleId { index: 1 }).is_none());
    }

    #[test]
    fn property_holds_name_and_value() {
        let p = Property::new("title", "Example");
        assert_eq!(p.name, "title");
        assert_eq!(p.value, "Example");
    }
}
