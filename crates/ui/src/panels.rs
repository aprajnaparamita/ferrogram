//! The Shape Data and Size & Position panels (ADR 0035).
//!
//! With no selection the panels show honest page and document data;
//! with a selection they show the anchor shape (the panels' subject,
//! ADR 0041) and its size, position and angle as editable fields. An
//! edit dispatches the matching `shape.set.*` command through the
//! registry (P5-05); the panel itself never mutates `AppCtx`.
//!
//! # Missing values
//!
//! A missing, non-finite or empty value renders `"—"` and records one
//! warning on [`AppCtx::warnings`] (the P4-07 warning-sink fix). The row
//! builders take `&mut AppCtx` so the warning goes to the status bar
//! directly instead of riding along in a fake row.

use egui::{RichText, Ui};
use ferrogram_commands::{
    AppCtx, ArgValue, Args, CommandId, DisplayUnits, DocumentSession, Invocation, Source,
};
use ferrogram_model::{CellName, CellReader, Page, PageId, ShapeId, SheetRef};

use crate::Intent;
use crate::UiError;
use crate::canvas::format_length;

/// The value shown for a missing, non-finite or empty value (ADR 0035).
const MISSING_VALUE: &str = "—";

/// One label/value row of a panel (ADR 0035).
#[derive(Debug, Clone, PartialEq)]
pub struct PanelRow {
    /// The row label ("Width", "Name", ...).
    pub label: String,
    /// The formatted value; "—" for missing or non-finite values.
    pub value: String,
    /// The edit command and arguments for Phase 5; `None` while the
    /// panel is read-only.
    pub edit: Option<(CommandId, Args)>,
    /// The numeric value behind an editable row (inches or degrees).
    pub numeric: Option<f64>,
}

impl PanelRow {
    /// A read-only row.
    pub fn read_only(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            edit: None,
            numeric: None,
        }
    }

    /// An editable numeric row.
    fn editable(
        label: &str,
        value: String,
        numeric: f64,
        command: &'static str,
        args: Args,
    ) -> Result<Self, UiError> {
        let command = CommandId::parse(command).map_err(|error| UiError::Io {
            message: format!("the panel command id {command} is not valid: {error}"),
        })?;
        Ok(Self {
            label: label.to_owned(),
            value,
            edit: Some((command, args)),
            numeric: Some(numeric),
        })
    }
}

/// Builds the Shape Data rows (ADR 0035).
///
/// With a selection: the anchor shape's name and Visio id. Without one:
/// the document's `Property` rows (name, value) and the active page's
/// name. A missing value renders "—" and records one warning.
pub fn shape_data_rows(app: &mut AppCtx) -> Result<Vec<PanelRow>, UiError> {
    let mut rows = Vec::new();
    let Some(session) = app.active_document() else {
        return Ok(rows);
    };
    if let Some(anchor) = app.selection.anchor
        && let Some(shape) = session.document.shape(anchor)
    {
        let name = if !shape.name.is_empty() {
            shape.name.clone()
        } else if !shape.name_u.is_empty() {
            shape.name_u.clone()
        } else {
            format!("Shape {}", shape.visio_id)
        };
        let visio_id = shape.visio_id;
        rows.push(PanelRow::read_only("Name", name));
        rows.push(PanelRow::read_only("ID", visio_id.to_string()));
        return Ok(rows);
    }
    let properties: Vec<(String, String)> = session
        .document
        .properties
        .iter()
        .map(|property| (property.name.clone(), property.value.clone()))
        .collect();
    let name = active_page(session)
        .map(|page| page.name.clone())
        .unwrap_or_default();
    for (property, value) in properties {
        push_text(app, &mut rows, &property, &value);
    }
    push_text(app, &mut rows, "Name", &name);
    Ok(rows)
}

