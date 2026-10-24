//! Behavior of the formula and cell-reference types (task P2-10, extended
//! by task P6-02).
//!
//! These tests pin construction, the `Display`/`Debug` spellings and the
//! derived equality and hashing, so the parser and the printer can rely
//! on them. The parser's own behavior lives in `tests/parser.rs`.

// Test code: a failing `expect` is the bug under test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use larrygram_shapesheet::{BinaryOp, CellRef, Expr, ExprKind, Formula, parse};

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
fn a_cell_ref_keeps_every_part_as_written() {
    let full = CellRef {
        sheets: vec![String::from("Sheet.4")],
        section: Some(String::from("Geometry1")),
        names: vec![String::from("X2")],
    };
    assert_eq!(full.sheets, vec![String::from("Sheet.4")]);
    assert_eq!(full.section.as_deref(), Some("Geometry1"));
    assert_eq!(full.name().as_deref(), Some("X2"));
    assert_eq!(CellRef::new("PinX").name().as_deref(), Some("PinX"));
}

#[test]
fn bare_cell_refs_compare_and_hash_by_value() {
    let a = CellRef::new("PinX");
    let b = CellRef {
        names: vec![String::from("PinX")],
        ..CellRef::default()
    };
    assert_eq!(a, b);
    assert_eq!(hash(&a), hash(&b));
    assert_ne!(a, CellRef::new("PinY"));
    assert_eq!(CellRef::default().name(), None);
}

#[test]
fn parsing_reads_a_formula_into_a_tree() {
    let expr = parse("1+1").expect("the formula parses");
    assert!(matches!(
        expr.kind,
        ExprKind::Binary {
            op: BinaryOp::Add,
            ..
        }
    ));
    let empty = parse("").expect_err("an empty formula names no expression");
    assert_eq!(
        empty.to_string(),
        "syntax error at 0..0: unexpected end of formula"
    );
}

#[test]
fn an_expression_carries_its_span_and_kind() {
    let expr = Expr {
        span: larrygram_shapesheet::Span::new(0, 1),
        kind: larrygram_shapesheet::ExprKind::Literal(larrygram_shapesheet::Literal::Bool(true)),
    };
    assert_eq!(expr.span.len(), 1);
    assert!(matches!(
        expr.kind,
        larrygram_shapesheet::ExprKind::Literal(larrygram_shapesheet::Literal::Bool(true))
    ));
}

fn hash<T: Hash>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}
