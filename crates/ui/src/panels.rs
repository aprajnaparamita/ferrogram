//! The Shape Data and Size & Position panels (ADR 0035).
//!
//! Read-only in Phase 4: with no selection both panels show honest page
//! and document data, lengths formatted in
//! [`Settings::units`](ferrogram_commands::Settings). Phase 5 fills the
//! `edit` slot of each [`PanelRow`] and the row's command dispatches
//! through the registry.
//!
//! # Missing values
//!
//! A missing, non-finite or empty value renders `"—"` and adds one
//! warning row (`PanelRow::read_only("Warning", "<label>: <reason>")`)
//! after it. The row builders take `&AppCtx`, so they cannot push to
//! [`AppCtx::warnings`](ferrogram_commands::AppCtx::warnings); the
//! warning text therefore rides along in the `value` of a `PanelRow`,
//! which the caller can show or record. Task P4-07 note: giving the
//! builders `&mut AppCtx` (or a warning channel) would let them record
//! on the `AppCtx` directly; that is an open issue for the master.
//!
//! Owned by task P4-07. Alpha status (task P4-01): the interface is
//! defined as stubs.

use egui::{RichText, Ui};
use ferrogram_commands::{AppCtx, Args, CommandId, DisplayUnits, DocumentSession};
use ferrogram_model::{CellName, CellReader, Page, PageId, SheetRef};

use crate::UiError;
use crate::canvas::format_length;

/// The value shown for a missing, non-finite or empty value (ADR 0035).
const MISSING_VALUE: &str = "—";

/// The label of the warning rows this module adds next to a missing
/// value.
const WARNING_LABEL: &str = "Warning";

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
}

impl PanelRow {
    /// A read-only row.
    pub fn read_only(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            edit: None,
        }
    }
}

/// Builds the Shape Data rows (ADR 0035).
///
/// With no selection: the document's `Property` rows (name, value) and
/// the active page's name. A missing value renders "—" with one warning
/// row after it (see the module docs). No open document: no rows.
pub fn shape_data_rows(app: &AppCtx) -> Result<Vec<PanelRow>, UiError> {
    let mut rows = Vec::new();
    let Some(session) = app.active_document() else {
        return Ok(rows);
    };
    for property in &session.document.properties {
        push_text(&mut rows, property.name.as_str(), &property.value);
    }
    let name = active_page(session).map(|page| page.name.as_str());
    push_text(&mut rows, "Name", name.unwrap_or_default());
    Ok(rows)
}

/// Builds the Size & Position rows (ADR 0035).
///
/// With no selection: the active page's width and height in
/// [`Settings::units`](ferrogram_commands::Settings::units), then its
/// name. A missing value renders "—" with one warning row after it (see
/// the module docs). No open document: no rows.
pub fn size_position_rows(app: &AppCtx) -> Result<Vec<PanelRow>, UiError> {
    let units = app.settings.units;
    let mut rows = Vec::new();
    let Some(session) = app.active_document() else {
        return Ok(rows);
    };
    let page = active_page(session);
    let width = page.and_then(|page| page_number(session, page.id, "PageWidth"));
    let height = page.and_then(|page| page_number(session, page.id, "PageHeight"));
    push_length(&mut rows, "Width", width, units);
    push_length(&mut rows, "Height", height, units);
    push_text(
        &mut rows,
        "Name",
        page.map(|page| page.name.as_str()).unwrap_or_default(),
    );
    Ok(rows)
}

/// Draws the Shape Data panel (ADR 0035).
///
/// The rows render as a two-column grid. With no open document the panel
/// says so (the status bar wording, ADR 0033).
pub fn show_shape_data(ui: &mut Ui, app: &AppCtx) -> Result<(), UiError> {
    let rows = shape_data_rows(app)?;
    show_panel(ui, app, "Shape Data", &rows);
    Ok(())
}

