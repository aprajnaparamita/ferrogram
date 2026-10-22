//! The formula printer (ADR 0051).
//!
//! [`print()`] turns an [`Expr`] back into source text that
//! [`crate::parse`] parses to an equal tree (spans aside). That round
//! trip is the property test of ADR 0060 and the reason the tree keeps
//! literals and references close to their written form.
//!
//! Every part is written the way the parser reads it back: numbers in
//! their shortest round-trip form, strings with doubled quotes,
//! references with their parts as written. A hand-built tree the grammar
//! cannot express is an error, never silently wrong text — a negative
//! magnitude (the grammar has no signed literal), a unit or flag with no
//! spelling, a name that is not one piece (`Prop.Cost` as one name part
//! reads back as a section and a name).
//!
//! Style is the corpus's: `Width*0.5`, `5 in`, `5 [deg, A]`, `37%`.
//! Operators keep their symbolic spelling (`>` not `_GT_`); both parse
//! to the same operator (ADR 0051).

use crate::ast::{BinaryOp, Expr, ExprKind, Literal, NumberLiteral, UnaryOp};
use crate::error::{Error, Span, SyntaxKind};
use crate::formula::CellRef;
use crate::lexer::{MAX_NESTING, MAX_SOURCE_LEN};
use crate::parser::parse;

/// Precedence of an atomic expression (a literal, a reference, a call).
const PREC_ATOMIC: u8 = 8;

/// Precedence of a unary operator (`about-operators.md`, first).
const PREC_UNARY: u8 = 7;

/// Which side of a binary operator an operand sits on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    /// The left operand.
    Left,
    /// The right operand.
    Right,
}

/// One level of the left spine of a binary tree, kept for the fold in
/// [`print_binary`].
struct Level<'e> {
    /// The operator of this level.
    op: BinaryOp,
    /// Its right operand.
    right: &'e Expr,
    /// The nesting depth of this level.
    depth: usize,
    /// Whether the level wraps its left operand in parentheses.
    wrap_left: bool,
}

/// Prints one expression as formula source text (no leading `=`).
///
/// The text parses back to an equal tree (ADR 0051); spans are not part
/// of the comparison. A tree the grammar cannot express is an
/// [`Error::Syntax`]: [`SyntaxKind::InvalidNumber`] for a magnitude with
/// no spelling, [`SyntaxKind::InvalidUnit`] for a unit or flag with no
/// spelling, [`SyntaxKind::InvalidReference`] for a name that is not one
/// piece as written.
pub fn print(expr: &Expr) -> Result<String, Error> {
    let text = print_expr(expr, 0)?;
    if text.len() > MAX_SOURCE_LEN {
        return Err(Error::Syntax {
            span: expr.span,
            kind: SyntaxKind::InputTooLong,
        });
    }
    Ok(text)
}

/// Prints `expr`, which sits `depth` nesting levels into the formula.
fn print_expr(expr: &Expr, depth: usize) -> Result<String, Error> {
    if depth > MAX_NESTING {
        return Err(Error::Syntax {
            span: expr.span,
            kind: SyntaxKind::NestingTooDeep,
        });
    }
    match &expr.kind {
        ExprKind::Literal(literal) => print_literal(literal, expr.span),
        ExprKind::Ref(r) => print_ref(r, expr.span),
        ExprKind::Call { name, args } => print_call(args, expr.span, depth, name),
        ExprKind::Unary { op, operand } => print_unary(*op, operand, depth),
        ExprKind::Binary { .. } => print_binary(expr, depth),
    }
}

/// Prints a literal as written: the magnitude with its unit suffix or
/// its implicit `[unit, flag]` form, the string with doubled quotes.
fn print_literal(literal: &Literal, span: Span) -> Result<String, Error> {
    match literal {
        Literal::Str(text) => Ok(format!("\"{}\"", text.replace('"', "\"\""))),
        Literal::Bool(true) => Ok(String::from("TRUE")),
        Literal::Bool(false) => Ok(String::from("FALSE")),
        Literal::Number(number) => print_number(number, span),
    }
}

