//! Reference resolution and inheritance over a small in-memory document
//! (task P6-05, ADR 0054).
//!
//! One test per row of the ADR 0054 reference-form table, plus the rules
//! around them: the `Inh` snapshot rule, an inherited formula binding
//! local names to the sheet it is evaluated for, `#REF!` with one warning
//! for a name that resolves to nothing, and case-insensitive lookup.

use ferrogram_geom::Unit;
use ferrogram_model::{
    Cell, CellLookup, CellName, CellValue, Color, Document, Master, MasterId, Page, PageId,
    Property, Section, SectionName, Shape, ShapeId, SheetRef, Style, StyleId, ValueSource,
};
use ferrogram_shapesheet::{
    CellRef, DocumentResolver, ErrorCode, EvalContext, EvalWarningKind, Value,
};

/// A cell with a cached number.
fn number_cell(name: &str, value: f64) -> Cell {
    let mut cell = Cell::new(CellName::new(name));
    cell.value = Some(CellValue::Number(value));
    cell.unit = Unit::Inches;
    cell
}

/// A cell with a cached number and the unit it was written with.
fn number_cell_in(name: &str, value: f64, unit: Unit) -> Cell {
    let mut cell = number_cell(name, value);
    cell.unit = unit;
    cell
}

/// A cell with a cached string.
fn str_cell(name: &str, value: &str) -> Cell {
    let mut cell = Cell::new(CellName::new(name));
    cell.value = Some(CellValue::Str(value.to_owned()));
    cell.unit = Unit::Unitsless;
    cell
}

/// A cell whose formula is the `Inh` inherit marker, with its snapshot.
fn inherit_cell(name: &str, snapshot: f64) -> Cell {
    let mut cell = number_cell(name, snapshot);
    cell.formula = Some(String::from("Inh"));
    cell
}

/// A cell with a formula and a cached number.
fn formula_cell(name: &str, formula: &str, value: f64) -> Cell {
    let mut cell = number_cell(name, value);
    cell.formula = Some(formula.to_owned());
    cell
}

/// A section named `name` with one row per entry of `rows`.
fn section(name: &str, rows: Vec<Vec<Cell>>) -> Section {
    let mut section = Section::new(SectionName::new(name));
    for (index, cells) in rows.into_iter().enumerate() {
        let mut row = ferrogram_model::Row::new();
        row.ix = Some(i32::try_from(index).unwrap_or(0));
        row.cells = cells;
        section.rows.push(row);
    }
    section
}

/// A section named `name` with named rows (`Property`, `User`).
fn named_section(name: &str, rows: Vec<(&str, Vec<Cell>)>) -> Section {
    let mut section = Section::new(SectionName::new(name));
    for (row_name, cells) in rows {
        let mut row = ferrogram_model::Row::new();
        row.name = Some(row_name.to_owned());
        row.cells = cells;
        section.rows.push(row);
    }
    section
}

/// The expected value of one cached cell (the mapping is P6-03's
/// `Value::from_cached`; the tests compare through it).
fn expected(value: CellValue, unit: Unit) -> Option<Value> {
    Value::from_cached(&value, unit).ok()
}

/// What `cell_value` reads back for one cached cell: the mapped value,
/// or the `#REF!` placeholder while the mapping is not implemented yet.
fn evaluated(value: CellValue, unit: Unit) -> Value {
    Value::from_cached(&value, unit).unwrap_or(Value::Err(ErrorCode::Ref))
}

/// A document with one page and one shape (`target`), and the helpers to
/// grow it.
struct Fixture {
    doc: Document,
    target: ShapeId,
}

impl Fixture {
    /// One shape on the page, with the given name and Visio `ID`.
    fn new() -> Self {
        let mut doc = Document::new();
        let target = doc.add_shape(Shape::new(ShapeId { index: 0 }, 100));
        let page = PageId { index: 0 };
        let mut page_entity = Page::new(page, 1);
        page_entity.shapes.push(target);
        doc.pages.push(page_entity);
        Self { doc, target }
    }

