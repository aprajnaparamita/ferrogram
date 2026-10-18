//! Package edits the writer needs for Phase 5 (ADR 0048): removing a part
//! (page or master delete) and allocating relationship ids for new parts
//! (new pages).
//!
//! Task P5-02 implements the bodies. The bodies are
//! `Err(Error::NotImplemented)` stubs until then.

use crate::error::Error;
use crate::package::{Package, PartName, RelationshipId};

impl Package {
    /// Removes the part and every relationship whose source or target is
    /// it, plus its `[Content_Types].xml` override (a deleted page or
    /// master must not leave its part behind: the P2-07 deferred item
    /// that Phase 5 page delete triggers, ADR 0048).
    ///
    /// `Ok(false)` when no part of that name existed. Removing the
    /// package's root part is an `Err`.
    pub fn remove_part(&mut self, name: &PartName) -> Result<bool, Error> {
        let _ = name;
        Err(Error::NotImplemented)
    }

    /// The next free relationship id for `source` (`rId1`, `rId2`, ...),
    /// never reusing one that part already declares (a new page part's
    /// rels, ADR 0048).
    pub fn next_relationship_id(&self, source: &PartName) -> Result<RelationshipId, Error> {
        let _ = source;
        Err(Error::NotImplemented)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The P5-01 stubs: replaced by real behaviour tests in task P5-02.
    #[test]
    fn the_p5_01_stubs_are_not_implemented() {
        let mut package = Package::new();
        let name = PartName::new("/visio/page1.xml");
        assert_eq!(package.remove_part(&name), Err(Error::NotImplemented));
        assert_eq!(
            package.next_relationship_id(&name),
            Err(Error::NotImplemented)
        );
    }
}
