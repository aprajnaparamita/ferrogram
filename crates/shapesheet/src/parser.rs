//! The formula parser (ADR 0051).
//!
//! [`parse`] turns one formula source string into an [`Expr`], with spans
//! into the source for error reporting. The grammar, the reference forms
//! and the input limits are decided in ADR 0051; the tree is
//! [`crate::ast`]. Task P6-02 implements this module.
//!
//! Parsing never panics: bad input is [`Error::Syntax`] with a span, and
//! input over the limits is a syntax error of the matching
//! [`crate::SyntaxKind`].

use crate::ast::Expr;
use crate::{Error, Formula};

/// Parses one formula source text into an expression tree.
///
/// The source has no leading `=`, as [`Formula`] stores it. The stub
/// returns [`Error::NotImplemented`] until task P6-02 lands.
pub fn parse(source: &str) -> Result<Expr, Error> {
    let _ = source;
    Err(Error::NotImplemented("parse"))
}

/// Parses the source text of a [`Formula`] into an expression tree.
///
/// Same contract as [`parse`]; this wrapper is the seam the recalc pass
/// uses (ADR 0055).
pub fn parse_formula(formula: &Formula) -> Result<Expr, Error> {
    parse(formula.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_is_a_stub() {
        assert_eq!(parse("1+1"), Err(Error::NotImplemented("parse")));
        assert_eq!(
            parse("").unwrap_err().to_string(),
            "not implemented: parse".to_owned()
        );
    }

    #[test]
    fn parse_formula_reads_the_source_text() {
        let f = Formula::new("Width*0.5");
        assert_eq!(parse_formula(&f), Err(Error::NotImplemented("parse")));
    }
}
