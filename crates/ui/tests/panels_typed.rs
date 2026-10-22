//! The typed Shape Data and Size & Position panels (task P6-09, ADR 0058).
//!
//! The panels are drawn in real `egui_kittest` frames (ADR 0060) and the
//! typed fields are driven the way a user drives them: focus, type,
//! Enter. What the criteria name is proven here:
//!
//! - Shape Data shows each `Prop` row's value typed — a number with its
//!   unit in `Settings::units`, a boolean, a colour, a string — one test
//!   per kind.
//! - Size & Position shows the evaluated `PinX`/`PinY`/`Width`/`Height`/
//!   `Angle` and marks the cells that carry a formula read-only.
//! - Entering `2.5 cm` into an inch field stores the converted value;
//!   `37%`, `45 deg` and `5 1/2` parse like the grammar (ADR 0051); a bad
//!   entry is refused with one status line and no model change.
//! - A formula cell refuses edits (one status line) and the document is
//!   unchanged.
//! - Every accepted edit dispatches one registry command and is one undo
//!   step (the panel never touches the model directly).

// The never-crash lints apply to non-test code only (clippy.toml).
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::cell::RefCell;
use std::rc::Rc;

use egui::accesskit::Role;
use egui_kittest::{
    Harness,
    kittest::{NodeT, Queryable},
};
use ferrogram_commands::app::{AppCtx, DocumentSession, Settings};
use ferrogram_commands::{DisplayUnits, dispatch};
use ferrogram_geom::Unit as GeomUnit;
use ferrogram_model::{
    Cell, CellName, CellValue, Color, Document, Page, PageId, Row, Section, SectionName, Shape,
    ShapeId,
};
use ferrogram_ui::Intent;
use ferrogram_ui::panels::{
    shape_data_rows, show_shape_data, show_size_position, size_position_rows,
};

/// The unit under test: a text field of one of the two panels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Panel {
    /// The Shape Data panel.
    ShapeData,
    /// The Size & Position panel.
    SizePosition,
}

/// One panel driven headlessly (the P4-04 rig pattern).
struct PanelRig {
    /// The application state under test.
    app: Rc<RefCell<AppCtx>>,
    /// The intents the panel produced since the last take.
    intents: Rc<RefCell<Vec<Intent>>>,
    /// The harness drawing the panel every frame.
    harness: Harness<'static>,
}

impl PanelRig {
    /// A rig over one app state, drawing `panel` every frame.
    fn new(app: AppCtx, panel: Panel) -> Self {
        let app = Rc::new(RefCell::new(app));
        let intents = Rc::new(RefCell::new(Vec::new()));
        let harness_app = Rc::clone(&app);
        let harness_intents = Rc::clone(&intents);
        let harness = Harness::builder()
            .with_size(egui::vec2(600.0, 900.0))
            .build_ui(move |ui| {
                let mut app = harness_app.borrow_mut();
                let produced = match panel {
                    Panel::ShapeData => show_shape_data(ui, &mut app),
                    Panel::SizePosition => show_size_position(ui, &mut app),
                }
                .expect("the panel renders");
                drop(app);
                harness_intents.borrow_mut().extend(produced);
            });
        let mut rig = Self {
            app,
            intents,
            harness,
        };
        rig.harness.run();
        rig
    }

    /// The intents produced since the last take.
    fn take_intents(&mut self) -> Vec<Intent> {
        std::mem::take(&mut *self.intents.borrow_mut())
    }

    /// Types `text` into the `field`th text field and commits with
    /// Enter, the way a user edits a panel field.
    fn type_entry(&mut self, field: usize, text: &str) -> Vec<Intent> {
        self.harness
            .query_all_by_role(Role::TextInput)
            .nth(field)
            .expect("the field exists")
            .focus();
        self.harness.run();
        self.harness
            .key_combination_modifiers(egui::Modifiers::COMMAND, &[egui::Key::A]);
        self.harness.run();
        self.harness
            .query_all_by_role(Role::TextInput)
            .nth(field)
            .expect("the field exists")
            .type_text(text);
        self.harness.run();
        self.harness.key_press(egui::Key::Enter);
        self.harness.run();
        self.take_intents()
    }

