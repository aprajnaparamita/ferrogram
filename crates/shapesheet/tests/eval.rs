//! Evaluator tests (ADR 0053, P6-04).
//!
//! One test per function family of the ADR 0053 evaluated set, the
//! operator coercions of ADR 0052 and the never-panic rules of the
//! acceptance criteria. The trees are built by hand here: `parse` is
//! P6-02's and lands in parallel.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use larrygram_model::{Color, PageId, SheetRef};
use larrygram_shapesheet::{
    BinaryOp, CellRef, Error, ErrorCode, EvalContext, EvalWarning, EvalWarningKind, Expr, ExprKind,
    Literal, Number, NumberLiteral, Span, UnaryOp, Unit, Value, eval,
};

// ---------- helpers ----------

fn sheet() -> SheetRef {
    SheetRef::Page(PageId { index: 0 })
}

fn span() -> Span {
    Span::new(0, 0)
}

fn num(value: f64, unit: Option<&str>) -> Expr {
    Expr {
        span: span(),
        kind: ExprKind::Literal(Literal::Number(NumberLiteral {
            value,
            unit: unit.map(String::from),
            flag: None,
        })),
    }
}

fn plain(value: f64) -> Expr {
    num(value, None)
}

fn text(value: &str) -> Expr {
    Expr {
        span: span(),
        kind: ExprKind::Literal(Literal::Str(String::from(value))),
    }
}

fn boolean(value: bool) -> Expr {
    Expr {
        span: span(),
        kind: ExprKind::Literal(Literal::Bool(value)),
    }
}

fn cell(name: &str) -> Expr {
    Expr {
        span: span(),
        kind: ExprKind::Ref(CellRef::new(name)),
    }
}

fn call(name: &str, args: Vec<Expr>) -> Expr {
    Expr {
        span: span(),
        kind: ExprKind::Call {
            name: String::from(name),
            args,
        },
    }
}

fn bin(op: BinaryOp, left: Expr, right: Expr) -> Expr {
    Expr {
        span: span(),
        kind: ExprKind::Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
        },
    }
}

fn un(op: UnaryOp, operand: Expr) -> Expr {
    Expr {
        span: span(),
        kind: ExprKind::Unary {
            op,
            operand: Box::new(operand),
        },
    }
}

/// A context with a few named cells, recording writes and warnings.
struct Ctx {
    cells: Vec<(String, Value)>,
    writes: Vec<(String, Value)>,
    warnings: Vec<EvalWarningKind>,
    refuse: bool,
}

impl Ctx {
    fn new() -> Self {
        Self {
            cells: Vec::new(),
            writes: Vec::new(),
            warnings: Vec::new(),
            refuse: false,
        }
    }

    fn with(name: &str, value: Value) -> Self {
        let mut ctx = Self::new();
        ctx.cells.push((String::from(name), value));
        ctx
    }
}

impl EvalContext for Ctx {
    fn cell_value(&mut self, _sheet: SheetRef, reference: &CellRef) -> Result<Value, Error> {
        let name = reference.name().unwrap_or_default();
        Ok(match self.cells.iter().find(|(n, _)| *n == name) {
            Some((_, value)) => value.clone(),
            None => Value::Err(ErrorCode::Ref),
        })
    }

    fn set_cell(
        &mut self,
        _sheet: SheetRef,
        reference: &CellRef,
        value: Value,
    ) -> Result<(), Error> {
        if self.refuse {
            return Err(Error::Conversion("test context refuses this write"));
        }
        self.writes
            .push((reference.name().unwrap_or_default(), value));
        Ok(())
    }

    fn warn(&mut self, warning: EvalWarning) {
        self.warnings.push(warning.kind);
    }
}

fn run(expr: &Expr, ctx: &mut Ctx) -> Value {
    eval(expr, sheet(), ctx).expect("the test context never fails")
}

fn number(value: f64, unit: Unit) -> Value {
    Value::Number(Number::new(value, unit))
}

/// Asserts a numeric result and returns its magnitude for comparison.
fn expect_number(value: Value, unit: Unit) -> f64 {
    match value {
        Value::Number(n) => {
            assert_eq!(n.unit, unit, "result carries the wrong unit");
            n.magnitude
        }
        other => panic!("expected a number, got {other:?}"),
    }
}

