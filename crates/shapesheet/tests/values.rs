//! Values, units and coercions (task P6-03, ADR 0052 and ADR 0060).
//!
//! The contract of `crates/shapesheet/src/{value,units,coerce}.rs`:
//! every unit spelling of "About Units of Measure" parses, canonical
//! conversions are exact where the factor is exact and round-trip within
//! `1e-12`, `coerce` is the one documented rule, and
//! `Value::to_cached`/`Value::from_cached` round-trip every value kind
//! (P2-14).

use ferrogram_model::{CellValue, Color};
use ferrogram_shapesheet::{
    Error, ErrorCode, Number, Span, SyntaxKind, Unit, UnitFamily, Value, ValueKind, as_bool,
    as_color, as_list, as_number, as_str, coerce, parse_unit,
};

/// Every unit the module defines.
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

/// The spellings of "About Units of Measure" (source 19): the `Use`
/// column of the units table (the localized spellings), the compound and
/// fractional abbreviations, the universal strings and the implicit
/// units. All of them must parse (acceptance criterion 1).
const SOURCE_SPELLINGS: &[&str] = &[
    // The `Use` column (localized spellings).
    "cm",
    "c",
    "date",
    "deg",
    "d",
    "ew",
    "ed",
    "eh",
    "em",
    "es",
    "ft",
    "in",
    "km",
    "m",
    "mi",
    "mm",
    "'",
    "nm",
    "%",
    "p",
    "pt",
    "rad",
    "\"",
    "yd",
    // Compound and fractional abbreviations.
    "CICERO/DIDOT",
    "°",
    "FEET/INCH",
    "PICAPOINTS",
    "IN_F",
    "MI_F",
    // Universal strings.
    "CM",
    "C",
    "DATE",
    "DEG",
    "D",
    "EW",
    "ED",
    "EH",
    "EM",
    "ES",
    "FT",
    "IN",
    "KM",
    "M",
    "MI",
    "MM",
    "NM",
    "P",
    "PT",
    "RAD",
    "YD",
    // The implicit units: universal only, no localized counterpart.
    "DL",
    "DP",
    "DT",
    "DA",
    "DE",
];

/// A tiny deterministic generator for the round-trip properties.
///
/// No `proptest` dependency here: `crates/shapesheet/Cargo.toml` is not
/// task P6-03's file (see the task Result), so the property loops use a
/// seeded xorshift and cover the same ground deterministically.
struct Rng(u64);

impl Rng {
    /// The next pseudo-random word.
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// A finite magnitude over many orders of magnitude.
    fn magnitude(&mut self) -> f64 {
        let word = self.next();
        let exponent = (word % 61) as i32 - 30;
        let sign = if word & 1 == 0 { 1.0 } else { -1.0 };
        let mantissa = 1.0 + ((word >> 8) % 1_000) as f64 / 1_000.0;
        sign * mantissa * 10f64.powi(exponent)
    }

    /// Any unit.
    fn unit(&mut self) -> Unit {
        let len = u64::try_from(ALL_UNITS.len()).unwrap_or(1);
        let pick = self.next() % len;
        ALL_UNITS[usize::try_from(pick).unwrap_or(0)]
    }

    /// A string that reads back as a string in a list entry: no `;`, no
    /// number, boolean, colour or error spelling.
    fn text(&mut self) -> String {
        let word = self.next();
        format!("t{word:x}")
    }
}

/// Asserts `got` equals `want` within `1e-12` relative (ADR 0060).
fn assert_close(got: f64, want: f64) {
    let tolerance = 1e-12 * want.abs().max(1.0);
    assert!(
        (got - want).abs() <= tolerance,
        "got {got}, want {want} (tolerance {tolerance})"
    );
}

#[test]
fn every_spelling_of_the_source_parses() {
    for spelling in SOURCE_SPELLINGS {
        assert!(
            parse_unit(spelling).is_ok(),
            "spelling {spelling:?} does not parse"
        );
    }
}

