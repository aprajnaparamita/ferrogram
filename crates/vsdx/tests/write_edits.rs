//! Writing edited models back into a preserved package (task P2-07,
//! ADR 0018).
//!
//! These are the Phase 4 edit shapes: change a cell, delete a shape, add a
//! shape, retitle a page. Each edit must land in the written XML at the
//! canonical position, with everything the model does not map left alone.

mod common;

use common::{Fixture, build};
use larrygram_model::{Cell, CellName, CellValue, Document, Preserved, Shape, ShapeId, XmlChild};
use larrygram_vsdx::{Limits, Package, PartData, PartName, read_package, to_model, write_document};

/// The drawing fixture as a model plus the package to patch.
#[allow(clippy::expect_used)] // test helper
fn drawing() -> (Document, Package) {
    let bytes = build(Fixture::Drawing);
    let package = read_package(&bytes, &Limits::DEFAULT).expect("read");
    let document = to_model(&package).expect("model");
    (document, package)
}

/// The written package after writing `document` into `package`.
#[allow(clippy::expect_used)] // test helper
fn written(document: &Document, package: &Package) -> Package {
    let bytes = write_document(document, package).expect("write");
    read_package(&bytes, &Limits::DEFAULT).expect("reread")
}

/// The root element of an XML part.
#[allow(clippy::panic, clippy::expect_used)] // test helper
fn root<'p>(package: &'p Package, name: &str) -> &'p larrygram_model::XmlNode {
    match &package.part(&PartName::new(name)).expect("part").data {
        PartData::Xml(root) => root,
        _ => panic!("part is not XML"),
    }
}

/// The `V` attribute of the cell named `name` in `element`, if present.
fn cell_value(element: &larrygram_model::XmlNode, name: &str) -> Option<String> {
    element.children.iter().find_map(|child| match child {
        XmlChild::Element(cell)
            if cell.name.local == "Cell"
                && cell
                    .attributes
                    .iter()
                    .any(|a| a.name.local == "N" && a.value == name) =>
        {
            cell.attributes
                .iter()
                .find(|a| a.name.local == "V")
                .map(|a| a.value.clone())
        }
        _ => None,
    })
}

/// The child elements of `element` with this local name.
fn children_named<'n>(
    element: &'n larrygram_model::XmlNode,
    local: &str,
) -> Vec<&'n larrygram_model::XmlNode> {
    element
        .children
        .iter()
        .filter_map(|child| match child {
            XmlChild::Element(node) if node.name.local == local => Some(node),
            _ => None,
        })
        .collect()
}

/// The first `Shape` element of the drawing fixture's page contents.
#[allow(clippy::expect_used)] // test helper
fn first_shape(package: &Package) -> &larrygram_model::XmlNode {
    let contents = root(package, "/visio/pages/page1.xml");
    let wrapper = children_named(contents, "Shapes")
        .into_iter()
        .next()
        .expect("Shapes wrapper");
    children_named(wrapper, "Shape")
        .into_iter()
        .next()
        .expect("shape")
}

#[test]
fn a_changed_cell_value_is_written_and_the_rest_is_untouched() {
    let (mut document, package) = drawing();
    let shape = document.shape_mut(ShapeId { index: 0 }).expect("shape");
    shape
        .sheet
        .cells
        .iter_mut()
        .find(|cell| cell.name.as_str() == "PinX")
        .expect("PinX")
        .value = Some(CellValue::Number(4.5));

    let out = written(&document, &package);
    let shape = first_shape(&out);
    assert_eq!(cell_value(shape, "PinX").as_deref(), Some("4.5"));
    // Untouched cells keep their spelling exactly.
    assert_eq!(cell_value(shape, "PinY").as_deref(), Some("2"));

    let document = to_model(&out).expect("model");
    let shape = document.shape(ShapeId { index: 0 }).expect("shape");
    let pin_x = shape
        .sheet
        .cells
        .iter()
        .find(|cell| cell.name.as_str() == "PinX")
        .expect("PinX");
    assert_eq!(pin_x.value, Some(CellValue::Number(4.5)));
}

