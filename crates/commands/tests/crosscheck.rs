//! The cached-value cross-check (task P6-10, ADR 0059).
//!
//! KICKOFF's rule, in code: "where every function in a formula is
//! supported, our result must match the stored value." The harness runs
//! over every `corpus/vsdx` file (the fetched corpus, ADR 0005; the
//! test self-skips with a message when it is absent and fails instead
//! when `FERROGRAM_REQUIRE_CORPUS=1` is set) and prints one
//! machine-readable block per file plus a total, the skip counts per
//! reason and one line per disagreement (ADR 0059's report format).
//! Run `cargo test -p ferrogram-commands --test crosscheck -- --nocapture`
//! to see it.
//!
//! ## Which cells are scanned
//!
//! A cell is scanned when it carries a formula string (`F=`) that is not
//! the `Inh` marker. ADR 0059 rule 1 excludes `Inh` ("nothing to
//! evaluate"): the corpus writes one on every inherited cell, and the
//! engine's inheritance rules (ADR 0054) are tested by the recalc tests,
//! not here. The scanned count is the `formulas` column, and the corpus
//! holds 11 903 of them (the denominator ADR 0053's 78.7 % uses).
//!
//! ## Why a scanned cell is skipped (by reason)
//!
//! In ADR 0059's test order:
//!
//! 1. `no-formula` — the formula is a `No Formula` marker (the file's
//!    spelling for "this cell has no formula") or empty: nothing to
//!    evaluate.
//! 2. `does-not-parse` — the parser rejects it (the `FORMATEX` calls
//!    with omitted arguments, P6-02 deviation 6).
//! 3. `unknown-function` — a call outside the evaluated set
//!    (ADR 0053); the engine keeps the cached value.
//! 4. `missing-reference` — a reference resolves to no cell with a
//!    value (ADR 0054).
//! 5. `no-cached-value` / `cached-error` — the file caches nothing, or
//!    caches an error value (nothing to agree with).
//!
//! Every other cell is **counted**: the formula is evaluated through the
//! production engine ([`DocumentResolver`]) and its value is compared
//! with the cached one by ADR 0059's table (numbers within a relative
//! 1e-9 with a 1e-9 absolute floor, everything else exact). One
//! spelling bridge is part of the table here: a boolean caches in the
//! file as the number `0`/`1` (the `V=` of a `BOOL` cell without a
//! `U="BOOL"` marker), so an evaluated `Bool` agrees with the cached
//! `Number(0.0)`/`Number(1.0)` exactly (and the other way round). A
//! disagreement fails the test with the cell address, the formula and
//! both values in the message; a divergence we accept is listed in
//! [`EXCLUSIONS`] (one entry per line of ADR 0059's "Documented
//! exclusions") and skipped as `documented-exclusion` instead.
//!
//! Determinism (ADR 0055) makes the run reproducible: two runs over the
//! same files report the same numbers, pinned by a test below.

// The never-crash lints apply to non-test code only (clippy.toml).
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

#[path = "crosscheck/mod.rs"]
mod helpers;

use std::collections::BTreeMap;

use ferrogram_model::{CellValue, Document};
use ferrogram_shapesheet::{
    CellAddress, DepGraph, DocumentEngine, DocumentResolver, ErrorCode, Expr, ExprKind, Formula,
    RecalcOptions, Value, eval_formula, is_supported, parse_formula, recalc_with,
};

use helpers::{LocatedCell, corpus_files, located_cells, read_corpus_file, skip_without_corpus};

// ---------------------------------------------------------------------------
// The comparison rules (ADR 0059's "How values are compared").
// ---------------------------------------------------------------------------

/// The relative tolerance on a number (ADR 0059).
const REL_TOLERANCE: f64 = 1e-9;

/// The absolute floor for near-zero numbers (ADR 0059).
const ABS_FLOOR: f64 = 1e-9;

/// ADR 0059's "Documented exclusions" section, mirrored here one entry
/// per line: the corpus file, the cell address (in [`helpers`]'
/// spelling) and the reason. The harness prints an excluded divergence
/// and counts it as `documented-exclusion`.
struct Exclusion {
    /// The corpus file name (`blue-box.vsdx`).
    file: &'static str,
    /// The cell address, in [`helpers`]' spelling (`shape1!Geometry1.X2`).
    address: &'static str,
    /// The reason the divergence is accepted (one line, ADR 0059).
    reason: &'static str,
}

