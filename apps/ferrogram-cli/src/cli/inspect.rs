//! The `inspect` summary and its text and JSON forms (ADR 0026).
//!
//! The summary reports the document kind, each page (name, size in inches,
//! shape count, rendering warning count) and the counts of masters, styles
//! and themes. `--json` prints the same data as one JSON object with a
//! stable key order; the text form is compact.

use ferrogram_model::{Document, DocumentKind, Page};

/// One page row of the inspect summary (ADR 0026).
pub(crate) struct PageSummary {
    /// The page number, counted from 1 in document order.
    pub index: usize,
    /// The page name, as written in the file.
    pub name: String,
    /// The page width in inches.
    pub width: f64,
    /// The page height in inches.
    pub height: f64,
    /// The number of shapes on the page, groups included.
    pub shapes: usize,
    /// The number of rendering warnings of the page (ADR 0021).
    pub warnings: usize,
}

/// The inspect summary of a document (ADR 0026).
pub(crate) struct Summary {
    /// The document kind: `drawing`, `stencil` or `template`.
    pub kind: &'static str,
    /// The pages, in document order.
    pub pages: Vec<PageSummary>,
    /// The number of masters.
    pub masters: usize,
    /// The number of stylesheet rows.
    pub styles: usize,
    /// The number of theme parts.
    pub themes: usize,
}

impl Summary {
    /// The total rendering warning count of the summarised pages.
    pub(crate) fn warnings(&self) -> usize {
        self.pages.iter().map(|page| page.warnings).sum()
    }

    /// The compact text form (ADR 0026).
    pub(crate) fn text(&self) -> String {
        let mut text = String::new();
        text.push_str(&format!("kind: {}\n", self.kind));
        text.push_str(&format!("pages: {}\n", self.pages.len()));
        for page in &self.pages {
            let name = if page.name.is_empty() {
                "(unnamed)"
            } else {
                page.name.as_str()
            };
            text.push_str(&format!(
                "  {}: {} {} x {} in, {} shapes, {} warnings\n",
                page.index, name, page.width, page.height, page.shapes, page.warnings
            ));
        }
        text.push_str(&format!("masters: {}\n", self.masters));
        text.push_str(&format!("styles: {}\n", self.styles));
        text.push_str(&format!("themes: {}\n", self.themes));
        text.push_str(&format!("warnings: {}\n", self.warnings()));
        text
    }

    /// The JSON form: one object, stable key order (ADR 0026).
    pub(crate) fn json(&self) -> String {
        let value = serde_json::json!({
            "kind": self.kind,
            "pages": self.pages.iter().map(|page| serde_json::json!({
                "index": page.index,
                "name": page.name,
                "width": finite(page.width),
                "height": finite(page.height),
                "shapes": page.shapes,
                "warnings": page.warnings,
            })).collect::<Vec<_>>(),
            "masters": self.masters,
            "styles": self.styles,
            "themes": self.themes,
            "warnings": self.warnings(),
        });
        serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string())
    }
}

/// The document kind as a word (ADR 0026).
pub(crate) fn kind_text(kind: DocumentKind) -> &'static str {
    match kind {
        DocumentKind::Drawing => "drawing",
        DocumentKind::Stencil => "stencil",
        DocumentKind::Template => "template",
        _ => "unknown",
    }
}

/// The number of shapes on a page, group sub-shapes included.
///
/// A broken or cyclic `children` list is walked once, never looped on.
pub(crate) fn shape_count(doc: &Document, page: &Page) -> usize {
    let mut seen = vec![false; doc.shapes.len()];
    let mut count = 0usize;
    let mut stack: Vec<ferrogram_model::ShapeId> = page.shapes.to_vec();
    while let Some(id) = stack.pop() {
        let Some(slot) = seen.get_mut(id.index as usize) else {
            continue;
        };
        if *slot {
            continue;
        }
        *slot = true;
        count = count.saturating_add(1);
        if let Some(shape) = doc.shape(id) {
            stack.extend(shape.children.iter().copied());
        }
    }
    count
}

