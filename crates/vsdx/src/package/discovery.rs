//! Part and relationship discovery (task P2-14, review finding 3).
//!
//! The rules "which part is the main document part" and "where does a
//! relationship point" were copy-pasted in [`crate::read`] and
//! [`crate::write`]: the two halves of one round-trip must agree on them, so
//! the copies live here once. No behavior change; the call sites keep their
//! names.

use super::{Package, PartName, TargetMode};
use crate::Error;

/// The content type of the main drawing document part.
pub(crate) const CT_DRAWING_MAIN: &str = "application/vnd.ms-visio.drawing.main+xml";
/// The content type of the main stencil document part.
pub(crate) const CT_STENCIL_MAIN: &str = "application/vnd.ms-visio.stencil.main+xml";
/// The content type of the main template document part.
pub(crate) const CT_TEMPLATE_MAIN: &str = "application/vnd.ms-visio.template.main+xml";
/// The part name every package must name through its root relationships.
pub(crate) const MAIN_PART: &str = "/visio/document.xml";

/// The main document part of the package.
///
/// The package root relationship whose type ends in `/document` wins (the
/// content type alone cannot find the part of a package with no
/// `[Content_Types].xml`); a part whose content type names a main document
/// part is the fallback. A root relationship to a missing part is skipped.
pub(crate) fn main_part(package: &Package) -> Result<PartName, Error> {
    let root = PartName::new("/");
    for relationship in package.relationships_from(&root) {
        if relationship.target_mode != TargetMode::Internal {
            continue;
        }
        if rel_type_local(&relationship.rel_type) != "document" {
            continue;
        }
        let name = resolve_target(root.as_str(), &relationship.target);
        if package.part(&name).is_some() {
            return Ok(name);
        }
    }
    for part in &package.parts {
        if is_main_content_type(part.content_type.as_str()) {
            return Ok(part.name.clone());
        }
    }
    Err(Error::MissingPart(MAIN_PART.to_owned()))
}

/// Whether a content type names a main document part (ADR 0016).
pub(crate) fn is_main_content_type(content_type: &str) -> bool {
    matches!(
        content_type,
        CT_DRAWING_MAIN | CT_STENCIL_MAIN | CT_TEMPLATE_MAIN
    )
}

/// The last segment of a relationship type, e.g. `document` for the Visio
/// document relationship.
pub(crate) fn rel_type_local(rel_type: &str) -> &str {
    match rel_type.rsplit('/').next() {
        Some(local) => local,
        None => rel_type,
    }
}

/// The absolute part name a relationship `target` points at, resolved
/// against the directory of the part holding the relationship.
///
/// A target that is already absolute is used as written; `.` and `..`
/// segments are resolved. The result is not checked against the package.
pub(crate) fn resolve_target(from: &str, target: &str) -> PartName {
    if target.starts_with('/') {
        return PartName::new(target.to_owned());
    }
    let directory = from.rsplit_once('/').map_or("", |(dir, _)| dir);
    let mut path: Vec<&str> = Vec::new();
    for segment in directory.split('/').chain(target.split('/')) {
        match segment {
            "" | "." => {}
            ".." => {
                path.pop();
            }
            other => path.push(other),
        }
    }
    PartName::new(format!("/{}", path.join("/")))
}
