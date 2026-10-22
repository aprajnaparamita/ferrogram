//! The writer's typed `V`/`U` spelling (task P6-08, P2-14): every
//! cached value kind survives write then read unchanged.
//!
//! The test drives the cached forms [`Value::to_cached`] produces for
//! each [`Value`] kind (ADR 0052): a number in canonical units with the
//! unit the cell records, and the plain non-numeric kinds. They are
//! built here as [`CellValue`]s directly because `Value::to_cached`
//! lives in `ferrogram-shapesheet`, an L2 crate this L2 crate may not
//! depend on (ADR 0002; the gate's `layers` step enforces it). The
//! `Value::to_cached`-driven test of the same contract is
//! `crates/commands/tests/value_roundtrip.rs`.
//!
//! The writer spells `U` per kind (`BOOL`, `STR`, `COLOR`, `LIST`, or a
//! unit code) and drops the unit of a non-numeric kind; the reader maps
//! the code back. The one documented difference the file makes: a
//! non-numeric kind with a hand-fitted unit loses the unit (ADR 0052).

use ferrogram_geom::Unit;
use ferrogram_model::{Cell, CellName, Color, Document, Shape, ShapeId};

use ferrogram_vsdx::{Limits, from_model, read_package, to_model, write_document};

/// Writes one cell carrying `value` and `unit` through the writer and
/// reads it back. Returns the value and unit the read produced.
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)] // test helper
fn round_trip(value: ferrogram_model::CellValue, unit: Unit) -> (ferrogram_model::CellValue, Unit) {
    let mut cell = Cell::new(CellName::from_static("Probe"));
    cell.value = Some(value);
    cell.unit = unit;

    let mut doc = Document::new();
    let mut shape = Shape::new(ShapeId { index: 0 }, 1);
    shape.sheet.cells.push(cell);
    let page = doc.add_page("Page-1").expect("page");
    doc.insert_shape(page, shape, None).expect("shape");

    let package = from_model(&doc).expect("from_model");
    let bytes = write_document(&doc, &package).expect("write");
    let reparsed = read_package(&bytes, &Limits::DEFAULT).expect("read");
    let model = to_model(&reparsed).expect("to_model");
    let read = model
        .shapes
        .first()
        .expect("one shape")
        .sheet
        .cells
        .iter()
        .find(|cell| cell.name.as_str() == "Probe")
        .expect("the probe cell is written and read");
    (read.value.clone().expect("a value"), read.unit)
}

/// Asserts the exact round trip of one cached value and unit.
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)] // test helper
fn assert_round_trip(value: ferrogram_model::CellValue, unit: Unit) {
    let (read_value, read_unit) = round_trip(value.clone(), unit);
    assert_eq!(read_value, value, "{value:?} changed value");
    assert_eq!(read_unit, unit, "{value:?} changed unit");
}

#[test]
fn a_number_round_trips_in_the_unit_the_cell_recorded() {
    for unit in [
        Unit::Unitsless,
        Unit::Inches,
        Unit::Feet,
        Unit::Yards,
        Unit::Miles,
        Unit::Millimeters,
        Unit::Centimeters,
        Unit::Meters,
        Unit::Kilometers,
        Unit::Points,
        Unit::Picas,
        Unit::Ciceros,
        Unit::Degrees,
        Unit::Radians,
        Unit::Percent,
    ] {
        assert_round_trip(ferrogram_model::CellValue::Number(2.5), unit);
        assert_round_trip(
            ferrogram_model::CellValue::Number(-0.003472222222222222),
            unit,
        );
    }
}

#[test]
fn a_string_round_trips_every_spelling() {
    for text in [
        "Calibri",
        "2",           // reads as a number without `U`
        "#4d4d4d",     // reads as a colour without `U`
        "a;b",         // a list spelling without `U`
        "Themed",      // the theme marker
        "",            // the empty string
        "1 in & \"\"", // formula-looking text
    ] {
        assert_round_trip(
            ferrogram_model::CellValue::Str(text.to_owned()),
            Unit::Unitsless,
        );
    }
}

#[test]
fn a_boolean_round_trips_both_spelled_values() {
    assert_round_trip(ferrogram_model::CellValue::Bool(true), Unit::Unitsless);
    assert_round_trip(ferrogram_model::CellValue::Bool(false), Unit::Unitsless);
}

#[test]
fn a_colour_round_trips() {
    for color in [
        Color::new(0, 0, 0),
        Color::new(255, 255, 255),
        Color::new(1, 2, 3),
    ] {
        assert_round_trip(ferrogram_model::CellValue::Color(color), Unit::Unitsless);
    }
}

#[test]
fn a_list_round_trips_as_a_list() {
    // `U="LIST"` (our own spelling, see `read::value`) keeps the raw
    // `;`-separated text a list caches, whatever its entries look like.
    for raw in ["a;1", "x", "Themed", ""] {
        assert_round_trip(
            ferrogram_model::CellValue::List(raw.to_owned()),
            Unit::Unitsless,
        );
    }
}

#[test]
fn an_error_value_round_trips() {
    for error in ["#DIV/0!", "#VALUE!", "#REF!", "#NUM!", "#N/A!", "#DIM!"] {
        assert_round_trip(
            ferrogram_model::CellValue::Error(error.to_owned()),
            Unit::Unitsless,
        );
    }
}

#[test]
fn a_non_numeric_kind_drops_a_hand_fitted_unit_on_write() {
    // The P2-14 close-out statement (ADR 0052): a `Bool`, `Str`,
    // `Color` or `List` with a real unit reads back as the same value,
    // without the unit.
    for value in [
        ferrogram_model::CellValue::Bool(true),
        ferrogram_model::CellValue::Str("plain".to_owned()),
        ferrogram_model::CellValue::Str("2".to_owned()),
        ferrogram_model::CellValue::Color(Color::new(7, 7, 7)),
        ferrogram_model::CellValue::List("a".to_owned()),
    ] {
        let (read_value, read_unit) = round_trip(value.clone(), Unit::Inches);
        assert_eq!(read_value, value, "{value:?} changed");
        assert_eq!(read_unit, Unit::Unitsless);
    }
}

#[test]
fn a_written_value_reads_back_the_same_twice() {
    // The writer is a pure function of the model (ADR 0057): a value
    // that survived one round trip survives the next one unchanged.
    for (value, unit) in [
        (ferrogram_model::CellValue::Number(3.0), Unit::Points),
        (
            ferrogram_model::CellValue::Str("2".to_owned()),
            Unit::Unitsless,
        ),
        (ferrogram_model::CellValue::Bool(true), Unit::Unitsless),
        (
            ferrogram_model::CellValue::Color(Color::new(9, 9, 9)),
            Unit::Unitsless,
        ),
        (
            ferrogram_model::CellValue::List("a".to_owned()),
            Unit::Unitsless,
        ),
        (
            ferrogram_model::CellValue::Error("#N/A!".to_owned()),
            Unit::Unitsless,
        ),
    ] {
        let (first_value, first_unit) = round_trip(value, unit);
        let (second_value, second_unit) = round_trip(first_value.clone(), first_unit);
        assert_eq!(second_value, first_value);
        assert_eq!(second_unit, first_unit);
    }
}