/// A finite JSON number, or 0 (serde_json rejects NaN and infinity).
fn finite(value: f64) -> f64 {
    if value.is_finite() { value } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferrogram_model::{Master, PageId, Shape, ShapeId};

    fn summary() -> Summary {
        Summary {
            kind: "drawing",
            pages: vec![
                PageSummary {
                    index: 1,
                    name: "Page-1".to_owned(),
                    width: 8.5,
                    height: 11.0,
                    shapes: 3,
                    warnings: 0,
                },
                PageSummary {
                    index: 2,
                    name: String::new(),
                    width: 11.0,
                    height: 8.5,
                    shapes: 0,
                    warnings: 2,
                },
            ],
            masters: 1,
            styles: 4,
            themes: 1,
        }
    }

    #[test]
    fn text_form_lists_every_page() {
        insta::assert_snapshot!(summary().text(), @r"
        kind: drawing
        pages: 2
          1: Page-1 8.5 x 11 in, 3 shapes, 0 warnings
          2: (unnamed) 11 x 8.5 in, 0 shapes, 2 warnings
        masters: 1
        styles: 4
        themes: 1
        warnings: 2
        ");
    }

    #[test]
    fn json_form_has_stable_keys() {
        insta::assert_snapshot!(summary().json(), @r###"
        {
          "kind": "drawing",
          "masters": 1,
          "pages": [
            {
              "height": 11.0,
              "index": 1,
              "name": "Page-1",
              "shapes": 3,
              "warnings": 0,
              "width": 8.5
            },
            {
              "height": 8.5,
              "index": 2,
              "name": "",
              "shapes": 0,
              "warnings": 2,
              "width": 11.0
            }
          ],
          "styles": 4,
          "themes": 1,
          "warnings": 2
        }
        "###);
    }

    #[test]
    fn json_form_parses_and_carries_the_warning_count() {
        let value: serde_json::Value = serde_json::from_str(&summary().json()).expect("valid JSON");
        assert_eq!(value["warnings"], 2);
        assert_eq!(value["pages"][0]["warnings"], 0);
        assert_eq!(value["pages"][1]["name"], "");
    }

    #[test]
    fn non_finite_sizes_become_zero() {
        let mut summary = summary();
        summary.pages[0].width = f64::NAN;
        summary.pages[0].height = f64::INFINITY;
        let value: serde_json::Value = serde_json::from_str(&summary.json()).expect("valid JSON");
        assert_eq!(value["pages"][0]["width"], 0.0);
        assert_eq!(value["pages"][0]["height"], 0.0);
    }

    #[test]
    fn kind_text_names_the_document_kind() {
        assert_eq!(kind_text(DocumentKind::Drawing), "drawing");
        assert_eq!(kind_text(DocumentKind::Stencil), "stencil");
        assert_eq!(kind_text(DocumentKind::Template), "template");
    }

    #[test]
    fn shape_count_walks_groups_once() {
        let mut doc = Document::new();
        let group = doc.add_shape(Shape::new(ShapeId { index: 0 }, 1));
        let child = doc.add_shape(Shape::new(ShapeId { index: 1 }, 2));
        if let Some(shape) = doc.shape_mut(group) {
            shape.children.push(child);
            shape.children.push(child); // a repeated child is counted once
        }
        if let Some(shape) = doc.shape_mut(child) {
            shape.children.push(group); // a cycle does not loop
            shape.parent = Some(group);
        }
        let mut page = Page::new(PageId { index: 0 }, 1);
        page.shapes.push(group);
        assert_eq!(shape_count(&doc, &page), 2);
    }

    #[test]
    fn shape_count_ignores_stale_ids() {
        let doc = Document::new();
        let mut page = Page::new(PageId { index: 0 }, 1);
        page.shapes.push(ShapeId { index: 9 });
        assert_eq!(shape_count(&doc, &page), 0);
    }

    #[test]
    fn summary_counts_what_the_document_holds() {
        let mut doc = Document::new();
        doc.masters
            .push(Master::new(ferrogram_model::MasterId { index: 0 }, 1));
        let summary = Summary {
            kind: kind_text(doc.kind),
            pages: Vec::new(),
            masters: doc.masters.len(),
            styles: doc.stylesheet.styles.len(),
            themes: doc.themes.len(),
        };
        assert_eq!(summary.warnings(), 0);
        assert_eq!(summary.masters, 1);
    }
}
