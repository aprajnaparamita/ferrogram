//! The formula lexer (ADR 0051).
//!
//! One formula is one line of source text without the leading `=`. The
//! lexer is bounded like every other part of the crate: input length,
//! token count and nesting depth have limits, and a limit is an
//! [`Error::Syntax`], never a panic (AGENTS.md section 6).
//!
//! Task P6-02 implements [`tokenize`] and may change [`TokenKind`] as
//! the grammar requires; the stubs below are the shape it starts from.

use crate::{Error, Span};

/// The longest formula source text the parser accepts, in bytes.
pub const MAX_SOURCE_LEN: usize = 65_536;

/// The most tokens one formula may have.
pub const MAX_TOKENS: usize = 4_096;

/// The deepest `()`, `[]` or `''` nesting the parser accepts.
pub const MAX_NESTING: usize = 32;

/// One lexical token with its source span (ADR 0051).
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// Where the token sits in the source.
    pub span: Span,
    /// What the token is.
    pub kind: TokenKind,
}

/// What a token is (ADR 0051).
///
/// Identifiers, references and unit suffixes are carried as written; the
/// parser assembles them into [`crate::CellRef`] and
/// [`crate::NumberLiteral`] values.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum TokenKind {
    /// A number literal (`2`, `0.5`, `1e-3`, `5 1/2` folded).
    Number(f64),
    /// A quoted string body, unescaped (`abc` of `"abc"`).
    Str(String),
    /// An identifier or name part (`IF`, `Width`, `Sheet.4`).
    Ident(String),
    /// `TRUE` or `FALSE`.
    Bool(bool),
    /// A punctuation or operator character (`+`, `(`, `,`, `.`).
    Symbol(char),
    /// The end of the source.
    End,
}

/// Splits `source` into tokens.
///
/// Returns [`Error::Syntax`] with the offending [`Span`] on bad input, or
/// [`Error::NotImplemented`] while the lexer is a stub (task P6-02).
pub fn tokenize(source: &str) -> Result<Vec<Token>, Error> {
    let _ = source;
    Err(Error::NotImplemented("tokenize"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SyntaxKind;

    #[test]
    fn tokenize_is_a_stub() {
        assert_eq!(tokenize("1+1"), Err(Error::NotImplemented("tokenize")));
        assert_eq!(
            tokenize("").unwrap_err().to_string(),
            "not implemented: tokenize"
        );
    }

    #[test]
    fn limits_are_bounded_and_ordered() {
        const {
            assert!(MAX_SOURCE_LEN > 0);
            assert!(MAX_TOKENS > 0);
            assert!(MAX_NESTING > 0);
        }
    }

    #[test]
    fn a_token_carries_its_span_and_kind() {
        let t = Token {
            span: Span::new(0, 2),
            kind: TokenKind::Number(1.0),
        };
        assert_eq!(t.span.len(), 2);
        assert!(matches!(t.kind, TokenKind::Number(1.0)));
    }

    #[test]
    fn every_token_kind_exists() {
        let kinds = [
            TokenKind::Number(0.0),
            TokenKind::Str(String::new()),
            TokenKind::Ident(String::from("Width")),
            TokenKind::Bool(false),
            TokenKind::Symbol('+'),
            TokenKind::End,
        ];
        assert_eq!(kinds.len(), 6);
    }

    #[test]
    fn the_limits_count_as_syntax_kinds() {
        let bad = Error::Syntax {
            span: Span::empty(0),
            kind: SyntaxKind::InputTooLong,
        };
        assert!(bad.to_string().contains("too long"));
    }
}