fn approx(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}

fn one_warning(ctx: &Ctx) -> &EvalWarningKind {
    assert_eq!(ctx.warnings.len(), 1, "expected exactly one warning");
    ctx.warnings.first().expect("one warning")
}

// ---------- operators ----------

#[test]
fn arithmetic_adds_mixed_units_in_canonical_inches() {
    let mut ctx = Ctx::new();
    let out = run(
        &bin(BinaryOp::Add, num(1.0, Some("in")), num(2.0, Some("cm"))),
        &mut ctx,
    );
    approx(expect_number(out, Unit::Inches), 1.0 + 2.0 / 2.54);
}

#[test]
fn a_plain_number_joins_the_family_it_is_used_with() {
    let mut ctx = Ctx::with("Width", number(4.0, Unit::Inches));
    let out = run(&bin(BinaryOp::Mul, cell("Width"), plain(0.5)), &mut ctx);
    assert_eq!(out, number(2.0, Unit::Inches));
}

#[test]
fn two_operands_that_measure_different_things_are_value_error() {
    let mut ctx = Ctx::new();
    let out = run(
        &bin(BinaryOp::Add, num(1.0, Some("in")), num(1.0, Some("es"))),
        &mut ctx,
    );
    assert_eq!(out, Value::Err(ErrorCode::Value));
}

#[test]
fn two_dimensioned_factors_are_dim_error() {
    let mut ctx = Ctx::new();
    let out = run(
        &bin(BinaryOp::Mul, num(1.0, Some("in")), num(1.0, Some("in"))),
        &mut ctx,
    );
    assert_eq!(out, Value::Err(ErrorCode::Dim));
    // A quotient of the same family is a plain ratio instead.
    let out = run(
        &bin(BinaryOp::Div, num(6.0, Some("in")), num(3.0, Some("in"))),
        &mut ctx,
    );
    assert_eq!(out, number(2.0, Unit::Unitsless));
}

#[test]
fn division_by_zero_is_div0() {
    let mut ctx = Ctx::new();
    for out in [
        run(&bin(BinaryOp::Div, plain(10.0), plain(0.0)), &mut ctx),
        run(&bin(BinaryOp::Div, plain(0.0), plain(0.0)), &mut ctx),
    ] {
        assert_eq!(out, Value::Err(ErrorCode::Div0));
    }
}

#[test]
fn a_non_finite_result_is_num_with_one_warning() {
    let mut ctx = Ctx::new();
    let out = run(&bin(BinaryOp::Mul, plain(1e308), plain(10.0)), &mut ctx);
    assert_eq!(out, Value::Err(ErrorCode::Num));
    assert!(matches!(one_warning(&ctx), EvalWarningKind::NonFinite));
}

#[test]
fn comparisons_coerce_both_sides_to_numbers() {
    let mut ctx = Ctx::new();
    let out = run(&bin(BinaryOp::Gt, text("10"), text("9")), &mut ctx);
    assert_eq!(out, Value::Bool(true));
    // Strings that convert to no number become 0 (about-operators.md).
    let out = run(&bin(BinaryOp::Eq, text("sheep"), text("sheep")), &mut ctx);
    assert_eq!(out, Value::Bool(true));
    assert_eq!(ctx.warnings.len(), 2, "both sides defaulted to zero");
    // TRUE converts to 1 without a warning.
    let out = run(&bin(BinaryOp::Eq, boolean(true), plain(1.0)), &mut ctx);
    assert_eq!(out, Value::Bool(true));
}

#[test]
fn the_ampersand_builds_a_string() {
    let mut ctx = Ctx::new();
    let cents = bin(
        BinaryOp::Concat,
        bin(BinaryOp::Mul, plain(2.0), plain(5.0)),
        text(" cents"),
    );
    assert_eq!(run(&cents, &mut ctx), Value::Str(String::from("10 cents")));
    let flag = bin(BinaryOp::Concat, boolean(true), text(""));
    assert_eq!(run(&flag, &mut ctx), Value::Str(String::from("TRUE")));
}