/// Prints a number literal (ADR 0051, "Implicit units of measure").
///
/// The grammar has no signed literal (`-5` is `Unary(Minus, 5)`), so a
/// negative or non-finite magnitude is [`SyntaxKind::InvalidNumber`].
fn print_number(number: &NumberLiteral, span: Span) -> Result<String, Error> {
    if !number.value.is_finite() || number.value.is_sign_negative() {
        return Err(Error::Syntax {
            span,
            kind: SyntaxKind::InvalidNumber,
        });
    }
    // Rust's `Display` prints the shortest form that parses back to the
    // same `f64` (round-trip by value, ADR 0051).
    let magnitude = format!("{}", number.value);
    let invalid_unit = Error::Syntax {
        span,
        kind: SyntaxKind::InvalidUnit,
    };
    match (&number.unit, number.flag) {
        (None, None) => Ok(magnitude),
        (None, Some(_)) => Err(invalid_unit),
        // `5'`, `5"`, `5°`: printed without a gap, no room for a flag.
        (Some(unit), None) if is_symbol_unit(unit) => Ok(format!("{magnitude}{unit}")),
        // `%` after a number is the percentage operator (ADR 0051); the
        // unit spelling only fits in the implicit form.
        (Some(unit), None) if unit == "%" => Ok(format!("{magnitude} [%]")),
        // `5 in`, `3 FEET/INCH`.
        (Some(unit), None) if is_unit_spelling(unit) => Ok(format!("{magnitude} {unit}")),
        (Some(_), None) => Err(invalid_unit),
        (Some(unit), Some(flag)) => {
            let printable = matches!(unit.as_str(), "%" | "°") || is_unit_spelling(unit);
            if printable && is_flag(flag) {
                Ok(format!("{magnitude} [{unit}, {flag}]"))
            } else {
                Err(invalid_unit)
            }
        }
    }
}

/// Prints a call: `NAME(args)`, parentheses always (`about-functions.md`).
fn print_call(args: &[Expr], span: Span, depth: usize, name: &str) -> Result<String, Error> {
    if !is_call_name(name) {
        return Err(Error::Syntax {
            span,
            kind: SyntaxKind::InvalidReference,
        });
    }
    let mut text = String::from(name);
    text.push('(');
    for (index, arg) in args.iter().enumerate() {
        if index > 0 {
            text.push_str(", ");
        }
        text.push_str(&print_expr(arg, depth + 1)?);
    }
    text.push(')');
    Ok(text)
}

/// Prints a reference: `sheets!Section.Names`, every part as written.
///
/// The parts must read back as written: `Prop.Cost` as *one* name part
/// would parse as a section and a name, and `My Sheet` as one sheet
/// segment would parse as two pieces.
fn print_ref(r: &CellRef, span: Span) -> Result<String, Error> {
    let invalid = Error::Syntax {
        span,
        kind: SyntaxKind::InvalidReference,
    };
    let Some((first, more)) = r.names.split_first() else {
        return Err(invalid);
    };
    let mut text = String::new();
    for sheet in &r.sheets {
        if !text.is_empty() {
            text.push('!');
        }
        text.push_str(sheet);
    }
    if !text.is_empty() {
        text.push('!');
    }
    match (&r.section, more.is_empty()) {
        // Without a section the name is one piece: `Width`, not `A.B`,
        // which the parser reads as a section and a name.
        (None, false) => return Err(invalid),
        (None, true) => text.push_str(first),
        (Some(section), _) => {
            text.push_str(section);
            for name in std::iter::once(first).chain(more.iter()) {
                text.push('.');
                text.push_str(name);
            }
        }
    }
    match parse(&text) {
        Ok(Expr {
            kind: ExprKind::Ref(back),
            ..
        }) if back == *r => Ok(text),
        _ => Err(invalid),
    }
}

