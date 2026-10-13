//! The crate error type.

use std::fmt;

/// A model error.
///
/// The model never panics on input (AGENTS.md section 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The operation is a planning stub and is not implemented yet.
    ///
    /// The payload names the operation, e.g. `"Document::resolve"`.
    NotImplemented(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotImplemented(op) => write!(f, "not implemented: {op}"),
        }
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_names_the_operation() {
        let e = Error::NotImplemented("Document::resolve");
        assert_eq!(e.to_string(), "not implemented: Document::resolve");
    }
}
