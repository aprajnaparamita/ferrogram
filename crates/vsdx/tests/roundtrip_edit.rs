//! The ADR 0048 round-trip identity for edited documents (task P5-02).
//!
//! A document built with the mutation API (new shapes, a new page, a
//! master instance, `Connects`, text) is written, re-read and written
//! again. The two reads compare **equal** as `model::Document` values
//! (ignoring the volatile document properties and `Preserved` attribute
//! order) and the second and third saves are byte-identical (the writer
//! is deterministic). A deleted page loses its part, its relationships
//! and its content-type entry.

use ferrogram_model::mutate::{CellWrite, VIS_BEGIN, VIS_END, VIS_WHOLE_SHAPE};
use ferrogram_model::{
    Cell, CellName, CellValue, Connect, Document, Master, MasterId, PageId, Preserved, Row,
    Section, SectionName, Shape, ShapeId, Sheet, TextContent, TextPart, XmlChild, XmlNode,
};
use ferrogram_vsdx::{Limits, Package, PartName, read_package, to_model, write_document};

/// A cell with a cached number and a literal formula.
#[allow(clippy::expect_used, clippy::panic)] // test helper
fn number_cell(name: &str, value: f64) -> Cell {
    let mut cell = Cell::new(CellName::new(name));
    cell.formula = Some(value.to_string());
    cell.value = Some(CellValue::Number(value));
    cell.unit = ferrogram_geom::Unit::Inches;
    cell
}

/// A rectangle shape sheet with transform cells and one `Geometry` path.
#[allow(clippy::expect_used, clippy::panic)] // test helper
fn rectangle(width: f64, height: f64) -> Shape {
    let mut shape = Shape::new(ShapeId { index: 0 }, 0);
    shape.sheet.cells = vec![
        number_cell("Width", width),
        number_cell("Height", height),
        number_cell("PinX", width * 0.5),
        number_cell("PinY", height * 0.5),
    ];
    let mut section = Section::new(SectionName::from_static("Geometry"));
    for (index, (x, y)) in [
        (0.0, 0.0),
        (width, 0.0),
        (width, height),
        (0.0, height),
        (0.0, 0.0),
    ]
    .iter()
    .enumerate()
    {
        let mut row = Row::new();
        row.row_type = Some(if index == 0 { "MoveTo" } else { "LineTo" }.to_owned());
        row.ix = Some(i32::try_from(index).expect("index"));
        row.cells = vec![number_cell("X", *x), number_cell("Y", *y)];
        section.rows.push(row);
    }
    shape.sheet.sections.push(section);
    shape
}