    /// Dispatches the intents the way the shell does (one registry
    /// command each, one undo step).
    fn dispatch(&mut self, intents: Vec<Intent>) {
        let registry = ferrogram_commands::builtins::registry().expect("the registry builds");
        for intent in intents {
            let mut app = self.app.borrow_mut();
            dispatch(&registry, &mut app, &intent.command, intent.invocation)
                .expect("the command runs");
        }
    }
}

/// The texts a frame shows: every accessible name and value.
fn shown(harness: &Harness<'_>) -> Vec<String> {
    let root = harness.root();
    let mut out = Vec::new();
    for node in root.children_recursive() {
        let data = node.accesskit_node().data();
        if let Some(label) = data.label()
            && !label.is_empty()
        {
            out.push(label.to_owned());
        }
        if let Some(value) = data.value()
            && !value.is_empty()
        {
            out.push(value.to_owned());
        }
    }
    out
}

/// Whether the frame shows a check box labelled `label`.
fn check_box(harness: &Harness<'_>, label: &str) -> bool {
    let root = harness.root();
    root.children_recursive().any(|node| {
        let data = node.accesskit_node().data();
        data.role() == Role::CheckBox && data.label() == Some(label)
    })
}

/// A page with an 8.5 x 11 in sheet.
fn page() -> Page {
    let mut page = Page::new(PageId { index: 0 }, 0);
    page.name = String::from("Page-1");
    page.sheet.cells = vec![cell("PageWidth", 8.5), cell("PageHeight", 11.0)];
    page
}

/// A cell with a cached number and no formula.
fn cell(name: &str, value: f64) -> Cell {
    let mut cell = Cell::new(CellName::new(name));
    cell.value = Some(CellValue::Number(value));
    cell
}

/// A cell with a cached boolean.
fn flag_cell(name: &str, flag: bool) -> Cell {
    let mut cell = Cell::new(CellName::new(name));
    cell.value = Some(CellValue::Bool(flag));
    cell
}

/// A `Value` cell with a cached value, its unit and no formula.
fn value_cell(value: CellValue, unit: GeomUnit) -> Cell {
    let mut cell = Cell::new(CellName::new("Value"));
    cell.value = Some(value);
    cell.unit = unit;
    cell
}

/// One `Prop` row with the given `Label` and `Value` cells.
fn prop_row(label: &str, value: CellValue, unit: GeomUnit) -> Row {
    let mut row = Row::new();
    row.name = Some(format!("Row_{label}"));
    let mut name = Cell::new(CellName::new("Label"));
    name.value = Some(CellValue::Str(label.to_owned()));
    row.cells.push(name);
    row.cells.push(value_cell(value, unit));
    row
}

/// The nine Size & Position cells of a 2x1 shape at (3, 4), turned 30°.
fn geometry_cells() -> Vec<Cell> {
    vec![
        cell("Width", 2.0),
        cell("Height", 1.0),
        cell("PinX", 3.0),
        cell("PinY", 4.0),
        cell("Angle", core::f64::consts::FRAC_PI_6),
        cell("LocPinX", 1.0),
        cell("LocPinY", 0.5),
        flag_cell("FlipX", false),
        flag_cell("FlipY", false),
    ]
}

/// A selected shape with the given cells and `Prop` rows.
fn fixture(cells: Vec<Cell>, props: Vec<Row>, units: DisplayUnits) -> (AppCtx, ShapeId) {
    let mut doc = Document::new();
    let mut shape = Shape::new(ShapeId { index: 0 }, 7);
    shape.name = String::from("Box");
    shape.sheet.cells = cells;
    if !props.is_empty() {
        let mut section = Section::new(SectionName::new("Property"));
        section.rows = props;
        shape.sheet.sections.push(section);
    }
    let id = doc.add_shape(shape);
    let mut page = page();
    page.shapes.push(id);
    doc.pages.push(page);
    let mut settings = Settings::default();
    settings.units = units;
    let mut app = AppCtx::new(settings);
    app.open_document(DocumentSession::new(doc, None));
    app.selection.shapes = vec![id];
    app.selection.anchor = Some(id);
    app.selection.page = Some(PageId { index: 0 });
    (app, id)
}

