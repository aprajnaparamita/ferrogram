//! The scene builder: model to scene graph through the lookup (ADR 0020).
//!
//! `build_scene` walks one page of a document, resolves every cell through
//! the inheritance lookup with the cached-value policy (ADR 0021), composes
//! the shape hierarchy transforms (ADR 0020) and maps the typed geometry
//! rows to paths (ADR 0022). It never panics: bad values become warnings
//! and defaults.

use ferrogram_model::{Document, PageId};

use crate::error::Error;
use crate::scene::Scene;

/// Options for building a scene (ADR 0020).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BuildOptions {
    /// Whether to include the background page (drawn first).
    pub background: bool,
}

impl BuildOptions {
    /// The default options: include the background page.
    pub fn new() -> Self {
        Self { background: true }
    }
}

/// Builds the scene graph of one page (ADR 0020).
///
/// Returns [`Error::NotImplemented`] until the builder work lands (P3-05).
pub fn build_scene(doc: &Document, page: PageId, options: &BuildOptions) -> Result<Scene, Error> {
    let _ = (doc, page, options);
    Err(Error::NotImplemented)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_options_include_the_background() {
        assert!(BuildOptions::new().background);
        assert!(!BuildOptions::default().background);
    }

    #[test]
    fn build_scene_is_not_implemented_yet() {
        let doc = Document::new();
        assert_eq!(
            build_scene(&doc, PageId { index: 0 }, &BuildOptions::new()),
            Err(Error::NotImplemented)
        );
    }
}
