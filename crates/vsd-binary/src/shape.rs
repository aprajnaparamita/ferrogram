// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Shapes, transforms and geometry: chunk records into the model
//! (ADR 0070).
//!
//! This module reads the shape records (shape groups, shapes, guide
//! shapes), the transform records (pin, size, angle, flips, 1-D
//! endpoints) and the geometry records (move, line, arc, elliptical
//! arc, spline, NURBS, polyline, infinite line) and maps them onto
//! [`larrygram_model::Shape`] and its ShapeSheet: transforms to the
//! `Xform` cells, `Xform1D` to the 1-D cells, geometry rows to
//! `Geometry` sections. Lengths are inches and angles are radians, the
//! model's units (AGENTS.md section 6).
//!
//! Task P8-01 defines the entry point as an interface stub; task P8-04
//! implements it.

use larrygram_model::Document;

use crate::chunks::Chunk;
use crate::error::Error;
use crate::warning::ImportWarning;

/// Maps every shape, transform and geometry record in `chunks` into
/// `document`.
///
/// Shapes are added to the document arena in stream order, with the
/// parent and child links of the nesting level preserved and the
/// master, line, fill and text style references kept as Visio ids.
/// Values the model must interpret but cannot map are recorded in
/// `warnings`, never fatal.
///
/// Task P8-01 leaves this an interface stub; task P8-04 implements it.
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