#[test]
fn unary_operators_negate_and_scale_percent() {
    let mut ctx = Ctx::new();
    assert_eq!(
        run(&un(UnaryOp::Minus, num(5.0, Some("ft"))), &mut ctx),
        number(-60.0, Unit::Inches)
    );
    approx(
        expect_number(
            run(&un(UnaryOp::Percent, plain(37.0)), &mut ctx),
            Unit::Unitsless,
        ),
        0.37,
    );
    // A unary plus is a no-op on the value (ADR 0051).
    assert_eq!(
        run(&un(UnaryOp::Plus, text("x")), &mut ctx),
        Value::Str(String::from("x"))
    );
}

#[test]
fn errors_propagate_through_operators_and_arguments() {
    let mut ctx = Ctx::new();
    assert_eq!(
        run(
            &bin(BinaryOp::Add, plain(1.0), call("NA", vec![])),
            &mut ctx
        ),
        Value::Err(ErrorCode::Na)
    );
    assert_eq!(
        run(&call("ABS", vec![call("NA", vec![])]), &mut ctx),
        Value::Err(ErrorCode::Na)
    );
}

// ---------- logic and control flow ----------

#[test]
fn if_evaluates_only_the_taken_branch() {
    let mut ctx = Ctx::new();
    let out = run(
        &call(
            "IF",
            vec![
                boolean(true),
                plain(1.0),
                bin(BinaryOp::Div, plain(1.0), plain(0.0)),
            ],
        ),
        &mut ctx,
    );
    assert_eq!(out, number(1.0, Unit::Unitsless));
    assert!(ctx.warnings.is_empty(), "the untaken branch never ran");
    // The untaken branch must not write either.
    let out = run(
        &call(
            "IF",
            vec![
                boolean(true),
                plain(1.0),
                call("SETF", vec![cell("PinX"), plain(9.0)]),
            ],
        ),
        &mut ctx,
    );
    assert_eq!(out, number(1.0, Unit::Unitsless));
    assert!(ctx.writes.is_empty());
}

#[test]
fn iferror_returns_the_alternate_on_error_only() {
    let mut ctx = Ctx::new();
    assert_eq!(
        run(
            &call(
                "IFERROR",
                vec![bin(BinaryOp::Div, plain(1.0), plain(0.0)), plain(7.0)]
            ),
            &mut ctx
        ),
        number(7.0, Unit::Unitsless)
    );
    assert_eq!(
        run(&call("IFERROR", vec![plain(3.0), plain(7.0)]), &mut ctx),
        number(3.0, Unit::Unitsless)
    );
}

#[test]
fn and_or_and_not_coerce_to_booleans() {
    let mut ctx = Ctx::new();
    assert_eq!(
        run(&call("AND", vec![plain(1.0), boolean(true)]), &mut ctx),
        Value::Bool(true)
    );
    assert_eq!(
        run(&call("OR", vec![plain(0.0), text("no")]), &mut ctx),
        Value::Bool(false)
    );
    assert_eq!(
        run(&call("NOT", vec![plain(0.0)]), &mut ctx),
        Value::Bool(true)
    );
}

// ---------- arithmetic functions ----------

#[test]
fn min_and_max_take_the_extreme_of_unit_values() {
    let mut ctx = Ctx::new();
    let out = run(
        &call("MIN", vec![num(1.0, Some("in")), num(2.0, Some("cm"))]),
        &mut ctx,
    );
    approx(expect_number(out, Unit::Inches), 2.0 / 2.54);
    let out = run(
        &call("MAX", vec![num(1.0, Some("ft")), num(5.0, Some("in"))]),
        &mut ctx,
    );
    assert_eq!(expect_number(out, Unit::Inches), 12.0);
}

#[test]
fn sum_adds_within_one_family() {
    let mut ctx = Ctx::new();
    let out = run(
        &call(
            "SUM",
            vec![num(1.0, Some("in")), num(1.0, Some("ft")), plain(6.0)],
        ),
        &mut ctx,
    );
    approx(expect_number(out, Unit::Inches), 19.0);
    // A sum across families is the same `#VALUE!` as the `+` operator.
    assert_eq!(
        run(
            &call("SUM", vec![num(1.0, Some("in")), num(1.0, Some("es"))]),
            &mut ctx
        ),
        Value::Err(ErrorCode::Value)
    );
}

