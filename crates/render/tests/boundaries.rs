//! Boundary tests for the scene builder and both backends (task P3-11,
//! ADR 0027): empty and single-shape scenes, zero-size and huge pages,
//! and text that overflows its shape box. Each case must produce a valid
//! output or a clean `Err`, never a panic.

use ferrogram_model::{
    Cell, CellName, CellValue, Document, Page, PageId, Row, Section, SectionName, Shape, ShapeId,
    TextContent, TextPart,
};
use ferrogram_render::scene::{Node, Scene};
use ferrogram_render::{
    BuildOptions, Error, PngOptions, SvgOptions, build_scene, render_png, render_svg,
};

/// A cell with a cached number.
fn number_cell(name: &str, value: f64) -> Cell {
    let mut cell = Cell::new(CellName::new(name));
    cell.value = Some(CellValue::Number(value));
    cell
}

/// A cell with a cached string.
fn string_cell(name: &str, value: &str) -> Cell {
    let mut cell = Cell::new(CellName::new(name));
    cell.value = Some(CellValue::Str(value.to_owned()));
    cell
}

/// One typed geometry row with numeric cells.
fn grow(row_type: &str, cells: &[(&str, f64)]) -> Row {
    let mut row = Row::new();
    row.row_type = Some(row_type.to_owned());
    row.cells = cells
        .iter()
        .map(|(name, value)| number_cell(name, *value))
        .collect();
    row
}

/// A one-page document with one empty shape (2 x 1 in page).
fn document_with_shape() -> (Document, PageId, ShapeId) {
    let mut doc = Document::new();
    let shape = doc.add_shape(Shape::new(ShapeId { index: 0 }, 1));
    let page = PageId { index: 0 };
    let mut entity = Page::new(page, 1);
    entity.shapes.push(shape);
    doc.pages.push(entity);
    (doc, page, shape)
}

/// Adds one `Geometry` section of `rows` to a shape.
fn with_geometry(doc: &mut Document, shape: ShapeId, rows: Vec<Row>) {
    let mut section = Section::new(SectionName::new("Geometry"));
    section.rows = rows;
    if let Some(entity) = doc.shape_mut(shape) {
        entity.sheet.sections.push(section);
    }
}

/// Sets the page sheet cells of a page.
fn page_cells(doc: &mut Document, page: PageId, cells: Vec<Cell>) {
    if let Some(entity) = doc.pages.get_mut(page.index as usize) {
        entity.sheet.cells = cells;
    }
}

/// Builds page 0 of `doc`.
#[allow(clippy::expect_used)] // test helper; a failing build is a test failure
fn build(doc: &Document) -> Scene {
    build_scene(doc, PageId { index: 0 }, &BuildOptions::new()).expect("builds")
}

/// Asserts the SVG output is well-formed XML and returns it.
#[allow(clippy::expect_used, clippy::panic)] // test helper; a broken output is a failure
fn parse_svg(svg: &str) -> &str {
    let mut reader = quick_xml::Reader::from_str(svg);
    let mut depth = 0usize;
    let mut seen = false;
    loop {
        match reader.read_event() {
            Ok(quick_xml::events::Event::Start(_)) => {
                depth += 1;
                seen = true;
            }
            Ok(quick_xml::events::Event::End(_)) => depth = depth.saturating_sub(1),
            Ok(quick_xml::events::Event::Eof) => break,
            Ok(_) => {}
            Err(error) => panic!("the SVG output does not parse: {error}"),
        }
    }
    assert!(seen, "no elements in the SVG output");
    assert_eq!(depth, 0, "unbalanced tags");
    svg
}

/// Decodes the PNG output and returns the raster.
#[allow(clippy::expect_used)] // test helper; a broken output is a failure
fn decode_png(png: &[u8]) -> tiny_skia::Pixmap {
    tiny_skia::Pixmap::decode_png(png).expect("the PNG output decodes")
}

#[test]
fn an_empty_page_renders_in_both_backends() {
    let (mut doc, page, _shape) = document_with_shape();
    // The page keeps its size but no shapes at all: the scene has no
    // nodes.
    if let Some(entity) = doc.pages.get_mut(page.index as usize) {
        entity.shapes.clear();
    }
    page_cells(
        &mut doc,
        page,
        vec![
            number_cell("PageWidth", 2.0),
            number_cell("PageHeight", 1.0),
        ],
    );
    let scene = build(&doc);
    assert!(scene.nodes.is_empty(), "{:?}", scene.nodes);
    let png = render_png(&scene, &PngOptions::new()).expect("renders");
    let raster = decode_png(&png);
    assert_eq!(raster.width(), 2 * 96);
    assert_eq!(raster.height(), 96);
    let svg = render_svg(&scene, &SvgOptions::new()).expect("writes");
    assert!(parse_svg(&svg).contains("<svg"));
}