/// Draws the Size & Position panel (ADR 0035).
///
/// The rows render as a two-column grid. With no open document the panel
/// says so (the status bar wording, ADR 0033).
pub fn show_size_position(ui: &mut Ui, app: &AppCtx) -> Result<(), UiError> {
    let rows = size_position_rows(app)?;
    show_panel(ui, app, "Size & Position", &rows);
    Ok(())
}

/// Draws one panel: its title and the rows as a two-column grid.
fn show_panel(ui: &mut Ui, app: &AppCtx, title: &str, rows: &[PanelRow]) {
    ui.label(RichText::new(title).strong());
    if app.active_document().is_none() {
        ui.label("No document");
        return;
    }
    egui::Grid::new(title).striped(true).show(ui, |ui| {
        for row in rows {
            ui.label(row.label.as_str());
            ui.label(row.value.as_str());
            ui.end_row();
        }
    });
}

/// The active page of a session: the page `active_page` names, or `None`
/// when there is no active page or the id is stale.
fn active_page(session: &DocumentSession) -> Option<&Page> {
    let id = session.active_page?;
    session.document.pages.iter().find(|page| page.id == id)
}

/// Reads one page number cell (inches).
///
/// `None` when the cell is missing or unusable: the ADR 0021 reader
/// substitutes `0` and records a warning, which the panel treats as "no
/// value" (ADR 0035) instead of showing the substitute.
fn page_number(session: &DocumentSession, page: PageId, cell: &str) -> Option<f64> {
    let mut reader = CellReader::new(&session.document);
    let before = reader.warnings().len();
    match reader.number(SheetRef::Page(page), &CellName::new(cell)) {
        Ok(value) if reader.warnings().len() == before => Some(value),
        _ => None,
    }
}

/// Adds one length row: formatted with units, or "—" plus one warning
/// row when the value is missing or not a number.
fn push_length(rows: &mut Vec<PanelRow>, label: &str, value: Option<f64>, units: DisplayUnits) {
    match value {
        Some(value) if value.is_finite() => {
            rows.push(PanelRow::read_only(label, format_length(value, units)));
        }
        Some(_) => push_missing(rows, label, "not a number"),
        None => push_missing(rows, label, "no value"),
    }
}

/// Adds one text row: the value, or "—" plus one warning row when it is
/// empty.
fn push_text(rows: &mut Vec<PanelRow>, label: &str, value: &str) {
    if value.is_empty() {
        push_missing(rows, label, "no value");
    } else {
        rows.push(PanelRow::read_only(label, value));
    }
}