/// Prints a unary operator: `+` and `-` before the operand, `%` after
/// (`37%` is 0.37).
fn print_unary(op: UnaryOp, operand: &Expr, depth: usize) -> Result<String, Error> {
    match op {
        UnaryOp::Plus | UnaryOp::Minus => {
            let sign = if matches!(op, UnaryOp::Plus) {
                "+"
            } else {
                "-"
            };
            let wrap = precedence_of(operand) < PREC_UNARY;
            let inner = print_expr(operand, depth + 1 + usize::from(wrap))?;
            Ok(if wrap {
                format!("{sign}({inner})")
            } else {
                format!("{sign}{inner}")
            })
        }
        UnaryOp::Percent => {
            // A chain unwraps to one `%` per level (`x%%`), and the
            // postfix operator adds no nesting of its own.
            let mut levels = 1usize;
            let mut inner = operand;
            while let ExprKind::Unary {
                op: UnaryOp::Percent,
                operand: next,
            } = &inner.kind
            {
                levels += 1;
                inner = next;
            }
            let wrap = precedence_of(inner) < PREC_ATOMIC;
            let text = print_expr(inner, depth + usize::from(wrap))?;
            let body = if wrap { format!("({text})") } else { text };
            Ok(format!("{body}{}", "%".repeat(levels)))
        }
    }
}

/// Prints a binary expression, walking the left spine iteratively so a
/// flat chain (`a + b + c + ...`) does not recurse.
fn print_binary(expr: &Expr, depth: usize) -> Result<String, Error> {
    let mut levels: Vec<Level<'_>> = Vec::new();
    let mut node = expr;
    let mut at = depth;
    let mut out;
    loop {
        let ExprKind::Binary { op, left, right } = &node.kind else {
            out = print_expr(node, at)?;
            break;
        };
        let wrap_left = needs_parens(left, *op, Side::Left);
        levels.push(Level {
            op: *op,
            right,
            depth: at,
            wrap_left,
        });
        at += usize::from(wrap_left);
        node = left;
    }
    for level in levels.into_iter().rev() {
        let left = if level.wrap_left {
            format!("({out})")
        } else {
            out
        };
        let right = print_operand(level.right, level.op, Side::Right, level.depth)?;
        out = format!("{left} {} {right}", operator_text(level.op));
    }
    Ok(out)
}

/// Prints one operand of a binary operator, parenthesised when the
/// grammar would regroup it otherwise.
fn print_operand(
    child: &Expr,
    parent: BinaryOp,
    side: Side,
    depth: usize,
) -> Result<String, Error> {
    let wrap = needs_parens(child, parent, side);
    // `^` descends on its right (ADR 0051), like its parentheses do.
    let mut step = usize::from(wrap);
    if matches!(parent, BinaryOp::Pow) && matches!(side, Side::Right) {
        step += 1;
    }
    let text = print_expr(child, depth + step)?;
    Ok(if wrap { format!("({text})") } else { text })
}

/// Whether `child` needs parentheses next to `parent`.
fn needs_parens(child: &Expr, parent: BinaryOp, side: Side) -> bool {
    let parent_precedence = operator_precedence(parent);
    let child_precedence = precedence_of(child);
    if child_precedence != parent_precedence {
        return child_precedence < parent_precedence;
    }
    match side {
        // `^` is right-associative, every other operator is not.
        Side::Left => matches!(parent, BinaryOp::Pow),
        Side::Right => !matches!(parent, BinaryOp::Pow),
    }
}

/// The precedence of an expression next to a binary operator.
fn precedence_of(expr: &Expr) -> u8 {
    match &expr.kind {
        ExprKind::Binary { op, .. } => operator_precedence(*op),
        ExprKind::Unary { .. } => PREC_UNARY,
        ExprKind::Literal(_) | ExprKind::Ref(_) | ExprKind::Call { .. } => PREC_ATOMIC,
    }
}

/// The order of evaluation of one binary operator (ADR 0051).
const fn operator_precedence(op: BinaryOp) -> u8 {
    match op {
        BinaryOp::Eq | BinaryOp::Ne => 1,
        BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => 2,
        BinaryOp::Concat => 3,
        BinaryOp::Add | BinaryOp::Sub => 4,
        BinaryOp::Mul | BinaryOp::Div => 5,
        BinaryOp::Pow => 6,
    }
}

/// The symbolic spelling of one binary operator.
const fn operator_text(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
        BinaryOp::Pow => "^",
        BinaryOp::Concat => "&",
        BinaryOp::Eq => "=",
        BinaryOp::Ne => "<>",
        BinaryOp::Lt => "<",
        BinaryOp::Le => "<=",
        BinaryOp::Gt => ">",
        BinaryOp::Ge => ">=",
    }
}

