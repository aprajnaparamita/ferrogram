//! Writing the model back to parts (task P2-07).

use ferrogram_model::Document;

use crate::package::Package;

/// Maps a document back to package parts.
///
/// Supported parts are regenerated from the model; every other part comes
/// from `preserved` unchanged (ADR 0012). The result is semantically
/// equivalent to the input of [`crate::to_model`] (ADR 0012).
pub fn from_model(document: &Document) -> Result<Package, crate::Error> {
    let _ = document;
    Err(crate::Error::NotImplemented("from_model"))
}

/// Writes a document to the bytes of a file, preserving everything `preserve`
/// holds that the model does not map.
///
/// `preserve` is the package the document was read from (or an empty
/// [`Package`] for a new document). Unknown parts are copied from it; unknown
/// attributes and children are copied from the model's preserved fragments
/// (ADR 0012).
pub fn write_document(document: &Document, preserve: &Package) -> Result<Vec<u8>, crate::Error> {
    let _ = (document, preserve);
    Err(crate::Error::NotImplemented("write_document"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writers_are_stubs() {
        let d = Document::new();
        let p = Package::new();
        assert_eq!(
            from_model(&d),
            Err(crate::Error::NotImplemented("from_model"))
        );
        assert_eq!(
            write_document(&d, &p),
            Err(crate::Error::NotImplemented("write_document"))
        );
    }
}