/// Adds the "—" row for a missing value and its warning row (ADR 0035).
fn push_missing(rows: &mut Vec<PanelRow>, label: &str, reason: &str) {
    rows.push(PanelRow::read_only(label, MISSING_VALUE));
    rows.push(PanelRow::read_only(
        WARNING_LABEL,
        format!("{label}: {reason}"),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::with_ui;
    use ferrogram_commands::{DocumentSession, Settings};
    use ferrogram_model::{Cell, CellValue, Document, Page, Property};

    /// A page with the given name and an 8.5 x 11 in sheet.
    fn page(name: &str) -> Page {
        let mut page = Page::new(PageId { index: 0 }, 0);
        page.name = name.to_owned();
        page.sheet.cells = vec![cell("PageWidth", 8.5), cell("PageHeight", 11.0)];
        page
    }

    /// A page number cell with a cached value.
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

    #[test]
    fn a_read_only_row_has_no_edit() {
        let row = PanelRow::read_only("Width", "8.5 in");
        assert_eq!(row.label, "Width");
        assert_eq!(row.value, "8.5 in");
        assert!(row.edit.is_none());
    }

    #[test]
    fn shape_data_rows_list_properties_and_the_page_name() {
        let app = app_with(document(), DisplayUnits::Inches);
        let rows = shape_data_rows(&app).expect("rows");
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
    fn size_position_rows_show_the_page_size_and_name() {
        let app = app_with(document(), DisplayUnits::Inches);
        let rows = size_position_rows(&app).expect("rows");
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
        let mm = app_with(document(), DisplayUnits::Millimeters);
        let rows = size_position_rows(&mm).expect("rows");
        assert_eq!(rows[0].value, "215.9 mm");
        assert_eq!(rows[1].value, "279.4 mm");

        let cm = app_with(document(), DisplayUnits::Centimeters);
        let rows = size_position_rows(&cm).expect("rows");
        assert_eq!(rows[0].value, "21.59 cm");
    }

    #[test]
    fn missing_values_render_a_dash_with_one_warning() {
        let mut doc = Document::new();
        doc.pages.push(Page::new(PageId { index: 0 }, 0));
        let app = app_with(doc, DisplayUnits::Inches);
        let rows = size_position_rows(&app).expect("rows");
        assert_eq!(
            rows,
            vec![
                PanelRow::read_only("Width", MISSING_VALUE),
                PanelRow::read_only(WARNING_LABEL, "Width: no value"),
                PanelRow::read_only("Height", MISSING_VALUE),
                PanelRow::read_only(WARNING_LABEL, "Height: no value"),
                PanelRow::read_only("Name", MISSING_VALUE),
                PanelRow::read_only(WARNING_LABEL, "Name: no value"),
            ]
        );

        let rows = shape_data_rows(&app).expect("rows");
        assert_eq!(
            rows,
            vec![
                PanelRow::read_only("Name", MISSING_VALUE),
                PanelRow::read_only(WARNING_LABEL, "Name: no value"),
            ]
        );
    }

    #[test]
    fn an_empty_property_value_renders_a_dash_with_one_warning() {
        let mut doc = Document::new();
        doc.properties.push(Property::new("title", ""));
        let app = app_with(doc, DisplayUnits::Inches);
        let rows = shape_data_rows(&app).expect("rows");
        assert_eq!(
            rows,
            vec![
                PanelRow::read_only("title", MISSING_VALUE),
                PanelRow::read_only(WARNING_LABEL, "title: no value"),
                PanelRow::read_only("Name", MISSING_VALUE),
                PanelRow::read_only(WARNING_LABEL, "Name: no value"),
            ]
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
        let app = app_with(doc, DisplayUnits::Inches);
        let rows = size_position_rows(&app).expect("rows");
        for label in ["Width", "Height"] {
            let row = rows
                .iter()
                .find(|row| row.label == label)
                .expect("the row exists");
            assert_eq!(row.value, MISSING_VALUE, "{label}");
            let warning = rows
                .iter()
                .find(|row| row.label == WARNING_LABEL && row.value.starts_with(label))
                .expect("one warning row");
            assert!(
                warning.value == format!("{label}: not a number")
                    || warning.value == format!("{label}: no value"),
                "{}",
                warning.value
            );
        }
    }

    #[test]
    fn no_document_gives_no_rows_and_says_so() {
        let app = AppCtx::new(Settings::default());
        assert!(shape_data_rows(&app).expect("rows").is_empty());
        assert!(size_position_rows(&app).expect("rows").is_empty());
        with_ui(|ui| {
            show_shape_data(ui, &app).expect("shape data");
            show_size_position(ui, &app).expect("size and position");
        });
    }

    #[test]
    fn both_panels_render_in_an_egui_context() {
        let app = app_with(document(), DisplayUnits::Inches);
        with_ui(|ui| {
            show_shape_data(ui, &app).expect("shape data");
            show_size_position(ui, &app).expect("size and position");
        });
        for rows in [
            shape_data_rows(&app).expect("rows"),
            size_position_rows(&app).expect("rows"),
        ] {
            assert!(rows.iter().all(|row| row.edit.is_none()));
        }
    }

    #[test]
    fn a_stale_active_page_id_falls_back_to_missing_values() {
        let mut app = app_with(document(), DisplayUnits::Inches);
        let session = app.active_document_mut().expect("an open document");
        session.active_page = Some(PageId { index: 9 });
        let rows = size_position_rows(&app).expect("rows");
        for row in rows.iter().filter(|row| row.label != WARNING_LABEL) {
            assert_eq!(row.value, MISSING_VALUE, "{}", row.label);
        }
    }
}