    /// Adds one shape to the page and returns its id.
    fn add_shape(&mut self, name: &str, visio_id: i32) -> ShapeId {
        let id = self
            .doc
            .add_shape(Shape::new(ShapeId { index: 0 }, visio_id));
        if let Some(shape) = self.doc.shape_mut(id) {
            shape.name = name.to_owned();
            shape.name_u = name.to_owned();
        }
        if let Some(page) = self.doc.pages.get_mut(0) {
            page.shapes.push(id);
        }
        id
    }

    /// Adds one master whose content shape is named `name`, and returns
    /// its id.
    fn add_master(&mut self, name: &str) -> (MasterId, ShapeId) {
        let content = self.doc.add_shape(Shape::new(ShapeId { index: 0 }, 1));
        let index = self.doc.masters.len();
        let mut master = Master::new(MasterId::from_index(index), 1);
        master.name = name.to_owned();
        master.name_u = name.to_owned();
        master.shapes.push(content);
        self.doc.masters.push(master);
        (MasterId::from_index(index), content)
    }

    /// Makes `shape` an instance of `master`.
    fn instance_of(&mut self, shape: ShapeId, master: MasterId) {
        if let Some(entity) = self.doc.shape_mut(shape) {
            entity.master = Some(master);
        }
    }
}

/// `MasterId` has no constructor beyond its field; this keeps the tests
/// readable.
trait FromIndex {
    fn from_index(index: usize) -> Self;
}

impl FromIndex for MasterId {
    fn from_index(index: usize) -> Self {
        MasterId {
            index: u32::try_from(index).unwrap_or(u32::MAX),
        }
    }
}

#[test]
fn a_bare_name_resolves_to_the_cell_on_the_target_sheet() {
    // `Width` -> the cell on the target sheet (ADR 0054 table).
    let mut fixture = Fixture::new();
    let cells = &mut fixture
        .doc
        .shape_mut(fixture.target)
        .expect("target")
        .sheet
        .cells;
    cells.push(number_cell("Width", 2.165354330708662));
    let mut resolver = DocumentResolver::new(&mut fixture.doc);
    let resolved = resolver
        .resolve(SheetRef::Shape(fixture.target), &CellRef::new("Width"))
        .expect("resolves");
    assert_eq!(resolved.source, ValueSource::Local);
    assert_eq!(resolved.formula, None);
    assert_eq!(
        resolved.value,
        expected(CellValue::Number(2.165354330708662), Unit::Inches)
    );
    assert!(resolver.warnings().is_empty());
}

#[test]
fn a_named_shape_resolves_on_the_same_page() {
    // `Star!Angle` -> the named shape's cell on the same page.
    let mut fixture = Fixture::new();
    let star = fixture.add_shape("Star", 2);
    fixture
        .doc
        .shape_mut(star)
        .expect("star")
        .sheet
        .cells
        .push(number_cell_in("Angle", 0.5, Unit::Radians));
    let mut resolver = DocumentResolver::new(&mut fixture.doc);
    let reference = CellRef {
        sheets: vec![String::from("Star")],
        section: None,
        names: vec![String::from("Angle")],
    };
    let resolved = resolver
        .resolve(SheetRef::Shape(fixture.target), &reference)
        .expect("resolves");
    assert_eq!(resolved.source, ValueSource::Local);
    assert_eq!(
        resolved.value,
        expected(CellValue::Number(0.5), Unit::Radians)
    );
}

#[test]
fn a_named_shape_with_an_id_picks_that_shape() {
    // `Executive.2!Height` -> the shape named `Executive` with `ID` 2.
    let mut fixture = Fixture::new();
    let other = fixture.add_shape("Executive", 5);
    let wanted = fixture.add_shape("Executive", 2);
    fixture
        .doc
        .shape_mut(other)
        .expect("other")
        .sheet
        .cells
        .push(number_cell("Height", 9.0));
    fixture
        .doc
        .shape_mut(wanted)
        .expect("wanted")
        .sheet
        .cells
        .push(number_cell("Height", 3.0));
    let mut resolver = DocumentResolver::new(&mut fixture.doc);
    let reference = CellRef {
        sheets: vec![String::from("Executive.2")],
        section: None,
        names: vec![String::from("Height")],
    };
    let resolved = resolver
        .resolve(SheetRef::Shape(fixture.target), &reference)
        .expect("resolves");
    assert_eq!(
        resolved.value,
        expected(CellValue::Number(3.0), Unit::Inches)
    );
}