/// Builds the Size & Position rows (ADR 0035).
///
/// With a selection: the anchor shape's width, height, pin and angle,
/// each editable through its `shape.set.*` command. Without one: the
/// active page's width and height in [`ferrogram_commands::Settings::units`],
/// then its name. A missing value renders "—" and records one warning.
pub fn size_position_rows(app: &mut AppCtx) -> Result<Vec<PanelRow>, UiError> {
    let units = app.settings.units;
    let mut rows = Vec::new();
    let Some(session) = app.active_document() else {
        return Ok(rows);
    };
    if let Some(anchor) = app.selection.anchor
        && session.document.shape(anchor).is_some()
    {
        let width = shape_number(session, anchor, "Width");
        let height = shape_number(session, anchor, "Height");
        let pin_x = shape_number(session, anchor, "PinX");
        let pin_y = shape_number(session, anchor, "PinY");
        let angle = shape_number(session, anchor, "Angle");
        push_editable_length(app, &mut rows, "Width", width, "shape.set.width", "width")?;
        push_editable_length(
            app,
            &mut rows,
            "Height",
            height,
            "shape.set.height",
            "height",
        )?;
        let x = pin_x.unwrap_or(0.0);
        let y = pin_y.unwrap_or(0.0);
        if pin_x.is_some() {
            rows.push(PanelRow::editable(
                "Pin X",
                format_length(x, units),
                x,
                "shape.set.pin",
                Args::new()
                    .with("x", ArgValue::Number(x))
                    .with("y", ArgValue::Number(y)),
            )?);
        } else {
            push_missing(app, &mut rows, "Pin X", "no value");
        }
        if pin_y.is_some() {
            rows.push(PanelRow::editable(
                "Pin Y",
                format_length(y, units),
                y,
                "shape.set.pin",
                Args::new()
                    .with("x", ArgValue::Number(x))
                    .with("y", ArgValue::Number(y)),
            )?);
        } else {
            push_missing(app, &mut rows, "Pin Y", "no value");
        }
        if let Some(angle) = angle {
            rows.push(PanelRow::editable(
                "Angle",
                format!("{:.1}°", angle.to_degrees()),
                angle.to_degrees(),
                "shape.set.angle",
                Args::new().with("angle", ArgValue::Number(angle)),
            )?);
        } else {
            push_missing(app, &mut rows, "Angle", "no value");
        }
        return Ok(rows);
    }
    let page = active_page(session);
    let width = page.and_then(|page| page_number(session, page.id, "PageWidth"));
    let height = page.and_then(|page| page_number(session, page.id, "PageHeight"));
    let name = page.map(|page| page.name.clone()).unwrap_or_default();
    push_length(app, &mut rows, "Width", width, units);
    push_length(app, &mut rows, "Height", height, units);
    push_text(app, &mut rows, "Name", &name);
    Ok(rows)
}

/// Draws the Shape Data panel (ADR 0035) and returns the edit intents.
pub fn show_shape_data(ui: &mut Ui, app: &mut AppCtx) -> Result<Vec<Intent>, UiError> {
    let rows = shape_data_rows(app)?;
    Ok(show_panel(ui, app, "Shape Data", &rows))
}

/// Draws the Size & Position panel (ADR 0035) and returns the edit
/// intents.
pub fn show_size_position(ui: &mut Ui, app: &mut AppCtx) -> Result<Vec<Intent>, UiError> {
    let rows = size_position_rows(app)?;
    Ok(show_panel(ui, app, "Size & Position", &rows))
}

/// Draws one panel: its title and the rows as a two-column grid.
fn show_panel(ui: &mut Ui, app: &AppCtx, title: &str, rows: &[PanelRow]) -> Vec<Intent> {
    ui.label(RichText::new(title).strong());
    if app.active_document().is_none() {
        ui.label("No document");
        return Vec::new();
    }
    let mut intents = Vec::new();
    egui::Grid::new(title).striped(true).show(ui, |ui| {
        for row in rows {
            ui.label(row.label.as_str());
            if let (Some((command, args)), Some(numeric)) = (&row.edit, row.numeric) {
                let mut value = numeric;
                let suffix = if row.label == "Angle" { "°" } else { "" };
                let changed = ui
                    .add(egui::DragValue::new(&mut value).speed(0.01).suffix(suffix))
                    .changed();
                if changed {
                    let mut args = args.clone();
                    set_edited_argument(&row.label, &mut args, value);
                    intents.push(Intent::new(*command, Invocation::new(args, Source::Ui)));
                }
            } else {
                ui.label(row.value.as_str());
            }
            ui.end_row();
        }
    });
    intents
}

