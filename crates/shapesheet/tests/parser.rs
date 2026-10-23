//! The formula parser and the printer, through the crate's public API
//! (task P6-02, ADR 0051 and ADR 0060).
//!
//! Table-driven cases cover every reference form of the ADR 0054 table,
//! the operator order of `about-operators.md`, the unit spellings of
//! `about-units-of-measure.md` and the three input limits. The property
//! test is the round trip ADR 0060 asks for: parse-print-parse is the
//! identity on generated trees (spans aside), and a formula written the
//! way the corpus writes one survives the same trip.

// Test code: a failing `expect` is the bug under test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
use ferrogram_shapesheet::{
    BinaryOp, CellRef, Error, Expr, ExprKind, Formula, Literal, MAX_SOURCE_LEN, MAX_TOKENS,
    NumberLiteral, Span, SyntaxKind, UnaryOp, parse, parse_formula, print,
};

/// The parsed tree. `Expr` equality ignores spans (ADR 0051), so the
/// parse itself is the shape to compare.
fn tree(source: &str) -> Expr {
    parse(source).expect("the source parses")
}

/// The [`SyntaxKind`] the source fails with.
fn syntax_kind(source: &str) -> SyntaxKind {
    match parse(source).expect_err("the source does not parse") {
        Error::Syntax { kind, .. } => kind,
        other => panic!("expected a syntax error, got {other}"),
    }
}

/// The number literal `value [unit, flag]`.
fn number(value: f64, unit: Option<&str>, flag: Option<char>) -> Expr {
    Expr {
        span: Span::default(),
        kind: ExprKind::Literal(Literal::Number(NumberLiteral {
            value,
            unit: unit.map(str::to_owned),
            flag,
        })),
    }
}

/// The unary expression `op operand`.
fn unary(op: UnaryOp, operand: Expr) -> Expr {
    Expr {
        span: Span::default(),
        kind: ExprKind::Unary {
            op,
            operand: Box::new(operand),
        },
    }
}

/// The binary expression `left op right`.
fn binary(op: BinaryOp, left: Expr, right: Expr) -> Expr {
    Expr {
        span: Span::default(),
        kind: ExprKind::Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
        },
    }
}

#[test]
fn every_reference_form_of_the_table_parses_and_prints_back() {
    // ADR 0054's table of reference forms, plus the two corpus shapes
    // where a call names the sheet or the cell.
    for source in [
        "Width",
        "Wheel!Angle",
        "Frame.2!Height",
        "Char.Size[3]",
        "Scratch.B3",
        "Prop.Cost",
        "Geometry1.X2",
        "Sheet.12!FillForegnd",
        "Masters[Bearing]!Hub!Geometry1.X1",
        "ThePage!User.Anchor_Point",
        "Pages[Sheet-3]!Sheet.4!BeginX",
        "Styles!Ledger!LineColor",
        "TheDoc!DocScale",
        "'2-C'!LineColor",
        "LISTSHEETREF()!User.Direction",
        "Sheet.30!SheetRef()",
    ] {
        let expr = tree(source);
        let text = print(&expr).expect("the reference prints");
        assert_eq!(tree(&text), expr, "{source} printed as {text}");
    }
}

#[test]
fn a_reference_keeps_every_part_as_written() {
    let expr = tree("Masters[Bearing]!Hub!Geometry1.X1");
    let ExprKind::Ref(r) = expr.kind else {
        panic!("expected a reference");
    };
    assert_eq!(
        r.sheets,
        vec![String::from("Masters[Bearing]"), String::from("Hub")]
    );
    assert_eq!(r.section.as_deref(), Some("Geometry1"));
    assert_eq!(r.names, vec![String::from("X1")]);
    assert_eq!(r.name().as_deref(), Some("X1"));
    // The parser does not decide what a part *means* (ADR 0054).
    let expr = tree("Scratch.B3");
    let ExprKind::Ref(r) = expr.kind else {
        panic!("expected a reference");
    };
    assert_eq!(r.section.as_deref(), Some("Scratch"));
    assert_eq!(r.names, vec![String::from("B3")]);
}