/// The cached value of one cell of `shape`.
fn cached(app: &AppCtx, shape: ShapeId, cell: &str) -> Option<CellValue> {
    app.active_document()
        .and_then(|session| session.document.shape(shape))
        .and_then(|shape| {
            shape
                .sheet
                .cells
                .iter()
                .find(|entry| entry.name.as_str() == cell)
                .and_then(|entry| entry.value.clone())
        })
}

/// The undo steps of the open document.
fn undo_len(app: &AppCtx) -> usize {
    app.active_document().expect("document").undo.len()
}

#[test]
fn shape_data_shows_a_number_with_its_unit() {
    let (app, _id) = fixture(
        geometry_cells(),
        vec![prop_row("Cost", CellValue::Number(2.0), GeomUnit::Inches)],
        DisplayUnits::Millimeters,
    );
    let rig = PanelRig::new(app, Panel::ShapeData);
    let texts = shown(&rig.harness);
    assert!(
        texts.iter().any(|text| text == "50.8 mm"),
        "the number shows in Settings::units: {texts:?}"
    );
}

#[test]
fn shape_data_shows_a_boolean_as_a_check_box() {
    let (app, _id) = fixture(
        geometry_cells(),
        vec![prop_row(
            "Rounded",
            CellValue::Bool(true),
            GeomUnit::Unitsless,
        )],
        DisplayUnits::Inches,
    );
    let rig = PanelRig::new(app, Panel::ShapeData);
    assert!(
        check_box(&rig.harness, "TRUE"),
        "the boolean shows as a check box: {:?}",
        shown(&rig.harness)
    );
}

#[test]
fn shape_data_shows_a_colour_as_its_rgb_value() {
    let (app, _id) = fixture(
        geometry_cells(),
        vec![prop_row(
            "Tint",
            CellValue::Color(Color::new(255, 0, 0)),
            GeomUnit::Unitsless,
        )],
        DisplayUnits::Inches,
    );
    let rig = PanelRig::new(app, Panel::ShapeData);
    let texts = shown(&rig.harness);
    assert!(
        texts.iter().any(|text| text == "#FF0000"),
        "the colour shows as its RGB value: {texts:?}"
    );
}

#[test]
fn shape_data_shows_a_string_as_written() {
    let (app, _id) = fixture(
        geometry_cells(),
        vec![prop_row(
            "Note",
            CellValue::Str(String::from("hello")),
            GeomUnit::Unitsless,
        )],
        DisplayUnits::Inches,
    );
    let rig = PanelRig::new(app, Panel::ShapeData);
    let texts = shown(&rig.harness);
    assert!(
        texts.iter().any(|text| text == "hello"),
        "the string shows as written: {texts:?}"
    );
}

#[test]
fn size_position_shows_the_evaluated_cells() {
    let (app, _id) = fixture(geometry_cells(), Vec::new(), DisplayUnits::Inches);
    let rig = PanelRig::new(app, Panel::SizePosition);
    let texts = shown(&rig.harness);
    for expected in ["2 in", "1 in", "3 in", "4 in", "30°"] {
        assert!(
            texts.iter().any(|text| text == expected),
            "the evaluated {expected} shows: {texts:?}"
        );
    }
}