#[test]
fn spellings_parse_in_any_case() {
    for spelling in SOURCE_SPELLINGS {
        let upper = spelling.to_ascii_uppercase();
        let lower = spelling.to_ascii_lowercase();
        assert_eq!(parse_unit(&upper), parse_unit(spelling), "{spelling}");
        assert_eq!(parse_unit(&lower), parse_unit(spelling), "{spelling}");
    }
}

#[test]
fn spelling_prints_the_universal_form_and_round_trips() {
    for unit in ALL_UNITS {
        let spelling = unit.spelling().expect("spelling never fails");
        assert_eq!(parse_unit(spelling), Ok(*unit), "{unit:?}");
        if *unit != Unit::Unitsless {
            assert_eq!(
                spelling,
                spelling.to_ascii_uppercase(),
                "{unit:?} not universal"
            );
        }
    }
}

#[test]
fn area_spellings_are_out_of_scope_and_rejected_as_invalid_units() {
    for name in [
        "SQ IN", "SQ. IN.", "IN^2", "CM^2", "HA", "ACRES", "HECTARES", "SQ FT",
    ] {
        match parse_unit(name) {
            Err(Error::Syntax { span, kind }) => {
                assert_eq!(kind, SyntaxKind::InvalidUnit, "{name}");
                assert_eq!(span, Span::new(0, name.len() as u32), "{name}");
            }
            other => panic!("{name} gave {other:?}"),
        }
    }
}

#[test]
fn exact_factors_are_exact() {
    // 1 in = 72 pt and 1 deg = pi/180 rad, exactly.
    assert_eq!(Unit::Points.to_canonical(72.0), Ok(1.0));
    assert_eq!(Unit::Points.from_canonical(1.0), Ok(72.0));
    assert_eq!(
        Unit::Degrees.to_canonical(1.0),
        Ok(core::f64::consts::PI / 180.0)
    );
    assert_eq!(Unit::Radians.to_canonical(2.5), Ok(2.5));
    assert_eq!(Unit::Feet.to_canonical(2.0), Ok(24.0));
    assert_eq!(Unit::Feet.from_canonical(24.0), Ok(2.0));
    assert_eq!(Unit::Miles.to_canonical(1.0), Ok(63_360.0));
    assert_eq!(Unit::Percent.to_canonical(50.0), Ok(0.5));
    assert_eq!(Unit::ElapsedWeeks.to_canonical(1.0), Ok(604_800.0));
    assert_eq!(Unit::Unitsless.to_canonical(3.0), Ok(3.0));
}

#[test]
fn conversions_round_trip_within_1e_12_for_every_unit() {
    let mut rng = Rng(0x5EED_CAFE_F00D_D00D);
    for unit in ALL_UNITS {
        for _ in 0..64 {
            let magnitude = rng.magnitude();
            let canonical = unit
                .to_canonical(magnitude)
                .expect("to_canonical never fails");
            let back = unit
                .from_canonical(canonical)
                .expect("from_canonical never fails");
            assert_close(back, magnitude);
        }
    }
}

#[test]
fn the_geom_bridge_keeps_the_shared_set() {
    for unit in ALL_UNITS {
        if let Ok(geom) = unit.to_geom() {
            assert_eq!(Unit::from_geom(geom), Ok(*unit), "{unit:?}");
        }
    }
}

#[test]
fn units_the_model_lacks_are_conversion_errors() {
    assert_eq!(
        Unit::Didots.to_geom(),
        Err(Error::Conversion("unit has no model spelling"))
    );
    assert_eq!(
        Unit::from_geom(ferrogram_geom::Unit::Millipoints),
        Err(Error::Conversion("model unit has no formula spelling"))
    );
}

#[test]
fn families_follow_the_adr_table() {
    assert_eq!(Unit::Inches.family(), Ok(UnitFamily::Length));
    assert_eq!(Unit::Picas.family(), Ok(UnitFamily::Typographic));
    assert_eq!(Unit::Degrees.family(), Ok(UnitFamily::Angle));
    assert_eq!(Unit::Percent.family(), Ok(UnitFamily::Ratio));
    assert_eq!(Unit::ElapsedDays.family(), Ok(UnitFamily::Duration));
    assert_eq!(Unit::Unitsless.family(), Ok(UnitFamily::Dimensionless));
}