#[test]
fn a_column_of_the_section_row_shape_resolves_with_its_row_index() {
    // `Char.Size` -> `Character`, `Size` column, row 0;
    // `Char.Font[3]` -> `Character`, `Font` column, row `IX`/position 3.
    let mut fixture = Fixture::new();
    let character = section(
        "Character",
        vec![
            vec![number_cell("Size", 12.0), number_cell("Font", 4.0)],
            vec![number_cell("Size", 13.0), number_cell("Font", 5.0)],
            vec![number_cell("Size", 14.0), number_cell("Font", 6.0)],
            vec![number_cell("Size", 15.0), number_cell("Font", 7.0)],
        ],
    );
    fixture
        .doc
        .shape_mut(fixture.target)
        .expect("target")
        .sheet
        .sections
        .push(character);
    let mut resolver = DocumentResolver::new(&mut fixture.doc);
    for (name, expected_value) in [("Size", 12.0), ("Font[3]", 7.0)] {
        let reference = CellRef {
            sheets: Vec::new(),
            section: Some(String::from("Char")),
            names: vec![name.to_owned()],
        };
        let resolved = resolver
            .resolve(SheetRef::Shape(fixture.target), &reference)
            .expect("resolves");
        assert_eq!(resolved.source, ValueSource::Local);
        assert_eq!(
            resolved.value,
            expected(CellValue::Number(expected_value), Unit::Inches)
        );
    }
    assert!(resolver.warnings().is_empty());
}

#[test]
fn a_scratch_row_resolves_by_column_and_row_number() {
    // `Scratch.A5` -> `Scratch`, `A` column, row 5.
    let mut fixture = Fixture::new();
    let rows: Vec<Vec<Cell>> = (0..6)
        .map(|index| vec![number_cell("A", f64::from(index) + 0.25)])
        .collect();
    fixture
        .doc
        .shape_mut(fixture.target)
        .expect("target")
        .sheet
        .sections
        .push(section("Scratch", rows));
    let mut resolver = DocumentResolver::new(&mut fixture.doc);
    let reference = CellRef {
        sheets: Vec::new(),
        section: Some(String::from("Scratch")),
        names: vec![String::from("A5")],
    };
    let resolved = resolver
        .resolve(SheetRef::Shape(fixture.target), &reference)
        .expect("resolves");
    assert_eq!(
        resolved.value,
        expected(CellValue::Number(5.25), Unit::Inches)
    );
}

#[test]
fn a_named_row_resolves_to_its_default_cell() {
    // `Prop.Cost`, `User.Name` -> the named row's default cell (`Value`).
    let mut fixture = Fixture::new();
    let property = named_section(
        "Property",
        vec![(
            "Cost",
            vec![str_cell("Value", "100"), number_cell("Prompt", 0.0)],
        )],
    );
    let user = named_section("User", vec![("Name", vec![str_cell("Value", "Dara")])]);
    {
        let sheet = &mut fixture.doc.shape_mut(fixture.target).expect("target").sheet;
        sheet.sections.push(property);
        sheet.sections.push(user);
    }
    let mut resolver = DocumentResolver::new(&mut fixture.doc);
    for (section_name, row_name, expected_value) in
        [("Prop", "Cost", "100"), ("User", "Name", "Dara")]
    {
        let reference = CellRef {
            sheets: Vec::new(),
            section: Some(section_name.to_owned()),
            names: vec![row_name.to_owned()],
        };
        let resolved = resolver
            .resolve(SheetRef::Shape(fixture.target), &reference)
            .expect("resolves");
        assert_eq!(resolved.source, ValueSource::Local);
        assert_eq!(
            resolved.value,
            expected(CellValue::Str(expected_value.to_owned()), Unit::Unitsless)
        );
    }
    // An explicit cell name of the named row reads that cell.
    let reference = CellRef {
        sheets: Vec::new(),
        section: Some(String::from("Prop")),
        names: vec![String::from("Cost"), String::from("Prompt")],
    };
    let resolved = resolver
        .resolve(SheetRef::Shape(fixture.target), &reference)
        .expect("resolves");
    assert_eq!(
        resolved.value,
        expected(CellValue::Number(0.0), Unit::Inches)
    );
}