#[test]
fn int_rounds_down_to_the_next_integer() {
    let mut ctx = Ctx::new();
    assert_eq!(
        run(&call("INT", vec![plain(2.9)]), &mut ctx),
        number(2.0, Unit::Unitsless)
    );
    assert_eq!(
        run(&call("INT", vec![plain(-2.1)]), &mut ctx),
        number(-3.0, Unit::Unitsless)
    );
}

#[test]
fn round_and_trunc_take_a_digit_count() {
    let mut ctx = Ctx::new();
    assert_eq!(
        run(&call("ROUND", vec![plain(2.5), plain(0.0)]), &mut ctx),
        number(3.0, Unit::Unitsless)
    );
    assert_eq!(
        run(&call("ROUND", vec![plain(-2.5), plain(0.0)]), &mut ctx),
        number(-3.0, Unit::Unitsless)
    );
    assert_eq!(
        run(&call("ROUND", vec![plain(1234.0), plain(-2.0)]), &mut ctx),
        number(1200.0, Unit::Unitsless)
    );
    assert_eq!(
        run(&call("TRUNC", vec![plain(-2.9), plain(0.0)]), &mut ctx),
        number(-2.0, Unit::Unitsless)
    );
    // An absurd digit count cannot overflow the scale.
    assert_eq!(
        run(&call("ROUND", vec![plain(2.5), plain(1e9)]), &mut ctx),
        number(2.5, Unit::Unitsless)
    );
}

#[test]
fn ceiling_and_floor_move_away_from_and_toward_zero() {
    let mut ctx = Ctx::new();
    assert_eq!(
        run(&call("CEILING", vec![plain(2.5), plain(2.0)]), &mut ctx),
        number(4.0, Unit::Unitsless)
    );
    assert_eq!(
        run(&call("CEILING", vec![plain(-2.5), plain(1.0)]), &mut ctx),
        number(-3.0, Unit::Unitsless)
    );
    // The one-argument form rounds away from zero to the next integer.
    assert_eq!(
        run(&call("CEILING", vec![plain(2.5)]), &mut ctx),
        number(3.0, Unit::Unitsless)
    );
    assert_eq!(
        run(&call("FLOOR", vec![plain(2.9), plain(2.0)]), &mut ctx),
        number(2.0, Unit::Unitsless)
    );
    assert_eq!(
        run(&call("FLOOR", vec![plain(-2.9), plain(1.0)]), &mut ctx),
        number(-2.0, Unit::Unitsless)
    );
    assert_eq!(
        run(&call("FLOOR", vec![plain(2.5)]), &mut ctx),
        number(2.0, Unit::Unitsless)
    );
    assert_eq!(
        run(&call("CEILING", vec![plain(5.0), plain(0.0)]), &mut ctx),
        Value::Err(ErrorCode::Div0)
    );
}

#[test]
fn modulus_keeps_the_sign_of_the_dividend() {
    let mut ctx = Ctx::new();
    assert_eq!(
        run(&call("MODULUS", vec![plain(7.0), plain(3.0)]), &mut ctx),
        number(1.0, Unit::Unitsless)
    );
    assert_eq!(
        run(&call("MOD", vec![plain(-7.0), plain(3.0)]), &mut ctx),
        number(-1.0, Unit::Unitsless)
    );
    assert_eq!(
        run(&call("MODULUS", vec![plain(7.0), plain(0.0)]), &mut ctx),
        Value::Err(ErrorCode::Div0)
    );
}

#[test]
fn sign_sees_the_fuzz() {
    let mut ctx = Ctx::new();
    assert_eq!(
        run(&call("SIGN", vec![plain(-5.0)]), &mut ctx),
        number(-1.0, Unit::Unitsless)
    );
    assert_eq!(
        run(&call("SIGN", vec![plain(3.0)]), &mut ctx),
        number(1.0, Unit::Unitsless)
    );
    assert_eq!(
        run(&call("SIGN", vec![plain(0.5), plain(1.0)]), &mut ctx),
        number(0.0, Unit::Unitsless)
    );
}

