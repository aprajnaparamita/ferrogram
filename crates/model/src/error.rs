//! The crate error type.

use std::fmt;

use crate::id::{PageId, ShapeId};

/// A model error.
///
/// The model never panics on input (AGENTS.md section 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The arena id does not name a shape of the document.
    UnknownShape(ShapeId),
    /// The list id does not name a page of the document.
    UnknownPage(PageId),
    /// The operation is not implemented yet (a Phase 5 interface stub,
    /// task P5-01).
    NotImplemented,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::UnknownShape(id) => write!(f, "unknown shape id {}", id.index),
            Error::UnknownPage(id) => write!(f, "unknown page id {}", id.index),
            Error::NotImplemented => write!(f, "not implemented yet"),
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

    #[test]
    fn unknown_page_names_the_id() {
        let e = Error::UnknownPage(PageId { index: 3 });
        assert_eq!(e.to_string(), "unknown page id 3");
    }

    #[test]
    fn not_implemented_says_so() {
        assert_eq!(Error::NotImplemented.to_string(), "not implemented yet");
    }
}
