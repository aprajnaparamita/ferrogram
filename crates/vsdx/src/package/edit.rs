//! Package edits the writer needs for Phase 5 (ADR 0048): removing a part
//! (page or master delete) and allocating relationship ids for new parts
//! (new pages).

use crate::error::Error;
use crate::package::discovery::resolve_target;
use crate::package::{ContentTypeKind, Package, PartName, RelationshipId, TargetMode};

/// The package root part (the source of `/_rels/.rels`): it is never
/// removed.
const PACKAGE_ROOT: &str = "/";

impl Package {
    /// Removes the part and every relationship whose source or target is
    /// it, plus its `[Content_Types].xml` override (a deleted page or
    /// master must not leave its part behind: the P2-07 deferred item
    /// that Phase 5 page delete triggers, ADR 0048).
    ///
    /// `Ok(false)` when no part of that name existed. Removing the
    /// package's root part is an [`Error::InvalidPackage`].
    pub fn remove_part(&mut self, name: &PartName) -> Result<bool, Error> {
        if name.as_str() == PACKAGE_ROOT {
            return Err(Error::InvalidPackage(
                "cannot remove the package root part".to_owned(),
            ));
        }
        let existed = self.parts.iter().any(|part| &part.name == name);
        if !existed {
            return Ok(false);
        }
        self.parts.retain(|part| &part.name != name);
        // Every relationship whose source or target is the part goes with
        // it; a target is resolved against its source first (an external
        // target is never a part).
        self.relationships.retain(|relationship| {
            let targets_part = relationship.target_mode == TargetMode::Internal
                && resolve_target(relationship.source.as_str(), &relationship.target) == *name;
            &relationship.source != name && !targets_part
        });
        self.content_types.retain(|decl| {
            !(decl.kind == ContentTypeKind::Override && normalize(&decl.name) == name.as_str())
        });
        Ok(true)
    }

    /// The next free relationship id for `source` (`rId1`, `rId2`, ...),
    /// never reusing one that part already declares (a new page part's
    /// rels, ADR 0048).
    ///
    /// Among the spellings `rId1`..`rIdN+1` (N relationships of the part)
    /// one is always free, so the allocation is bounded.
    pub fn next_relationship_id(&self, source: &PartName) -> Result<RelationshipId, Error> {
        let used: Vec<&str> = self
            .relationships_from(source)
            .map(|relationship| relationship.id.as_str())
            .collect();
        let bound = used.len().saturating_add(1);
        for n in 1..=bound {
            let candidate = format!("rId{n}");
            if !used.iter().any(|id| *id == candidate) {
                return Ok(RelationshipId::new(candidate));
            }
        }
        Ok(RelationshipId::new(format!(
            "rId{}",
            bound.saturating_add(1)
        )))
    }
}

/// A declaration name as an absolute part name.
fn normalize(name: &str) -> String {
    let trimmed = name.trim_start_matches('/');
    format!("/{trimmed}")
}

#[cfg(test)]
mod tests {
    use ferrogram_model::Preserved;

    use super::*;
    use crate::package::{
        ContentType, ContentTypeDecl, Part, PartData, Relationship, write_package,
    };

    /// A package with a main part, a page list and two page contents
    /// parts.
    #[allow(clippy::expect_used)] // test helper
    fn package() -> Package {
        let mut package = Package::new();
        for name in [
            "/visio/document.xml",
            "/visio/pages/pages.xml",
            "/visio/pages/page1.xml",
            "/visio/pages/page2.xml",
        ] {
            package.parts.push(Part {
                name: PartName::new(name),
                content_type: ContentType::new("application/xml"),
                data: PartData::Xml(ferrogram_model::XmlNode {
                    name: ferrogram_model::QName::local("Root"),
                    attributes: Vec::new(),
                    children: Vec::new(),
                    self_closing: true,
                }),
            });
            package.content_types.push(ContentTypeDecl {
                kind: ContentTypeKind::Override,
                name: name.to_owned(),
                content_type: "application/vnd.ms-visio.page+xml".to_owned(),
            });
        }
        for (source, id, target) in [
            ("/", "rId1", "visio/document.xml"),
            ("/visio/document.xml", "rId1", "pages/pages.xml"),
            ("/visio/pages/pages.xml", "rId1", "page1.xml"),
            ("/visio/pages/pages.xml", "rId2", "page2.xml"),
        ] {
            package.relationships.push(Relationship {
                source: PartName::new(source),
                id: RelationshipId::new(id),
                rel_type: "http://schemas.microsoft.com/visio/2010/relationships/page".to_owned(),
                target: target.to_owned(),
                target_mode: TargetMode::Internal,
                preserved: Preserved::new(),
            });
        }
        package
    }

