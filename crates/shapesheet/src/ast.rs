//! The expression tree a formula parses to (ADR 0051).
//!
//! [`Expr`] is what [`crate::parse`] produces and what [`crate::print`]
//! turns back into source text. Parse-print-parse is the identity on the
//! tree, so a property test can round-trip any tree (ADR 0060).
//!
//! The tree keeps literals and references close to their written form:
//! a number keeps its unit suffix, a reference keeps its parts as
//! written. Interpreting them (unit conversion, row resolution) belongs
//! to evaluation and resolution (ADR 0054), not to the tree.

use crate::Span;
use crate::formula::CellRef;

/// One parsed expression with its source span.
#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    /// Where the expression sits in the formula source.
    pub span: Span,
    /// What the expression is.
    pub kind: ExprKind,
}

/// What an [`Expr`] is (ADR 0051).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ExprKind {
    /// A literal number, string or boolean.
    Literal(Literal),
    /// A cell reference.
    Ref(CellRef),
    /// A unary operator (`-1`, `+3`, `25%`).
    Unary {
        /// The operator.
        op: UnaryOp,
        /// The operand.
        operand: Box<Expr>,
    },
    /// A binary operator (`a + b`, `a & b`, `a <= b`).
    Binary {
        /// The operator.
        op: BinaryOp,
        /// The left operand.
        left: Box<Expr>,
        /// The right operand.
        right: Box<Expr>,
    },
    /// A function call (`IF(a, b, c)`).
    Call {
        /// The function name as written (`If`, `ROUND`).
        name: String,
        /// The arguments, in order.
        args: Vec<Expr>,
    },
}

/// A literal value in a formula (ADR 0051).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Literal {
    /// A number, with the unit it was written with.
    Number(NumberLiteral),
    /// A string (`"abc"`); the value is unescaped.
    Str(String),
    /// A boolean (`TRUE`, `FALSE`).
    Bool(bool),
}

/// A numeric literal (ADR 0051, "Implicit units of measure").
#[derive(Debug, Clone, PartialEq)]
pub struct NumberLiteral {
    /// The magnitude as written. A mixed fraction (`5 1/2`) is folded
    /// into one number on parse (5.5).
    pub value: f64,
    /// The unit suffix as written (`in`, `deg`, `%`), or `None`.
    pub unit: Option<String>,
    /// The measurement-system flag of the implicit form `5 [in, d]`
    /// (`A`ngular, `D`rawing, `E`lapsed, `P`age, `T`ype), or `None`.
    pub flag: Option<char>,
}

/// A unary operator (ADR 0051).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum UnaryOp {
    /// `+`, a no-op on the value.
    Plus,
    /// `-`, arithmetic negation.
    Minus,
    /// `%`, the percentage operator (`37%` is 0.37).
    Percent,
}

/// A binary operator (ADR 0051, "Order of evaluation").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BinaryOp {
    /// `+`.
    Add,
    /// `-`.
    Sub,
    /// `*`.
    Mul,
    /// `/`.
    Div,
    /// `^`, exponentiation.
    Pow,
    /// `&`, string concatenation.
    Concat,
    /// `=`, equal (`_EQ_` is the same operator spelled otherwise).
    Eq,
    /// `<>`, not equal (`_NE_`).
    Ne,
    /// `<`, less than (`_LT_`).
    Lt,
    /// `<=`, less than or equal (`_LE_`).
    Le,
    /// `>`, greater than (`_GT_`).
    Gt,
    /// `>=`, greater than or equal (`_GE_`).
    Ge,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_expression_carries_its_span_and_kind() {
        let e = Expr {
            span: Span::new(0, 1),
            kind: ExprKind::Literal(Literal::Bool(true)),
        };
        assert_eq!(e.span, Span::new(0, 1));
        assert!(matches!(e.kind, ExprKind::Literal(Literal::Bool(true))));
    }

    #[test]
    fn a_number_literal_keeps_its_unit_and_flag() {
        let n = NumberLiteral {
            value: 5.5,
            unit: Some(String::from("in")),
            flag: Some('d'),
        };
        assert_eq!(n.value, 5.5);
        assert_eq!(n.unit.as_deref(), Some("in"));
        assert_eq!(n.flag, Some('d'));
    }

    #[test]
    fn the_operator_families_are_closed() {
        assert_ne!(UnaryOp::Minus, UnaryOp::Percent);
        assert_ne!(BinaryOp::Add, BinaryOp::Concat);
        assert_eq!(BinaryOp::Ge, BinaryOp::Ge);
    }
}