#[test]
fn abs_and_powers_keep_the_family() {
    let mut ctx = Ctx::new();
    assert_eq!(
        run(&call("ABS", vec![num(-2.0, Some("ft"))]), &mut ctx),
        number(24.0, Unit::Inches)
    );
    assert_eq!(
        run(&call("POW", vec![plain(2.0), plain(10.0)]), &mut ctx),
        number(1024.0, Unit::Unitsless)
    );
    // A non-integer power of a dimensioned value is `#DIM!`.
    assert_eq!(
        run(
            &call("POW", vec![num(5.2, Some("cm")), plain(1.5)]),
            &mut ctx
        ),
        Value::Err(ErrorCode::Dim)
    );
}

#[test]
fn sqrt_exp_ln_and_log10_check_their_domains() {
    let mut ctx = Ctx::new();
    assert_eq!(
        run(&call("SQRT", vec![plain(9.0)]), &mut ctx),
        number(3.0, Unit::Unitsless)
    );
    assert_eq!(
        run(&call("SQRT", vec![plain(-1.0)]), &mut ctx),
        Value::Err(ErrorCode::Num)
    );
    assert_eq!(
        run(&call("SQRT", vec![num(4.0, Some("in"))]), &mut ctx),
        Value::Err(ErrorCode::Dim)
    );
    approx(
        expect_number(
            run(&call("EXP", vec![plain(1.0)]), &mut ctx),
            Unit::Unitsless,
        ),
        std::f64::consts::E,
    );
    assert_eq!(
        run(&call("LN", vec![plain(0.0)]), &mut ctx),
        Value::Err(ErrorCode::Num)
    );
    assert_eq!(
        run(&call("LOG10", vec![plain(100.0)]), &mut ctx),
        number(2.0, Unit::Unitsless)
    );
}

// ---------- trigonometry and angle units ----------

#[test]
fn trigonometry_works_on_radians() {
    let mut ctx = Ctx::new();
    assert_eq!(
        run(&call("SIN", vec![plain(0.0)]), &mut ctx),
        number(0.0, Unit::Unitsless)
    );
    assert_eq!(
        run(&call("COS", vec![plain(0.0)]), &mut ctx),
        number(1.0, Unit::Unitsless)
    );
    // A plain number counts as a radians measure (Visio's internal unit).
    approx(
        expect_number(
            run(&call("SIN", vec![num(30.0, Some("deg"))]), &mut ctx),
            Unit::Unitsless,
        ),
        0.5,
    );
    approx(
        expect_number(
            run(&call("ATAN2", vec![plain(1.0), plain(1.0)]), &mut ctx),
            Unit::Radians,
        ),
        std::f64::consts::FRAC_PI_4,
    );
    approx(
        expect_number(
            run(&call("ASIN", vec![plain(1.0)]), &mut ctx),
            Unit::Radians,
        ),
        std::f64::consts::FRAC_PI_2,
    );
    assert_eq!(
        run(&call("ACOS", vec![plain(2.0)]), &mut ctx),
        Value::Err(ErrorCode::Num)
    );
    approx(
        expect_number(
            run(&call("TAN", vec![num(45.0, Some("deg"))]), &mut ctx),
            Unit::Unitsless,
        ),
        1.0,
    );
    // A length is not an angle.
    assert_eq!(
        run(&call("SIN", vec![num(1.0, Some("in"))]), &mut ctx),
        Value::Err(ErrorCode::Value)
    );
}

#[test]
fn deg_and_rad_convert_the_angle_measure() {
    let mut ctx = Ctx::new();
    approx(
        expect_number(
            run(&call("DEG", vec![call("PI", vec![])]), &mut ctx),
            Unit::Degrees,
        ),
        180.0,
    );
    approx(
        expect_number(
            run(&call("RAD", vec![plain(180.0)]), &mut ctx),
            Unit::Radians,
        ),
        std::f64::consts::PI,
    );
    // An angle value re-expresses itself.
    approx(
        expect_number(
            run(&call("DEG", vec![num(90.0, Some("deg"))]), &mut ctx),
            Unit::Degrees,
        ),
        90.0,
    );
    approx(
        expect_number(
            run(&call("RAD", vec![num(90.0, Some("deg"))]), &mut ctx),
            Unit::Radians,
        ),
        std::f64::consts::FRAC_PI_2,
    );
    assert_eq!(
        run(&call("DEG", vec![num(1.0, Some("in"))]), &mut ctx),
        Value::Err(ErrorCode::Value)
    );
}