/// ADR 0059's "Documented exclusions" section, one entry per line of
/// the ADR (task P6-10; six causes, 32 corpus cells).
const EXCLUSIONS: &[Exclusion] = &[
    Exclusion {
        file: "bgcolor.vsdx",
        address: "shape3!TextBkgnd",
        reason: "colour arithmetic: Visio caches the RGB unchanged where our evaluator coerces the colour to a number first (ADR 0052 has no colour arithmetic)",
    },
    Exclusion {
        file: "tdf154379-QuickStyleFillMatrix.vsdx",
        address: "page0!Actions1.Action1",
        reason: "`SETF` takes a `GetRef(...)` target (ADR 0053's basic SETF); the string-literal target form is refused and an action cell caches 0",
    },
    Exclusion {
        file: "tdf154379-QuickStyleFillMatrix.vsdx",
        address: "shape17!Property1.LangID1",
        reason: "Visio maps an LCID number to a locale tag in a `LangID` cell; ADR 0052 has no such coercion, so we keep the number",
    },
    Exclusion {
        file: "tdf154379-QuickStyleFillMatrix.vsdx",
        address: "shape17!Property1.LangID2",
        reason: "Visio maps an LCID number to a locale tag in a `LangID` cell; ADR 0052 has no such coercion, so we keep the number",
    },
    Exclusion {
        file: "tdf154379-QuickStyleFillMatrix.vsdx",
        address: "shape19!Character1.LangID0",
        reason: "Visio maps an LCID number to a locale tag in a `LangID` cell; ADR 0052 has no such coercion, so we keep the number",
    },
    Exclusion {
        file: "tdf154379-QuickStyleFillMatrix.vsdx",
        address: "shape20!Character1.LangID0",
        reason: "Visio maps an LCID number to a locale tag in a `LangID` cell; ADR 0052 has no such coercion, so we keep the number",
    },
    Exclusion {
        file: "tdf154379-QuickStyleFillMatrix.vsdx",
        address: "shape21!Character1.LangID0",
        reason: "Visio maps an LCID number to a locale tag in a `LangID` cell; ADR 0052 has no such coercion, so we keep the number",
    },
    Exclusion {
        file: "tdf154379-QuickStyleFillMatrix.vsdx",
        address: "shape22!Character1.LangID0",
        reason: "Visio maps an LCID number to a locale tag in a `LangID` cell; ADR 0052 has no such coercion, so we keep the number",
    },
    Exclusion {
        file: "tdf154379-QuickStyleFillMatrix.vsdx",
        address: "shape23!Character1.LangID0",
        reason: "Visio maps an LCID number to a locale tag in a `LangID` cell; ADR 0052 has no such coercion, so we keep the number",
    },
    Exclusion {
        file: "tdf154379-QuickStyleFillMatrix.vsdx",
        address: "shape24!Character1.LangID0",
        reason: "Visio maps an LCID number to a locale tag in a `LangID` cell; ADR 0052 has no such coercion, so we keep the number",
    },
    Exclusion {
        file: "tdf154379-QuickStyleFillMatrix.vsdx",
        address: "shape25!Character1.LangID0",
        reason: "Visio maps an LCID number to a locale tag in a `LangID` cell; ADR 0052 has no such coercion, so we keep the number",
    },
    Exclusion {
        file: "tdf154379-QuickStyleFillMatrix.vsdx",
        address: "shape67!EventDblClick",
        reason: "`NA()` is `#N/A!` by definition; Visio caches 0 on an event cell whatever its formula evaluates to",
    },
    Exclusion {
        file: "tdf154379-QuickStyleFillMatrix.vsdx",
        address: "shape69!EventDblClick",
        reason: "`NA()` is `#N/A!` by definition; Visio caches 0 on an event cell whatever its formula evaluates to",
    },
    Exclusion {
        file: "tdf154379-QuickStyleFillMatrix.vsdx",
        address: "shape70!EventDblClick",
        reason: "`NA()` is `#N/A!` by definition; Visio caches 0 on an event cell whatever its formula evaluates to",
    },
    Exclusion {
        file: "tdf154379-QuickStyleFillMatrix.vsdx",
        address: "shape71!EventDblClick",
        reason: "`NA()` is `#N/A!` by definition; Visio caches 0 on an event cell whatever its formula evaluates to",
    },
    Exclusion {
        file: "tdf154379-QuickStyleFillMatrix.vsdx",
        address: "shape73!EventDblClick",
        reason: "`NA()` is `#N/A!` by definition; Visio caches 0 on an event cell whatever its formula evaluates to",
    },
    Exclusion {
        file: "tdf154379-QuickStyleFillMatrix.vsdx",
        address: "shape84!Actions1.Action1",
        reason: "`SETF` takes a `GetRef(...)` target (ADR 0053's basic SETF); the string-literal target form is refused and an action cell caches 0",
    },
    Exclusion {
        file: "tdf154379-QuickStyleFillMatrix.vsdx",
        address: "shape84!Actions1.Action2",
        reason: "`SETF` takes a `GetRef(...)` target (ADR 0053's basic SETF); the string-literal target form is refused and an action cell caches 0",
    },
    Exclusion {
        file: "tdf154379-QuickStyleFillMatrix.vsdx",
        address: "shape84!Actions1.Action3",
        reason: "`SETF` takes a `GetRef(...)` target (ADR 0053's basic SETF); the string-literal target form is refused and an action cell caches 0",
    },
    Exclusion {
        file: "tdf154379-QuickStyleFillMatrix.vsdx",
        address: "shape84!Actions1.Action4",
        reason: "`SETF` takes a `GetRef(...)` target (ADR 0053's basic SETF); the string-literal target form is refused and an action cell caches 0",
    },
    Exclusion {
        file: "testfile1.vsdx",
        address: "shape2!FillBkgndTrans",
        reason: "the theme-driven `LineColorTrans` caches the `Themed` marker the model keeps (ADR 0021 resolves theme values in the renderer)",
    },
    Exclusion {
        file: "testfile1.vsdx",
        address: "shape2!FillForegndTrans",
        reason: "the theme-driven `LineColorTrans` caches the `Themed` marker the model keeps (ADR 0021 resolves theme values in the renderer)",
    },
    Exclusion {
        file: "testfile1.vsdx",
        address: "shape4!Character1.Size0",
        reason: "Visio clamps a character size to 1 pt (the file caches 0.0139 in) where the formula computes 0",
    },
    Exclusion {
        file: "testfile3.vsdx",
        address: "shape2!FillBkgndTrans",
        reason: "the theme-driven `LineColorTrans` caches the `Themed` marker the model keeps (ADR 0021 resolves theme values in the renderer)",
    },
    Exclusion {
        file: "testfile3.vsdx",
        address: "shape2!FillForegndTrans",
        reason: "the theme-driven `LineColorTrans` caches the `Themed` marker the model keeps (ADR 0021 resolves theme values in the renderer)",
    },
    Exclusion {
        file: "testfile3.vsdx",
        address: "shape4!Character1.Size0",
        reason: "Visio clamps a character size to 1 pt (the file caches 0.0139 in) where the formula computes 0",
    },
    Exclusion {
        file: "testfile4.vsdx",
        address: "shape0!FillBkgndTrans",
        reason: "the theme-driven `LineColorTrans` caches the `Themed` marker the model keeps (ADR 0021 resolves theme values in the renderer)",
    },
    Exclusion {
        file: "testfile4.vsdx",
        address: "shape0!FillForegndTrans",
        reason: "the theme-driven `LineColorTrans` caches the `Themed` marker the model keeps (ADR 0021 resolves theme values in the renderer)",
    },
    Exclusion {
        file: "testfile4.vsdx",
        address: "shape2!Character1.Size0",
        reason: "Visio clamps a character size to 1 pt (the file caches 0.0139 in) where the formula computes 0",
    },
    Exclusion {
        file: "testfile6.vsdx",
        address: "shape1!FillBkgndTrans",
        reason: "the theme-driven `LineColorTrans` caches the `Themed` marker the model keeps (ADR 0021 resolves theme values in the renderer)",
    },
    Exclusion {
        file: "testfile6.vsdx",
        address: "shape1!FillForegndTrans",
        reason: "the theme-driven `LineColorTrans` caches the `Themed` marker the model keeps (ADR 0021 resolves theme values in the renderer)",
    },
    Exclusion {
        file: "testfile6.vsdx",
        address: "shape3!Character1.Size0",
        reason: "Visio clamps a character size to 1 pt (the file caches 0.0139 in) where the formula computes 0",
    },
];

