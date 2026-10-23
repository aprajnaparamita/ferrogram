// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Pages, masters, stencils and templates: chunk records into the
//! model (ADR 0070).
//!
//! This module reads the page records (pages, page sheets, page
//! properties, background pages, connects), the master records
//! (stencils, stencil pages, master shapes, names) and the document
//! properties, and maps them onto [`ferrogram_model::Page`],
//! [`ferrogram_model::Master`], [`ferrogram_model::Connect`],
//! [`ferrogram_model::Property`] and [`ferrogram_model::DocumentKind`].
//!
//! The document kind comes from the records (a file of stencil pages is
//! a stencil), not from the file name, so `.vsd`, `.vss` and `.vst`
//! all import through the same path (ADR 0016).
//!
//! Task P8-01 defines the entry point as an interface stub; task P8-07
//! implements it.

use ferrogram_model::Document;

use crate::chunks::Chunk;
use crate::error::Error;
use crate::warning::ImportWarning;

/// Maps every page, master and document record in `chunks` into
/// `document`.
///
/// Pages and masters are created in stream order, connects are attached
/// to their page, and the document kind and properties are set. Names
/// are resolved through the name lists; a reference that resolves to
/// nothing is recorded in `warnings`, never fatal.
///
/// Task P8-01 leaves this an interface stub; task P8-07 implements it.
#[allow(dead_code)] // no caller until P8-07 wires the pipeline
pub fn collect(
    chunks: &[Chunk<'_>],
    document: &mut Document,
    warnings: &mut Vec<ImportWarning>,
) -> Result<(), Error> {
    let _ = (chunks, document, warnings);
    Err(Error::NotImplemented)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_is_a_stub_returning_not_implemented() {
        let err = collect(&[], &mut Document::new(), &mut Vec::new()).unwrap_err();
        assert_eq!(err, Error::NotImplemented);
    }
}