#[test]
fn a_single_shape_page_renders_in_both_backends() {
    let (mut doc, _page, shape) = document_with_shape();
    if let Some(entity) = doc.shape_mut(shape) {
        entity.sheet.cells = vec![
            number_cell("Width", 1.0),
            number_cell("Height", 1.0),
            number_cell("FillPattern", 1.0),
        ];
    }
    with_geometry(
        &mut doc,
        shape,
        vec![
            grow("MoveTo", &[("X", 0.0), ("Y", 0.0)]),
            grow("LineTo", &[("X", 1.0), ("Y", 0.0)]),
            grow("LineTo", &[("X", 1.0), ("Y", 1.0)]),
        ],
    );
    let scene = build(&doc);
    assert_eq!(scene.nodes.len(), 1);
    let png = render_png(&scene, &PngOptions::new()).expect("renders");
    let raster = decode_png(&png);
    let ink = raster.pixels().iter().any(|pixel| pixel.alpha() > 0);
    assert!(ink, "the filled shape left the raster transparent");
    let svg = render_svg(&scene, &SvgOptions::new()).expect("writes");
    assert!(parse_svg(&svg).contains("<path"));
}

#[test]
fn a_zero_page_size_falls_back_to_letter() {
    let (mut doc, page, _shape) = document_with_shape();
    page_cells(
        &mut doc,
        page,
        vec![
            number_cell("PageWidth", 0.0),
            number_cell("PageHeight", -1.0),
        ],
    );
    let scene = build(&doc);
    assert_eq!((scene.width, scene.height), (8.5, 11.0));
}

#[test]
fn a_huge_page_is_capped_not_a_panic() {
    let (mut doc, page, shape) = document_with_shape();
    page_cells(
        &mut doc,
        page,
        vec![
            number_cell("PageWidth", 1.0e6),
            number_cell("PageHeight", 1.0e6),
        ],
    );
    with_geometry(
        &mut doc,
        shape,
        vec![grow("MoveTo", &[("X", 0.0), ("Y", 0.0)])],
    );
    let scene = build(&doc);
    assert_eq!((scene.width, scene.height), (1.0e6, 1.0e6));
    // The raster is over the dimension cap: a clean `TooLarge`.
    assert_eq!(render_png(&scene, &PngOptions::new()), Err(Error::TooLarge));
    // The SVG is text: it writes and parses at any page size.
    let svg = render_svg(&scene, &SvgOptions::new()).expect("writes");
    assert!(parse_svg(&svg).contains("<svg"));
}

#[test]
fn a_huge_scale_caps_the_raster_not_the_svg_text() {
    let (mut doc, _page, shape) = document_with_shape();
    with_geometry(
        &mut doc,
        shape,
        vec![grow("MoveTo", &[("X", 0.0), ("Y", 0.0)])],
    );
    let scene = build(&doc);
    let mut options = PngOptions::new();
    options.scale = 1.0e300;
    assert_eq!(render_png(&scene, &options), Err(Error::TooLarge));
    // The SVG is text and its byte cap bounds the output, so a huge
    // scale writes a valid document instead of erroring (the documented
    // boundary of `render_svg`).
    let svg_options = SvgOptions { scale: 1.0e300 };
    let svg = render_svg(&scene, &svg_options).expect("writes");
    parse_svg(&svg);
}

#[test]
fn text_overflows_its_box_and_still_lays_out() {
    let (mut doc, _page, shape) = document_with_shape();
    {
        let entity = doc.shape_mut(shape).expect("shape");
        entity.sheet.cells = vec![number_cell("Width", 0.5), number_cell("Height", 0.1)];
        let mut section = Section::new(SectionName::new("Character"));
        let mut row = Row::new();
        row.ix = Some(0);
        row.cells = vec![string_cell("Font", "Calibri"), number_cell("Size", 0.5)];
        section.rows.push(row);
        entity.sheet.sections.push(section);
        entity.text = Some(TextContent {
            parts: vec![TextPart::Text(
                "overflow overflow overflow overflow".to_owned(),
            )],
        });
    }
    with_geometry(
        &mut doc,
        shape,
        vec![grow("MoveTo", &[("X", 0.0), ("Y", 0.0)])],
    );
    let scene = build(&doc);
    let text = match scene.nodes.first() {
        Some(Node::Shape(shape)) => shape.text.as_ref().expect("text"),
        other => panic!("expected a shape with text, got {other:?}"),
    };
    assert!(text.layout.overflow, "{:?}", text.layout);
    assert!(!text.layout.lines.is_empty());
    let glyphs: usize = text.layout.lines.iter().map(|line| line.glyphs.len()).sum();
    assert!(glyphs > 0, "overflowing text still lays out its glyphs");
    // Both backends render it without a panic or an error.
    render_png(&scene, &PngOptions::new()).expect("renders");
    let svg = render_svg(&scene, &SvgOptions::new()).expect("writes");
    parse_svg(&svg);
}
