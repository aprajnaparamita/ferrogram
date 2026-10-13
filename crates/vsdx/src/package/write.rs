//! Writing a [`Package`] back to a zip container (task P2-05).

use super::Package;

/// Serializes a package to the bytes of a `.vsdx`, `.vssx` or `.vstx` file.
///
/// Parts are written in `Package::parts` order, relationships and
/// `[Content_Types].xml` are generated from the package's own lists, so a
/// read followed by a write keeps every part and relationship (ADR 0012).
pub fn write_package(package: &Package) -> Result<Vec<u8>, crate::Error> {
    let _ = package;
    Err(crate::Error::NotImplemented("write_package"))
}