// ---------- colour and text ----------

#[test]
fn rgb_builds_a_colour_from_three_components() {
    let mut ctx = Ctx::new();
    assert_eq!(
        run(
            &call("RGB", vec![plain(255.0), plain(0.0), plain(255.0)]),
            &mut ctx
        ),
        Value::Color(Color::new(255, 0, 255))
    );
    assert_eq!(
        run(
            &call("RGB", vec![plain(300.0), plain(0.0), plain(0.0)]),
            &mut ctx
        ),
        Value::Err(ErrorCode::Num)
    );
    assert_eq!(
        run(
            &call("RGB", vec![num(1.0, Some("in")), plain(0.0), plain(0.0)]),
            &mut ctx
        ),
        Value::Err(ErrorCode::Value)
    );
}

#[test]
fn strsame_compares_case_by_default() {
    let mut ctx = Ctx::new();
    assert_eq!(
        run(&call("STRSAME", vec![text("A"), text("a")]), &mut ctx),
        Value::Bool(false)
    );
    assert_eq!(
        run(
            &call("STRSAME", vec![text("A"), text("a"), boolean(true)]),
            &mut ctx
        ),
        Value::Bool(true)
    );
}

// ---------- reference and lookup ----------

#[test]
fn index_counts_from_zero_and_takes_an_errorvalue() {
    let mut ctx = Ctx::new();
    let list = text("cat;rat;;goat");
    assert_eq!(
        run(&call("INDEX", vec![plain(3.0), list.clone()]), &mut ctx),
        Value::Str(String::from("goat"))
    );
    // A doubled delimiter implies a null string at its position.
    assert_eq!(
        run(&call("INDEX", vec![plain(2.0), list]), &mut ctx),
        Value::Str(String::new())
    );
    assert_eq!(
        run(&call("INDEX", vec![plain(54.0), text(";1;2;3;")]), &mut ctx),
        Value::Str(String::new())
    );
    assert_eq!(
        run(
            &call(
                "INDEX",
                vec![plain(54.0), text(";1;2;3;"), text(";"), text("ERROR")]
            ),
            &mut ctx
        ),
        Value::Str(String::from("ERROR"))
    );
}

#[test]
fn a_bad_index_is_value_error_not_a_panic() {
    let mut ctx = Ctx::new();
    for index in [-1.0, 2.5] {
        let out = run(&call("INDEX", vec![plain(index), text("a;b")]), &mut ctx);
        assert_eq!(out, Value::Err(ErrorCode::Value));
    }
    // A list value indexes as values, not as a string.
    let list = Value::List(vec![
        Value::Str(String::from("x")),
        number(7.0, Unit::Inches),
    ]);
    let ctx = &mut Ctx::with("L", list);
    assert_eq!(
        run(&call("INDEX", vec![plain(1.0), cell("L")]), ctx),
        number(7.0, Unit::Inches)
    );
    assert_eq!(
        run(&call("INDEX", vec![plain(9.0), cell("L")]), ctx),
        Value::Str(String::new())
    );
}