/// A document built with the mutation API (ADR 0048's `roundtrip_edit`
/// scenario): two pages, new shapes, text, a master instance and glue.
#[allow(clippy::expect_used, clippy::panic)] // test helper
fn built() -> Document {
    let mut doc = Document::new();
    let page = doc.add_page("Page-1").expect("page");
    let rect = doc
        .insert_shape(page, rectangle(2.0, 1.0), None)
        .expect("rect");
    doc.write_cell(
        ferrogram_model::SheetRef::Shape(rect),
        &CellName::from_static("PinX"),
        &CellWrite::new("3", CellValue::Number(3.0)),
    )
    .expect("pin");
    let mut text = TextContent::new();
    text.parts.push(TextPart::Text("Hello".to_owned()));
    doc.set_shape_text(rect, text).expect("text");

    // A master with one shape, instantiated once.
    let mut master_shape = rectangle(1.0, 1.0);
    master_shape.name_u = "Box".to_owned();
    let content = doc.add_shape(master_shape);
    let mut master = Master::new(MasterId { index: 0 }, 0);
    master.name = "Box".to_owned();
    master.name_u = "Box".to_owned();
    master.shapes.push(content);
    let master = doc.add_master(master).expect("master");
    doc.instantiate_master(master, page, ferrogram_geom::Point::new(5.0, 5.0), None)
        .expect("instance");

    // A connector glued to the rectangle.
    let connector = doc
        .insert_shape(page, rectangle(1.0, 0.0), None)
        .expect("connector");
    let mut begin = Connect::new();
    begin.from_sheet = doc.shape(connector).expect("c").visio_id;
    begin.from_cell = "BeginX".to_owned();
    begin.from_part = VIS_BEGIN;
    begin.to_sheet = doc.shape(rect).expect("r").visio_id;
    begin.to_cell = "PinX".to_owned();
    begin.to_part = VIS_WHOLE_SHAPE;
    doc.add_connect(page, begin).expect("begin");
    let mut end = Connect::new();
    end.from_sheet = doc.shape(connector).expect("c").visio_id;
    end.from_cell = "EndX".to_owned();
    end.from_part = VIS_END;
    end.to_sheet = doc.shape(rect).expect("r").visio_id;
    end.to_cell = "PinX".to_owned();
    end.to_part = VIS_WHOLE_SHAPE;
    doc.add_connect(page, end).expect("end");

    // A second page with one shape, and a resize of the first shape.
    let second = doc.add_page("Page-2").expect("page");
    doc.insert_shape(second, rectangle(1.0, 1.0), None)
        .expect("s");
    doc.write_cell(
        ferrogram_model::SheetRef::Shape(rect),
        &CellName::from_static("Width"),
        &CellWrite::new("4", CellValue::Number(4.0)),
    )
    .expect("width");
    doc.write_cell(
        ferrogram_model::SheetRef::Shape(rect),
        &CellName::from_static("Height"),
        &CellWrite::new("2", CellValue::Number(2.0)),
    )
    .expect("height");
    doc.scale_geometry(
        rect,
        ferrogram_geom::Size::new(2.0, 1.0),
        ferrogram_geom::Size::new(4.0, 2.0),
    )
    .expect("scale");
    doc
}

/// Sorts the attributes of every preserved element (attribute order is
/// not semantic, ADR 0012).
fn sort_node(node: &mut XmlNode) {
    node.attributes
        .sort_by(|a, b| (&a.name.uri, &a.name.local).cmp(&(&b.name.uri, &b.name.local)));
    for child in &mut node.children {
        if let XmlChild::Element(element) = child {
            sort_node(element);
        }
    }
}

/// Sorts the attributes of one preserved fragment.
fn sort_preserved(preserved: &mut Preserved) {
    preserved
        .attributes
        .sort_by(|a, b| (&a.name.uri, &a.name.local).cmp(&(&b.name.uri, &b.name.local)));
    for child in &mut preserved.children {
        sort_node(child);
    }
}

/// Sorts the preserved fragments of one sheet.
fn sort_sheet(sheet: &mut Sheet) {
    for cell in &mut sheet.cells {
        sort_preserved(&mut cell.preserved);
    }
    for section in &mut sheet.sections {
        sort_preserved(&mut section.preserved);
        for cell in &mut section.cells {
            sort_preserved(&mut cell.preserved);
        }
        for row in &mut section.rows {
            sort_preserved(&mut row.preserved);
            for cell in &mut row.cells {
                sort_preserved(&mut cell.preserved);
            }
        }
    }
}

/// Normalises a document for the ADR 0048 model comparison: `Preserved`
/// attribute order is not semantic and the volatile document properties
/// are not compared.
fn normalised(document: &Document) -> Document {
    let mut document = document.clone();
    sort_preserved(&mut document.preserved);
    for style in &mut document.stylesheet.styles {
        sort_preserved(&mut style.preserved);
        sort_sheet(&mut style.sheet);
    }
    for page in &mut document.pages {
        sort_preserved(&mut page.preserved);
        sort_sheet(&mut page.sheet);
        for connect in &mut page.connects {
            sort_preserved(&mut connect.preserved);
        }
    }
    for master in &mut document.masters {
        sort_preserved(&mut master.preserved);
    }
    for shape in &mut document.shapes {
        sort_preserved(&mut shape.preserved);
        sort_sheet(&mut shape.sheet);
    }
    document.properties.retain(|property| {
        !matches!(
            property.name.as_str(),
            "modified" | "lastModifiedBy" | "revision" | "lastPrinted"
        )
    });
    document
}

