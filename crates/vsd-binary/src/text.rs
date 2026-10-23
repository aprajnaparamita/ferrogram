// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Shape text: text, character, paragraph and field records into the
//! model (ADR 0070).
//!
//! This module reads the text records (the text string itself, the
//! character and paragraph lists and their index records, the field
//! list and text fields) and maps them onto
//! [`ferrogram_model::TextContent`]: literal text to
//! `TextPart::Text`, run boundaries to `CharRun`, `ParaRun` and
//! `TabsRun` markers, fields to `TextPart::Field` with their cached
//! display text.
//!
//! Text is stored with a code page in versions 6 and 11: the stream's
//! code page decodes byte strings, UTF-16 strings are read as they are
//! (the corpus pair `fdo86729-ms1252.vsd` / `fdo86729-utf8.vsd` covers
//! both spellings).
//!
//! Task P8-01 defines the entry point as an interface stub; task P8-05
//! implements it.

use ferrogram_model::Document;

use crate::chunks::Chunk;
use crate::error::Error;
use crate::warning::ImportWarning;

/// Maps every text, character, paragraph and field record in `chunks`
/// onto the shapes of `document`.
///
/// Character and paragraph records become the `Character` and
/// `Paragraph` rows of the owning shape's sheet; run markers reference
/// their row `IX`. Text the model cannot represent is recorded in
/// `warnings`, never fatal.
///
/// Task P8-01 leaves this an interface stub; task P8-05 implements it.
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
