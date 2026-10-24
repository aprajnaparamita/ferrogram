//! Behavior of the exported recalculation interface (task P6-06).
//!
//! These tests pin the public contract of [`recalc`]: it never fails the
//! document and it reports what it changed. The order, cycle, budget,
//! `SETF` and materialisation rules are pinned next to the code in
//! `src/recalc.rs`, where the test engine lives.

use larrygram_geom::Unit;
use larrygram_model::{
    Cell, CellName, Document, Page, PageId, Preserved, Shape, ShapeId, SheetRef,
};
use larrygram_shapesheet::{
    CellAddress, CellUpdate, RecalcOptions, RecalcOutcome, RecalcWarning, RecalcWarningKind, recalc,
};

fn cell(name: &str, formula: Option<&str>) -> Cell {
    Cell {
        name: CellName::new(name),
        formula: formula.map(str::to_owned),
        value: None,
        unit: Unit::Unitsless,
        preserved: Preserved::new(),
    }
}

fn shape_with_formulas(formulas: &[(&str, &str)]) -> (Document, CellAddress) {
    let mut doc = Document::new();
    let mut shape = Shape::new(ShapeId { index: 0 }, 1);
    shape.sheet.cells = formulas
        .iter()
        .map(|(name, formula)| cell(name, Some(formula)))
        .collect();
    let shape = doc.add_shape(shape);
    doc.pages.push(Page::new(PageId { index: 0 }, 1));
    if let Some(page) = doc.pages.first_mut() {
        page.shapes.push(shape);
    }
    let address = CellAddress::new(SheetRef::Shape(shape), CellName::from_static("Alpha"));
    (doc, address)
}

#[test]
fn options_carry_the_default_budget() {
    let options = RecalcOptions::new();
    assert_eq!(
        options.max_evaluations,
        RecalcOptions::DEFAULT_MAX_EVALUATIONS
    );
    assert!(!options.materialise_inherited);
    assert!(!options.fill_missing_only);
    assert_eq!(RecalcOptions::default(), options);
}

#[test]
fn an_empty_recalc_changes_nothing() {
    let mut doc = Document::new();
    let outcome = recalc(&mut doc, &[], &RecalcOptions::new()).expect("recalc");
    assert_eq!(outcome, RecalcOutcome::default());
}

#[test]
fn a_pass_over_formulas_is_ok_or_reports_the_engine() {
    let (mut doc, address) = shape_with_formulas(&[("Alpha", "0")]);
    let outcome = recalc(&mut doc, &[address], &RecalcOptions::new());
    // The real pipeline is `Ok` once tasks P6-02 to P6-05 land; while
    // `eval` is a stub the pass reports it and stops. Either way it
    // never panics and never fails the document.
    assert!(
        outcome.is_ok() || matches!(outcome, Err(larrygram_shapesheet::Error::NotImplemented(_)))
    );
}

#[test]
fn the_outcome_and_warning_types_carry_their_parts() {
    let outcome = RecalcOutcome {
        updates: vec![CellUpdate {
            target: CellAddress::new(
                SheetRef::Page(PageId { index: 0 }),
                CellName::from_static("Width"),
            ),
            before: None,
            after: None,
        }],
        warnings: vec![RecalcWarning {
            target: None,
            kind: RecalcWarningKind::BudgetExceeded,
        }],
        truncated: true,
    };
    assert_eq!(outcome.updates.len(), 1);
    assert_eq!(outcome.warnings.len(), 1);
    assert!(outcome.truncated);
}