#[test]
fn operator_precedence_matches_the_order_of_evaluation() {
    // `*` binds tighter than `+`.
    assert_eq!(tree("2+3*4"), tree("2+(3*4)"));
    assert_ne!(tree("2+3*4"), tree("(2+3)*4"));
    // `^` is right-associative.
    assert_eq!(tree("2^3^2"), tree("2^(3^2)"));
    assert_ne!(tree("2^3^2"), tree("(2^3)^2"));
    // A mixed fraction is one number.
    assert_eq!(tree("5 1/2"), number(5.5, None, None));
    // `%` is the unary percentage: 37% is 0.37.
    assert_eq!(
        tree("37%"),
        unary(UnaryOp::Percent, number(37.0, None, None))
    );
    assert_eq!(tree("%37"), tree("37%"));
    // `&` concatenates strings.
    assert_eq!(
        tree("\"a\" & 2"),
        binary(
            BinaryOp::Concat,
            Expr {
                span: Span::default(),
                kind: ExprKind::Literal(Literal::Str(String::from("a")))
            },
            number(2.0, None, None)
        )
    );
    // A unary sign binds tighter than `^` (`about-operators.md`, first).
    assert_eq!(tree("-2^2"), tree("(-2)^2"));
    assert_ne!(tree("-2^2"), tree("-(2^2)"));
    // The ordering comparisons bind tighter than the equality ones.
    assert_eq!(tree("1<2=3<4"), tree("(1<2)=(3<4)"));
    assert_eq!(tree("5<>2"), tree("5 _NE_ 2"));
}

#[test]
fn every_unit_spelling_parses_and_prints_back() {
    for (source, value, unit, flag) in [
        ("10MM", 10.0, "MM", None),
        ("0.5 in", 0.5, "in", None),
        ("3FEET/INCH", 3.0, "FEET/INCH", None),
        ("5'", 5.0, "'", None),
        ("5\"", 5.0, "\"", None),
        ("5°", 5.0, "°", None),
        ("0DA", 0.0, "DA", None),
        ("2ED", 2.0, "ED", None),
        ("5[deg,A]", 5.0, "deg", Some('A')),
        ("2 [in, d]", 2.0, "in", Some('d')),
        ("5[%]", 5.0, "%", None),
        ("5 1/2 in", 5.5, "in", None),
    ] {
        let expr = tree(source);
        assert_eq!(expr, number(value, Some(unit), flag), "{source}");
        let text = print(&expr).expect("the literal prints");
        assert_eq!(tree(&text), expr, "{source} printed as {text}");
    }
}

#[test]
fn a_function_call_needs_its_parentheses() {
    let ExprKind::Call { name, args } = tree("DEFAULTEVENT()").kind else {
        panic!("expected a call");
    };
    assert_eq!(name, "DEFAULTEVENT");
    assert!(args.is_empty());
    // Without parentheses the name is a reference.
    assert!(matches!(tree("Width").kind, ExprKind::Ref(_)));
}

#[test]
fn unknown_names_parse_they_fail_later_if_at_all() {
    // An unknown function name is a call like any other (ADR 0053).
    let ExprKind::Call { name, .. } = tree("THEMEVAL(1, TRUE, \"x\")").kind else {
        panic!("expected a call");
    };
    assert_eq!(name, "THEMEVAL");
    // An unknown reference form keeps its parts as written.
    let ExprKind::Ref(r) = tree("Some.New.Fangled[3].Name").kind else {
        panic!("expected a reference");
    };
    assert_eq!(r.section.as_deref(), Some("Some"));
    assert_eq!(
        r.names,
        vec![
            String::from("New"),
            String::from("Fangled[3]"),
            String::from("Name")
        ]
    );
}