/// Writes the edited field into the command arguments.
fn set_edited_argument(label: &str, args: &mut Args, value: f64) {
    match label {
        "Width" => args.set("width", ArgValue::Number(value)),
        "Height" => args.set("height", ArgValue::Number(value)),
        "Pin X" => args.set("x", ArgValue::Number(value)),
        "Pin Y" => args.set("y", ArgValue::Number(value)),
        "Angle" => args.set("angle", ArgValue::Number(value.to_radians())),
        _ => {}
    }
}

/// The active page of a session: the page `active_page` names, or `None`
/// when there is no active page or the id is stale.
fn active_page(session: &DocumentSession) -> Option<&Page> {
    let id = session.active_page?;
    session.document.pages.iter().find(|page| page.id == id)
}

/// Reads one shape number cell (inches or radians).
///
/// `None` when the cell is missing or unusable: the ADR 0021 reader
/// substitutes `0` and records a warning, which the panel treats as "no
/// value" (ADR 0035) instead of showing the substitute.
fn shape_number(session: &DocumentSession, shape: ShapeId, cell: &str) -> Option<f64> {
    let mut reader = CellReader::new(&session.document);
    let before = reader.warnings().len();
    match reader.number(SheetRef::Shape(shape), &CellName::new(cell)) {
        Ok(value) if reader.warnings().len() == before => Some(value),
        _ => None,
    }
}

/// Reads one page number cell (inches).
fn page_number(session: &DocumentSession, page: PageId, cell: &str) -> Option<f64> {
    let mut reader = CellReader::new(&session.document);
    let before = reader.warnings().len();
    match reader.number(SheetRef::Page(page), &CellName::new(cell)) {
        Ok(value) if reader.warnings().len() == before => Some(value),
        _ => None,
    }
}

/// Adds one editable length row.
fn push_editable_length(
    app: &mut AppCtx,
    rows: &mut Vec<PanelRow>,
    label: &str,
    value: Option<f64>,
    command: &'static str,
    argument: &str,
) -> Result<(), UiError> {
    let units = app.settings.units;
    match value {
        Some(value) if value.is_finite() => {
            rows.push(PanelRow::editable(
                label,
                format_length(value, units),
                value,
                command,
                Args::new().with(argument, ArgValue::Number(value)),
            )?);
        }
        Some(_) => push_missing(app, rows, label, "not a number"),
        None => push_missing(app, rows, label, "no value"),
    }
    Ok(())
}

/// Adds one length row: formatted with units, or "—" plus one warning.
fn push_length(
    app: &mut AppCtx,
    rows: &mut Vec<PanelRow>,
    label: &str,
    value: Option<f64>,
    units: DisplayUnits,
) {
    match value {
        Some(value) if value.is_finite() => {
            rows.push(PanelRow::read_only(label, format_length(value, units)));
        }
        Some(_) => push_missing(app, rows, label, "not a number"),
        None => push_missing(app, rows, label, "no value"),
    }
}

/// Adds one text row: the value, or "—" plus one warning when empty.
fn push_text(app: &mut AppCtx, rows: &mut Vec<PanelRow>, label: &str, value: &str) {
    if value.is_empty() {
        push_missing(app, rows, label, "no value");
    } else {
        rows.push(PanelRow::read_only(label, value));
    }
}

