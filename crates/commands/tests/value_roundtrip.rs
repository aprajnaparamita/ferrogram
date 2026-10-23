//! The P2-14 round trip of `Value::to_cached` output (task P6-08, ADR
//! 0052 and ADR 0057): what [`Value::to_cached`] writes into a model
//! cell comes back unchanged through write then read, for every value
//! kind.
//!
//! The test lives in `crates/commands/tests` and not in
//! `crates/vsdx/tests` as the task file names it: `Value::to_cached`
//! lives in `larrygram-shapesheet`, and an L2 crate may not depend on
//! an L2 crate (ADR 0002, the gate's `layers` step). `larrygram-commands`
//! is the first crate above both and already depends on them (it drives
//! the value pipeline in `builtins::file.rs`). The file-level kind
//! spelling is pinned next to the writer in
//! `crates/vsdx/tests/value_roundtrip.rs`.
//!
//! [`Value::to_cached`] already folds a unit the model does not carry to
//! the family's canonical unit ("the spelling is lost, the value is
//! not"), so its output always round-trips **exactly**; the unit drop
//! for a hand-fitted non-numeric unit is the one documented difference
//! the file makes (ADR 0052).

use larrygram_geom::Unit as GeomUnit;
use larrygram_model::{Cell, CellName, Color, Document, Shape, ShapeId};
use larrygram_shapesheet::{ErrorCode, Number, Unit, Value};
use larrygram_vsdx::{Limits, from_model, read_package, to_model, write_document};

/// Every unit of the formula-side unit set (ADR 0052).
const ALL_UNITS: &[Unit] = &[
    Unit::Inches,
    Unit::Feet,
    Unit::Yards,
    Unit::Miles,
    Unit::NauticalMiles,
    Unit::Millimeters,
    Unit::Centimeters,
    Unit::Meters,
    Unit::Kilometers,
    Unit::Points,
    Unit::Picas,
    Unit::Ciceros,
    Unit::Didots,
    Unit::Degrees,
    Unit::Radians,
    Unit::Minutes,
    Unit::Seconds,
    Unit::DegreeMinSec,
    Unit::Percent,
    Unit::ElapsedWeeks,
    Unit::ElapsedDays,
    Unit::ElapsedHours,
    Unit::ElapsedMinutes,
    Unit::ElapsedSeconds,
    Unit::DateTime,
    Unit::InchesFractional,
    Unit::MilesFractional,
    Unit::FeetAndInches,
    Unit::PicasAndPoints,
    Unit::CicerosAndDidots,
    Unit::ImplicitDrawing,
    Unit::ImplicitPage,
    Unit::ImplicitType,
    Unit::ImplicitAngle,
    Unit::ImplicitDuration,
    Unit::Unitsless,
];

/// Writes one cell carrying `value` and `unit` through the writer and
/// reads it back. Returns the value and unit the read produced.
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)] // test helper
fn round_trip_cell(
    value: larrygram_model::CellValue,
    unit: GeomUnit,
) -> (larrygram_model::CellValue, GeomUnit) {
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

/// The round trip of a value's cached form ([`Value::to_cached`]).
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)] // test helper
fn round_trip(value: &Value) -> (larrygram_model::CellValue, GeomUnit) {
    let cached = value.to_cached().expect("to_cached never fails");
    round_trip_cell(cached.value, cached.unit)
}

/// Asserts the exact round trip of `value`'s cached form.
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)] // test helper
fn assert_round_trip(value: &Value) {
    let cached = value.to_cached().expect("to_cached never fails");
    let (read_value, read_unit) = round_trip(value);
    assert_eq!(read_value, cached.value, "{value:?} changed value");
    assert_eq!(read_unit, cached.unit, "{value:?} changed unit");
}

#[test]
fn a_number_round_trips_in_the_unit_the_cell_recorded() {
    for unit in ALL_UNITS {
        assert_round_trip(&Value::Number(Number::new(2.5, *unit)));
        assert_round_trip(&Value::Number(Number::new(-0.003472222222222222, *unit)));
    }
}

#[test]
fn a_string_round_trips_every_spelling() {
    for text in ["Calibri", "2", "#4d4d4d", "a;b", "Themed", ""] {
        assert_round_trip(&Value::Str(text.to_owned()));
    }
}

#[test]
fn a_boolean_round_trips_both_spelled_values() {
    assert_round_trip(&Value::Bool(true));
    assert_round_trip(&Value::Bool(false));
}

#[test]
fn a_colour_round_trips() {
    assert_round_trip(&Value::Color(Color::new(0, 0, 0)));
    assert_round_trip(&Value::Color(Color::new(255, 255, 255)));
    assert_round_trip(&Value::Color(Color::new(1, 2, 3)));
}

#[test]
fn a_list_round_trips_as_a_list() {
    for list in [
        Value::List(vec![
            Value::Str("a".to_owned()),
            Value::Number(Number::new(1.0, Unit::Unitsless)),
        ]),
        Value::List(vec![Value::Str("x".to_owned())]),
        Value::List(vec![Value::Str("Themed".to_owned())]),
        Value::List(Vec::new()),
    ] {
        assert_round_trip(&list);
    }
}

#[test]
fn an_error_value_round_trips() {
    for code in [
        ErrorCode::Div0,
        ErrorCode::Value,
        ErrorCode::Ref,
        ErrorCode::Num,
        ErrorCode::Na,
        ErrorCode::Dim,
    ] {
        assert_round_trip(&Value::Err(code));
    }
}

#[test]
fn a_unit_the_model_does_not_carry_folds_before_the_file_sees_it() {
    // `to_cached` records the family's canonical unit for a unit the
    // model does not carry ("the spelling is lost, the value is not",
    // ADR 0052); what the file sees round-trips exactly.
    let value = Value::Number(Number::new(1.0, Unit::NauticalMiles));
    let cached = value.to_cached().expect("to_cached never fails");
    assert_eq!(cached.unit, GeomUnit::Inches);
    assert_round_trip(&value);

    let value = Value::Number(Number::new(1.0, Unit::ElapsedDays));
    let cached = value.to_cached().expect("to_cached never fails");
    assert_eq!(cached.unit, GeomUnit::Unitsless);
    assert_round_trip(&value);
}

#[test]
fn a_written_value_reads_back_the_same_twice() {
    // The writer is a pure function of the model (ADR 0057): a value
    // that survived one round trip survives the next one unchanged.
    for value in [
        Value::Number(Number::new(3.0, Unit::Points)),
        Value::Str("2".to_owned()),
        Value::Bool(true),
        Value::Color(Color::new(9, 9, 9)),
        Value::List(vec![Value::Str("a".to_owned())]),
        Value::Err(ErrorCode::Na),
    ] {
        let (first_value, first_unit) = round_trip(&value);
        let again = Value::from_cached(&first_value, first_unit).expect("from_cached");
        let (second_value, second_unit) = round_trip(&again);
        assert_eq!(second_value, first_value);
        assert_eq!(second_unit, first_unit);
    }
}