#[test]
fn a_numbered_section_names_the_section_instance() {
    // `Geometry1.X2` -> the **first** `Geometry` section, the row with
    // `IX` 2, `X` cell (ADR 0054's table; the corpus' cached values pin
    // both: `Geometry1.X1` caches the first section's `IX` 1 row).
    let mut fixture = Fixture::new();
    let first = section(
        "Geometry",
        vec![
            vec![number_cell("X", 1.0)],
            vec![number_cell("X", 1.5)],
            vec![number_cell("X", 2.0)],
        ],
    );
    let second = section(
        "Geometry",
        vec![
            vec![number_cell("X", 10.0)],
            vec![number_cell("X", 10.5)],
            vec![number_cell("X", 11.0)],
        ],
    );
    {
        let sheet = &mut fixture.doc.shape_mut(fixture.target).expect("target").sheet;
        for mut section in [first, second] {
            for (position, row) in section.rows.iter_mut().enumerate() {
                row.ix = Some(i32::try_from(position.saturating_add(1)).unwrap_or(i32::MAX));
            }
            sheet.sections.push(section);
        }
    }
    let mut resolver = DocumentResolver::new(&mut fixture.doc);
    for (section_name, row_name, expected_value) in [
        ("Geometry", "X2", 1.5),
        ("Geometry1", "X2", 1.5),
        ("Geometry2", "X2", 10.5),
        ("Geometry2", "X3", 11.0),
    ] {
        let reference = CellRef {
            sheets: Vec::new(),
            section: Some(section_name.to_owned()),
            names: vec![String::from(row_name)],
        };
        let resolved = resolver
            .resolve(SheetRef::Shape(fixture.target), &reference)
            .expect("resolves");
        assert_eq!(
            resolved.value,
            expected(CellValue::Number(expected_value), Unit::Inches),
            "{section_name}.{row_name}"
        );
    }
}

#[test]
fn a_shape_id_on_the_targets_page_resolves() {
    // `Sheet.8!FillForegnd` -> the shape with `ID` 8 on the target's page.
    let mut fixture = Fixture::new();
    let shape = fixture.add_shape("Other", 8);
    let mut cell = Cell::new(CellName::new("FillForegnd"));
    cell.value = Some(CellValue::Color(Color::new(1, 2, 3)));
    fixture
        .doc
        .shape_mut(shape)
        .expect("shape")
        .sheet
        .cells
        .push(cell);
    let mut resolver = DocumentResolver::new(&mut fixture.doc);
    let reference = CellRef {
        sheets: vec![String::from("Sheet.8")],
        section: None,
        names: vec![String::from("FillForegnd")],
    };
    let resolved = resolver
        .resolve(SheetRef::Shape(fixture.target), &reference)
        .expect("resolves");
    assert_eq!(
        resolved.value,
        expected(CellValue::Color(Color::new(1, 2, 3)), Unit::Unitsless)
    );
}

#[test]
fn a_masters_named_sub_shape_resolves() {
    // `Masters[Gear]!Shaft!Geometry1.X1` -> a master's named sub-shape's
    // cell.
    let mut fixture = Fixture::new();
    let (gear, content) = fixture.add_master("Gear");
    let shaft = fixture.doc.add_shape(Shape::new(ShapeId { index: 0 }, 2));
    if let Some(entity) = fixture.doc.shape_mut(shaft) {
        entity.name = String::from("Shaft");
        entity.name_u = String::from("Shaft");
        entity.parent = Some(content);
    }
    if let Some(entity) = fixture.doc.shape_mut(content) {
        entity.children.push(shaft);
    }
    let first = section("Geometry", vec![vec![number_cell("X", 0.25)]]);
    let second = section(
        "Geometry",
        vec![vec![number_cell("X", 0.5)], vec![number_cell("X", 0.75)]],
    );
    {
        let sheet = &mut fixture.doc.shape_mut(shaft).expect("shaft").sheet;
        for mut section in [first, second] {
            for (position, row) in section.rows.iter_mut().enumerate() {
                row.ix = Some(i32::try_from(position.saturating_add(1)).unwrap_or(i32::MAX));
            }
            sheet.sections.push(section);
        }
    }
    let mut resolver = DocumentResolver::new(&mut fixture.doc);
    let reference = CellRef {
        sheets: vec![String::from("Masters[Gear]"), String::from("Shaft")],
        section: Some(String::from("Geometry2")),
        names: vec![String::from("X2")],
    };
    let resolved = resolver
        .resolve(SheetRef::Shape(fixture.target), &reference)
        .expect("resolves");
    assert_eq!(resolved.source, ValueSource::Local);
    assert_eq!(
        resolved.value,
        expected(CellValue::Number(0.75), Unit::Inches)
    );
    // The master is reachable by name only; a name that is not there is
    // `#REF!`.
    assert_eq!(gear.index, 0);
}