#[test]
fn sheep_is_zero_with_a_coercion_warning() {
    let coerced = coerce(&Value::Str(String::from("sheep")), ValueKind::Number);
    assert_eq!(
        coerced.value,
        Value::Number(Number::new(0.0, Unit::Unitsless))
    );
    assert!(
        coerced.defaulted,
        "a defaulted coercion earns a Coercion warning"
    );
}

#[test]
fn a_numeric_string_converts_to_its_number() {
    let coerced = coerce(&Value::Str(String::from("2")), ValueKind::Number);
    assert_eq!(
        coerced.value,
        Value::Number(Number::new(2.0, Unit::Unitsless))
    );
    assert!(!coerced.defaulted);
}

#[test]
fn a_number_to_a_string_and_a_boolean_to_a_number_follow_the_docs() {
    // `2 * 5 & " cents"` is `"10 cents"`: a number becomes its magnitude.
    let coerced = coerce(
        &Value::Number(Number::new(10.0, Unit::Unitsless)),
        ValueKind::Str,
    );
    assert_eq!(coerced.value, Value::Str(String::from("10")));
    assert!(!coerced.defaulted);
    // A boolean is the `0`/`1` the files write (ADR 0021).
    let coerced = coerce(&Value::Bool(true), ValueKind::Number);
    assert_eq!(
        coerced.value,
        Value::Number(Number::new(1.0, Unit::Unitsless))
    );
    assert!(!coerced.defaulted);
    let coerced = coerce(&Value::Bool(false), ValueKind::Number);
    assert_eq!(
        coerced.value,
        Value::Number(Number::new(0.0, Unit::Unitsless))
    );
    assert!(!coerced.defaulted);
}

#[test]
fn the_typed_views_read_what_coercion_expresses() {
    assert_eq!(
        as_number(&Value::Str(String::from("2"))),
        Ok(Number::new(2.0, Unit::Unitsless))
    );
    assert_eq!(
        as_str(&Value::Number(Number::new(10.0, Unit::Unitsless))),
        Ok(String::from("10"))
    );
    assert_eq!(
        as_bool(&Value::Number(Number::new(0.0, Unit::Unitsless))),
        Ok(false)
    );
    assert!(as_color(&Value::Number(Number::new(1.0, Unit::Unitsless))).is_err());
    assert!(as_list(&Value::Number(Number::new(1.0, Unit::Unitsless))).is_err());
}

#[test]
fn every_value_kind_round_trips_through_the_cached_form() {
    // Asserts the P2-14 round trip: `value` reads back through
    // `Value::to_cached` and `Value::from_cached` as the same value
    // kind. Numbers round-trip their canonical magnitude within `1e-12`
    // (the conversions are exact where the factor is exact); the other
    // kinds round-trip exactly.
    //
    // A closure inside the test function: `expect` and `panic!` are test
    // code (clippy.toml) and a plain helper outside `#[test]` is not.
    let assert_round_trip = |value: &Value| {
        let cached = value.to_cached().expect("to_cached never fails");
        let back = Value::from_cached(&cached.value, cached.unit).expect("from_cached round-trips");
        match (value, &back) {
            (Value::Number(before), Value::Number(after)) => {
                let canonical_before = before
                    .unit
                    .to_canonical(before.magnitude)
                    .expect("to_canonical never fails");
                let canonical_after = after
                    .unit
                    .to_canonical(after.magnitude)
                    .expect("to_canonical never fails");
                assert_close(canonical_after, canonical_before);
            }
            (Value::Str(_), Value::Str(_))
            | (Value::Bool(_), Value::Bool(_))
            | (Value::Color(_), Value::Color(_))
            | (Value::List(_), Value::List(_))
            | (Value::Err(_), Value::Err(_)) => {
                assert_eq!(&back, value, "{value:?} did not round-trip")
            }
            _ => panic!("{value:?} came back as {back:?}"),
        }
    };
    let mut rng = Rng(0x1234_5678_9ABC_DEF0);
    for _ in 0..256 {
        let unit = rng.unit();
        let number = Value::Number(Number::new(rng.magnitude(), unit));
        assert_round_trip(&number);
        assert_round_trip(&Value::Str(rng.text()));
        assert_round_trip(&Value::Bool(rng.next() & 1 == 0));
        assert_round_trip(&Value::Color(Color::new(
            (rng.next() % 256) as u8,
            (rng.next() % 256) as u8,
            (rng.next() % 256) as u8,
        )));
        assert_round_trip(&Value::List(vec![
            Value::Number(Number::new(rng.magnitude(), Unit::Unitsless)),
            Value::Str(rng.text()),
        ]));
        assert_round_trip(&Value::Err(ErrorCode::Num));
    }
}