/// Adds the "—" row for a missing value and records its warning (the
/// P4-07 sink fix: no warning pseudo-row).
fn push_missing(app: &mut AppCtx, rows: &mut Vec<PanelRow>, label: &str, reason: &str) {
    rows.push(PanelRow::read_only(label, MISSING_VALUE));
    let message = format!("{label}: {reason}");
    if !app.warnings.iter().any(|warning| warning == &message) {
        app.record_warning(message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::with_ui;
    use ferrogram_commands::{DocumentSession, Settings};
    use ferrogram_model::{Cell, CellValue, Document, Page, PageId, Property, Shape};

    /// A page with the given name and an 8.5 x 11 in sheet.
    fn page(name: &str) -> Page {
        let mut page = Page::new(PageId { index: 0 }, 0);
        page.name = name.to_owned();
        page.sheet.cells = vec![cell("PageWidth", 8.5), cell("PageHeight", 11.0)];
        page
    }

    /// A cell with a cached number.
    fn cell(name: &str, value: f64) -> Cell {
        let mut cell = Cell::new(CellName::new(name));
        cell.value = Some(CellValue::Number(value));
        cell
    }

    /// An app with one open document in the given display units.
    fn app_with(doc: Document, units: DisplayUnits) -> AppCtx {
        let mut settings = Settings::default();
        settings.units = units;
        let mut app = AppCtx::new(settings);
        app.open_document(DocumentSession::new(doc, None));
        app
    }

    /// A document with one named page and one property.
    fn document() -> Document {
        let mut doc = Document::new();
        doc.properties.push(Property::new("title", "Diagram 1"));
        doc.pages.push(page("Page-1"));
        doc
    }

    /// A document with one selected 2x1 shape at (3, 4), turned 30°.
    fn selected_document() -> (AppCtx, ShapeId) {
        let mut doc = Document::new();
        let mut shape = Shape::new(ShapeId { index: 0 }, 7);
        shape.name = "Box".to_owned();
        shape.sheet.cells = vec![
            cell("Width", 2.0),
            cell("Height", 1.0),
            cell("PinX", 3.0),
            cell("PinY", 4.0),
            cell("Angle", core::f64::consts::FRAC_PI_6),
        ];
        let id = doc.add_shape(shape);
        let mut page = page("Page-1");
        page.shapes.push(id);
        doc.pages.push(page);
        let mut app = app_with(doc, DisplayUnits::Inches);
        app.selection.shapes = vec![id];
        app.selection.anchor = Some(id);
        app.selection.page = Some(PageId { index: 0 });
        (app, id)
    }

    #[test]
    fn editable_rows_reject_an_invalid_command_id() {
        let error = PanelRow::editable("Width", "2 in".to_owned(), 2.0, "", Args::new())
            .expect_err("an empty command id is invalid");
        assert!(matches!(error, UiError::Io { .. }));
    }

    #[test]
    fn edited_arguments_use_the_command_contract_names() {
        for (label, name, expected) in [
            ("Width", "width", 2.0),
            ("Height", "height", 3.0),
            ("Pin X", "x", 4.0),
            ("Pin Y", "y", 5.0),
        ] {
            let mut args = Args::new();
            set_edited_argument(label, &mut args, expected);
            assert_eq!(args.number(name), Some(expected), "{label}");
        }
        let mut args = Args::new();
        set_edited_argument("Angle", &mut args, 180.0);
        let angle = args.number("angle").expect("angle");
        assert!((angle - core::f64::consts::PI).abs() < 1.0e-12);
        set_edited_argument("Unknown", &mut args, 1.0);
        assert_eq!(args.len(), 1, "an unknown label writes nothing");
    }

    #[test]
    fn a_selection_without_a_name_or_cells_still_shows_rows() {
        let mut doc = Document::new();
        let mut shape = Shape::new(ShapeId { index: 0 }, 12);
        shape.name_u = "Connector".to_owned();
        let id = doc.add_shape(shape);
        let mut page = page("Page-1");
        page.shapes.push(id);
        doc.pages.push(page);
        let mut app = app_with(doc, DisplayUnits::Inches);
        app.selection.shapes = vec![id];
        app.selection.anchor = Some(id);
        let data = shape_data_rows(&mut app).expect("rows");
        assert_eq!(data[0].value, "Connector");
        let size = size_position_rows(&mut app).expect("rows");
        assert_eq!(size[0].value, MISSING_VALUE);
        assert!(
            size.iter()
                .any(|row| row.label == "Pin X" && row.value == MISSING_VALUE)
        );
        assert!(
            size.iter()
                .any(|row| row.label == "Pin Y" && row.value == MISSING_VALUE)
        );
        assert!(
            size.iter()
                .any(|row| row.label == "Angle" && row.value == MISSING_VALUE)
        );

        // The unnamed fallback names the Visio id.
        let mut app2 = app;
        let session = app2.active_document_mut().expect("session");
        if let Some(shape) = session.document.shape_mut(id) {
            shape.name_u.clear();
        }
        let data = shape_data_rows(&mut app2).expect("rows");
        assert_eq!(data[0].value, "Shape 12");
    }

    #[test]
    fn non_finite_panel_values_take_the_warning_path() {
        let mut app = AppCtx::new(Settings::default());
        let mut rows = Vec::new();
        push_length(
            &mut app,
            &mut rows,
            "Width",
            Some(f64::NAN),
            DisplayUnits::Inches,
        );
        assert_eq!(rows[0].value, MISSING_VALUE);
        push_editable_length(
            &mut app,
            &mut rows,
            "Height",
            Some(f64::INFINITY),
            "shape.set.height",
            "height",
        )
        .expect("no error");
        assert_eq!(rows[1].value, MISSING_VALUE);
    }

    #[test]
    fn a_read_only_row_has_no_edit() {
        let row = PanelRow::read_only("Width", "8.5 in");
        assert_eq!(row.label, "Width");
        assert_eq!(row.value, "8.5 in");
        assert!(row.edit.is_none());
        assert_eq!(row.numeric, None);
    }

    #[test]
    fn shape_data_rows_list_properties_and_the_page_name() {
        let mut app = app_with(document(), DisplayUnits::Inches);
        let rows = shape_data_rows(&mut app).expect("rows");
        assert_eq!(
            rows,
            vec![
                PanelRow::read_only("title", "Diagram 1"),
                PanelRow::read_only("Name", "Page-1"),
            ]
        );
        assert!(rows.iter().all(|row| row.edit.is_none()));
    }

    #[test]
    fn a_selection_makes_the_anchor_the_panels_subject() {
        let (mut app, _id) = selected_document();
        let data = shape_data_rows(&mut app).expect("rows");
        assert_eq!(data[0].value, "Box");
        assert_eq!(data[1].value, "7");
        let size = size_position_rows(&mut app).expect("rows");
        assert_eq!(size[0].label, "Width");
        assert_eq!(size[0].numeric, Some(2.0));
        assert_eq!(
            size[0].edit.as_ref().map(|(id, _)| id.as_str()),
            Some("shape.set.width")
        );
        assert_eq!(size[1].numeric, Some(1.0));
        assert_eq!(size[2].label, "Pin X");
        assert_eq!(size[2].numeric, Some(3.0));
        assert_eq!(size[3].label, "Pin Y");
        assert_eq!(size[3].numeric, Some(4.0));
        assert_eq!(size[4].label, "Angle");
        assert!((size[4].numeric.unwrap_or_default() - 30.0).abs() < 1.0e-9);
    }

    #[test]
    fn size_position_rows_show_the_page_size_and_name() {
        let mut app = app_with(document(), DisplayUnits::Inches);
        let rows = size_position_rows(&mut app).expect("rows");
        assert_eq!(
            rows,
            vec![
                PanelRow::read_only("Width", "8.5 in"),
                PanelRow::read_only("Height", "11 in"),
                PanelRow::read_only("Name", "Page-1"),
            ]
        );
        assert!(rows.iter().all(|row| row.edit.is_none()));
    }

    #[test]
    fn page_size_converts_at_the_units_edge() {
        let mut mm = app_with(document(), DisplayUnits::Millimeters);
        let rows = size_position_rows(&mut mm).expect("rows");
        assert_eq!(rows[0].value, "215.9 mm");
        assert_eq!(rows[1].value, "279.4 mm");

        let mut cm = app_with(document(), DisplayUnits::Centimeters);
        let rows = size_position_rows(&mut cm).expect("rows");
        assert_eq!(rows[0].value, "21.59 cm");
    }

    #[test]
    fn missing_values_render_a_dash_and_record_one_warning() {
        let mut doc = Document::new();
        doc.pages.push(Page::new(PageId { index: 0 }, 0));
        let mut app = app_with(doc, DisplayUnits::Inches);
        let rows = size_position_rows(&mut app).expect("rows");
        assert_eq!(
            rows,
            vec![
                PanelRow::read_only("Width", MISSING_VALUE),
                PanelRow::read_only("Height", MISSING_VALUE),
                PanelRow::read_only("Name", MISSING_VALUE),
            ]
        );
        assert_eq!(app.warnings.len(), 3);
        assert_eq!(app.warnings[0], "Width: no value");

        // A second frame does not duplicate the same warning.
        let _ = size_position_rows(&mut app).expect("rows");
        assert_eq!(app.warnings.len(), 3);

        let rows = shape_data_rows(&mut app).expect("rows");
        assert_eq!(rows, vec![PanelRow::read_only("Name", MISSING_VALUE)]);
        assert_eq!(
            app.warnings.last().map(String::as_str),
            Some("Name: no value")
        );
    }

    #[test]
    fn an_empty_property_value_renders_a_dash() {
        let mut doc = Document::new();
        doc.properties.push(Property::new("title", ""));
        let mut app = app_with(doc, DisplayUnits::Inches);
        let rows = shape_data_rows(&mut app).expect("rows");
        assert_eq!(
            rows,
            vec![
                PanelRow::read_only("title", MISSING_VALUE),
                PanelRow::read_only("Name", MISSING_VALUE),
            ]
        );
        assert!(
            app.warnings
                .iter()
                .any(|warning| warning == "title: no value")
        );
    }

    #[test]
    fn a_non_finite_value_renders_a_dash() {
        let mut doc = Document::new();
        let mut page = page("Page-1");
        page.sheet.cells = vec![
            cell("PageWidth", f64::NAN),
            cell("PageHeight", f64::INFINITY),
        ];
        doc.pages.push(page);
        let mut app = app_with(doc, DisplayUnits::Inches);
        let rows = size_position_rows(&mut app).expect("rows");
        for label in ["Width", "Height"] {
            let row = rows
                .iter()
                .find(|row| row.label == label)
                .expect("the row exists");
            assert_eq!(row.value, MISSING_VALUE, "{label}");
        }
        assert!(
            app.warnings
                .iter()
                .any(|warning| warning.starts_with("Width:")),
            "{:?}",
            app.warnings
        );
    }

    #[test]
    fn no_document_gives_no_rows_and_says_so() {
        let mut app = AppCtx::new(Settings::default());
        assert!(shape_data_rows(&mut app).expect("rows").is_empty());
        assert!(size_position_rows(&mut app).expect("rows").is_empty());
        with_ui(|ui| {
            show_shape_data(ui, &mut app).expect("shape data");
            show_size_position(ui, &mut app).expect("size and position");
        });
    }

    #[test]
    fn both_panels_render_in_an_egui_context() {
        let (mut app, _id) = selected_document();
        with_ui(|ui| {
            show_shape_data(ui, &mut app).expect("shape data");
            show_size_position(ui, &mut app).expect("size and position");
        });
    }

    #[test]
    fn a_stale_active_page_id_falls_back_to_missing_values() {
        let mut app = app_with(document(), DisplayUnits::Inches);
        let session = app.active_document_mut().expect("an open document");
        session.active_page = Some(PageId { index: 9 });
        let rows = size_position_rows(&mut app).expect("rows");
        for row in rows.iter().filter(|row| row.label != "Name") {
            assert_eq!(row.value, MISSING_VALUE, "{}", row.label);
        }
    }
}