/// The exclusion `label` is listed under for `file`, if any.
fn exclusion_for(file: &str, label: &str) -> Option<&'static Exclusion> {
    EXCLUSIONS
        .iter()
        .find(|entry| entry.file == file && entry.address == label)
}

/// ADR 0059's number rule: relative difference at most
/// [`REL_TOLERANCE`], with an [`ABS_FLOOR`] absolute floor.
fn numbers_agree(cached: f64, evaluated: f64) -> bool {
    let difference = (cached - evaluated).abs();
    let tolerance = ABS_FLOOR.max(REL_TOLERANCE * cached.abs().max(evaluated.abs()));
    difference <= tolerance
}

/// ADR 0059's table, as "why these two disagree" (`None` = agree).
///
/// A boolean is bridged over the file's spelling (the module docs): a
/// cached `0`/`1` is the `FALSE`/`TRUE` a `BOOL` cell caches when the
/// file does not spell `U="BOOL"`.
fn disagreement(cached: &CellValue, evaluated: &CellValue) -> Option<String> {
    match (cached, evaluated) {
        (CellValue::Number(a), CellValue::Number(b)) => {
            (!numbers_agree(*a, *b)).then(|| format!("abs diff {:e}", (a - b).abs()))
        }
        (CellValue::Number(a), CellValue::Bool(flag))
        | (CellValue::Bool(flag), CellValue::Number(a)) => {
            let spelled = if *flag { 1.0 } else { 0.0 };
            (!numbers_agree(*a, spelled)).then(|| format!("bool {flag} does not spell {a}"))
        }
        (CellValue::Str(a), CellValue::Str(b)) => (a != b).then(|| format!("str {a:?} != {b:?}")),
        (CellValue::Bool(a), CellValue::Bool(b)) => (a != b).then(|| format!("bool {a} != {b}")),
        (CellValue::Color(a), CellValue::Color(b)) => {
            (a != b).then(|| format!("color {a:?} != {b:?}"))
        }
        (CellValue::List(a), CellValue::List(b)) => list_disagreement(a, b),
        (CellValue::Error(a), CellValue::Error(b)) => {
            (a != b).then(|| format!("error {a:?} != {b:?}"))
        }
        (a, b) => Some(format!("kind mismatch: cached {a:?}, evaluated {b:?}")),
    }
}

