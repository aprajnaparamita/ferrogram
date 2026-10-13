//! Reading parts into the model (task P2-06).

use ferrogram_model::Document;

use crate::package::{Limits, Package};

/// Reads a whole document from the bytes of a file.
///
/// Convenience composition: [`crate::read_package`] followed by
/// [`to_model`]. The same [`Limits`] caps apply (ADR 0017).
pub fn read_document(bytes: &[u8], limits: &Limits) -> Result<Document, crate::Error> {
    let _ = (bytes, limits);
    Err(crate::Error::NotImplemented("read_document"))
}

/// Maps a package to a document.
///
/// Known parts (inventory status `supported`, `docs/vsdx-inventory.md`) are
/// mapped to model entities; everything else stays in the package untouched
/// (ADR 0012). Inheritance is not flattened: master and stylesheet resolution
/// happens later in the lookup (ADR 0013).
pub fn to_model(package: &Package) -> Result<Document, crate::Error> {
    let _ = package;
    Err(crate::Error::NotImplemented("to_model"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readers_are_stubs() {
        let p = Package::new();
        assert_eq!(
            read_document(&[], &Limits::DEFAULT),
            Err(crate::Error::NotImplemented("read_document"))
        );
        assert_eq!(to_model(&p), Err(crate::Error::NotImplemented("to_model")));
    }
}
