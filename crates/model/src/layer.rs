//! Layers and layer membership (ADR 0019).
//!
//! A page sheet's `Layer` section rows name the page's layers; a shape's
//! `LayerMember` cell lists the layer indices it belongs to. The renderer
//! drops shapes on invisible layers.

use crate::document::Document;
use crate::error::Error;
use crate::id::{PageId, ShapeId};
use crate::name::CellName;
use crate::values::{CellReader, SheetRef};

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
    /// A row's `IX` is the layer index (the value `LayerMember` lists); a
    /// row without one uses its position. A row with no `Name` cell has an
    /// empty name, and a row with no `Visible` cell is visible (the
    /// rendering default; the ShapeSheet table records no default and a
    /// `false` here would hide every shape of the layer).
    pub fn page_layers(&self, page: PageId) -> Result<Vec<Layer>, Error> {
        let entity = self.page(page).ok_or(Error::UnknownPage(page))?;
        let mut layers = Vec::new();
        for section in entity
            .sheet
            .sections
            .iter()
            .filter(|section| section.name.as_str() == "Layer")
        {
            for (position, row) in section.rows.iter().enumerate() {
                let ix = row
                    .ix
                    .unwrap_or_else(|| i32::try_from(position).unwrap_or(i32::MAX));
                let name = row
                    .cells
                    .iter()
                    .find(|cell| cell.name == CellName::from_static("Name"))
                    .and_then(|cell| cell.value.as_ref())
                    .and_then(|value| match value {
                        crate::cell::CellValue::Str(text) => Some(text.clone()),
                        _ => None,
                    })
                    .unwrap_or_default();
                let visible = row
                    .cells
                    .iter()
                    .find(|cell| cell.name == CellName::from_static("Visible"))
                    .and_then(|cell| cell.value.as_ref())
                    .map(|value| match value {
                        crate::cell::CellValue::Bool(flag) => *flag,
                        crate::cell::CellValue::Number(number) => *number != 0.0,
                        _ => true,
                    })
                    .unwrap_or(true);
                layers.push(Layer { ix, name, visible });
            }
        }
        Ok(layers)
    }

    /// The layer indices a shape belongs to (its `LayerMember` cell, ADR
    /// 0019). The cached list resolves through the lookup (ADR 0013); a
    /// shape that belongs to no layer has an empty list.
    pub fn shape_layers(&self, shape: ShapeId) -> Result<Vec<i32>, Error> {
        let mut reader = CellReader::new(self);
        let values = reader.list(
            SheetRef::Shape(shape),
            &CellName::from_static("LayerMember"),
        )?;
        Ok(values
            .into_iter()
            .map(|value| if value.is_finite() { value as i32 } else { 0 })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use crate::cell::{Cell, CellValue};
    use crate::document::Document;
    use crate::name::SectionName;
    use crate::page::Page;
    use crate::section::{Row, Section};
    use crate::shape::Shape;

    use super::*;

    /// A cell with a cached value.
    fn cell(name: &str, value: CellValue) -> Cell {
        let mut cell = Cell::new(CellName::new(name));
        cell.value = Some(value);
        cell
    }

    /// A document with one page whose sheet has one `Layer` section.
    fn document_with_layers(rows: Vec<Row>) -> (Document, PageId) {
        let mut doc = Document::new();
        let page_id = PageId { index: 0 };
        let mut page = Page::new(page_id, 1);
        let mut section = Section::new(SectionName::from_static("Layer"));
        section.rows = rows;
        page.sheet.sections.push(section);
        doc.pages.push(page);
        (doc, page_id)
    }

    /// A layer row with a name and a visibility flag.
    fn layer_row(ix: i32, name: &str, visible: bool) -> Row {
        let mut row = Row::new();
        row.ix = Some(ix);
        row.cells
            .push(cell("Name", CellValue::Str(name.to_owned())));
        row.cells.push(cell("Visible", CellValue::Bool(visible)));
        row
    }

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
    fn page_layers_reads_the_layer_section_rows() {
        let (doc, page) = document_with_layers(vec![
            layer_row(0, "Connectors", true),
            layer_row(1, "Hidden", false),
        ]);
        let layers = doc.page_layers(page).expect("reads");
        assert_eq!(
            layers,
            vec![
                Layer {
                    ix: 0,
                    name: "Connectors".to_owned(),
                    visible: true
                },
                Layer {
                    ix: 1,
                    name: "Hidden".to_owned(),
                    visible: false
                },
            ]
        );
    }

    #[test]
    fn a_layer_row_without_ix_name_or_visible_degrades() {
        let mut row = Row::new();
        row.cells.push(cell("Visible", CellValue::Number(0.0)));
        let (doc, page) = document_with_layers(vec![layer_row(5, "Named", true), row]);
        let layers = doc.page_layers(page).expect("reads");
        assert_eq!(layers.len(), 2);
        // No `IX`: the row position is the index.
        assert_eq!(layers[1].ix, 1);
        assert_eq!(layers[1].name, "");
        // A numeric `0` is `false`; a missing `Visible` is visible.
        assert!(!layers[1].visible);
        let (doc, page) = document_with_layers(vec![Row::new()]);
        let layers = doc.page_layers(page).expect("reads");
        assert!(layers[0].visible);
        // A `Name` of another kind degrades to the empty name.
        let mut row = Row::new();
        row.cells.push(cell("Name", CellValue::Number(3.0)));
        let (doc, page) = document_with_layers(vec![row]);
        let layers = doc.page_layers(page).expect("reads");
        assert_eq!(layers[0].name, "");
    }

    #[test]
    fn a_page_without_a_layer_section_has_no_layers() {
        let doc = Document::new();
        assert_eq!(
            doc.page_layers(PageId { index: 1 }),
            Err(Error::UnknownPage(PageId { index: 1 }))
        );
        let mut doc = Document::new();
        doc.pages.push(Page::new(PageId { index: 0 }, 1));
        assert_eq!(doc.page_layers(PageId { index: 0 }), Ok(Vec::new()));
    }

    #[test]
    fn shape_layers_parses_the_layer_member_list() {
        let mut doc = Document::new();
        let id = doc.add_shape(Shape::new(ShapeId { index: 0 }, 1));
        {
            let shape = doc.shape_mut(id).expect("exists");
            shape
                .sheet
                .cells
                .push(cell("LayerMember", CellValue::List("0;2;5".to_owned())));
        }
        assert_eq!(doc.shape_layers(id), Ok(vec![0, 2, 5]));
    }

    #[test]
    fn shape_layers_accepts_a_plain_number_and_degrades_beyond() {
        let mut doc = Document::new();
        let single = doc.add_shape(Shape::new(ShapeId { index: 0 }, 1));
        {
            let shape = doc.shape_mut(single).expect("exists");
            shape
                .sheet
                .cells
                .push(cell("LayerMember", CellValue::Number(3.0)));
        }
        assert_eq!(doc.shape_layers(single), Ok(vec![3]));
        // A shape with no `LayerMember` cell belongs to no layer.
        let none = doc.add_shape(Shape::new(ShapeId { index: 0 }, 2));
        assert_eq!(doc.shape_layers(none), Ok(Vec::new()));
        // A stale id is a caller error.
        assert_eq!(
            doc.shape_layers(ShapeId { index: 9 }),
            Err(Error::UnknownShape(ShapeId { index: 9 }))
        );
    }
}
