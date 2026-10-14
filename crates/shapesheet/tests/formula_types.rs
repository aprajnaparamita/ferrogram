//! Behavior of the formula and cell-reference types (task P2-10).
//!
//! They are value types until the formula engine lands (ADR 0014): these
//! tests pin construction, the `Display`/`Debug` spellings and the derived
//! equality and hashing, so a later parser can rely on them.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use ferrogram_shapesheet::{CellRef, Error, Expr, Formula, parse};

#[test]
fn a_formula_keeps_its_source_verbatim() {
    let formula = Formula::new(String::from("0.5 in"));
    assert_eq!(formula.as_str(), "0.5 in");
    assert_eq!(formula.to_string(), "0.5 in");
    // `Display` never adds the leading `=`: the source text is stored as
    // written without it.
    assert!(!formula.to_string().starts_with('='));
}

#[test]
fn formulas_compare_and_hash_by_source() {
    let a = Formula::new("2");
    let b = Formula::new(String::from("2"));
    let c = Formula::new("2.0");
    assert_eq!(a, b);
    assert_ne!(a, c);
    assert_eq!(hash(&a), hash(&b));
    assert_eq!(a.clone(), a);
}

#[test]
fn a_cell_ref_keeps_every_part() {
    let full = CellRef {
        sheet: Some("Sheet.4".to_owned()),
        section: Some("Geometry".to_owned()),
        row: Some("Row_2".to_owned()),
        cell: "X".to_owned(),
    };
    assert_eq!(full.sheet.as_deref(), Some("Sheet.4"));
    assert_eq!(full.section.as_deref(), Some("Geometry"));
    assert_eq!(full.row.as_deref(), Some("Row_2"));
    assert_eq!(full.cell, "X");
    assert_eq!(CellRef::new("PinX").cell, "PinX");
}

#[test]
fn bare_cell_refs_compare_and_hash_by_value() {
    let a = CellRef::new("PinX");
    let b = CellRef {
        cell: "PinX".to_owned(),
        ..CellRef::default()
    };
    assert_eq!(a, b);
    assert_eq!(hash(&a), hash(&b));
    assert_ne!(a, CellRef::new("PinY"));
    assert_eq!(CellRef::default(), CellRef::new(""));
}

#[test]
fn parsing_still_reports_the_stub_error() {
    assert_eq!(parse("1+1"), Err(Error::NotImplemented("parse")));
    assert_eq!(
        parse("").unwrap_err().to_string(),
        "not implemented: parse".to_owned()
    );
}

#[test]
fn an_expression_is_opaque_until_the_parser_lands() {
    // `Expr` has no public constructor while `parse` is a stub; its `Debug`
    // spelling is the only surface a test can pin today.
    let no_expr: Option<Expr> = None;
    assert_eq!(format!("{no_expr:?}"), "None");
}

fn hash<T: Hash>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}
