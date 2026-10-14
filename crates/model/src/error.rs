//! The crate error type.

use std::fmt;

use crate::id::ShapeId;

/// A model error.
///
/// The model never panics on input (AGENTS.md section 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The arena id does not name a shape of the document.
    UnknownShape(ShapeId),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::UnknownShape(id) => write!(f, "unknown shape id {}", id.index),
        }
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_shape_names_the_id() {
        let e = Error::UnknownShape(ShapeId { index: 7 });
        assert_eq!(e.to_string(), "unknown shape id 7");
    }
}
