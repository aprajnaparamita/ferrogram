//! The crate error type.

use std::fmt;

/// A rendering error.
///
/// Every fallible operation in this crate returns `Result<T, Error>`. The
/// crate never panics on input (AGENTS.md section 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// An output dimension or byte count exceeds the caps for
    /// input-derived values (AGENTS.md section 6).
    TooLarge,
    /// The operation is not implemented yet (a Phase 3 stub).
    NotImplemented,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::TooLarge => write!(f, "output is larger than the allowed caps"),
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
            Error::TooLarge.to_string(),
            "output is larger than the allowed caps"
        );
        assert_eq!(Error::NotImplemented.to_string(), "not implemented yet");
    }
}
