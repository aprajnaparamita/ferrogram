//! Masters: the shape definitions a stencil holds and a drawing instantiates.

use crate::id::{MasterId, ShapeId};
use crate::raw::Preserved;

/// One master of a document's master list (`masters.xml`).
#[derive(Debug, Clone, PartialEq)]
pub struct Master {
    /// The list id of this master.
    pub id: MasterId,
    /// Visio's `ID` attribute, unique within the document.
    pub visio_id: i32,
    /// Visio's `Name` attribute (user-visible name), as written.
    pub name: String,
    /// Visio's `NameU` attribute (unique name), as written.
    pub name_u: String,
    /// Visio's `UniqueID` attribute (a GUID string), or `None`.
    pub unique_id: Option<String>,
    /// Visio's `Prompt` attribute (the tooltip text), or `None`.
    pub prompt: Option<String>,
    /// Whether the master is hidden (`Hidden="1"`).
    pub hidden: bool,
    /// The arena id of the master's content shape, or `None` when the master
    /// part is missing (a malformed but recoverable file).
    pub shape: Option<ShapeId>,
    /// Unmapped attributes and children of the `Master` element kept
    /// verbatim (ADR 0012).
    pub preserved: Preserved,
}

impl Master {
    /// Creates a master with the given list id and Visio `ID`, empty
    /// otherwise.
    pub fn new(id: MasterId, visio_id: i32) -> Self {
        Self {
            id,
            visio_id,
            name: String::new(),
            name_u: String::new(),
            unique_id: None,
            prompt: None,
            hidden: false,
            shape: None,
            preserved: Preserved::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_master_is_empty() {
        let m = Master::new(MasterId { index: 1 }, 3);
        assert_eq!(m.id, MasterId { index: 1 });
        assert_eq!(m.visio_id, 3);
        assert!(m.name.is_empty());
        assert!(m.name_u.is_empty());
        assert_eq!(m.unique_id, None);
        assert_eq!(m.prompt, None);
        assert!(!m.hidden);
        assert_eq!(m.shape, None);
        assert!(m.preserved.children.is_empty());
    }
}
