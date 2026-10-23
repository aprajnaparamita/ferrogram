//! `data.*` commands (task P6-18): the Shape Data panel's edits (ADR
//! 0035's reserved ids, ADR 0058).
//!
//! - `data.set.property` — `row` names the Shape Data row (its `N`
//!   name, or `Row_<n>` for the row at 1-based position `n`) and
//!   `value` is the value's own spelling. The command writes
//!   `Prop.<row>.Value` through [`CellEdit::direct`] (a direct user
//!   edit, ADR 0056: it replaces a `GUARD`-ed formula), returns one
//!   undo step and gets the recalc pass of every other cell edit
//!   ([`AppCtx::push_edit`](crate::AppCtx::push_edit), ADR 0056).
//!   Programmatic calls never open dialogs (AGENTS.md section 6).
//!
//! # How `value` is read (ADR 0058: "the cached value wins")
//!
//! The kind the row's `Value` cell caches decides how the text is read:
//!
//! - **Number** — one literal of the formula grammar (`2.5`, `2.5 in`,
//!   `37%`, `45 deg`, `5 1/2`; ADR 0051) converted through the engine's
//!   unit table (ADR 0052). A literal without a unit is a number of the
//!   cell's own unit.
//! - **Bool** — `TRUE` or `FALSE` (or `1`/`0`).
//! - **Color** — a `#RGB` or `#RRGGBB` literal.
//! - **String** — the text as written (the formula stores the grammar's
//!   quoted literal).
//!
//! A list or error value and a row with no cached value are not
//! editable: one `CommandError`, nothing written (a list has no literal
//! spelling and an error is what the file holds).

use ferrogram_model::{CellValue, CellWrite, ShapeId, SheetRef};
use ferrogram_shapesheet::{
    CellRef, Error as FormulaError, EvalContext, EvalWarning, Expr, ExprKind, Number, UnaryOp,
    Unit, Value, eval, parse,
};

use crate::app::AppCtx;
use crate::error::CommandError;
use crate::undo::Edit;
use crate::{Invocation, Outcome};

use super::edit::{CellEdit, document_mut, number_literal};
use super::shape::anchor_target;
use super::{CommandTable, edit_outcome, has_selection};

/// The implemented `data.*` commands (task P6-18).
pub(crate) const TABLE: CommandTable = &[(
    "data.set.property",
    "Value",
    has_selection,
    run_set_property,
)];

/// `data.set.property`: writes one Shape Data `Prop.<row>.Value` cell of
/// the anchor shape (the panel's subject, ADR 0041).
pub(super) fn run_set_property(
    ctx: &mut AppCtx,
    inv: &Invocation,
) -> Result<Outcome, CommandError> {
    let row = need_text(&inv.args, "row", "the Shape Data row name")?;
    let text = need_value(&inv.args)?;
    let (session, shape, _) = anchor_target(ctx)?;
    let write = {
        let doc = document_mut(ctx, session)?;
        let entry = doc
            .property_rows(shape)
            .into_iter()
            .find(|entry| entry.key.eq_ignore_ascii_case(&row))
            .ok_or_else(|| CommandError::invalid_args(format!("no Shape Data row {row:?}")))?;
        let value = entry
            .row
            .cells
            .iter()
            .find(|cell| cell.name.as_str() == "Value");
        let cached = value.map(|cell| (cell.value.as_ref(), cell.unit));
        property_write(cached, &text)?
    };
    let label = format!("Set {row}");
    let cell = format!("Prop.{row}.Value");
    let sheet = SheetRef::Shape(shape);
    let doc = document_mut(ctx, session)?;
    let mut edit = CellEdit::direct(doc, session, label.clone(), sheet, &cell, write)?;
    edit.apply(ctx)?;
    Ok(edit_outcome(label, Vec::new(), Box::new(edit)))
}

/// The `row` argument (one required text argument).
fn need_text(args: &crate::Args, name: &str, prompt: &str) -> Result<String, CommandError> {
    match args.get(name) {
        None => Err(CommandError::NeedsInput(crate::NeededInput::Text {
            prompt: prompt.to_owned(),
            default: None,
        })),
        Some(crate::ArgValue::Text(text)) if !text.is_empty() => Ok(text.clone()),
        Some(crate::ArgValue::Text(_)) => Err(CommandError::invalid_args(format!(
            "{name} must not be empty"
        ))),
        Some(_) => Err(CommandError::invalid_args(format!("{name} must be text"))),
    }
}

/// The `value` argument: free text, empty allowed (a string value may be
/// empty).
fn need_value(args: &crate::Args) -> Result<String, CommandError> {
    match args.get("value") {
        None => Err(CommandError::NeedsInput(crate::NeededInput::Text {
            prompt: String::from("the Shape Data value"),
            default: None,
        })),
        Some(crate::ArgValue::Text(text)) => Ok(text.clone()),
        Some(_) => Err(CommandError::invalid_args("value must be text")),
    }
}