#[test]
fn a_malformed_formula_is_a_spanned_syntax_error() {
    for (source, kind, span) in [
        ("", SyntaxKind::UnexpectedEnd, Span::new(0, 0)),
        ("1+", SyntaxKind::UnexpectedEnd, Span::new(2, 2)),
        ("(Width", SyntaxKind::UnexpectedEnd, Span::new(6, 6)),
        ("Width)", SyntaxKind::UnexpectedToken, Span::new(5, 6)),
        ("1 2", SyntaxKind::UnexpectedToken, Span::new(2, 3)),
        (r#""abc"#, SyntaxKind::UnterminatedString, Span::new(0, 4)),
        ("1e400", SyntaxKind::InvalidNumber, Span::new(0, 5)),
        ("5 1/0", SyntaxKind::InvalidNumber, Span::new(2, 5)),
        ("5[in,XX]", SyntaxKind::InvalidUnit, Span::new(5, 7)),
        ("Width.!PinX", SyntaxKind::InvalidReference, Span::new(6, 7)),
    ] {
        assert_eq!(
            parse(source).expect_err("the source does not parse"),
            Error::Syntax { span, kind },
            "{source}"
        );
    }
}

#[test]
fn the_input_length_limit_is_a_syntax_error() {
    let long = "1".repeat(MAX_SOURCE_LEN + 1);
    assert_eq!(syntax_kind(&long), SyntaxKind::InputTooLong);
    // Exactly at the limit still lexes.
    let exact = format!("\"{}\"", "a".repeat(MAX_SOURCE_LEN - 2));
    assert!(parse(&exact).is_ok());
}

#[test]
fn the_token_limit_is_a_syntax_error() {
    let many = "1 ".repeat(MAX_TOKENS + 1);
    assert_eq!(syntax_kind(&many), SyntaxKind::TooManyTokens);
}

#[test]
fn the_nesting_limit_is_a_syntax_error() {
    let deep = format!("{}1{}", "(".repeat(33), ")".repeat(33));
    assert_eq!(syntax_kind(&deep), SyntaxKind::NestingTooDeep);
    let max = format!("{}1{}", "(".repeat(32), ")".repeat(32));
    assert!(parse(&max).is_ok());
}

#[test]
fn parse_formula_reads_the_source_text_of_the_formula() {
    let formula = Formula::new("Width*0.5");
    let back = parse_formula(&formula).expect("the formula parses");
    assert_eq!(back, tree("Width*0.5"));
}

#[test]
fn a_formula_written_the_way_the_corpus_writes_one_round_trips() {
    // Formula shapes taken from the corpus census (docs/sources.md
    // source 20) and written here as our own examples: a guarded
    // half-size, a millimetre literal scaled by a cell, a quoted name,
    // a percentage and an angle, nested calls and both comparison
    // spellings.
    for source in [
        "GUARD(Width*0.5)",
        "GUARD(Height*0.5)",
        "10MM*Width",
        "IF(Width>Height, 1 in, 2 in)",
        "IF(OR(STRSAME(TheText, \"\"), HideText), 5, 0)",
        "MAX(TEXTWIDTH(TheText), 5*Char.Size)",
        "(Height/2)-(Sheet.8!User.BarHeight/2)",
        "IF(User.HeadingPos=1, 270DEG, IF(User.HeadingPos=3, 90DEG, 0DEG))",
        "SETATREF(Controls.TextPosition.Y)",
        "-TxtHeight*0.5",
        "2*Geometry1.X1/Height",
        "IF(_MOD(User.Style, 6)>0, 80%, 0%)",
        "AND(Height=User.DefaultHeight, Width=User.DefaultWidth)",
    ] {
        let expr = tree(source);
        let text = print(&expr).expect("the formula prints");
        assert_eq!(tree(&text), expr, "{source} printed as {text}");
    }
}

/// A unit suffix as a number literal can carry it.
fn arb_number_literal() -> impl proptest::strategy::Strategy<Value = NumberLiteral> {
    use proptest::prelude::*;
    let magnitude = any::<f64>().prop_filter("finite and non-negative", |v| {
        v.is_finite() && !v.is_sign_negative()
    });
    let spelling = prop_oneof![
        Just((None, None)),
        Just((Some(String::from("in")), None)),
        Just((Some(String::from("MM")), None)),
        Just((Some(String::from("eh")), None)),
        Just((Some(String::from("FEET/INCH")), None)),
        Just((Some(String::from("%")), None)),
        Just((Some(String::from("'")), None)),
        Just((Some(String::from("\"")), None)),
        Just((Some(String::from("°")), None)),
        Just((Some(String::from("deg")), Some('A'))),
        Just((Some(String::from("in")), Some('d'))),
        Just((Some(String::from("%")), Some('E'))),
        Just((Some(String::from("°")), Some('t'))),
    ];
    (magnitude, spelling).prop_map(|(value, (unit, flag))| NumberLiteral { value, unit, flag })
}

/// A name that can stand alone: a call part only follows `!` or `.`,
/// because `SheetRef()` on its own is a call, not a reference.
fn arb_bare_name() -> impl proptest::strategy::Strategy<Value = String> {
    use proptest::prelude::*;
    prop_oneof![
        Just(String::from("Width")),
        Just(String::from("X2")),
        Just(String::from("Font[3]")),
        Just(String::from("A5")),
        Just(String::from("Cost")),
        Just(String::from("Anchor_Point")),
        Just(String::from("'2-C'")),
    ]
}

/// Any reference part as the parser writes it back.
fn arb_part() -> impl proptest::strategy::Strategy<Value = String> {
    use proptest::prelude::*;
    prop_oneof![arb_bare_name(), Just(String::from("SheetRef()")),]
}

/// A reference in one of its forms (ADR 0054).
fn arb_cell_ref() -> impl proptest::strategy::Strategy<Value = CellRef> {
    use proptest::prelude::*;
    let sheets = proptest::collection::vec(arb_part(), 0..3);
    let tail = prop_oneof![
        (Just(None), arb_bare_name().prop_map(|part| vec![part])),
        (
            arb_part().prop_map(Some),
            proptest::collection::vec(arb_part(), 1..3)
        ),
    ];
    (sheets, tail).prop_map(|(sheets, (section, names))| CellRef {
        sheets,
        section,
        names,
    })
}

/// A tree of bounded depth: literals, references, calls, unary and
/// binary operators (ADR 0060's generated trees).
fn arb_expr() -> impl proptest::strategy::Strategy<Value = Expr> {
    use proptest::prelude::*;
    fn literal(literal: Literal) -> Expr {
        Expr {
            span: Span::default(),
            kind: ExprKind::Literal(literal),
        }
    }
    let leaf = prop_oneof![
        arb_number_literal().prop_map(|n| literal(Literal::Number(n))),
        ".*".prop_map(|s: String| literal(Literal::Str(s))),
        any::<bool>().prop_map(|b| literal(Literal::Bool(b))),
        arb_cell_ref().prop_map(|r| Expr {
            span: Span::default(),
            kind: ExprKind::Ref(r)
        }),
    ];
    let unary_op = prop_oneof![
        Just(UnaryOp::Plus),
        Just(UnaryOp::Minus),
        Just(UnaryOp::Percent)
    ];
    let binary_op = prop_oneof![
        Just(BinaryOp::Add),
        Just(BinaryOp::Sub),
        Just(BinaryOp::Mul),
        Just(BinaryOp::Div),
        Just(BinaryOp::Pow),
        Just(BinaryOp::Concat),
        Just(BinaryOp::Eq),
        Just(BinaryOp::Ne),
        Just(BinaryOp::Lt),
        Just(BinaryOp::Le),
        Just(BinaryOp::Gt),
        Just(BinaryOp::Ge)
    ];
    leaf.prop_recursive(4, 24, 3, move |inner| {
        prop_oneof![
            (inner.clone(), unary_op.clone()).prop_map(|(operand, op)| Expr {
                span: Span::default(),
                kind: ExprKind::Unary {
                    op,
                    operand: Box::new(operand)
                }
            }),
            (inner.clone(), binary_op.clone(), inner.clone()).prop_map(|(left, op, right)| Expr {
                span: Span::default(),
                kind: ExprKind::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right)
                }
            }),
            (
                prop_oneof![Just(String::from("IF")), Just(String::from("ROUND"))],
                proptest::collection::vec(inner.clone(), 0..3)
            )
                .prop_map(|(name, args)| Expr {
                    span: Span::default(),
                    kind: ExprKind::Call { name, args }
                }),
        ]
    })
}

proptest::proptest! {
    /// The property of ADR 0060: parse-print-parse is the identity on
    /// generated trees (spans aside).
    #[test]
    fn parse_print_parse_is_the_identity(expr in arb_expr()) {
        let text = print(&expr).expect("the tree prints");
        let back = parse(&text).expect("the text parses");
        proptest::prop_assert_eq!(back, expr);
    }

    /// The same trip for a number literal alone: the printer writes the
    /// shortest form that reads back as the same value.
    #[test]
    fn a_number_literal_round_trips_by_value(literal in arb_number_literal()) {
        let expr = Expr { span: Span::default(), kind: ExprKind::Literal(Literal::Number(literal)) };
        let text = print(&expr).expect("the literal prints");
        let back = parse(&text).expect("the text parses");
        proptest::prop_assert_eq!(back, expr);
    }
}
