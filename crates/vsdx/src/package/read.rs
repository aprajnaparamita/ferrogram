//! Reading a zip container into a [`Package`] (task P2-05).

use super::{Limits, Package};

/// Reads an OPC package from the bytes of a `.vsdx`, `.vssx` or `.vstx` file.
///
/// Every [`Limits`] cap is enforced while reading: entry count, per-part and
/// total uncompressed size, and the compression ratio (ADR 0017). Malformed
/// input returns an [`crate::Error`], never a panic.
pub fn read_package(bytes: &[u8], limits: &Limits) -> Result<Package, crate::Error> {
    let _ = (bytes, limits);
    Err(crate::Error::NotImplemented("read_package"))
}