/// Element-wise list comparison (ADR 0059): the same rules per element,
/// where two elements that both parse as numbers use the number rule.
fn list_disagreement(cached: &str, evaluated: &str) -> Option<String> {
    let cached: Vec<&str> = cached.split(';').map(str::trim).collect();
    let evaluated: Vec<&str> = evaluated.split(';').map(str::trim).collect();
    if cached.len() != evaluated.len() {
        return Some(format!(
            "list length {} != {}",
            cached.len(),
            evaluated.len()
        ));
    }
    for (index, (a, b)) in cached.iter().zip(evaluated.iter()).enumerate() {
        match (a.parse::<f64>(), b.parse::<f64>()) {
            (Ok(a), Ok(b)) => {
                if !numbers_agree(a, b) {
                    return Some(format!(
                        "list element {index}: abs diff {:e}",
                        (a - b).abs()
                    ));
                }
            }
            _ => {
                if a != b {
                    return Some(format!("list element {index}: {a:?} != {b:?}"));
                }
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// The classification of one scanned cell.
// ---------------------------------------------------------------------------

/// Why a scanned cell is not counted (ADR 0059's reasons).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum SkipReason {
    /// A `No Formula` marker or an empty formula.
    NoFormula,
    /// The parser rejects the formula.
    DoesNotParse,
    /// A call outside the evaluated set (ADR 0053).
    UnknownFunction,
    /// A reference to no cell with a value (ADR 0054).
    MissingReference,
    /// The file caches nothing.
    NoCachedValue,
    /// The file caches an error value.
    CachedError,
    /// An accepted divergence (ADR 0059's exclusions).
    DocumentedExclusion,
}

impl SkipReason {
    /// Every reason, in report order.
    const ALL: [SkipReason; 7] = [
        SkipReason::NoFormula,
        SkipReason::DoesNotParse,
        SkipReason::UnknownFunction,
        SkipReason::MissingReference,
        SkipReason::NoCachedValue,
        SkipReason::CachedError,
        SkipReason::DocumentedExclusion,
    ];

    /// The name the report prints (ADR 0059's spelling).
    const fn as_str(self) -> &'static str {
        match self {
            SkipReason::NoFormula => "no-formula",
            SkipReason::DoesNotParse => "does-not-parse",
            SkipReason::UnknownFunction => "unknown-function",
            SkipReason::MissingReference => "missing-reference",
            SkipReason::NoCachedValue => "no-cached-value",
            SkipReason::CachedError => "cached-error",
            SkipReason::DocumentedExclusion => "documented-exclusion",
        }
    }
}

/// One counted cell that disagrees (the report's per-disagreement line).
#[derive(Debug, Clone, PartialEq)]
struct Disagreement {
    /// The corpus file name.
    file: String,
    /// The cell address (ADR 0059's failure message).
    label: String,
    /// The formula source text.
    formula: String,
    /// The cached value, spelled for the report.
    cached: String,
    /// The evaluated value, spelled for the report.
    evaluated: String,
    /// Why the two disagree.
    difference: String,
}

impl Disagreement {
    /// The one-line report spelling (ADR 0059: "cell address, formula,
    /// cached value, evaluated value, difference").
    fn line(&self) -> String {
        format!(
            "disagree {} {} F={} cached={} evaluated={} ({})",
            self.file, self.label, self.formula, self.cached, self.evaluated, self.difference
        )
    }
}

/// The per-file (and total) tally of the report.
#[derive(Debug, Clone, Default, PartialEq)]
struct Tally {
    /// Scanned cells (the `formulas` column).
    formulas: usize,
    /// Counted cells.
    counted: usize,
    /// Counted cells that agree within tolerance.
    agree: usize,
    /// Counted cells that disagree (empty for the corpus).
    disagree: Vec<Disagreement>,
    /// Skipped cells per reason.
    skips: BTreeMap<&'static str, usize>,
}

impl Tally {
    /// Adds `other` to this tally (the `total` row).
    fn add(&mut self, other: &Tally) {
        self.formulas = self.formulas.saturating_add(other.formulas);
        self.counted = self.counted.saturating_add(other.counted);
        self.agree = self.agree.saturating_add(other.agree);
        self.disagree.extend(other.disagree.iter().cloned());
        for (reason, count) in &other.skips {
            *self.skips.entry(reason).or_insert(0) = self
                .skips
                .get(reason)
                .copied()
                .unwrap_or(0)
                .saturating_add(*count);
        }
    }

    /// Records one skip.
    fn skip(&mut self, reason: SkipReason) {
        *self.skips.entry(reason.as_str()).or_insert(0) += 1;
    }

    /// The `skipped` column: every skip, per reason.
    fn skipped(&self) -> usize {
        self.skips.values().sum()
    }

    /// The `rate` column: agreeing counted cells as a percentage.
    fn rate(&self) -> String {
        if self.counted == 0 {
            return String::from("n/a");
        }
        format!("{:.1}%", 100.0 * self.agree as f64 / self.counted as f64)
    }
}

/// One scanned cell: everything the classifier needs, owned so the
/// document can be borrowed mutably by the resolver.
#[derive(Debug, Clone)]
struct Candidate {
    /// The engine address.
    #[allow(dead_code)]
    address: CellAddress,
    /// The report label (the cell address).
    label: String,
    /// The formula source text.
    formula: String,
    /// The cached value of the file.
    cached: Option<CellValue>,
}

/// The scan set (the module docs): a formula that is not the `Inh`
/// marker.
fn candidates(document: &Document) -> Vec<Candidate> {
    located_cells(document)
        .into_iter()
        .filter_map(
            |LocatedCell {
                 address,
                 label,
                 cell,
             }| {
                let formula = cell.formula.clone()?;
                if formula.trim().eq_ignore_ascii_case("Inh") {
                    return None;
                }
                Some(Candidate {
                    address,
                    label,
                    formula,
                    cached: cell.value.clone(),
                })
            },
        )
        .collect()
}

/// The function names `expr` calls and the references it makes, in one
/// iterative walk (no recursion on input).
fn calls_and_refs(expr: &Expr) -> (Vec<String>, Vec<ferrogram_shapesheet::CellRef>) {
    let mut calls = Vec::new();
    let mut refs = Vec::new();
    let mut stack: Vec<&Expr> = vec![expr];
    while let Some(next) = stack.pop() {
        match &next.kind {
            ExprKind::Literal(_) => {}
            ExprKind::Ref(reference) => refs.push(reference.clone()),
            ExprKind::Unary { operand, .. } => stack.push(operand),
            ExprKind::Binary { left, right, .. } => {
                stack.push(right);
                stack.push(left);
            }
            ExprKind::Call { name, args } => {
                calls.push(name.clone());
                stack.extend(args.iter());
            }
            // `ExprKind` is non-exhaustive; a future node kind holds no
            // new call or reference this walk must see.
            _ => {}
        }
    }
    (calls, refs)
}

/// The `No Formula` marker: the file's spelling for "this cell has no
/// formula" (it parses to nothing).
fn is_no_formula_marker(text: &str) -> bool {
    text.is_empty() || text.eq_ignore_ascii_case("No Formula")
}

/// What the classifier decided about one scanned cell.
enum Class {
    /// Counted and agreeing.
    Agree,
    /// Counted and disagreeing (or an evaluation error).
    Disagree(Disagreement),
    /// Skipped, by reason.
    Skip(SkipReason),
}

/// Classifies one scanned cell against the document (ADR 0059's tests,
/// in order: formula, functions, references, cached value).
fn classify(document: &mut Document, file: &str, candidate: &Candidate) -> Class {
    let text = candidate.formula.trim();
    if is_no_formula_marker(text) {
        return Class::Skip(SkipReason::NoFormula);
    }
    let expr = match parse_formula(&Formula::new(text)) {
        Ok(expr) => expr,
        Err(_) => return Class::Skip(SkipReason::DoesNotParse),
    };
    let (calls, refs) = calls_and_refs(&expr);
    if calls.iter().any(|name| !is_supported(name)) {
        return Class::Skip(SkipReason::UnknownFunction);
    }
    let mut resolver = DocumentResolver::new(document);
    let mut missing = false;
    for reference in &refs {
        let resolved = match resolver.resolve(candidate_sheet(candidate), reference) {
            Ok(resolved) => resolved,
            Err(_) => {
                missing = true;
                break;
            }
        };
        // ADR 0059 rule 3: the reference must resolve to a cell that
        // has a value. A name only the ShapeSheet table knows resolves
        // to its hard default (`DropOnPageScale`, `LockTextEdit`): no
        // cell of the document holds it, so there is nothing to agree
        // with (ADR 0054 names the rungs; the hard default is the
        // "nothing defines it" fallback).
        let unusable = match resolved.value {
            None => true,
            Some(Value::Err(ErrorCode::Ref)) => true,
            Some(_) => resolved.source == ferrogram_model::ValueSource::HardDefault,
        };
        if unusable {
            missing = true;
            break;
        }
    }
    if missing {
        return Class::Skip(SkipReason::MissingReference);
    }
    let Some(cached) = candidate.cached.clone() else {
        return Class::Skip(SkipReason::NoCachedValue);
    };
    if matches!(cached, CellValue::Error(_)) {
        return Class::Skip(SkipReason::CachedError);
    }
    let evaluated = match eval_formula(
        &Formula::new(text),
        candidate_sheet(candidate),
        &mut resolver,
    ) {
        Ok(value) => value,
        Err(error) => {
            return Class::Disagree(Disagreement {
                file: file.to_owned(),
                label: candidate.label.clone(),
                formula: candidate.formula.clone(),
                cached: show(&cached),
                evaluated: String::from("<evaluation error>"),
                difference: format!("{error:?}"),
            });
        }
    };
    let evaluated = match evaluated.to_cached() {
        Ok(converted) => converted.value,
        Err(error) => {
            return Class::Disagree(Disagreement {
                file: file.to_owned(),
                label: candidate.label.clone(),
                formula: candidate.formula.clone(),
                cached: show(&cached),
                evaluated: String::from("<conversion error>"),
                difference: format!("{error:?}"),
            });
        }
    };
    match disagreement(&cached, &evaluated) {
        None => Class::Agree,
        Some(difference) => {
            let entry = Disagreement {
                file: file.to_owned(),
                label: candidate.label.clone(),
                formula: candidate.formula.clone(),
                cached: show(&cached),
                evaluated: show(&evaluated),
                difference,
            };
            if let Some(exclusion) = exclusion_for(file, &candidate.label) {
                println!("{} [documented: {}]", entry.line(), exclusion.reason);
                Class::Skip(SkipReason::DocumentedExclusion)
            } else {
                Class::Disagree(entry)
            }
        }
    }
}

/// The sheet a scanned cell lives on.
fn candidate_sheet(candidate: &Candidate) -> ferrogram_model::SheetRef {
    candidate.address.sheet
}

/// The report spelling of a cached value.
fn show(value: &CellValue) -> String {
    match value {
        CellValue::Number(number) => format!("{number}"),
        CellValue::Str(text) => format!("{text:?}"),
        CellValue::Bool(flag) => format!("{flag}"),
        CellValue::Color(color) => format!("#{:02X}{:02X}{:02X}", color.r, color.g, color.b),
        CellValue::List(entries) => format!("{entries:?}"),
        CellValue::Error(text) => text.clone(),
        other => format!("{other:?}"),
    }
}

// ---------------------------------------------------------------------------
// The harness over one file and the report block.
// ---------------------------------------------------------------------------

/// Runs the cross-check over one corpus file.
fn run_file(path: &std::path::Path, document: &mut Document) -> Tally {
    let file = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut tally = Tally::default();
    for candidate in candidates(document) {
        tally.formulas = tally.formulas.saturating_add(1);
        match classify(document, &file, &candidate) {
            Class::Agree => {
                tally.counted = tally.counted.saturating_add(1);
                tally.agree = tally.agree.saturating_add(1);
            }
            Class::Disagree(entry) => {
                tally.counted = tally.counted.saturating_add(1);
                tally.disagree.push(entry);
            }
            Class::Skip(reason) => tally.skip(reason),
        }
    }
    tally
}

/// Prints one report row (ADR 0059's table).
fn print_row(file: &str, tally: &Tally) {
    println!(
        "{file:<36} {:>7} {:>7} {:>7} {:>8} {:>8} {:>7}",
        tally.formulas,
        tally.counted,
        tally.agree,
        tally.disagree.len(),
        tally.skipped(),
        tally.rate(),
    );
}

/// Prints the whole ADR 0059 block: the per-file table with its total,
/// the skip counts per reason and the disagreement lines (capped at 50).
fn print_report(rows: &[(String, Tally)], total: &Tally) {
    println!(
        "{:<36} {:>7} {:>7} {:>7} {:>8} {:>8} {:>7}",
        "file", "formulas", "counted", "agree", "disagree", "skipped", "rate"
    );
    for (file, tally) in rows {
        print_row(file, tally);
    }
    print_row("total", total);
    println!("skips by reason:");
    for reason in SkipReason::ALL {
        println!(
            "  {:<22} {:>7}",
            reason.as_str(),
            total.skips.get(reason.as_str()).copied().unwrap_or(0)
        );
    }
    let mut shown = 0usize;
    for entry in &total.disagree {
        if shown >= 50 {
            break;
        }
        println!("{}", entry.line());
        shown = shown.saturating_add(1);
    }
    if total.disagree.len() > shown {
        println!(
            "... {} more disagreements not shown",
            total.disagree.len().saturating_sub(shown)
        );
    }
}

/// Runs the whole harness and returns the per-file rows and the total.
///
/// This is the reproducible run of ADR 0059: the same files give the
/// same rows (determinism, ADR 0055), so the phase report's numbers can
/// be regenerated with one command.
fn run_all() -> (Vec<(String, Tally)>, Tally) {
    let files = corpus_files();
    assert_eq!(
        files.len(),
        15,
        "the cross-check runs over the 15 corpus files of corpus/PROVENANCE.md"
    );
    let mut rows = Vec::new();
    let mut total = Tally::default();
    for path in &files {
        let file = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let mut document = read_corpus_file(path);
        let tally = run_file(path, &mut document);
        total.add(&tally);
        rows.push((file, tally));
    }
    (rows, total)
}

/// The cross-check itself: every counted cell of every corpus file
/// agrees with its cached value within the ADR 0059 tolerance, and the
/// report block is printed (ADR 0059's format).
#[test]
fn every_counted_corpus_cell_agrees_with_its_cached_value() {
    let files = corpus_files();
    if skip_without_corpus(&files) {
        return;
    }
    let (rows, total) = run_all();
    print_report(&rows, &total);
    let mut message = String::from("counted cells disagree with their cached values:\n");
    for entry in total.disagree.iter().take(50) {
        message.push_str(&entry.line());
        message.push('\n');
    }
    assert!(total.disagree.is_empty(), "{}", message.trim_end());
}

/// ADR 0055's determinism rule: a second run over the same files
/// reports the same numbers.
#[test]
fn the_cross_check_run_is_deterministic() {
    let files = corpus_files();
    if skip_without_corpus(&files) {
        return;
    }
    let (first_rows, first_total) = run_all();
    let (second_rows, second_total) = run_all();
    assert_eq!(first_rows, second_rows, "the per-file rows repeat exactly");
    assert_eq!(first_total, second_total, "the total row repeats exactly");
}

/// `recursion-cycle.vsdx` opens and recalcs without a warning and
/// without a hang (ADR 0055). It holds no formulas at all: its
/// "recursion" is a page content relationship that points at `pages.xml`
/// itself, so P6-06's hand-built cycle is the formula-cycle test
/// (P6-06 open issue 6) and this file must simply survive the engine.
#[test]
fn recursion_cycle_opens_and_recalcs_without_a_warning_or_a_hang() {
    let files = corpus_files();
    if skip_without_corpus(&files) {
        return;
    }
    let mut document = read_corpus_file(&helpers::corpus_dir().join("recursion-cycle.vsdx"));
    let cells = located_cells(&document);
    let formulas = cells
        .iter()
        .filter(|entry| entry.cell.formula.is_some())
        .count();
    assert_eq!(formulas, 0, "the file holds no formulas to evaluate");

    // The open path of the app: the fill-in pass over a fresh session
    // (ADR 0057) records nothing and warns about nothing.
    let mut ctx =
        ferrogram_commands::app::AppCtx::new(ferrogram_commands::app::Settings::default());
    let index = ctx.open_document(ferrogram_commands::app::DocumentSession::new(
        document.clone(),
        None,
    ));
    let session = ctx.documents.get(index).expect("session").id;
    let report = ferrogram_commands::recalc::fill_missing(&mut ctx, session).expect("fill-in");
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    assert_eq!(report.updated, 0, "nothing to fill in");
    assert!(ctx.warnings.is_empty(), "{:?}", ctx.warnings);

    // The engine's own pass over every cell: it must terminate with no
    // cycle warning and change nothing (a hang is the test timing out).
    let mut graph = DepGraph::build(&document).expect("graph");
    let dirty: Vec<CellAddress> = cells.iter().map(|entry| entry.address.clone()).collect();
    let outcome = recalc_with(
        &mut document,
        &dirty,
        &RecalcOptions::new(),
        &mut graph,
        &mut DocumentEngine,
    )
    .expect("recalc");
    assert!(outcome.warnings.is_empty(), "{:?}", outcome.warnings);
    assert!(outcome.updates.is_empty(), "{:?}", outcome.updates);
    assert!(!outcome.truncated, "the pass completed");
}
