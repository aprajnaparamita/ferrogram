//! Crate errors and source spans (ADR 0051).
//!
//! The crate never panics on input (AGENTS.md section 6): a malformed
//! formula is an [`Error::Syntax`] with a [`Span`] into the source text,
//! and an operation that no task has implemented yet is an
//! [`Error::NotImplemented`]. Evaluation problems are *values*, not
//! errors: a formula that cannot compute a number evaluates to an error
//! value such as `#VALUE!` ([`crate::ErrorCode`], ADR 0052).

use std::fmt;

/// A half-open byte range in a formula's source text.
///
/// Spans are byte offsets into the formula as stored in the cell (without
/// the leading `=`), so a caller can underline the offending part of a
/// formula in an editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Span {
    /// Offset of the first byte.
    pub start: u32,
    /// Offset one past the last byte.
    pub end: u32,
}

impl Span {
    /// A span covering `start .. end`.
    ///
    /// A reversed range is not clamped: [`Span::len`] saturates at zero and
    /// [`Span::is_empty`] reports it as empty. The value is data, never a
    /// panic.
    pub const fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    /// An empty span positioned at `at`.
    pub const fn empty(at: u32) -> Self {
        Self { start: at, end: at }
    }

    /// The number of bytes the span covers.
    pub const fn len(self) -> u32 {
        self.end.saturating_sub(self.start)
    }

    /// True when the span covers no bytes.
    pub const fn is_empty(self) -> bool {
        self.start >= self.end
    }
}

/// Why a formula failed to parse (ADR 0051).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SyntaxKind {
    /// The source ended in the middle of a construct.
    UnexpectedEnd,
    /// A token appeared where the grammar allows none.
    UnexpectedToken,
    /// A string literal is missing its closing quote.
    UnterminatedString,
    /// A numeric literal does not parse as a finite number.
    InvalidNumber,
    /// A unit suffix is not a unit of measure Visio knows.
    InvalidUnit,
    /// A cell reference does not match any reference form.
    InvalidReference,
    /// The source is longer than [`crate::MAX_SOURCE_LEN`].
    InputTooLong,
    /// The formula has more tokens than [`crate::MAX_TOKENS`].
    TooManyTokens,
    /// Parentheses or brackets nest deeper than [`crate::MAX_NESTING`].
    NestingTooDeep,
}

impl fmt::Display for SyntaxKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            SyntaxKind::UnexpectedEnd => "unexpected end of formula",
            SyntaxKind::UnexpectedToken => "unexpected token",
            SyntaxKind::UnterminatedString => "unterminated string",
            SyntaxKind::InvalidNumber => "invalid number",
            SyntaxKind::InvalidUnit => "invalid unit",
            SyntaxKind::InvalidReference => "invalid cell reference",
            SyntaxKind::InputTooLong => "formula too long",
            SyntaxKind::TooManyTokens => "too many tokens",
            SyntaxKind::NestingTooDeep => "nesting too deep",
        };
        f.write_str(text)
    }
}

/// A formula or engine failure (ADR 0051).
///
/// The crate never panics on input (AGENTS.md section 6).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The operation is a planning stub and is not implemented yet.
    ///
    /// The payload names the operation, e.g. `"parse"`.
    NotImplemented(&'static str),
    /// A formula failed to parse at `span`.
    Syntax {
        /// Where in the source the problem is.
        span: Span,
        /// What kind of problem it is.
        kind: SyntaxKind,
    },
    /// A value or unit conversion failed (ADR 0052, P2-14).
    ///
    /// The payload names the conversion in one line.
    Conversion(&'static str),
    /// A `SETF` write was refused: its target names no cell the write
    /// can land on (ADR 0056).
    ///
    /// The payload names the target in one line. Requested by P6-05.
    SetfRefused(&'static str),
    /// The evaluation context broke: a cell it must read or write is
    /// not available (ADR 0055).
    ///
    /// The payload names the step in one line. Requested by P6-05.
    BrokenContext(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotImplemented(op) => write!(f, "not implemented: {op}"),
            Error::Syntax { span, kind } => {
                write!(f, "syntax error at {}..{}: {kind}", span.start, span.end)
            }
            Error::Conversion(what) => write!(f, "conversion failed: {what}"),
            Error::SetfRefused(target) => write!(f, "SETF refused: {target}"),
            Error::BrokenContext(step) => write!(f, "broken context: {step}"),
        }
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_names_the_operation() {
        let e = Error::NotImplemented("parse");
        assert_eq!(e.to_string(), "not implemented: parse");
    }

    #[test]
    fn display_reports_the_span_and_kind() {
        let e = Error::Syntax {
            span: Span::new(2, 5),
            kind: SyntaxKind::UnexpectedToken,
        };
        assert_eq!(e.to_string(), "syntax error at 2..5: unexpected token");
    }

    #[test]
    fn display_names_the_conversion() {
        assert_eq!(
            Error::Conversion("list with a unit").to_string(),
            "conversion failed: list with a unit"
        );
    }

    #[test]
    fn display_names_the_refused_setf_target() {
        assert_eq!(
            Error::SetfRefused("PinX of Sheet.8").to_string(),
            "SETF refused: PinX of Sheet.8"
        );
    }

    #[test]
    fn display_names_the_broken_context_step() {
        assert_eq!(
            Error::BrokenContext("read of User.Direction").to_string(),
            "broken context: read of User.Direction"
        );
    }

    #[test]
    fn every_syntax_kind_has_a_description() {
        for kind in [
            SyntaxKind::UnexpectedEnd,
            SyntaxKind::UnexpectedToken,
            SyntaxKind::UnterminatedString,
            SyntaxKind::InvalidNumber,
            SyntaxKind::InvalidUnit,
            SyntaxKind::InvalidReference,
            SyntaxKind::InputTooLong,
            SyntaxKind::TooManyTokens,
            SyntaxKind::NestingTooDeep,
        ] {
            assert!(!kind.to_string().is_empty());
        }
    }

    #[test]
    fn spans_clamp_and_measure() {
        assert_eq!(Span::new(1, 4).len(), 3);
        assert_eq!(Span::new(4, 4).len(), 0);
        assert!(Span::new(4, 4).is_empty());
        assert!(!Span::new(4, 5).is_empty());
        assert_eq!(Span::empty(7), Span::new(7, 7));
        // A reversed range measures zero and reads as empty; it never panics.
        assert_eq!(Span::new(9, 3).len(), 0);
        assert!(Span::new(9, 3).is_empty());
        assert_eq!(Span::default(), Span::new(0, 0));
    }
}