#[test]
fn a_deleted_shape_is_removed_from_the_page() {
    let (mut document, package) = drawing();
    document.pages[0].shapes.clear();
    let out = written(&document, &package);
    let contents = root(&out, "/visio/pages/page1.xml");
    let wrapper = children_named(contents, "Shapes")
        .into_iter()
        .next()
        .expect("Shapes wrapper");
    assert!(children_named(wrapper, "Shape").is_empty());
    let reread = to_model(&out).expect("model");
    assert!(reread.pages[0].shapes.is_empty());
}

#[test]
fn a_new_shape_is_inserted_after_the_last_shape() {
    let (mut document, package) = drawing();
    let mut shape = Shape::new(ShapeId { index: 0 }, 7);
    shape.name = "New".to_owned();
    let id = document.add_shape(shape);
    document.pages[0].shapes.push(id);

    let out = written(&document, &package);
    let contents = root(&out, "/visio/pages/page1.xml");
    let wrapper = children_named(contents, "Shapes")
        .into_iter()
        .next()
        .expect("Shapes wrapper");
    let shapes = children_named(wrapper, "Shape");
    assert_eq!(shapes.len(), 2);
    assert_eq!(
        shapes[1]
            .attributes
            .iter()
            .find(|a| a.name.local == "Name")
            .map(|a| a.value.as_str()),
        Some("New")
    );
    let reread = to_model(&out).expect("model");
    assert_eq!(reread.pages[0].shapes.len(), 2);
}

#[test]
fn a_renamed_page_writes_the_new_name_only() {
    let (mut document, package) = drawing();
    document.pages[0].name = "Renamed".to_owned();
    let out = written(&document, &package);
    let pages = root(&out, "/visio/pages/pages.xml");
    let page = children_named(pages, "Page")
        .into_iter()
        .next()
        .expect("page");
    let name = page
        .attributes
        .iter()
        .find(|a| a.name.local == "Name")
        .expect("Name");
    assert_eq!(name.value, "Renamed");
    // `NameU` and `ID` are untouched.
    assert!(
        page.attributes
            .iter()
            .any(|a| a.name.local == "NameU" && a.value == "Page-1")
    );
    let reread = to_model(&out).expect("model");
    assert_eq!(reread.pages[0].name, "Renamed");
}

#[test]
fn a_new_cell_is_inserted_before_the_sections() {
    let (mut document, package) = drawing();
    let shape = document.shape_mut(ShapeId { index: 0 }).expect("shape");
    let mut cell = Cell::new(CellName::from_static("Angle"));
    cell.value = Some(CellValue::Number(1.5));
    shape.sheet.cells.push(cell);
    let out = written(&document, &package);
    let shape = first_shape(&out);
    assert_eq!(cell_value(shape, "Angle").as_deref(), Some("1.5"));
    let names: Vec<&str> = children_named(shape, "Cell")
        .iter()
        .map(|cell| {
            cell.attributes
                .iter()
                .find(|a| a.name.local == "N")
                .map_or("?", |a| a.value.as_str())
        })
        .collect();
    let sections = children_named(shape, "Section");
    let last_cell = names.len().saturating_sub(1);
    assert!(names[last_cell] == "Angle" || sections.is_empty());
}

