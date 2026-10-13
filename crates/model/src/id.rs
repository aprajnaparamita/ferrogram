//! Stable identifiers for the in-memory arenas (ADR 0011).
//!
//! Every id is a dense index into the owning [`Document`](crate::Document)
//! arena. Ids are stable for the lifetime of a document: entries are never
//! removed or moved while any id is live, so undo (Phase 4) can hold ids
//! across mutations. Visio's own `ID` and `NameU` values are kept alongside
//! on each entity and are not used as arena keys.

/// An index into the shape arena of a [`Document`](crate::Document).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ShapeId {
    /// The arena index.
    pub index: u32,
}

/// An index into the page list of a [`Document`](crate::Document).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PageId {
    /// The list index.
    pub index: u32,
}

/// An index into the master list of a [`Document`](crate::Document).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MasterId {
    /// The list index.
    pub index: u32,
}

/// An index into the stylesheet rows of a [`Document`](crate::Document).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StyleId {
    /// The list index.
    pub index: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_hold_their_index() {
        assert_eq!(ShapeId { index: 7 }.index, 7);
        assert_eq!(PageId { index: 1 }.index, 1);
        assert_eq!(MasterId { index: 2 }.index, 2);
        assert_eq!(StyleId { index: 3 }.index, 3);
    }

    #[test]
    fn ids_are_distinct_types() {
        let s = ShapeId { index: 0 };
        assert_ne!(s, ShapeId { index: 1 });
    }
}
