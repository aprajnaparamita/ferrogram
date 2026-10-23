// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Styles, colours, fills and lines: chunk records into the model
//! (ADR 0070).
//!
//! This module reads the stylesheet records (style sheets and their
//! line, fill and shadow, text block, character and paragraph cells),
//! the colour table and the font list, and maps them onto
//! [`larrygram_model::Stylesheet`], [`larrygram_model::Style`] and
//! [`larrygram_model::ColorTableEntry`]. A style keeps its Visio id,
//! name and its line, fill and text style references, so shapes and
//! other styles resolve the same way the `.vsdx` reader leaves them
//! (ADR 0013: inheritance is never flattened on load).
//!
//! Task P8-01 defines the entry point as an interface stub; task P8-06
//! implements it.

use larrygram_model::Document;

use crate::chunks::Chunk;
use crate::error::Error;
use crate::warning::ImportWarning;

/// Maps every stylesheet, colour and font record in `chunks` into
/// `document`.
///
/// Styles become rows of the document stylesheet, colours fill the
/// document colour table, and font records are dropped with a warning
/// where the model has no place for them. Values the model cannot
/// represent are recorded in `warnings`, never fatal.
///
/// Task P8-01 leaves this an interface stub; task P8-06 implements it.
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