    #[test]
    fn remove_part_drops_relationships_and_the_content_type() {
        let mut package = package();
        assert!(
            package
                .remove_part(&PartName::new("/visio/pages/page2.xml"))
                .expect("remove")
        );
        assert!(
            package
                .part(&PartName::new("/visio/pages/page2.xml"))
                .is_none()
        );
        // The relationship from the page list to the part is gone.
        assert_eq!(
            package
                .relationships_from(&PartName::new("/visio/pages/pages.xml"))
                .count(),
            1
        );
        // The content-type override is gone.
        assert!(
            !package
                .content_types
                .iter()
                .any(|decl| decl.name.contains("page2.xml"))
        );
        // The rest is untouched.
        assert!(
            package
                .part(&PartName::new("/visio/pages/page1.xml"))
                .is_some()
        );
        assert_eq!(
            package
                .relationships_from(&PartName::new("/visio/pages/pages.xml"))
                .map(|relationship| relationship.id.as_str())
                .collect::<Vec<_>>(),
            vec!["rId1"]
        );
    }

    #[test]
    fn remove_part_drops_relationships_it_is_the_source_of() {
        let mut package = package();
        package.relationships.push(Relationship {
            source: PartName::new("/visio/pages/page1.xml"),
            id: RelationshipId::new("rId1"),
            rel_type: "http://schemas.microsoft.com/visio/2010/relationships/page".to_owned(),
            target: "page1.xml.rels".to_owned(),
            target_mode: TargetMode::Internal,
            preserved: Preserved::new(),
        });
        assert!(
            package
                .remove_part(&PartName::new("/visio/pages/page1.xml"))
                .expect("remove")
        );
        assert!(
            !package
                .relationships
                .iter()
                .any(|relationship| relationship.source.as_str() == "/visio/pages/page1.xml")
        );
    }

    #[test]
    fn removing_a_missing_part_says_so_and_the_root_never_goes() {
        let mut package = package();
        assert_eq!(package.remove_part(&PartName::new("/nope.xml")), Ok(false));
        assert!(matches!(
            package.remove_part(&PartName::new("/")),
            Err(Error::InvalidPackage(_))
        ));
    }

    #[test]
    fn remove_part_keeps_an_external_relationship_alone() {
        let mut package = package();
        package.relationships.push(Relationship {
            source: PartName::new("/visio/document.xml"),
            id: RelationshipId::new("rId9"),
            rel_type: "http://example.com/external".to_owned(),
            target: "page2.xml".to_owned(),
            target_mode: TargetMode::External,
            preserved: Preserved::new(),
        });
        assert!(
            package
                .remove_part(&PartName::new("/visio/pages/page2.xml"))
                .expect("remove")
        );
        assert!(
            package
                .relationships
                .iter()
                .any(|relationship| relationship.id.as_str() == "rId9")
        );
    }

    #[test]
    fn next_relationship_id_skips_the_declared_ones() {
        let package = package();
        assert_eq!(
            package
                .next_relationship_id(&PartName::new("/visio/pages/pages.xml"))
                .expect("id")
                .as_str(),
            "rId3"
        );
        assert_eq!(
            package
                .next_relationship_id(&PartName::new("/visio/pages/page1.xml"))
                .expect("id")
                .as_str(),
            "rId1"
        );
        // Gaps are filled: `rId1` and `rId3` taken leaves `rId2`.
        let mut sparse = Package::new();
        sparse.relationships.push(Relationship {
            source: PartName::new("/part.xml"),
            id: RelationshipId::new("rId1"),
            rel_type: "type".to_owned(),
            target: "a.xml".to_owned(),
            target_mode: TargetMode::Internal,
            preserved: Preserved::new(),
        });
        sparse.relationships.push(Relationship {
            source: PartName::new("/part.xml"),
            id: RelationshipId::new("rId3"),
            rel_type: "type".to_owned(),
            target: "b.xml".to_owned(),
            target_mode: TargetMode::Internal,
            preserved: Preserved::new(),
        });
        assert_eq!(
            sparse
                .next_relationship_id(&PartName::new("/part.xml"))
                .expect("id")
                .as_str(),
            "rId2"
        );
    }

    #[test]
    fn a_removed_part_writes_a_smaller_package() {
        let mut package = package();
        package
            .remove_part(&PartName::new("/visio/pages/page2.xml"))
            .expect("remove");
        let bytes = write_package(&package).expect("bytes");
        let reread = crate::read_package(&bytes, &crate::Limits::DEFAULT).expect("read");
        assert!(
            reread
                .part(&PartName::new("/visio/pages/page2.xml"))
                .is_none()
        );
    }
}
