//! The theme part and the document colour table (ADR 0019).
//!
//! A theme part ([MS-VSDX] `/visio/theme/theme{N}.xml`) is a DrawingML
//! theme: a colour scheme (`a:clrScheme`), a font scheme (`a:fontScheme`)
//! and an effects scheme (`a:fmtScheme`). Phase 3 maps the colours and
//! fonts as far as rendering needs; effects stay preserved. The
//! variation colour schemes (`vt:variationClrSchemeLst`) and the background
//! colour (`bkgnd`) resolve the `QuickStyle*Color` variant encodings (ADR
//! 0021 amendment); effects stay preserved. The document
//! colour table (`Colors`/`ColorEntry`) names colours by index (24 and
//! up; 0..23 are the fixed palette, `crate::values::palette`).

use crate::cell::Color;
use crate::raw::Preserved;

/// One entry of the document colour table (`ColorEntry`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorTableEntry {
    /// The colour index (24 and up in files we have seen).
    pub index: i32,
    /// The colour.
    pub color: Color,
}

/// A theme colour scheme (`a:clrScheme`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeColors {
    /// The scheme name as written.
    pub name: String,
    /// `dk1`: the dark 1 colour.
    pub dk1: Color,
    /// `lt1`: the light 1 colour.
    pub lt1: Color,
    /// `dk2`: the dark 2 colour.
    pub dk2: Color,
    /// `lt2`: the light 2 colour.
    pub lt2: Color,
    /// `accent1` .. `accent6`.
    pub accents: [Color; 6],
    /// `hlink`: the hyperlink colour.
    pub hlink: Color,
    /// `folHlink`: the followed-hyperlink colour.
    pub fol_hlink: Color,
}

/// A theme font scheme (`a:fontScheme`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeFonts {
    /// The scheme name as written.
    pub name: String,
    /// The major (heading) latin script font.
    pub major_latin: String,
    /// The minor (body) latin script font.
    pub minor_latin: String,
    /// The major east-asian font.
    pub major_ea: String,
    /// The minor east-asian font.
    pub minor_ea: String,
    /// The major complex-script font.
    pub major_cs: String,
    /// The minor complex-script font.
    pub minor_cs: String,
}

/// One theme variation colour scheme (`vt:variationClrScheme`, ADR 0021
/// amendment).
///
/// The seven `varColor1`..`varColor7` colours are what a `QuickStyle*Color`
/// value of 100..106 names ([MS-VSDX] `QuickStyleFillColor`: "the color
/// property ... of a `CT_VariationClrScheme` element specified by the
/// `VariationColorIndex` cell"). A theme part holds four of them
/// (`vt:variationClrSchemeLst`); the shape's `VariationColorIndex` cell
/// selects one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeVariation {
    /// `varColor1`..`varColor7`, in file order.
    pub colors: [Color; 7],
}

/// What one `a:fmtScheme` fill style does (ADR 0021 amendment): the kind
/// of the fill a themed `FillPattern` cell names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeFillKind {
    /// No fill (`noFill`).
    None,
    /// A solid fill (`solidFill`).
    Solid,
    /// A gradient fill (`gradFill`); the stops come from the shape's own
    /// `FillGradient` rows until the `a:fmtScheme` gradient stops are read
    /// (open issue of P3-15).
    Gradient,
}

/// The fill style indices of one theme variation style scheme
/// (`vt:variationStyleScheme`, ADR 0021 amendment): the `fillIdx`
/// attributes of its `vt:varStyle` children, in file order. Each is a
/// 1-based index into [`Theme::fill_kinds`]; a `QuickStyleFillMatrix`
/// value of 100..103 names the first..fourth entry ([MS-VSDX]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeVariationStyles {
    /// The `fillIdx` values, in `vt:varStyle` order.
    pub fill_indices: Vec<u32>,
}

/// One theme part (ADR 0019).
#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    /// The theme name (`a:theme/@name`).
    pub name: String,
    /// The colour scheme, when the part has one.
    pub colors: Option<ThemeColors>,
    /// The font scheme, when the part has one.
    pub fonts: Option<ThemeFonts>,
    /// The variation colour schemes (`vt:variationClrSchemeLst` inside
    /// `a:clrScheme`), in file order (ADR 0021 amendment).
    pub variations: Vec<ThemeVariation>,
    /// The theme's background colour (the `bkgnd` extension of
    /// `a:clrScheme`), which `QuickStyle*Color` 8 names (ADR 0021
    /// amendment).
    pub background: Option<Color>,
    /// The fill styles of `a:fmtScheme`'s `fillStyleLst`, in file order
    /// (ADR 0021 amendment): what a themed `FillPattern` resolves to.
    pub fill_kinds: Vec<ThemeFillKind>,
    /// The variation style schemes (`vt:variationStyleSchemeLst` inside
    /// `a:fmtScheme`), in file order (ADR 0021 amendment).
    pub variation_styles: Vec<ThemeVariationStyles>,
    /// The rest of the part kept verbatim (ADR 0012).
    pub preserved: Preserved,
}

impl Theme {
    /// A theme with no schemes and nothing preserved.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            colors: None,
            fonts: None,
            variations: Vec::new(),
            background: None,
            fill_kinds: Vec::new(),
            variation_styles: Vec::new(),
            preserved: Preserved::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_theme_has_no_schemes() {
        let t = Theme::new("Office 2016");
        assert_eq!(t.name, "Office 2016");
        assert_eq!(t.colors, None);
        assert_eq!(t.fonts, None);
        assert!(t.preserved.children.is_empty());
    }

    #[test]
    fn schemes_hold_their_data() {
        let c = ThemeColors {
            name: "Office 2016".to_owned(),
            dk1: Color::new(0, 0, 0),
            lt1: Color::new(255, 255, 255),
            dk2: Color::new(68, 84, 106),
            lt2: Color::new(231, 230, 230),
            accents: [Color::new(91, 155, 213); 6],
            hlink: Color::new(5, 99, 193),
            fol_hlink: Color::new(149, 79, 114),
        };
        assert_eq!(c.accents[0], Color::new(91, 155, 213));
        let f = ThemeFonts {
            name: "Office 2016".to_owned(),
            major_latin: "Calibri Light".to_owned(),
            minor_latin: "Calibri".to_owned(),
            major_ea: String::new(),
            minor_ea: String::new(),
            major_cs: String::new(),
            minor_cs: String::new(),
        };
        assert_eq!(f.minor_latin, "Calibri");
        let e = ColorTableEntry {
            index: 24,
            color: Color::new(127, 127, 127),
        };
        assert_eq!(e.index, 24);
    }
}
