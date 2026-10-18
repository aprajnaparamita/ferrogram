//! The crate error type.

use std::fmt;

use crate::id::{MasterId, PageId, ShapeId};

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
    /// The list id does not name a master of the document.
    UnknownMaster(MasterId),
    /// The mutation would make a shape its own ancestor (ADR 0039).
    WouldCycle,
    /// A cell write names a cell, row or value that is not valid (a missing
    /// row, a non-finite number, a zero-size `scale_geometry` source).
    BadCell(&'static str),
    /// A mutation would grow a document past its cap on input-derived
    /// counts (AGENTS.md section 6).
    TooManyShapes,
    /// The shape is not on the page the operation names.
    NotOnPage(ShapeId, PageId),
    /// The operation is not implemented yet (a Phase 5 interface stub,
    /// task P5-01).
    NotImplemented,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::UnknownShape(id) => write!(f, "unknown shape id {}", id.index),
            Error::UnknownPage(id) => write!(f, "unknown page id {}", id.index),
            Error::UnknownMaster(id) => write!(f, "unknown master id {}", id.index),
            Error::WouldCycle => write!(f, "the mutation would create a parent cycle"),
            Error::BadCell(why) => write!(f, "bad cell or value: {why}"),
            Error::TooManyShapes => write!(f, "too many shapes"),
            Error::NotOnPage(shape, page) => {
                write!(f, "shape {} is not on page {}", shape.index, page.index)
            }
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
    fn unknown_master_names_the_id() {
        let e = Error::UnknownMaster(MasterId { index: 5 });
        assert_eq!(e.to_string(), "unknown master id 5");
    }

    #[test]
    fn the_mutation_errors_say_what_failed() {
        assert_eq!(
            Error::WouldCycle.to_string(),
            "the mutation would create a parent cycle"
        );
        assert_eq!(
            Error::BadCell("non-finite value").to_string(),
            "bad cell or value: non-finite value"
        );
        assert_eq!(Error::TooManyShapes.to_string(), "too many shapes");
        assert_eq!(
            Error::NotOnPage(ShapeId { index: 2 }, PageId { index: 1 }).to_string(),
            "shape 2 is not on page 1"
        );
    }

    #[test]
    fn not_implemented_says_so() {
        assert_eq!(Error::NotImplemented.to_string(), "not implemented yet");
    }
}