#[test]
fn a_boolean_or_string_with_a_unit_drops_the_unit_on_write() {
    // The P2-14 fix: a unit on a non-numeric kind does not read back, so
    // the write drops it. This test reads it back (acceptance criterion).
    for cached_value in [
        CellValue::Str(String::from("x")),
        CellValue::Bool(true),
        CellValue::Color(Color::new(1, 2, 3)),
    ] {
        let value = Value::from_cached(&cached_value, ferrogram_geom::Unit::Inches)
            .expect("from_cached never fails here");
        let cached = value.to_cached().expect("to_cached never fails");
        assert_eq!(cached.unit, ferrogram_geom::Unit::Unitsless);
        assert_eq!(cached.value, cached_value);
        let back = Value::from_cached(&cached.value, ferrogram_geom::Unit::Inches)
            .expect("from_cached never fails here");
        assert_eq!(back, value);
    }
}

#[test]
fn a_non_finite_magnitude_is_num_in_both_directions() {
    let cached = Value::Number(Number::new(f64::INFINITY, Unit::Inches))
        .to_cached()
        .expect("to_cached never fails");
    assert_eq!(cached.value, CellValue::Error(String::from("#NUM!")));
    assert_eq!(
        Value::from_cached(
            &CellValue::Number(f64::NEG_INFINITY),
            ferrogram_geom::Unit::Inches
        ),
        Ok(Value::Err(ErrorCode::Num))
    );
    assert_eq!(
        Value::from_cached(&cached.value, cached.unit),
        Ok(Value::Err(ErrorCode::Num))
    );
}

#[test]
fn a_number_reads_back_in_the_unit_the_file_recorded() {
    // `PageWidth V='11.69291338582677' U='MM'` is 297 mm written in
    // inches (ADR 0011).
    let value = Value::from_cached(
        &CellValue::Number(11.69291338582677),
        ferrogram_geom::Unit::Millimeters,
    )
    .expect("from_cached never fails here");
    match value {
        Value::Number(number) => {
            assert_eq!(number.unit, Unit::Millimeters);
            assert_close(number.magnitude, 297.0);
        }
        other => panic!("wrong kind: {other:?}"),
    }
    let cached = value.to_cached().expect("to_cached never fails");
    assert_eq!(cached.unit, ferrogram_geom::Unit::Millimeters);
    assert_close(
        match cached.value {
            CellValue::Number(magnitude) => magnitude,
            other => panic!("wrong kind: {other:?}"),
        },
        11.69291338582677,
    );
}

#[test]
fn every_error_value_round_trips_its_cached_text() {
    for code in [
        ErrorCode::Div0,
        ErrorCode::Value,
        ErrorCode::Ref,
        ErrorCode::Num,
        ErrorCode::Na,
        ErrorCode::Dim,
    ] {
        let cached = Value::Err(code).to_cached().expect("to_cached never fails");
        assert_eq!(cached.value, CellValue::Error(String::from(code.as_str())));
        assert_eq!(
            Value::from_cached(&cached.value, cached.unit),
            Ok(Value::Err(code))
        );
    }
}