#[test]
fn a_pages_shape_resolves() {
    // `Pages[Page-3]!Sheet.4!BeginX` -> a page's shape's cell.
    let mut fixture = Fixture::new();
    let page_id = PageId { index: 1 };
    let shape = fixture.doc.add_shape(Shape::new(ShapeId { index: 0 }, 4));
    fixture
        .doc
        .shape_mut(shape)
        .expect("shape")
        .sheet
        .cells
        .push(number_cell("BeginX", 6.5));
    let mut page = Page::new(page_id, 3);
    page.name = String::from("Page-3");
    page.name_u = String::from("Page-3");
    page.shapes.push(shape);
    fixture.doc.pages.push(page);
    let mut resolver = DocumentResolver::new(&mut fixture.doc);
    let reference = CellRef {
        sheets: vec![String::from("Pages[Page-3]"), String::from("Sheet.4")],
        section: None,
        names: vec![String::from("BeginX")],
    };
    let resolved = resolver
        .resolve(SheetRef::Shape(fixture.target), &reference)
        .expect("resolves");
    assert_eq!(
        resolved.value,
        expected(CellValue::Number(6.5), Unit::Inches)
    );
}

#[test]
fn a_stylesheet_rows_cell_resolves() {
    // `Styles!Manager!LineColor` -> a stylesheet row's cell.
    let mut fixture = Fixture::new();
    let mut style = Style::new(StyleId { index: 0 });
    style.visio_id = 1;
    style.name = String::from("Manager");
    style.name_u = String::from("Manager");
    style.sheet.cells.push(number_cell("LineColor", 0.125));
    fixture.doc.stylesheet.styles.push(style);
    let mut resolver = DocumentResolver::new(&mut fixture.doc);
    let reference = CellRef {
        sheets: vec![String::from("Styles"), String::from("Manager")],
        section: None,
        names: vec![String::from("LineColor")],
    };
    let resolved = resolver
        .resolve(SheetRef::Shape(fixture.target), &reference)
        .expect("resolves");
    assert_eq!(resolved.source, ValueSource::Local);
    assert_eq!(
        resolved.value,
        expected(CellValue::Number(0.125), Unit::Inches)
    );
}

#[test]
fn the_page_sheet_and_the_document_sheet_resolve() {
    // `ThePage!User.Vanishing_Point` -> the page sheet;
    // `TheDoc!PreviewQuality` -> the document sheet.
    let mut fixture = Fixture::new();
    let user = named_section(
        "User",
        vec![("Vanishing_Point", vec![number_cell("Value", 1.5)])],
    );
    if let Some(page) = fixture.doc.pages.get_mut(0) {
        page.sheet.sections.push(user);
    }
    fixture
        .doc
        .properties
        .push(Property::new("PreviewQuality", "2"));
    let mut resolver = DocumentResolver::new(&mut fixture.doc);

    let page_reference = CellRef {
        sheets: vec![String::from("ThePage")],
        section: Some(String::from("User")),
        names: vec![String::from("Vanishing_Point")],
    };
    let resolved = resolver
        .resolve(SheetRef::Shape(fixture.target), &page_reference)
        .expect("resolves");
    assert_eq!(
        resolved.value,
        expected(CellValue::Number(1.5), Unit::Inches)
    );

    let doc_reference = CellRef {
        sheets: vec![String::from("TheDoc")],
        section: None,
        names: vec![String::from("PreviewQuality")],
    };
    let resolved = resolver
        .resolve(SheetRef::Shape(fixture.target), &doc_reference)
        .expect("resolves");
    assert_eq!(resolved.source, ValueSource::Local);
    assert_eq!(
        resolved.value,
        expected(CellValue::Str(String::from("2")), Unit::Unitsless)
    );
}