/// The write of one Shape Data value edit (task P6-18): `text` read in
/// the kind the row's `Value` cell caches (ADR 0058), stored in
/// canonical units (ADR 0011).
///
/// `cached` is the cell's cached value and its recorded unit, when the
/// row has a `Value` cell at all.
fn property_write(
    cached: Option<(Option<&CellValue>, ferrogram_geom::Unit)>,
    text: &str,
) -> Result<CellWrite, CommandError> {
    let (value, unit) =
        cached.ok_or_else(|| CommandError::invalid_args("the row has no value to edit"))?;
    match value {
        Some(CellValue::Number(_)) => number_write(unit, text),
        Some(CellValue::Bool(_)) => match text.trim() {
            "TRUE" | "true" | "True" | "1" => Ok(CellWrite::new("TRUE", CellValue::Bool(true))),
            "FALSE" | "false" | "False" | "0" => {
                Ok(CellWrite::new("FALSE", CellValue::Bool(false)))
            }
            _ => Err(CommandError::invalid_args(format!(
                "{text:?} is not TRUE or FALSE"
            ))),
        },
        Some(CellValue::Color(_)) => {
            // The `#RGB`/`#RRGGBB` spelling the pane shows (the file's
            // own `V` text): the formula grammar has no colour literal.
            let color = color_literal(text).ok_or_else(|| {
                CommandError::invalid_args(format!("{text:?} is not a #RRGGBB colour"))
            })?;
            let spelling = format!("#{:02X}{:02X}{:02X}", color.r, color.g, color.b);
            Ok(CellWrite::new(spelling, CellValue::Color(color)))
        }
        Some(CellValue::Str(_)) => Ok(CellWrite::new(
            string_literal(text),
            CellValue::Str(text.to_owned()),
        )),
        None => Err(CommandError::invalid_args("the row has no value to edit")),
        // A list, an error and any kind `CellValue` grows later (it is
        // non-exhaustive) have no literal spelling to type back: one
        // refusal, never a panic (AGENTS.md section 6).
        Some(_) => Err(CommandError::invalid_args(
            "this value is not editable in this release",
        )),
    }
}

/// The write of a numeric Shape Data value: one literal of the grammar
/// (ADR 0058), converted to canonical units. A literal without a unit
/// is a number of the cell's own unit. A spelling that is no number
/// literal, or that lands on a non-finite canonical value, is one
/// refusal (nothing written).
fn number_write(unit: ferrogram_geom::Unit, text: &str) -> Result<CellWrite, CommandError> {
    let not_a_number = || CommandError::invalid_args(format!("{text:?} is not a number"));
    let value = literal_value(text).map_err(|_| not_a_number())?;
    let Value::Number(number) = value else {
        return Err(not_a_number());
    };
    let number = if number.unit == Unit::Unitsless {
        Number::new(
            number.magnitude,
            Unit::from_geom(unit).unwrap_or(Unit::Unitsless),
        )
    } else {
        number
    };
    let cached = Value::Number(number)
        .to_cached()
        .map_err(|_| not_a_number())?;
    match cached.value {
        CellValue::Number(magnitude) if magnitude.is_finite() => Ok(CellWrite::new(
            number_literal(magnitude),
            CellValue::Number(magnitude),
        )),
        _ => Err(CommandError::invalid_args(format!(
            "{text:?} is not a finite number"
        ))),
    }
}

/// One literal of the formula grammar (ADR 0058's entry rule): a
/// number, string, boolean or colour literal under any chain of unary
/// `+`, `-` and `%`, evaluated with a literal-only context. A call or a
/// reference belongs to the formula editor of a later phase and is a
/// refused entry.
fn literal_value(text: &str) -> Result<Value, CommandError> {
    let bad = || CommandError::invalid_args(format!("{text:?} is not a plain value"));
    let expr = parse(text).map_err(|_| bad())?;
    if !is_literal(&expr) {
        return Err(bad());
    }
    let mut context = LiteralOnly::default();
    let value =
        eval(&expr, SheetRef::Shape(ShapeId { index: 0 }), &mut context).map_err(|_| bad())?;
    if context.warnings != 0 {
        return Err(bad());
    }
    Ok(value)
}

/// Whether an expression is one literal of the grammar: a literal under
/// any chain of unary `+`, `-` and `%` (the panels' rule, ADR 0058).
fn is_literal(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::Literal(_) => true,
        ExprKind::Unary {
            op: UnaryOp::Plus | UnaryOp::Minus | UnaryOp::Percent,
            operand,
        } => is_literal(operand),
        _ => false,
    }
}