#[test]
fn a_formula_cell_is_marked_read_only() {
    let mut cells = geometry_cells();
    for cell in &mut cells {
        if cell.name.as_str() == "Width" {
            cell.set_formula(Some(String::from("GUARD(2 in)")));
        }
    }
    let (mut app, _id) = fixture(cells, Vec::new(), DisplayUnits::Inches);
    let rows = size_position_rows(&mut app).expect("rows");
    let width = rows
        .iter()
        .find(|row| row.label == "Width")
        .expect("the width row");
    assert!(width.edit.is_none(), "a formula cell is read-only");
    assert_eq!(width.numeric, Some(2.0), "the evaluated value still shows");
    let height = rows
        .iter()
        .find(|row| row.label == "Height")
        .expect("the height row");
    assert!(height.edit.is_some());

    let rig = PanelRig::new(app, Panel::SizePosition);
    let texts = shown(&rig.harness);
    assert!(
        texts.iter().any(|text| text == "(formula)"),
        "the row marks the formula cell: {texts:?}"
    );
}

#[test]
fn entering_2_5_cm_into_an_inch_field_stores_the_converted_value() {
    let (app, id) = fixture(geometry_cells(), Vec::new(), DisplayUnits::Inches);
    let mut rig = PanelRig::new(app, Panel::SizePosition);
    let before = rig.app.borrow().active_document().unwrap().document.clone();
    let intents = rig.type_entry(0, "2.5 cm");

    // One registry command, and the panel itself never touched the model.
    assert_eq!(intents.len(), 1, "{intents:?}");
    assert_eq!(intents[0].command.as_str(), "shape.set.width");
    assert_eq!(
        rig.app.borrow().active_document().unwrap().document,
        before,
        "the panel itself never touches the model"
    );

    // Dispatching it stores the converted value as one undo step.
    rig.dispatch(intents);
    let app = rig.app.borrow();
    let width = cached(&app, id, "Width").expect("the width cell");
    match width {
        CellValue::Number(number) => {
            assert!(
                (number - 2.5 / 2.54).abs() < 1.0e-9,
                "2.5 cm is {number} in"
            )
        }
        other => panic!("the width is a number, got {other:?}"),
    }
    assert_eq!(undo_len(&app), 1);
}

#[test]
fn a_mixed_fraction_entry_parses_like_the_grammar() {
    let (app, id) = fixture(geometry_cells(), Vec::new(), DisplayUnits::Inches);
    let mut rig = PanelRig::new(app, Panel::SizePosition);
    let intents = rig.type_entry(0, "5 1/2");
    assert_eq!(intents.len(), 1, "{intents:?}");
    rig.dispatch(intents);
    let app = rig.app.borrow();
    assert_eq!(cached(&app, id, "Width"), Some(CellValue::Number(5.5)));
}

#[test]
fn a_percent_entry_parses_like_the_grammar() {
    let (app, id) = fixture(geometry_cells(), Vec::new(), DisplayUnits::Inches);
    let mut rig = PanelRig::new(app, Panel::SizePosition);
    let intents = rig.type_entry(0, "37%");
    assert_eq!(intents.len(), 1, "{intents:?}");
    rig.dispatch(intents);
    let app = rig.app.borrow();
    assert_eq!(cached(&app, id, "Width"), Some(CellValue::Number(0.37)));
}

#[test]
fn an_angle_entry_parses_like_the_grammar() {
    let (app, id) = fixture(geometry_cells(), Vec::new(), DisplayUnits::Inches);
    let mut rig = PanelRig::new(app, Panel::SizePosition);
    let intents = rig.type_entry(4, "45 deg");
    assert_eq!(intents.len(), 1, "{intents:?}");
    assert_eq!(intents[0].command.as_str(), "shape.set.angle");
    rig.dispatch(intents);
    let app = rig.app.borrow();
    assert_eq!(
        cached(&app, id, "Angle"),
        Some(CellValue::Number(core::f64::consts::FRAC_PI_4))
    );
}

