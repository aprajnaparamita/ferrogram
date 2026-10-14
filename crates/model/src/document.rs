//! The document: the arena of shapes plus pages, masters and the stylesheet.

use crate::id::{MasterId, PageId, ShapeId, StyleId};
use crate::master::Master;
use crate::page::Page;
use crate::raw::Preserved;
use crate::shape::Shape;
use crate::style::{Style, Stylesheet};

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
    pub fn style(&self, id: StyleId) -> Option<&Style> {
        self.stylesheet.styles.get(id.index as usize)
    }

    /// Appends a shape to the arena and returns its id.
    ///
    /// The stored shape's `id` is set to the returned id, so
    /// `document.shape(id)` gives back the same shape.
    pub fn add_shape(&mut self, mut shape: Shape) -> ShapeId {
        let id = ShapeId {
            index: u32::try_from(self.shapes.len()).unwrap_or(u32::MAX),
        };
        shape.id = id;
        self.shapes.push(shape);
        id
    }

    /// Finds a shape by Visio `ID` within a page, or `None`.
    ///
    /// The search walks the page's shapes in file order and descends into
    /// groups; a cyclic or broken `children` list is walked once, never
    /// looped on.
    pub fn shape_by_visio_id(&self, page: PageId, visio_id: i32) -> Option<ShapeId> {
        let page = self.page(page)?;
        let mut seen = vec![false; self.shapes.len()];
        let mut stack: Vec<ShapeId> = page.shapes.iter().rev().copied().collect();
        while let Some(id) = stack.pop() {
            let Some(shape) = self.shape(id) else {
                continue;
            };
            let Some(slot) = seen.get_mut(id.index as usize) else {
                continue;
            };
            if *slot {
                continue;
            }
            *slot = true;
            if shape.visio_id == visio_id {
                return Some(id);
            }
            stack.extend(shape.children.iter().rev().copied());
        }
        None
    }

    /// Finds a master by `NameU`, or `None`.
    ///
    /// The first master with this `NameU` wins; `Name` is not searched. The
    /// returned id is the list position, consistent with
    /// [`Document::master`].
    pub fn master_by_name_u(&self, name_u: &str) -> Option<MasterId> {
        self.masters
            .iter()
            .position(|m| m.name_u == name_u)
            .map(|index| MasterId {
                index: u32::try_from(index).unwrap_or(u32::MAX),
            })
    }

    /// Finds a style by `Name`, or `None`.
    ///
    /// The first stylesheet row with this `Name` wins; `NameU` is not
    /// searched. The returned id is the list position, consistent with
    /// [`Document::style`].
    pub fn style_by_name(&self, name: &str) -> Option<StyleId> {
        self.stylesheet
            .styles
            .iter()
            .position(|s| s.name == name)
            .map(|index| StyleId {
                index: u32::try_from(index).unwrap_or(u32::MAX),
            })
    }

    /// Finds a style by its Visio `ID`, or `None`.
    ///
    /// A `.vsdx` style reference (`LineStyle`, `FillStyle`, `TextStyle`)
    /// names a style by this id (P2-06b decision 3); the first stylesheet
    /// row with this `visio_id` wins.
    pub fn style_by_visio_id(&self, visio_id: u32) -> Option<&Style> {
        self.stylesheet
            .styles
            .iter()
            .find(|style| style.visio_id == visio_id)
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

    #[test]
    fn add_shape_appends_and_assigns_the_arena_id() {
        let mut d = Document::new();
        let first = d.add_shape(Shape::new(ShapeId { index: 99 }, 1));
        let second = d.add_shape(Shape::new(ShapeId { index: 99 }, 2));
        assert_eq!(first, ShapeId { index: 0 });
        assert_eq!(second, ShapeId { index: 1 });
        assert_eq!(d.shape(first).expect("exists").id, first);
        assert_eq!(d.shape(second).expect("exists").visio_id, 2);
    }

    #[test]
    fn shape_by_visio_id_finds_top_level_and_sub_shapes() {
        let mut d = Document::new();
        let root = d.add_shape(Shape::new(ShapeId { index: 0 }, 1));
        let group = d.add_shape(Shape::new(ShapeId { index: 0 }, 2));
        let sub = d.add_shape(Shape::new(ShapeId { index: 0 }, 3));
        d.shape_mut(group).expect("exists").parent = Some(root);
        d.shape_mut(sub).expect("exists").parent = Some(group);
        d.shape_mut(root).expect("exists").children.push(group);
        d.shape_mut(group).expect("exists").children.push(sub);
        let mut page = Page::new(PageId { index: 0 }, 1);
        page.shapes.push(root);
        d.pages.push(page);

        assert_eq!(d.shape_by_visio_id(PageId { index: 0 }, 1), Some(root));
        assert_eq!(d.shape_by_visio_id(PageId { index: 0 }, 2), Some(group));
        assert_eq!(d.shape_by_visio_id(PageId { index: 0 }, 3), Some(sub));
        assert_eq!(d.shape_by_visio_id(PageId { index: 0 }, 4), None);
        assert_eq!(d.shape_by_visio_id(PageId { index: 1 }, 1), None);
    }

    #[test]
    fn shape_by_visio_id_survives_cyclic_children() {
        let mut d = Document::new();
        let a = d.add_shape(Shape::new(ShapeId { index: 0 }, 1));
        let b = d.add_shape(Shape::new(ShapeId { index: 0 }, 2));
        d.shape_mut(a).expect("exists").children.push(b);
        d.shape_mut(b).expect("exists").children.push(a);
        let mut page = Page::new(PageId { index: 0 }, 1);
        page.shapes.push(a);
        d.pages.push(page);
        assert_eq!(d.shape_by_visio_id(PageId { index: 0 }, 1), Some(a));
        assert_eq!(d.shape_by_visio_id(PageId { index: 0 }, 2), Some(b));
        assert_eq!(d.shape_by_visio_id(PageId { index: 0 }, 3), None);
    }

    #[test]
    fn shape_by_visio_id_skips_a_broken_child_id() {
        let mut d = Document::new();
        let root = d.add_shape(Shape::new(ShapeId { index: 0 }, 1));
        d.shape_mut(root)
            .expect("exists")
            .children
            .push(ShapeId { index: 9 });
        let mut page = Page::new(PageId { index: 0 }, 1);
        page.shapes.push(root);
        d.pages.push(page);
        assert_eq!(d.shape_by_visio_id(PageId { index: 0 }, 1), Some(root));
        assert_eq!(d.shape_by_visio_id(PageId { index: 0 }, 5), None);
    }

    #[test]
    fn master_by_name_u_and_style_by_name_find_their_rows() {
        let mut d = Document::new();
        let mut master = Master::new(MasterId { index: 0 }, 1);
        master.name = "Box".to_owned();
        master.name_u = "Box.2".to_owned();
        d.masters.push(master);
        let mut style = crate::style::Style::new(StyleId { index: 0 });
        style.name = "Normal".to_owned();
        style.name_u = "Normal.2".to_owned();
        d.stylesheet.styles.push(style);

        assert_eq!(d.master_by_name_u("Box.2"), Some(MasterId { index: 0 }));
        assert_eq!(d.master_by_name_u("Box"), None);
        assert_eq!(d.master_by_name_u("Nope"), None);
        assert_eq!(d.style_by_name("Normal"), Some(StyleId { index: 0 }));
        assert_eq!(d.style_by_name("Normal.2"), None);
        assert_eq!(d.style_by_name("Nope"), None);
    }

    #[test]
    fn style_by_visio_id_finds_the_stylesheet_row() {
        let mut d = Document::new();
        let mut first = crate::style::Style::new(StyleId { index: 0 });
        first.visio_id = 7;
        d.stylesheet.styles.push(first);
        let mut second = crate::style::Style::new(StyleId { index: 1 });
        second.visio_id = 3;
        d.stylesheet.styles.push(second);

        assert_eq!(
            d.style_by_visio_id(7).map(|s| s.id),
            Some(StyleId { index: 0 })
        );
        assert_eq!(
            d.style_by_visio_id(3).map(|s| s.id),
            Some(StyleId { index: 1 })
        );
        assert_eq!(d.style_by_visio_id(4), None);
    }
}