/// The measurement-system flag of the implicit unit form (ADR 0051).
fn is_flag(c: char) -> bool {
    matches!(c.to_ascii_lowercase(), 'a' | 'd' | 'e' | 'p' | 't')
}

/// True for a unit spelled with words (`in`, `MM`) or a compound whose
/// parts sit next to each other (`FEET/INCH`).
fn is_unit_spelling(unit: &str) -> bool {
    !unit.is_empty()
        && unit.split('/').all(|word| {
            !word.is_empty()
                && word
                    .chars()
                    .next()
                    .is_some_and(|c| c == '_' || c.is_alphabetic())
                && word
                    .chars()
                    .skip(1)
                    .all(|c| c == '_' || c.is_alphanumeric())
        })
}

/// True for the single-character units `'`, `"`, `°`, which print
/// without a gap (`5'`).
fn is_symbol_unit(unit: &str) -> bool {
    matches!(unit, "'" | "\"" | "°")
}

/// True when `name` is one call name the parser reads back: a bare word
/// that is not `TRUE` or `FALSE`.
fn is_call_name(name: &str) -> bool {
    let source = format!("{name}()");
    match parse(&source) {
        Ok(Expr {
            kind: ExprKind::Call { name: back, args },
            ..
        }) => back == name && args.is_empty(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    fn printed(source: &str) -> String {
        print(&parse(source).expect("the source parses")).expect("the tree prints")
    }

    fn tree(source: &str) -> Expr {
        parse(source).expect("the source parses")
    }

    fn number(value: f64, unit: Option<&str>, flag: Option<char>) -> Expr {
        Expr {
            span: Span::new(0, 0),
            kind: ExprKind::Literal(Literal::Number(NumberLiteral {
                value,
                unit: unit.map(str::to_owned),
                flag,
            })),
        }
    }

    fn ref_of(names: &[&str]) -> Expr {
        Expr {
            span: Span::new(0, 0),
            kind: ExprKind::Ref(CellRef {
                sheets: Vec::new(),
                section: None,
                names: names.iter().map(|n| (*n).to_owned()).collect(),
            }),
        }
    }

    #[test]
    fn a_literal_prints_its_shortest_round_trip_form() {
        assert_eq!(printed("1+1"), "1 + 1");
        assert_eq!(printed("5 1/2"), "5.5");
        assert_eq!(printed("1e-3"), "0.001");
        assert_eq!(printed("0.1"), "0.1");
        assert_eq!(printed("1/3"), "1 / 3");
    }

    #[test]
    fn a_unit_prints_the_way_it_was_written() {
        assert_eq!(printed("10MM"), "10 MM");
        assert_eq!(printed("0.5 in"), "0.5 in");
        assert_eq!(printed("5'"), "5'");
        assert_eq!(printed("5\""), "5\"");
        assert_eq!(printed("5°"), "5°");
        assert_eq!(printed("3FEET/INCH"), "3 FEET/INCH");
        assert_eq!(printed("5[deg,A]"), "5 [deg, A]");
        assert_eq!(printed("2 [in, d]"), "2 [in, d]");
    }

    #[test]
    fn percent_prints_after_its_operand() {
        assert_eq!(printed("37%"), "37%");
        assert_eq!(printed("%37"), "37%");
        assert_eq!(printed("37%%"), "37%%");
        assert_eq!(printed("5 [%]"), "5 [%]");
    }

    #[test]
    fn strings_double_their_quotes() {
        assert_eq!(printed("\"say \"\"hi\"\"\""), "\"say \"\"hi\"\"\"");
        assert_eq!(printed("\"\""), "\"\"");
    }

    #[test]
    fn references_print_their_parts_as_written() {
        assert_eq!(printed("Width"), "Width");
        assert_eq!(printed("Sheet.4!PinX"), "Sheet.4!PinX");
        assert_eq!(
            printed("Masters[Gear]!Shaft!Geometry1.X1"),
            "Masters[Gear]!Shaft!Geometry1.X1"
        );
        assert_eq!(printed("'1-D'!LineColor"), "'1-D'!LineColor");
        assert_eq!(printed("Controls.Row_1.Y"), "Controls.Row_1.Y");
        assert_eq!(
            printed("Pages[Page-3]!Sheet.4!BeginX"),
            "Pages[Page-3]!Sheet.4!BeginX"
        );
    }

    #[test]
    fn calls_print_with_parentheses() {
        assert_eq!(printed("DEFAULTEVENT()"), "DEFAULTEVENT()");
        assert_eq!(printed("IF(a,b,c)"), "IF(a, b, c)");
        assert_eq!(
            printed("GUARD((BeginX+EndX)/2)"),
            "GUARD((BeginX + EndX) / 2)"
        );
    }

    #[test]
    fn parentheses_follow_the_order_of_evaluation() {
        assert_eq!(printed("2+3*4"), "2 + 3 * 4");
        assert_eq!(printed("(2+3)*4"), "(2 + 3) * 4");
        assert_eq!(printed("2^3^2"), "2 ^ 3 ^ 2");
        assert_eq!(printed("(2^3)^2"), "(2 ^ 3) ^ 2");
        assert_eq!(printed("a-b-c"), "a - b - c");
        assert_eq!(printed("a-(b-c)"), "a - (b - c)");
        assert_eq!(printed("a/(b*c)"), "a / (b * c)");
        assert_eq!(printed("(a*b)*c"), "a * b * c");
        assert_eq!(printed("1<2=3<4"), "1 < 2 = 3 < 4");
    }

    #[test]
    fn the_alternative_operators_print_symbolically() {
        assert_eq!(printed("5 _GT_ 2"), "5 > 2");
        assert_eq!(printed("5 _NE_ 2"), "5 <> 2");
    }

    #[test]
    fn unary_operators_keep_their_order() {
        assert_eq!(printed("-TxtHeight*0.5"), "-TxtHeight * 0.5");
        assert_eq!(printed("-(a+b)"), "-(a + b)");
        assert_eq!(printed("- -1"), "--1");
        assert_eq!(printed("(-1)%"), "(-1)%");
    }

    #[test]
    fn every_reference_form_round_trips() {
        for source in [
            "Width",
            "Star!Angle",
            "Executive.2!Height",
            "Char.Font[3]",
            "Scratch.A5",
            "Prop.Cost",
            "Geometry1.X2",
            "Sheet.8!FillForegnd",
            "Masters[Gear]!Shaft!Geometry1.X1",
            "ThePage!User.Vanishing_Point",
            "Pages[Page-3]!Sheet.4!BeginX",
            "Styles!Manager!LineColor",
            "TheDoc!PreviewQuality",
            "'1-D'!LineColor",
        ] {
            let expr = tree(source);
            let text = print(&expr).expect("the reference prints");
            let back = tree(&text);
            assert_eq!(back, expr, "{source} printed as {text} did not round-trip");
        }
    }

    #[test]
    fn a_negative_literal_has_no_spelling() {
        for value in [-5.0, f64::INFINITY, f64::NEG_INFINITY, f64::NAN, -0.0] {
            let bad = print(&number(value, None, None)).expect_err("no signed literal");
            assert_eq!(
                bad,
                Error::Syntax {
                    span: Span::new(0, 0),
                    kind: SyntaxKind::InvalidNumber
                }
            );
        }
    }

    #[test]
    fn an_unwritable_unit_is_an_error() {
        let invalid = Error::Syntax {
            span: Span::new(0, 0),
            kind: SyntaxKind::InvalidUnit,
        };
        for (unit, flag) in [
            (Some("SQ. IN."), None),
            (Some("2x"), None),
            (Some("a-b"), None),
            (Some("in/"), None),
            (Some(""), None),
            (Some("in"), Some('x')),
            (Some("SQ. IN."), Some('d')),
            (Some("'"), Some('d')),
            (Some("in"), Some('3')),
        ] {
            assert_eq!(
                print(&number(5.0, unit, flag)).expect_err("no spelling"),
                invalid
            );
        }
        // A flag without a unit has no spelling either.
        assert_eq!(
            print(&number(5.0, None, Some('d'))).expect_err("no unit"),
            invalid
        );
    }

    #[test]
    fn a_name_that_is_not_one_piece_is_an_error() {
        let invalid = Error::Syntax {
            span: Span::new(0, 0),
            kind: SyntaxKind::InvalidReference,
        };
        // `Prop.Cost` reads back as a section and a name, not one name.
        assert_eq!(
            print(&ref_of(&["Prop.Cost"])).expect_err("two pieces"),
            invalid
        );
        // A bare name cannot hold two pieces without a section.
        assert_eq!(print(&ref_of(&["A", "B"])).expect_err("two names"), invalid);
        // A number or a boolean is not a name.
        assert_eq!(print(&ref_of(&["5"])).expect_err("a number"), invalid);
        assert_eq!(print(&ref_of(&["TRUE"])).expect_err("a boolean"), invalid);
        // An empty reference names nothing.
        assert_eq!(print(&ref_of(&[])).expect_err("no name"), invalid);
    }

    #[test]
    fn a_name_that_is_not_a_word_is_an_error() {
        let invalid = Error::Syntax {
            span: Span::new(0, 0),
            kind: SyntaxKind::InvalidReference,
        };
        for name in ["my fn", "TRUE", "2x", "IF(", ""] {
            let call = Expr {
                span: Span::new(0, 0),
                kind: ExprKind::Call {
                    name: name.to_owned(),
                    args: Vec::new(),
                },
            };
            assert_eq!(print(&call).expect_err("no such name"), invalid, "{name}");
        }
    }

    #[test]
    fn a_section_and_its_names_must_each_be_one_piece() {
        let invalid = Error::Syntax {
            span: Span::new(0, 0),
            kind: SyntaxKind::InvalidReference,
        };
        let section_split = Expr {
            span: Span::new(0, 0),
            kind: ExprKind::Ref(CellRef {
                sheets: Vec::new(),
                section: Some(String::from("A.B")),
                names: vec![String::from("C")],
            }),
        };
        assert_eq!(print(&section_split).expect_err("two pieces"), invalid);
        let name_split = Expr {
            span: Span::new(0, 0),
            kind: ExprKind::Ref(CellRef {
                sheets: Vec::new(),
                section: Some(String::from("Prop")),
                names: vec![String::from("Cost.Value")],
            }),
        };
        assert_eq!(print(&name_split).expect_err("two pieces"), invalid);
    }

    #[test]
    fn a_sheet_that_is_not_one_segment_is_an_error() {
        let invalid = Error::Syntax {
            span: Span::new(0, 0),
            kind: SyntaxKind::InvalidReference,
        };
        let expr = Expr {
            span: Span::new(0, 0),
            kind: ExprKind::Ref(CellRef {
                sheets: vec![String::from("My Sheet")],
                section: None,
                names: vec![String::from("Width")],
            }),
        };
        assert_eq!(print(&expr).expect_err("two segments"), invalid);
    }

    #[test]
    fn a_tree_deeper_than_the_parser_prints_not() {
        let mut expr = number(1.0, None, None);
        for _ in 0..40 {
            expr = Expr {
                span: Span::new(0, 0),
                kind: ExprKind::Unary {
                    op: UnaryOp::Minus,
                    operand: Box::new(expr),
                },
            };
        }
        assert_eq!(
            print(&expr).expect_err("too deep"),
            Error::Syntax {
                span: Span::new(0, 0),
                kind: SyntaxKind::NestingTooDeep
            }
        );
    }

    #[test]
    fn a_flat_chain_prints_without_recursion() {
        let mut expr = number(1.0, None, None);
        for _ in 0..2000 {
            expr = Expr {
                span: Span::new(0, 0),
                kind: ExprKind::Binary {
                    op: BinaryOp::Add,
                    left: Box::new(expr),
                    right: Box::new(number(1.0, None, None)),
                },
            };
        }
        let text = print(&expr).expect("a flat chain prints");
        assert!(text.starts_with("1 + 1"));
        assert!(text.ends_with(" + 1"));
        assert_eq!(text.matches(" + ").count(), 2000);
    }
}
