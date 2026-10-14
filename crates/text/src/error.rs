//! The crate error type.

use std::fmt;

/// A text layout error.
///
/// Every fallible operation in this crate returns `Result<T, Error>`. The
/// crate never panics on input (AGENTS.md section 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The font bytes are not a font this crate can parse.
    InvalidFont,
    /// The operation is not implemented yet (a Phase 3 stub).
    NotImplemented,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidFont => write!(f, "bytes are not a parsable font"),
            Error::NotImplemented => write!(f, "not implemented yet"),
        }
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_describes_the_error() {
        assert_eq!(
            Error::InvalidFont.to_string(),
            "bytes are not a parsable font"
        );
        assert_eq!(Error::NotImplemented.to_string(), "not implemented yet");
    }

    #[test]
    fn is_a_std_error() {
        let e: Box<dyn std::error::Error> = Box::new(Error::InvalidFont);
        assert!(e.to_string().contains("font"));
    }
}
