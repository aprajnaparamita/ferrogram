//! The crate error type.

use std::fmt;

use crate::id::ShapeId;

/// A model error.
///
/// The model never panics on input (AGENTS.md section 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The operation is a planning stub and is not implemented yet.
    ///
    /// The payload names the operation, e.g. `"Document::resolve"`.
    ///
    /// No operation of this crate returns it any more (task P2-04
    /// implemented the stubs); the variant is kept because other crates use
    /// it as a stand-in model error in their tests.
    NotImplemented(&'static str),
    /// The arena id does not name a shape of the document.
    UnknownShape(ShapeId),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotImplemented(op) => write!(f, "not implemented: {op}"),
            Error::UnknownShape(id) => write!(f, "unknown shape id {}", id.index),
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

    #[test]
    fn unknown_shape_names_the_id() {
        let e = Error::UnknownShape(ShapeId { index: 7 });
        assert_eq!(e.to_string(), "unknown shape id 7");
        assert_ne!(e, Error::NotImplemented("x"));
    }
}