/// A literal-only evaluation context: any warning the evaluation
/// records (a coercion, a non-finite result) refuses the entry.
#[derive(Default)]
struct LiteralOnly {
    /// How many warnings the evaluation recorded.
    warnings: usize,
}

impl EvalContext for LiteralOnly {
    fn cell_value(
        &mut self,
        _sheet: SheetRef,
        _reference: &CellRef,
    ) -> Result<Value, FormulaError> {
        Err(FormulaError::Conversion("a value entry is a constant"))
    }

    fn set_cell(
        &mut self,
        _sheet: SheetRef,
        _reference: &CellRef,
        _value: Value,
    ) -> Result<(), FormulaError> {
        Err(FormulaError::Conversion("a value entry is a constant"))
    }

    fn warn(&mut self, _warning: EvalWarning) {
        self.warnings = self.warnings.saturating_add(1);
    }
}

/// The grammar's quoted spelling of a string literal (`"` doubled in
/// the text, ADR 0051).
fn string_literal(text: &str) -> String {
    format!("\"{}\"", text.replace('"', "\"\""))
}

/// The colour of a `#RGB` or `#RRGGBB` literal (any case), or `None`:
/// the file's own colour spelling (task P6-18; the formula grammar has
/// no colour literal).
fn color_literal(text: &str) -> Option<ferrogram_model::Color> {
    fn digit(byte: u8) -> Option<u8> {
        match byte {
            b'0'..=b'9' => Some(byte - b'0'),
            b'a'..=b'f' => Some(byte - b'a' + 10),
            b'A'..=b'F' => Some(byte - b'A' + 10),
            _ => None,
        }
    }
    fn pair(high: u8, low: u8) -> Option<u8> {
        Some(digit(high)? * 16 + digit(low)?)
    }
    let digits = text.trim().strip_prefix('#')?.as_bytes();
    match digits {
        [r1, r2, g1, g2, b1, b2] => Some(ferrogram_model::Color::new(
            pair(*r1, *r2)?,
            pair(*g1, *g2)?,
            pair(*b1, *b2)?,
        )),
        [r, g, b] => Some(ferrogram_model::Color::new(
            digit(*r)? * 17,
            digit(*g)? * 17,
            digit(*b)? * 17,
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ArgValue, Args};

    #[test]
    fn the_row_argument_is_required_text() {
        let prompt = "the Shape Data row name";
        assert!(matches!(
            need_text(&Args::new(), "row", prompt),
            Err(CommandError::NeedsInput(_))
        ));
        let error = need_text(
            &Args::new().with("row", ArgValue::Text(String::new())),
            "row",
            prompt,
        )
        .expect_err("an empty row is refused");
        assert!(matches!(error, CommandError::InvalidArgs { .. }), "{error}");
        let error = need_text(
            &Args::new().with("row", ArgValue::Integer(3)),
            "row",
            prompt,
        )
        .expect_err("a non-text row is refused");
        assert!(matches!(error, CommandError::InvalidArgs { .. }), "{error}");
        assert_eq!(
            need_text(
                &Args::new().with("row", ArgValue::Text(String::from("Cost"))),
                "row",
                prompt
            )
            .expect("a text row"),
            "Cost"
        );
    }

    #[test]
    fn the_value_argument_is_text_and_may_be_empty() {
        assert!(matches!(
            need_value(&Args::new()),
            Err(CommandError::NeedsInput(_))
        ));
        let error = need_value(&Args::new().with("value", ArgValue::Integer(3)))
            .expect_err("a non-text value is refused");
        assert!(matches!(error, CommandError::InvalidArgs { .. }), "{error}");
        assert_eq!(
            need_value(&Args::new().with("value", ArgValue::Text(String::new())))
                .expect("an empty value"),
            ""
        );
    }

    #[test]
    fn a_boolean_value_takes_every_spelling_of_true_and_false() {
        let cached = Some((
            Some(&CellValue::Bool(false)),
            ferrogram_geom::Unit::Unitsless,
        ));
        for (text, flag) in [
            ("TRUE", true),
            ("true", true),
            ("True", true),
            ("1", true),
            ("FALSE", false),
            ("false", false),
            ("False", false),
            ("0", false),
        ] {
            let write = property_write(cached, text).expect("the spelling writes");
            assert_eq!(write.value, CellValue::Bool(flag), "{text}");
            assert_eq!(write.formula, if flag { "TRUE" } else { "FALSE" }, "{text}");
        }
    }

    #[test]
    fn a_row_without_a_cached_value_is_not_editable() {
        // No `Value` cell at all ...
        let error = property_write(None, "x").expect_err("no cell");
        assert!(
            matches!(error, CommandError::InvalidArgs { .. }),
            "{error:?}"
        );
        // ... and a `Value` cell that caches nothing.
        let error = property_write(Some((None, ferrogram_geom::Unit::Unitsless)), "x")
            .expect_err("no cached value");
        assert!(error.to_string().contains("no value to edit"), "{error}");
    }

    #[test]
    fn a_number_row_reads_only_number_literals() {
        let unit = ferrogram_geom::Unit::Inches;
        let cached = Some((Some(&CellValue::Number(1.0)), unit));
        for text in ["TRUE", "hello", "Width*2", "1 + 2", "#FF0000"] {
            let error = property_write(cached, text).expect_err("not a number");
            assert!(
                error.to_string().contains("not a number"),
                "{text:?}: {error}"
            );
        }
    }

    #[test]
    fn a_literal_is_a_constant_of_the_grammar() {
        for text in ["2.5", "-3", "45 deg", "37%", "\"a\"", "TRUE"] {
            assert!(is_literal(&parse(text).expect("parses")), "{text}");
        }
        for text in ["Width", "ABS(-2)", "1 + 2", "Inh"] {
            assert!(!is_literal(&parse(text).expect("parses")), "{text}");
        }
        // A unary chain over a literal is still a literal.
        let expr = parse("--2%").expect("parses");
        assert!(is_literal(&expr));
    }

    #[test]
    fn a_literal_entry_refuses_calls_and_a_coercion() {
        assert!(literal_value("2.5 cm").is_ok());
        assert!(literal_value("Width").is_err());
        assert!(literal_value("ABS(-2)").is_err());
        assert!(literal_value("nonsense ((").is_err());
        // `"x"%` is a literal under unary `%`; the evaluation coerces
        // and warns, which refuses the entry.
        assert!(literal_value("\"x\"%").is_err());
    }

    #[test]
    fn the_literal_context_refuses_references_and_writes() {
        let mut context = LiteralOnly::default();
        let reference = CellRef::new("Width");
        let sheet = SheetRef::Shape(ShapeId { index: 0 });
        assert!(context.cell_value(sheet, &reference).is_err());
        assert!(
            context
                .set_cell(sheet, &reference, Value::Bool(false))
                .is_err()
        );
        assert_eq!(context.warnings, 0);
        context.warn(EvalWarning {
            sheet,
            kind: ferrogram_shapesheet::EvalWarningKind::Coercion(String::from("x")),
        });
        assert_eq!(context.warnings, 1);
    }

    #[test]
    fn colour_literals_take_the_files_own_spelling() {
        assert_eq!(
            color_literal("#FF8000"),
            Some(ferrogram_model::Color::new(255, 128, 0))
        );
        assert_eq!(
            color_literal("#abc"),
            Some(ferrogram_model::Color::new(170, 187, 204))
        );
        assert_eq!(color_literal("#GGGGGG"), None);
        assert_eq!(color_literal("#12345"), None);
        assert_eq!(color_literal("123456"), None);
        assert_eq!(color_literal(""), None);
    }

    #[test]
    fn string_literals_quote_the_text() {
        assert_eq!(string_literal("hello"), "\"hello\"");
        assert_eq!(string_literal("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(string_literal(""), "\"\"");
    }

    #[test]
    fn a_number_write_keeps_only_finite_values() {
        // A literal that lands on a non-finite canonical value (a huge
        // magnitude in a converting unit) is refused.
        let error = number_write(ferrogram_geom::Unit::Feet, "1e308")
            .expect_err("the conversion overflows");
        assert!(error.to_string().contains("not a finite number"), "{error}");
        // The plain-number rule: a literal without a unit is a number
        // of the cell's own unit.
        let write = number_write(ferrogram_geom::Unit::Inches, "5").expect("a plain number");
        assert_eq!(write.value, CellValue::Number(5.0));
        let write =
            number_write(ferrogram_geom::Unit::Unitsless, "2.5 cm").expect("a unit literal");
        assert!(matches!(write.value, CellValue::Number(_)));
        if let CellValue::Number(value) = write.value {
            assert!((value - 2.5 / 2.54).abs() < 1.0e-12, "{value}");
        }
    }

    #[test]
    fn a_colour_or_string_write_takes_the_text_as_written() {
        let cached = Some((
            Some(&CellValue::Color(ferrogram_model::Color::new(0, 0, 0))),
            ferrogram_geom::Unit::Unitsless,
        ));
        let write = property_write(cached, "#ff8000").expect("the colour writes");
        assert_eq!(write.formula, "#FF8000");
        assert_eq!(
            write.value,
            CellValue::Color(ferrogram_model::Color::new(255, 128, 0))
        );
        let cached = Some((
            Some(&CellValue::Str(String::new())),
            ferrogram_geom::Unit::Unitsless,
        ));
        let write = property_write(cached, "say \"hi\"").expect("the string writes");
        assert_eq!(write.formula, "\"say \"\"hi\"\"\"");
        assert_eq!(write.value, CellValue::Str(String::from("say \"hi\"")));
    }
}