#[test]
fn lookup_returns_the_zero_based_index_or_minus_one() {
    let mut ctx = Ctx::new();
    assert_eq!(
        run(
            &call("LOOKUP", vec![text("rat"), text("cat;rat;;goat")]),
            &mut ctx
        ),
        number(1.0, Unit::Unitsless)
    );
    // The search is case-insensitive (the reference's remarks).
    assert_eq!(
        run(
            &call("LOOKUP", vec![text("GOAT"), text("cat;rat;;goat")]),
            &mut ctx
        ),
        number(3.0, Unit::Unitsless)
    );
    assert_eq!(
        run(
            &call("LOOKUP", vec![text("t"), text("cat;rat;;goat"), text("a")]),
            &mut ctx
        ),
        number(3.0, Unit::Unitsless)
    );
    assert_eq!(
        run(&call("LOOKUP", vec![text("t"), text("cat;rat")]), &mut ctx),
        number(-1.0, Unit::Unitsless)
    );
    // A key that contains the delimiter cannot name an element.
    assert_eq!(
        run(&call("LOOKUP", vec![text("x;y"), text("a;b")]), &mut ctx),
        number(-1.0, Unit::Unitsless)
    );
    assert_eq!(
        run(
            &call("LOOKUP", vec![text("x"), text("x"), text("")]),
            &mut ctx
        ),
        number(-1.0, Unit::Unitsless)
    );
}

#[test]
fn na_is_the_na_error_value() {
    let mut ctx = Ctx::new();
    assert_eq!(
        run(&call("NA", vec![]), &mut ctx),
        Value::Err(ErrorCode::Na)
    );
}

#[test]
fn references_read_the_context_and_a_missing_cell_is_ref_error() {
    let mut ctx = Ctx::with("Width", number(2.0, Unit::Inches));
    assert_eq!(run(&cell("Width"), &mut ctx), number(2.0, Unit::Inches));
    assert_eq!(run(&cell("Gone"), &mut ctx), Value::Err(ErrorCode::Ref));
}

// ---------- GUARD and SETF ----------

#[test]
fn guard_is_a_value_pass_through() {
    let mut ctx = Ctx::new();
    assert_eq!(
        run(
            &call("GUARD", vec![bin(BinaryOp::Add, plain(2.0), plain(2.0))]),
            &mut ctx
        ),
        number(4.0, Unit::Unitsless)
    );
    assert_eq!(
        run(
            &call("GUARD", vec![bin(BinaryOp::Div, plain(1.0), plain(0.0))]),
            &mut ctx
        ),
        Value::Err(ErrorCode::Div0)
    );
}

#[test]
fn setf_writes_through_the_context_and_evaluates_to_the_value() {
    let mut ctx = Ctx::new();
    let out = run(
        &call("SETF", vec![cell("PinX"), num(3.0, Some("in"))]),
        &mut ctx,
    );
    assert_eq!(out, number(3.0, Unit::Inches));
    assert_eq!(
        ctx.writes,
        vec![(String::from("PinX"), number(3.0, Unit::Inches))]
    );
}

#[test]
fn a_refused_setf_target_is_ref_error_with_one_warning() {
    let mut ctx = Ctx::new();
    ctx.refuse = true;
    let out = run(&call("SETF", vec![cell("PinX"), plain(3.0)]), &mut ctx);
    assert_eq!(out, Value::Err(ErrorCode::Ref));
    assert!(matches!(
        one_warning(&ctx),
        EvalWarningKind::SetfRefused(name) if name == "PinX"
    ));
}

#[test]
fn a_setf_target_that_is_not_a_reference_is_ref_error() {
    let mut ctx = Ctx::new();
    let out = run(&call("SETF", vec![plain(5.0), plain(3.0)]), &mut ctx);
    assert_eq!(out, Value::Err(ErrorCode::Ref));
    assert!(matches!(one_warning(&ctx), EvalWarningKind::SetfRefused(_)));
    assert!(ctx.writes.is_empty());
}

#[test]
fn setf_does_not_write_an_error_value() {
    let mut ctx = Ctx::new();
    let out = run(
        &call("SETF", vec![cell("PinX"), call("NA", vec![])]),
        &mut ctx,
    );
    assert_eq!(out, Value::Err(ErrorCode::Na));
    assert!(ctx.writes.is_empty());
}

// ---------- the unknown-function and arity rules ----------

