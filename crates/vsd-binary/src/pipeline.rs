// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! The import pipeline: bytes to [`Import`] (ADR 0070, ADR 0071).
//!
//! The pipeline runs the layers in order and never flattens
//! inheritance:
//!
//! 1. [`crate::container::read_streams`] opens the compound file.
//! 2. [`crate::chunks::version`] and [`crate::chunks::read_chunks`]
//!    flatten the `VisioDocument` stream into records.
//! 3. [`crate::style::collect`], [`crate::pages::collect`],
//!    [`crate::shape::collect`] and [`crate::text::collect`] map the
//!    records into one [`ferrogram_model::Document`].
//!
//! Every step records degradations in [`crate::ImportWarning`] values;
//! only a structural failure is an [`crate::Error`].
//!
//! Task P8-01 defines the entry point as an interface stub; task P8-07
//! implements it.

use crate::Import;
use crate::error::Error;
use crate::limits::Limits;

/// Reads a whole binary `.vsd`, `.vss` or `.vst` file into the model.
///
/// The bytes are the whole file. `limits` caps every input-derived size
/// (ADR 0071). The result carries the document and the warnings
/// collected on the way; a file outside the alpha scope (ADR 0069) is
/// an error, never a panic.
///
/// Task P8-01 leaves this an interface stub; task P8-07 implements it.
pub fn import(bytes: &[u8], limits: &Limits) -> Result<Import, Error> {
    let _ = (bytes, limits);
    Err(Error::NotImplemented)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_is_a_stub_returning_not_implemented() {
        let err = import(&[], &Limits::DEFAULT).unwrap_err();
        assert_eq!(err, Error::NotImplemented);
    }
}