#[test]
fn a_quoted_nonstandard_name_resolves_with_its_quote_escape() {
    // `'1-D'!LineColor` -> a nonstandard name; `''` is the quote escape.
    let mut fixture = Fixture::new();
    let odd = fixture.add_shape("1-D", 3);
    fixture
        .doc
        .shape_mut(odd)
        .expect("odd")
        .sheet
        .cells
        .push(number_cell("LineColor", 0.75));
    let escaped = fixture.add_shape("A'B", 4);
    fixture
        .doc
        .shape_mut(escaped)
        .expect("escaped")
        .sheet
        .cells
        .push(number_cell("LineColor", 0.25));
    let mut resolver = DocumentResolver::new(&mut fixture.doc);

    let quoted = CellRef {
        sheets: vec![String::from("'1-D'")],
        section: None,
        names: vec![String::from("LineColor")],
    };
    let resolved = resolver
        .resolve(SheetRef::Shape(fixture.target), &quoted)
        .expect("resolves");
    assert_eq!(
        resolved.value,
        expected(CellValue::Number(0.75), Unit::Inches)
    );

    let escaped_ref = CellRef {
        sheets: vec![String::from("'A''B'")],
        section: None,
        names: vec![String::from("LineColor")],
    };
    let resolved = resolver
        .resolve(SheetRef::Shape(fixture.target), &escaped_ref)
        .expect("resolves");
    assert_eq!(
        resolved.value,
        expected(CellValue::Number(0.25), Unit::Inches)
    );
}

#[test]
fn the_inh_snapshot_is_the_value_and_the_master_still_is_the_source() {
    // Acceptance (ADR 0054 amendment): a cell with `F='Inh'` and a cached
    // value returns that value while `Document::resolve` still reports
    // `Master(..)` as the source.
    let mut fixture = Fixture::new();
    let (master, content) = fixture.add_master("Box");
    fixture.instance_of(fixture.target, master);
    fixture
        .doc
        .shape_mut(content)
        .expect("content")
        .sheet
        .cells
        .push(formula_cell("PinX", "0.98", 0.98));
    fixture
        .doc
        .shape_mut(fixture.target)
        .expect("target")
        .sheet
        .cells
        .push(inherit_cell("PinX", 2.165354330708662));

    let mut resolver = DocumentResolver::new(&mut fixture.doc);
    let resolved = resolver
        .resolve(SheetRef::Shape(fixture.target), &CellRef::new("PinX"))
        .expect("resolves");
    assert_eq!(
        resolved.value,
        expected(CellValue::Number(2.165354330708662), Unit::Inches)
    );
    // The formula comes from the rung that defines it: the master's.
    assert_eq!(resolved.formula.as_deref(), Some("0.98"));
    assert_eq!(resolved.source, ValueSource::Master(master));

    // The provenance the model reports is unchanged (the pinned rule).
    let lookup = fixture
        .doc
        .resolve(fixture.target, &CellName::from_static("PinX"))
        .expect("resolves");
    assert_eq!(lookup.source, ValueSource::Master(master));
}

#[test]
fn an_inh_snapshot_without_any_rung_is_the_value() {
    let mut fixture = Fixture::new();
    fixture
        .doc
        .shape_mut(fixture.target)
        .expect("target")
        .sheet
        .cells
        .push(inherit_cell("LineWeight", 0.02));
    let mut resolver = DocumentResolver::new(&mut fixture.doc);
    let resolved = resolver
        .resolve(SheetRef::Shape(fixture.target), &CellRef::new("LineWeight"))
        .expect("resolves");
    assert_eq!(
        resolved.value,
        expected(CellValue::Number(0.02), Unit::Inches)
    );
    assert!(resolver.warnings().is_empty());
}

