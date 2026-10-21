//! The formula printer (ADR 0051).
//!
//! [`print()`] turns an [`Expr`] back into source text that [`crate::parse`]
//! parses to an equal tree (spans aside). That round trip is the property
//! test of ADR 0060 and the reason the tree keeps literals and references
//! close to their written form.
//!
//! Task P6-02 implements this module.

use crate::Error;
use crate::ast::Expr;

/// Prints one expression as formula source text (no leading `=`).
///
/// The stub returns [`Error::NotImplemented`] until task P6-02 lands.
pub fn print(expr: &Expr) -> Result<String, Error> {
    let _ = expr;
    Err(Error::NotImplemented("print"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Span;
    use crate::ast::{ExprKind, Literal};

    #[test]
    fn print_is_a_stub() {
        let expr = Expr {
            span: Span::new(0, 1),
            kind: ExprKind::Literal(Literal::Bool(false)),
        };
        assert_eq!(print(&expr), Err(Error::NotImplemented("print")));
    }
}