/// Writes `document` into `package` and re-reads both bytes and model.
#[allow(clippy::expect_used, clippy::panic)] // test helper
fn write_and_read(document: &Document, package: &Package) -> (Vec<u8>, Package, Document) {
    let bytes = write_document(document, package).expect("write");
    let package = read_package(&bytes, &Limits::DEFAULT).expect("read");
    let document = to_model(&package).expect("model");
    (bytes, package, document)
}

#[test]
fn an_edited_document_re_opens_identically() {
    let document = built();
    // First save: generated from the model alone (a new document).
    let (first_bytes, first_package, first) = write_and_read(&document, &Package::new());
    // Second save: the re-opened document patched into its package.
    let (second_bytes, second_package, second) = write_and_read(&first, &first_package);
    // Third save: the twice re-opened document.
    let (third_bytes, _, third) = write_and_read(&second, &second_package);

    // ADR 0048: `read(write(read(write(d))))` equals `read(write(d))`.
    assert_eq!(
        normalised(&second),
        normalised(&third),
        "a re-opened document keeps its model"
    );
    // The writer is deterministic: a second save writes the same bytes.
    assert_eq!(second_bytes, third_bytes);
    assert!(!first_bytes.is_empty());
}

#[test]
fn the_edits_survive_the_first_re_open() {
    let document = built();
    let (_, _, first) = write_and_read(&document, &Package::new());
    assert_eq!(first.pages.len(), 2);
    assert_eq!(first.pages[0].name_u, "Page-1");
    assert_eq!(first.pages[1].name_u, "Page-2");
    assert_eq!(first.masters.len(), 1);
    let page = PageId { index: 0 };
    assert_eq!(first.page(page).expect("page").shapes.len(), 3);
    assert_eq!(first.page(page).expect("page").connects.len(), 2);
    // The text and the resize survive.
    let rect = first
        .shape_by_visio_id(PageId { index: 0 }, 1)
        .and_then(|id| first.shape(id))
        .expect("rect");
    assert_eq!(
        rect.text.as_ref().map(|text| text.parts.len()),
        Some(1),
        "the shape keeps its text"
    );
    let width = rect
        .sheet
        .cells
        .iter()
        .find(|cell| cell.name.as_str() == "Width")
        .and_then(|cell| cell.value.clone());
    assert_eq!(width, Some(CellValue::Number(4.0)));
    let geometry = rect
        .sheet
        .sections
        .iter()
        .find(|section| section.name.as_str() == "Geometry")
        .expect("geometry");
    assert_eq!(geometry.rows.len(), 5);
    assert_eq!(
        geometry.rows[1].cells[0].value,
        Some(CellValue::Number(4.0)),
        "the resized row is scaled"
    );
}

#[test]
fn a_deleted_page_drops_its_part_and_relationships() {
    let document = built();
    let (_, package, mut first) = write_and_read(&document, &Package::new());
    assert!(
        package
            .part(&PartName::new("/visio/pages/page2.xml"))
            .is_some(),
        "the second page has a contents part before the delete"
    );
    first.delete_page(PageId { index: 1 }).expect("delete");
    let (_, reread, second) = write_and_read(&first, &package);
    assert_eq!(second.pages.len(), 1);
    // The part is gone.
    assert!(
        reread
            .part(&PartName::new("/visio/pages/page2.xml"))
            .is_none()
    );
    // Its relationship is gone.
    assert!(
        !reread
            .relationships
            .iter()
            .any(|relationship| relationship.target.contains("page2.xml"))
    );
    // Its content-type override is gone.
    assert!(
        !reread
            .content_types
            .iter()
            .any(|decl| decl.name.contains("page2.xml"))
    );
    // The page list no longer names it.
    let pages_part = reread
        .part(&PartName::new("/visio/pages/pages.xml"))
        .expect("pages part");
    let ferrogram_vsdx::PartData::Xml(root) = &pages_part.data else {
        panic!("pages part is not XML");
    };
    let page_elements = root
        .children
        .iter()
        .filter(|child| matches!(child, XmlChild::Element(element) if element.name.local == "Page"))
        .count();
    assert_eq!(page_elements, 1);
}