#[test]
fn an_inherited_geometry_row_reads_the_instances_width() {
    // Acceptance: a master's formula evaluated for an instance reads the
    // **instance's** `Width`/`Height`. The formula a master's geometry
    // row defines binds its local names to the sheet it is evaluated for,
    // so `Width` in it is the instance's width (the resize-recompute
    // foundation).
    let mut fixture = Fixture::new();
    let (master, content) = fixture.add_master("Box");
    fixture.instance_of(fixture.target, master);
    // The master: `Width` 0.98, geometry row 0 `X = Width*0.5`.
    {
        let sheet = &mut fixture.doc.shape_mut(content).expect("content").sheet;
        sheet.cells.push(number_cell("Width", 0.98));
        sheet.sections.push(section(
            "Geometry",
            vec![vec![formula_cell("X", "Width*0.5", 0.49)]],
        ));
    }
    // The instance: `Width` 2.16, the row `F='Inh'` with its own snapshot.
    {
        let sheet = &mut fixture.doc.shape_mut(fixture.target).expect("target").sheet;
        sheet.cells.push(number_cell("Width", 2.16));
        sheet
            .sections
            .push(section("Geometry", vec![vec![inherit_cell("X", 1.08)]]));
    }
    let mut resolver = DocumentResolver::new(&mut fixture.doc);
    let sheet = SheetRef::Shape(fixture.target);

    // The row's formula is the master's, evaluated for the instance.
    let row = CellRef {
        sheets: Vec::new(),
        section: Some(String::from("Geometry")),
        names: vec![String::from("X0")],
    };
    let resolved = resolver.resolve(sheet, &row).expect("resolves");
    assert_eq!(resolved.formula.as_deref(), Some("Width*0.5"));
    assert_eq!(resolved.source, ValueSource::Master(master));

    // `Width` binds to the instance's sheet, not to the master's.
    let width = resolver
        .cell_value(sheet, &CellRef::new("Width"))
        .expect("value");
    assert_eq!(width, evaluated(CellValue::Number(2.16), Unit::Inches));
    let master_width = resolver
        .cell_value(SheetRef::Shape(content), &CellRef::new("Width"))
        .expect("value");
    assert_eq!(
        master_width,
        evaluated(CellValue::Number(0.98), Unit::Inches)
    );
}

#[test]
fn a_missing_cell_row_or_sheet_is_ref_with_one_warning() {
    // Acceptance: a missing cell, row or sheet produces `#REF!` plus
    // exactly one `MissingReference` warning (no document failure). The
    // evaluator's warning kind spells that `UnsupportedReference`.
    let mut fixture = Fixture::new();
    fixture
        .doc
        .shape_mut(fixture.target)
        .expect("target")
        .sheet
        .sections
        .push(section("Geometry", vec![vec![number_cell("X", 1.0)]]));
    fixture
        .doc
        .shape_mut(fixture.target)
        .expect("target")
        .sheet
        .sections
        .push(named_section(
            "Property",
            vec![("Cost", vec![number_cell("Value", 1.0)])],
        ));
    let mut resolver = DocumentResolver::new(&mut fixture.doc);
    let sheet = SheetRef::Shape(fixture.target);

    let missing_sheet = CellRef {
        sheets: vec![String::from("Sheet.9")],
        section: None,
        names: vec![String::from("PinX")],
    };
    let missing_cell = CellRef::new("NoSuchCell");
    let missing_row = CellRef {
        sheets: Vec::new(),
        section: Some(String::from("Prop")),
        names: vec![String::from("NoSuchRow")],
    };
    let missing_indexed_row = CellRef {
        sheets: Vec::new(),
        section: Some(String::from("Geometry")),
        names: vec![String::from("X9")],
    };

    for (index, reference) in [
        &missing_sheet,
        &missing_cell,
        &missing_row,
        &missing_indexed_row,
    ]
    .into_iter()
    .enumerate()
    {
        let before = resolver.warnings().len();
        let resolved = resolver.resolve(sheet, reference).expect("resolves");
        assert_eq!(resolved.value, Some(Value::Err(ErrorCode::Ref)));
        assert_eq!(resolved.formula, None);
        // Exactly one warning per missing name, and the document is fine.
        assert_eq!(resolver.warnings().len(), before + 1, "case {index}");
    }
    let warnings = resolver.warnings();
    assert_eq!(warnings.len(), 4);
    for warning in warnings {
        assert!(matches!(
            warning.kind,
            EvalWarningKind::UnsupportedReference(_)
        ));
    }
}

