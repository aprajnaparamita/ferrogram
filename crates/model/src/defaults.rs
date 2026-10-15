//! Documented cell defaults (ADR 0021 amendment).
//!
//! The ShapeSheet reference documents a default for some cells ("The
//! default is 0.1 inch", "The default formula for determining LocPinX is:
//! = Width \* 0.5"). A cell that is absent at every rung of the lookup and
//! has a documented default resolves to that default **without** a
//! warning; a cell with no documented default keeps the
//! [`WarningKind::Missing`](crate::values::WarningKind::Missing) warning
//! (ADR 0021 amendment).
//!
//! Every entry below is a transcription of the documented default of the
//! cell's ShapeSheet reference page, in canonical units (inches and
//! radians, ADR 0011); the source rows are in `docs/sources.md`. The table
//! covers the cells the scene builder reads (`crates/render`); a cell with
//! no entry has no documented default and warns as before.

/// The documented default of one ShapeSheet cell (ADR 0021 amendment).
///
/// A default that is a formula names another cell
/// ([`DocumentedDefault::HalfOf`] and [`DocumentedDefault::WholeOf`]); the
/// reader evaluates those one multiplication deep, with no formula engine
/// (Phase 6 replaces this).
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum DocumentedDefault {
    /// A number in canonical units (inches, radians or fraction).
    Number(f64),
    /// Half of another cell's value (`LocPinX = Width*0.5`).
    HalfOf(&'static str),
    /// Another cell's value (`TxtWidth = Width*1`).
    WholeOf(&'static str),
}

/// The documented default of `cell`, or `None` when the ShapeSheet
/// reference documents none (the caller then records a
/// [`WarningKind::Missing`](crate::values::WarningKind::Missing) warning).
///
/// The table is keyed by cell name; the names it holds are unambiguous in
/// the ShapeSheet table (`ui-spec/shapesheet.toml`), and the transcription
/// is of the ShapeSheet reference page of that cell.
pub fn documented_default(cell: &str) -> Option<DocumentedDefault> {
    // (cell name, documented default). Keep sorted by cell name.
    const TABLE: &[(&str, DocumentedDefault)] = &[
        (
            "BottomMargin",
            DocumentedDefault::Number(0.1), // "The default is 0.1 inch."
        ),
        (
            "ColorTrans",
            DocumentedDefault::Number(0.0), // "The default is 0% (completely opaque)."
        ),
        (
            "FillBkgndTrans",
            DocumentedDefault::Number(0.0), // "The default is 0% (completely opaque)."
        ),
        (
            "FillForegndTrans",
            DocumentedDefault::Number(0.0), // "The default is 0% (completely opaque)."
        ),
        (
            "LeftMargin",
            DocumentedDefault::Number(0.1), // "The default is 0.1 inch."
        ),
        (
            "LineColorTrans",
            DocumentedDefault::Number(0.0), // "The default is 0% (completely opaque)."
        ),
        (
            "LocPinX",
            DocumentedDefault::HalfOf("Width"), // "= Width * 0.5"
        ),
        (
            "LocPinY",
            DocumentedDefault::HalfOf("Height"), // "= Height * 0.5"
        ),
        (
            "RightMargin",
            DocumentedDefault::Number(0.1), // "The default is 0.1 inch."
        ),
        (
            "ShdwForegndTrans",
            DocumentedDefault::Number(0.0), // "The default is 0% (completely opaque)."
        ),
        (
            "TopMargin",
            DocumentedDefault::Number(4.0 / 72.0), // "The default is 4.0000 point."
        ),
        (
            "TxtAngle",
            DocumentedDefault::Number(0.0), // "The default is 0 degrees."
        ),
        (
            "TxtHeight",
            DocumentedDefault::WholeOf("Height"), // "= Height * 1"
        ),
        (
            // "= TxtWidth * 0.5", and `TxtWidth` defaults to "= Width * 1";
            // the table flattens the chain to one lookup.
            "TxtLocPinX",
            DocumentedDefault::HalfOf("Width"),
        ),
        (
            // "= TxtHeight * 0.5", and `TxtHeight` defaults to "= Height
            // * 1".
            "TxtLocPinY",
            DocumentedDefault::HalfOf("Height"),
        ),
        (
            "TxtPinX",
            DocumentedDefault::HalfOf("Width"), // "= Width * 0.5"
        ),
        (
            "TxtPinY",
            DocumentedDefault::HalfOf("Height"), // "= Height * 0.5"
        ),
        (
            "TxtWidth",
            DocumentedDefault::WholeOf("Width"), // "= Width * 1"
        ),
    ];
    TABLE
        .iter()
        .find(|(name, _)| *name == cell)
        .map(|(_, default)| *default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_holds_the_documented_literals() {
        assert_eq!(
            documented_default("LeftMargin"),
            Some(DocumentedDefault::Number(0.1))
        );
        assert_eq!(
            documented_default("TopMargin"),
            Some(DocumentedDefault::Number(4.0 / 72.0))
        );
        assert_eq!(
            documented_default("TxtAngle"),
            Some(DocumentedDefault::Number(0.0))
        );
        assert_eq!(
            documented_default("FillForegndTrans"),
            Some(DocumentedDefault::Number(0.0))
        );
    }

    #[test]
    fn the_table_holds_the_documented_formulas() {
        assert_eq!(
            documented_default("LocPinX"),
            Some(DocumentedDefault::HalfOf("Width"))
        );
        assert_eq!(
            documented_default("TxtLocPinY"),
            Some(DocumentedDefault::HalfOf("Height"))
        );
        assert_eq!(
            documented_default("TxtWidth"),
            Some(DocumentedDefault::WholeOf("Width"))
        );
    }

    #[test]
    fn a_cell_with_no_documented_default_has_no_entry() {
        // The ShapeSheet reference documents no default for these.
        assert_eq!(documented_default("FlipX"), None);
        assert_eq!(documented_default("Angle"), None);
        assert_eq!(documented_default("ShapeShdwShow"), None);
        assert_eq!(documented_default("NoSuchCell"), None);
    }
}