#[test]
fn an_unknown_function_keeps_the_cached_value_path_and_warns_once_per_formula() {
    let mut ctx = Ctx::new();
    let out = run(&call("THEMEVAL", vec![cell("Theme"), plain(0.0)]), &mut ctx);
    // The placeholder stands for "keep the cell's cached value" (ADR
    // 0053); the caller learns that from the warning.
    assert_eq!(out, Value::Err(ErrorCode::Na));
    assert!(matches!(
        one_warning(&ctx),
        EvalWarningKind::UnknownFunction(name) if name == "THEMEVAL"
    ));

    // Two calls in one formula still record one warning.
    let mut ctx = Ctx::new();
    let out = run(
        &bin(
            BinaryOp::Add,
            call("THEMEVAL", vec![]),
            call("THEMEVAL", vec![]),
        ),
        &mut ctx,
    );
    assert_eq!(out, Value::Err(ErrorCode::Na));
    assert_eq!(ctx.warnings.len(), 1);
}

#[test]
fn iferror_does_not_catch_the_unknown_function_path() {
    let mut ctx = Ctx::new();
    let out = run(
        &call("IFERROR", vec![call("THEMEVAL", vec![]), plain(7.0)]),
        &mut ctx,
    );
    assert_eq!(out, Value::Err(ErrorCode::Na));
    assert!(matches!(
        one_warning(&ctx),
        EvalWarningKind::UnknownFunction(_)
    ));
}

#[test]
fn a_reserved_internal_name_keeps_the_cached_value_path_with_its_own_text() {
    let mut ctx = Ctx::new();
    let out = run(&call("_MOD", vec![plain(1.0), plain(2.0)]), &mut ctx);
    assert_eq!(out, Value::Err(ErrorCode::Na));
    // The reserved case is the same keep-cached rule with its own
    // warning text (ADR 0053).
    assert!(matches!(
        one_warning(&ctx),
        EvalWarningKind::UnknownFunction(name) if name == "_MOD (reserved internal name)"
    ));
}

#[test]
fn a_wrong_argument_count_is_value_error_with_one_warning() {
    let mut ctx = Ctx::new();
    let out = run(&call("IF", vec![plain(1.0), plain(2.0)]), &mut ctx);
    assert_eq!(out, Value::Err(ErrorCode::Value));
    match one_warning(&ctx) {
        EvalWarningKind::WrongArgumentCount {
            function,
            min,
            max,
            got,
        } => {
            assert_eq!(function, "IF");
            assert_eq!(*min, 3);
            assert_eq!(*max, Some(3));
            assert_eq!(*got, 2);
        }
        other => panic!("expected a WrongArgumentCount warning, got {other:?}"),
    }
    // Nothing inside a mis-sized call evaluates: no write happens.
    let mut ctx = Ctx::new();
    let out = run(&call("SETF", vec![cell("PinX")]), &mut ctx);
    assert_eq!(out, Value::Err(ErrorCode::Value));
    assert!(ctx.writes.is_empty());
    assert_eq!(ctx.warnings.len(), 1);
}

#[test]
fn a_literal_with_an_unknown_unit_is_value_error() {
    let mut ctx = Ctx::new();
    let out = run(&num(5.0, Some("sq in")), &mut ctx);
    assert_eq!(out, Value::Err(ErrorCode::Value));
    assert!(matches!(one_warning(&ctx), EvalWarningKind::Coercion(_)));
}

// ---------- P6-17: the cheap coverage minors ----------

/// The arity edges the coverage audit found dark: `IF()` with no
/// arguments, `IF(FALSE, 1)` with no else branch and `SETF` with one
/// argument are all `#VALUE!`, and the one-argument `SETF` writes
/// nothing.
#[test]
fn the_arity_edges_evaluate_to_value_errors() {
    let mut ctx = Ctx::new();
    assert_eq!(
        run(&call("IF", vec![]), &mut ctx),
        Value::Err(ErrorCode::Value),
        "IF() with no arguments"
    );
    assert_eq!(
        run(&call("IF", vec![boolean(false), plain(1.0)]), &mut ctx),
        Value::Err(ErrorCode::Value),
        "IF(FALSE, 1) with no else branch"
    );
    assert_eq!(
        run(&call("SETF", vec![cell("Tgt")]), &mut ctx),
        Value::Err(ErrorCode::Value),
        "SETF with one argument"
    );
    assert!(ctx.writes.is_empty(), "a one-argument SETF writes nothing");
}