#[test]
fn a_missing_reference_read_as_a_value_is_ref_with_one_warning() {
    let mut fixture = Fixture::new();
    let mut resolver = DocumentResolver::new(&mut fixture.doc);
    let value = resolver
        .cell_value(SheetRef::Shape(fixture.target), &CellRef::new("NoSuchCell"))
        .expect("value");
    assert_eq!(value, Value::Err(ErrorCode::Ref));
    assert_eq!(resolver.warnings().len(), 1);
}

#[test]
fn names_match_case_insensitively() {
    // Acceptance: name lookup is case-insensitive; a case-only difference
    // resolves.
    let mut fixture = Fixture::new();
    let star = fixture.add_shape("Star", 2);
    fixture
        .doc
        .shape_mut(star)
        .expect("star")
        .sheet
        .cells
        .push(number_cell("Angle", 0.5));
    fixture
        .doc
        .shape_mut(fixture.target)
        .expect("target")
        .sheet
        .cells
        .push(number_cell("Width", 2.0));
    fixture
        .doc
        .shape_mut(fixture.target)
        .expect("target")
        .sheet
        .sections
        .push(section("Character", vec![vec![number_cell("Size", 12.0)]]));
    let mut resolver = DocumentResolver::new(&mut fixture.doc);
    let sheet = SheetRef::Shape(fixture.target);

    let lower_shape = CellRef {
        sheets: vec![String::from("star")],
        section: None,
        names: vec![String::from("angle")],
    };
    let resolved = resolver.resolve(sheet, &lower_shape).expect("resolves");
    assert_eq!(
        resolved.value,
        expected(CellValue::Number(0.5), Unit::Inches)
    );

    let upper_cell = CellRef::new("width");
    let resolved = resolver.resolve(sheet, &upper_cell).expect("resolves");
    assert_eq!(
        resolved.value,
        expected(CellValue::Number(2.0), Unit::Inches)
    );

    let mixed_section = CellRef {
        sheets: Vec::new(),
        section: Some(String::from("CHAR")),
        names: vec![String::from("size")],
    };
    let resolved = resolver.resolve(sheet, &mixed_section).expect("resolves");
    assert_eq!(
        resolved.value,
        expected(CellValue::Number(12.0), Unit::Inches)
    );
    assert!(resolver.warnings().is_empty());
}

#[test]
fn a_setf_write_lands_on_the_local_cell_or_is_refused() {
    let mut fixture = Fixture::new();
    fixture
        .doc
        .shape_mut(fixture.target)
        .expect("target")
        .sheet
        .cells
        .push(number_cell("PinX", 1.0));
    let mut resolver = DocumentResolver::new(&mut fixture.doc);
    let sheet = SheetRef::Shape(fixture.target);
    let value = Value::Number(ferrogram_shapesheet::Number::new(
        5.0,
        ferrogram_shapesheet::Unit::Inches,
    ));

    // A missing target is refused (the evaluator turns the refusal into
    // `#REF!`).
    let missing = resolver.set_cell(sheet, &CellRef::new("NoSuchCell"), value.clone());
    assert!(missing.is_err());

    // A write goes through once the value mapping (P6-03) does; before
    // that the mapping itself is the error.
    let write = resolver.set_cell(sheet, &CellRef::new("PinX"), value.clone());
    match value.to_cached() {
        Ok(cached) => {
            write.expect("writes");
            let cell = resolver
                .document()
                .shape(fixture.target)
                .expect("target")
                .sheet
                .cells
                .first()
                .expect("cell");
            assert_eq!(cell.value, Some(cached.value));
            assert_eq!(cell.formula, None);
        }
        Err(_) => assert!(write.is_err()),
    }
}