#[test]
fn every_accepted_edit_is_one_command_and_one_undo_step() {
    let (app, id) = fixture(geometry_cells(), Vec::new(), DisplayUnits::Inches);
    let mut rig = PanelRig::new(app, Panel::SizePosition);
    for (field, text, command, cell) in [
        (1usize, "3 in", "shape.set.height", "Height"),
        (2, "5 in", "shape.set.pin", "PinX"),
        (3, "6 in", "shape.set.pin", "PinY"),
        (4, "90 deg", "shape.set.angle", "Angle"),
    ] {
        let intents = rig.type_entry(field, text);
        assert_eq!(intents.len(), 1, "{text}: {intents:?}");
        assert_eq!(intents[0].command.as_str(), command, "{text}");
        let before = undo_len(&rig.app.borrow());
        rig.dispatch(intents);
        let app = rig.app.borrow();
        assert_eq!(undo_len(&app), before + 1, "{text} is one undo step");
        assert!(cached(&app, id, cell).is_some(), "{text} lands in {cell}");
    }
}

#[test]
fn a_bad_entry_is_refused_with_one_status_line_and_no_model_change() {
    let (app, id) = fixture(geometry_cells(), Vec::new(), DisplayUnits::Inches);
    let mut rig = PanelRig::new(app, Panel::SizePosition);
    let before = rig.app.borrow().active_document().unwrap().document.clone();
    let intents = rig.type_entry(0, "abc");

    assert!(
        intents.is_empty(),
        "a refused entry dispatches nothing: {intents:?}"
    );
    {
        let app = rig.app.borrow();
        assert_eq!(app.warnings.len(), 1, "{:?}", app.warnings);
        assert!(app.warnings[0].contains("Width"), "{:?}", app.warnings);
        assert_eq!(
            app.active_document().unwrap().document,
            before,
            "a refused entry changes nothing"
        );
    }
    rig.dispatch(intents);
    assert_eq!(
        cached(&rig.app.borrow(), id, "Width"),
        Some(CellValue::Number(2.0)),
        "the value is still the one before the entry"
    );
}

#[test]
fn a_formula_cell_refuses_the_edit_with_one_status_line() {
    let mut cells = geometry_cells();
    for cell in &mut cells {
        if cell.name.as_str() == "Width" {
            cell.set_formula(Some(String::from("GUARD(2 in)")));
        }
    }
    let (app, id) = fixture(cells, Vec::new(), DisplayUnits::Inches);
    let mut rig = PanelRig::new(app, Panel::SizePosition);
    let before = rig.app.borrow().active_document().unwrap().document.clone();
    let intents = rig.type_entry(0, "2.5 cm");

    assert!(
        intents.is_empty(),
        "the refusal dispatches nothing: {intents:?}"
    );
    let app = rig.app.borrow();
    assert_eq!(app.warnings.len(), 1, "{:?}", app.warnings);
    assert!(
        app.warnings[0].contains("Width is a formula cell"),
        "{:?}",
        app.warnings
    );
    assert_eq!(
        app.active_document().unwrap().document,
        before,
        "the document is unchanged"
    );
    assert_eq!(
        cached(&app, id, "Width").and_then(|value| match value {
            CellValue::Number(number) => Some(number),
            _ => None,
        }),
        Some(2.0)
    );
}

#[test]
fn shape_data_rows_show_the_typed_values() {
    let (app, _id) = fixture(
        geometry_cells(),
        vec![
            prop_row("Cost", CellValue::Number(2.0), GeomUnit::Inches),
            prop_row("Rounded", CellValue::Bool(false), GeomUnit::Unitsless),
            prop_row(
                "Tint",
                CellValue::Color(Color::new(0, 128, 255)),
                GeomUnit::Unitsless,
            ),
            prop_row(
                "Note",
                CellValue::Str(String::from("hello")),
                GeomUnit::Unitsless,
            ),
        ],
        DisplayUnits::Millimeters,
    );
    let mut app = app;
    let rows = shape_data_rows(&mut app).expect("rows");
    let values: Vec<&str> = rows.iter().map(|row| row.value.as_str()).collect();
    assert_eq!(
        values,
        vec!["Box", "7", "50.8 mm", "FALSE", "#0080FF", "hello"]
    );
}