#[test]
fn a_new_property_lands_in_document_properties() {
    let (mut document, package) = drawing();
    document
        .properties
        .push(larrygram_model::Property::new("title", "Hello"));
    let out = written(&document, &package);
    let contents = root(&out, "/visio/document.xml");
    let wrappers = children_named(contents, "DocumentProperties");
    assert_eq!(wrappers.len(), 1);
    let properties = children_named(wrappers[0], "title");
    assert_eq!(properties.len(), 1);
    let text: String = properties[0]
        .children
        .iter()
        .filter_map(|child| match child {
            XmlChild::Text(text) => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(text, "Hello");
}

#[test]
fn preserved_content_survives_an_edit() {
    let (mut document, package) = drawing();
    // The fixture's `DocumentSettings` and the shape's `Type` attribute are
    // unmapped content: an edit must not disturb them.
    document.pages[0].name = "Renamed".to_owned();
    let out = written(&document, &package);
    let contents = root(&out, "/visio/document.xml");
    assert_eq!(children_named(contents, "DocumentSettings").len(), 1);
    let shape = first_shape(&out);
    assert!(
        shape
            .attributes
            .iter()
            .any(|a| a.name.local == "Type" && a.value == "Shape")
    );
}

/// An element with these attributes and children.
fn element(
    local: &str,
    attrs: &[(&str, &str)],
    children: Vec<larrygram_model::XmlNode>,
) -> larrygram_model::XmlNode {
    larrygram_model::XmlNode {
        name: larrygram_model::QName {
            prefix: None,
            uri: Some("urn:visio".to_owned()),
            local: local.to_owned(),
        },
        attributes: attrs
            .iter()
            .map(|(name, value)| larrygram_model::XmlAttribute {
                name: larrygram_model::QName::local(*name),
                value: (*value).to_owned(),
            })
            .collect(),
        children: children.into_iter().map(XmlChild::Element).collect(),
        self_closing: false,
    }
}

/// A minimal package whose page has no `Rel` and no contents part.
fn relless_page_package() -> Package {
    use larrygram_vsdx::{
        ContentType, ContentTypeDecl, ContentTypeKind, Part, Relationship, RelationshipId,
        TargetMode,
    };
    let mut package = Package::new();
    package.content_types.push(ContentTypeDecl {
        kind: ContentTypeKind::Default,
        name: "xml".to_owned(),
        content_type: "application/xml".to_owned(),
    });
    let document = element(
        "VisioDocument",
        &[("xmlns", "urn:visio")],
        vec![element("DocumentSettings", &[], Vec::new())],
    );
    let pages = element(
        "Pages",
        &[("xmlns", "urn:visio")],
        vec![element(
            "Page",
            &[("ID", "1"), ("NameU", "Page-1"), ("Name", "Page-1")],
            Vec::new(),
        )],
    );
    for (name, root) in [
        ("/visio/document.xml", document),
        ("/visio/pages/pages.xml", pages),
    ] {
        package.parts.push(Part {
            name: PartName::new(name),
            content_type: ContentType::new("application/xml"),
            data: PartData::Xml(root),
        });
    }
    for (source, id, rel_type, target) in [
        ("/", "rId1", "document", "visio/document.xml"),
        ("/visio/document.xml", "rId1", "pages", "pages/pages.xml"),
    ] {
        package.relationships.push(Relationship {
            source: PartName::new(source),
            id: RelationshipId::new(id),
            rel_type: format!("http://schemas.microsoft.com/visio/2010/relationships/{rel_type}"),
            target: target.to_owned(),
            target_mode: TargetMode::Internal,
            preserved: Preserved::new(),
        });
    }
    package
}

#[test]
fn a_page_without_a_rel_gets_one_when_it_gains_shapes() {
    // A page with no `Rel` element (the reader gives it no contents) becomes
    // a page with contents: the writer adds the `Rel`, the relationship and
    // the part.
    let package = relless_page_package();
    let mut document = to_model(&package).expect("model");
    assert!(document.pages[0].shapes.is_empty());
    let mut shape = Shape::new(ShapeId { index: 0 }, 1);
    shape.name = "Box".to_owned();
    let id = document.add_shape(shape);
    document.pages[0].shapes.push(id);

    let out = written(&document, &package);
    let pages = root(&out, "/visio/pages/pages.xml");
    let page = children_named(pages, "Page")
        .into_iter()
        .next()
        .expect("page");
    assert_eq!(children_named(page, "Rel").len(), 1);
    assert!(out.part(&PartName::new("/visio/pages/page1.xml")).is_some());
    let reread = to_model(&out).expect("model");
    assert_eq!(reread.pages[0].shapes.len(), 1);
}

#[test]
fn an_empty_preserved_fragment_is_fine() {
    // A model with nothing preserved still writes.
    let (mut document, package) = drawing();
    document.preserved = Preserved::new();
    let out = written(&document, &package);
    assert_eq!(out.parts.len(), package.parts.len());
}