#[test]
fn a_new_page_gains_a_part_a_relationship_and_a_content_type() {
    let document = built();
    let (_, package, mut first) = write_and_read(&document, &Package::new());
    first.add_page("Page-3").expect("page");
    let (_, reread, second) = write_and_read(&first, &package);
    assert_eq!(second.pages.len(), 3);
    // The new page has a contents part (page3.xml), declared and related.
    assert!(
        reread
            .part(&PartName::new("/visio/pages/page3.xml"))
            .is_some()
    );
    assert!(
        reread
            .content_types
            .iter()
            .any(|decl| decl.name.contains("page3.xml"))
    );
    assert!(
        reread
            .relationships
            .iter()
            .any(|relationship| relationship.target.contains("page3.xml"))
    );
}

#[test]
fn a_new_shape_and_its_text_land_in_the_written_page() {
    let document = built();
    let (_, package, mut first) = write_and_read(&document, &Package::new());
    let page = PageId { index: 0 };
    let shape = first
        .insert_shape(page, rectangle(1.0, 1.0), None)
        .expect("shape");
    let visio_id = first.shape(shape).expect("shape").visio_id;
    let mut text = TextContent::new();
    text.parts.push(TextPart::CharRun { ix: 0 });
    text.parts.push(TextPart::Text("added".to_owned()));
    first.set_shape_text(shape, text).expect("text");
    let (_, reread, second) = write_and_read(&first, &package);
    assert_eq!(second.page(page).expect("page").shapes.len(), 4);
    let added = second
        .shape_by_visio_id(page, visio_id)
        .and_then(|id| second.shape(id))
        .expect("shape");
    let text = added.text.as_ref().expect("text");
    assert_eq!(
        text.parts,
        vec![
            ferrogram_model::TextPart::CharRun { ix: 0 },
            ferrogram_model::TextPart::Text("added".to_owned())
        ]
    );
    // The written `Text` element carries the markers.
    let page_part = reread
        .part(&PartName::new("/visio/pages/page1.xml"))
        .expect("page part");
    let ferrogram_vsdx::PartData::Xml(root) = &page_part.data else {
        panic!("page part is not XML");
    };
    fn text_elements(node: &XmlNode, out: &mut Vec<String>) {
        for child in &node.children {
            if let XmlChild::Element(element) = child {
                if element.name.local == "Text" {
                    out.push(format!("{element:?}"));
                }
                text_elements(element, out);
            }
        }
    }
    let mut found = Vec::new();
    text_elements(root, &mut found);
    assert!(
        found
            .iter()
            .any(|text| text.contains("added") && text.contains("cp")),
        "the written text carries the run marker: {found:?}"
    );
}

#[test]
fn an_edited_text_of_an_existing_shape_is_written_back() {
    let document = built();
    let (_, package, mut first) = write_and_read(&document, &Package::new());
    let mut text = TextContent::new();
    text.parts.push(TextPart::Text("edited".to_owned()));
    let shape = first
        .shape_by_visio_id(PageId { index: 0 }, 1)
        .expect("rect");
    first.set_shape_text(shape, text).expect("text");
    let (_, _, second) = write_and_read(&first, &package);
    let edited = second
        .shape_by_visio_id(PageId { index: 0 }, 1)
        .and_then(|id| second.shape(id))
        .expect("rect");
    assert_eq!(
        edited.text.as_ref().map(|text| text.parts.clone()),
        Some(vec![TextPart::Text("edited".to_owned())])
    );
}
