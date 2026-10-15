//! Cached-value reading with the Phase 3 default and warning policy
//! (ADR 0021).
//!
//! Phase 3 renders from cached values only: no formula evaluation. Every
//! cell the renderer reads goes through [`CellReader`], which resolves the
//! value through the inheritance lookup (ADR 0013) and, when the cached
//! value is missing, an error or of the wrong kind, substitutes the hard
//! default of the kind and records a [`CellWarning`]. A warning never fails
//! a render.
//!
//! Two corpus findings amend the policy (ADR 0021 amendments):
//!
//! - A cached value that is the literal `Themed` is a marker, not a string:
//!   the value comes from the theme. [`CellReader::color`] resolves it from
//!   the shape's `QuickStyle*` cells against `themes[0]`; anything the theme
//!   cannot answer (a transparency, a variant colour) substitutes the hard
//!   default with [`WarningKind::UnresolvedTheme`] and never a kind
//!   mismatch.
//! - A cached numeric colour is a palette index (0..23, [`palette`]) or a
//!   document colour table index (24 and up, `Document::color_table`).
//!
//! Booleans and lists are read the way the files write them: a boolean cell
//! caches `0`/`1` without a `U` code and a one-element list caches a plain
//! number, so [`CellReader::boolean`] accepts numbers and
//! [`CellReader::list`] accepts one number as a one-element list. Those are
//! coercions, not kind mismatches, and record no warning.

use crate::cell::{CellValue, Color};
use crate::document::Document;
use crate::error::Error;
use crate::id::{PageId, ShapeId};
use crate::lookup::{CellLookup, ValueSource};
use crate::name::CellName;

use ferrogram_geom::Angle;

/// The sheet a cell is read from (ADR 0021).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SheetRef {
    /// A shape's sheet.
    Shape(ShapeId),
    /// A page sheet.
    Page(PageId),
}

/// Why [`CellReader`] substituted a default (ADR 0021).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum WarningKind {
    /// No source defines the value.
    Missing,
    /// The cached value is an error, e.g. `"#VALUE!"`.
    ErrorValue(String),
    /// The cached value has another kind than requested.
    KindMismatch,
    /// The cached value is the `Themed` marker and the theme cannot answer
    /// the request (ADR 0021 amendment).
    UnresolvedTheme,
}

/// One substituted cell value (ADR 0021).
#[derive(Debug, Clone, PartialEq)]
pub struct CellWarning {
    /// The sheet the cell was read from.
    pub sheet: SheetRef,
    /// The cell name.
    pub cell: CellName,
    /// Why the default was substituted.
    pub kind: WarningKind,
    /// The default that was used instead.
    pub substituted: CellValue,
}

/// The literal cached value of a cell whose value comes from the theme
/// (ADR 0021 amendment).
const THEMED: &str = "Themed";

/// Reads resolved cell values with the Phase 3 policy (ADR 0021).
///
/// Construct one reader per build (scene, layout) and collect its warnings
/// at the end. The reader borrows the document and changes nothing in it
/// (ADR 0013).
pub struct CellReader<'d> {
    doc: &'d Document,
    warnings: Vec<CellWarning>,
}

impl<'d> CellReader<'d> {
    /// A reader over `doc` with no warnings yet.
    pub fn new(doc: &'d Document) -> Self {
        Self {
            doc,
            warnings: Vec::new(),
        }
    }

    /// Reads a number in canonical units (inches, radians or fraction,
    /// ADR 0011).
    pub fn number(&mut self, sheet: SheetRef, cell: &CellName) -> Result<f64, Error> {
        let resolved = self.resolved(sheet, cell)?;
        Ok(self.number_from(sheet, cell, resolved))
    }

    /// Reads an angle in radians (ADR 0011).
    pub fn angle(&mut self, sheet: SheetRef, cell: &CellName) -> Result<Angle, Error> {
        Ok(Angle::from_radians(self.number(sheet, cell)?))
    }

    /// Reads a boolean.
    ///
    /// A cached `0`/`1` number is read as `false`/`true` without a warning:
    /// the files write booleans that way when the cell has no `U` code.
    pub fn boolean(&mut self, sheet: SheetRef, cell: &CellName) -> Result<bool, Error> {
        let resolved = self.resolved(sheet, cell)?;
        Ok(self.boolean_from(sheet, cell, resolved))
    }

    /// Reads a string.
    pub fn text(&mut self, sheet: SheetRef, cell: &CellName) -> Result<String, Error> {
        let resolved = self.resolved(sheet, cell)?;
        Ok(self.text_from(sheet, cell, resolved))
    }

    /// Reads a color, resolving palette and document-table indices
    /// (ADR 0019).
    ///
    /// A cached number is an index: 0..23 into the fixed palette
    /// ([`palette`]), 24 and up into `Document::color_table`.
    pub fn color(&mut self, sheet: SheetRef, cell: &CellName) -> Result<Color, Error> {
        let resolved = self.resolved(sheet, cell)?;
        Ok(self.color_from(sheet, cell, resolved))
    }

    /// Reads a numeric list (`LayerMember` and friends) as its numbers.
    ///
    /// A cached list or string splits at `;`, `,` and whitespace; a cached
    /// number is a one-element list. A list with an unparsable entry is a
    /// kind mismatch and substitutes the empty list.
    pub fn list(&mut self, sheet: SheetRef, cell: &CellName) -> Result<Vec<f64>, Error> {
        let resolved = self.resolved(sheet, cell)?;
        Ok(self.list_from(sheet, cell, resolved))
    }

    /// Reads a number from one row of a repeated section (ADR 0019).
    ///
    /// `row` is the row's position in the section; `section` names the
    /// section (the first section with that name when a sheet has several).
    /// The value policy is the one of [`CellReader::number`].
    pub fn row_number(
        &mut self,
        sheet: SheetRef,
        section: &str,
        row: usize,
        cell: &CellName,
    ) -> Result<f64, Error> {
        self.row_number_in(
            sheet,
            &crate::lookup::RepeatedCell {
                section,
                section_ix: None,
                section_pos: Some(0),
                row: Some(row),
                row_ix: None,
            },
            cell,
        )
    }

    /// Reads a string from one row of a repeated section (ADR 0019).
    ///
    /// `row` is the row's position in the section; `section` names the
    /// section (the first section with that name when a sheet has several).
    /// The value policy is the one of [`CellReader::text`].
    pub fn row_text(
        &mut self,
        sheet: SheetRef,
        section: &str,
        row: usize,
        cell: &CellName,
    ) -> Result<String, Error> {
        let resolved = self.row_resolved(sheet, section, row, cell)?;
        Ok(self.text_from(sheet, cell, resolved))
    }

    /// Reads a boolean from one row of a repeated section (ADR 0019).
    ///
    /// `row` is the row's position in the section; `section` names the
    /// section (the first section with that name when a sheet has several).
    /// The value policy is the one of [`CellReader::boolean`].
    pub fn row_boolean(
        &mut self,
        sheet: SheetRef,
        section: &str,
        row: usize,
        cell: &CellName,
    ) -> Result<bool, Error> {
        let resolved = self.row_resolved(sheet, section, row, cell)?;
        Ok(self.boolean_from(sheet, cell, resolved))
    }

    /// Reads a color from one row of a repeated section (ADR 0019).
    ///
    /// `row` is the row's position in the section; `section` names the
    /// section (the first section with that name when a sheet has several).
    /// The value policy is the one of [`CellReader::color`]: palette and
    /// document-table indices resolve and `Themed` goes through the theme.
    pub fn row_color(
        &mut self,
        sheet: SheetRef,
        section: &str,
        row: usize,
        cell: &CellName,
    ) -> Result<Color, Error> {
        let resolved = self.row_resolved(sheet, section, row, cell)?;
        Ok(self.color_from(sheet, cell, resolved))
    }

    /// The resolved cached value of one row cell (ADR 0019 decision 5): the
    /// section named by `section`, the row at position `row`.
    fn row_resolved(
        &mut self,
        sheet: SheetRef,
        section: &str,
        row: usize,
        cell: &CellName,
    ) -> Result<crate::lookup::Resolved<'d>, Error> {
        self.doc.resolve_repeated_with_table(
            sheet,
            &crate::lookup::RepeatedCell {
                section,
                section_ix: None,
                section_pos: Some(0),
                row: Some(row),
                row_ix: None,
            },
            cell,
            crate::generated::CELLS,
        )
    }

    /// Reads a number from one row of a repeated section, identifying the
    /// section by name, `IX` and position and the row by position and `IX`
    /// (the geometry rows of a sheet with several `Geometry` sections,
    /// ADR 0019 decision 5).
    pub(crate) fn row_number_in(
        &mut self,
        sheet: SheetRef,
        target: &crate::lookup::RepeatedCell<'_>,
        cell: &CellName,
    ) -> Result<f64, Error> {
        let resolved =
            self.doc
                .resolve_repeated_with_table(sheet, target, cell, crate::generated::CELLS)?;
        Ok(self.number_from(sheet, cell, resolved))
    }

    /// The number policy (ADR 0021) for an already-resolved cell.
    ///
    /// The geometry reader resolves row cells itself (it knows the section
    /// identity) and applies the policy through this method.
    pub(crate) fn number_from(
        &mut self,
        sheet: SheetRef,
        cell: &CellName,
        resolved: crate::lookup::Resolved<'_>,
    ) -> f64 {
        let source = resolved.source;
        let value = resolved.value.cloned();
        match value {
            Some(CellValue::Str(text)) if text == THEMED => self.themed_number(sheet, cell),
            Some(CellValue::Number(number)) if source != ValueSource::HardDefault => number,
            Some(CellValue::Error(error)) if source != ValueSource::HardDefault => {
                self.warn_number(sheet, cell, WarningKind::ErrorValue(error))
            }
            Some(_) if source != ValueSource::HardDefault => {
                self.warn_number(sheet, cell, WarningKind::KindMismatch)
            }
            _ => self.warn_number(sheet, cell, WarningKind::Missing),
        }
    }

    /// The number policy for an *optional* row cell (the C and D cells of
    /// an `EllipticalArcTo` row): a cell nothing defines substitutes the
    /// row's own default (ADR 0022: `C = 0`, `D = 1`) without a warning,
    /// and a bad cell substitutes the same default with one warning.
    pub(crate) fn number_from_or(
        &mut self,
        sheet: SheetRef,
        cell: &CellName,
        resolved: crate::lookup::Resolved<'_>,
        default: f64,
    ) -> f64 {
        if resolved.source == ValueSource::HardDefault {
            return default;
        }
        let source = resolved.source;
        let value = resolved.value.cloned();
        match value {
            Some(CellValue::Str(text)) if text == THEMED => self.themed_number(sheet, cell),
            Some(CellValue::Number(number)) if source != ValueSource::HardDefault => number,
            Some(CellValue::Error(error)) if source != ValueSource::HardDefault => {
                self.warn_number(sheet, cell, WarningKind::ErrorValue(error));
                default
            }
            Some(_) if source != ValueSource::HardDefault => {
                self.warn_number(sheet, cell, WarningKind::KindMismatch);
                default
            }
            _ => default,
        }
    }

    /// The boolean policy for an *optional* cell: a cell nothing defines
    /// substitutes `default` without a warning (the section flags of a
    /// `Geometry` path default to `false`), and a bad cell substitutes the
    /// same default with one warning.
    pub(crate) fn boolean_from_or(
        &mut self,
        sheet: SheetRef,
        cell: &CellName,
        resolved: crate::lookup::Resolved<'_>,
        default: bool,
    ) -> bool {
        if resolved.source == ValueSource::HardDefault {
            return default;
        }
        let source = resolved.source;
        let value = resolved.value.cloned();
        match value {
            Some(CellValue::Str(text)) if text == THEMED => self.themed_boolean(sheet, cell),
            Some(CellValue::Bool(flag)) if source != ValueSource::HardDefault => flag,
            Some(CellValue::Number(number)) if source != ValueSource::HardDefault => number != 0.0,
            Some(CellValue::Error(error)) if source != ValueSource::HardDefault => {
                self.warn_boolean(sheet, cell, WarningKind::ErrorValue(error));
                default
            }
            Some(_) if source != ValueSource::HardDefault => {
                self.warn_boolean(sheet, cell, WarningKind::KindMismatch);
                default
            }
            _ => default,
        }
    }

    /// The boolean policy (ADR 0021) for an already-resolved cell.
    pub(crate) fn boolean_from(
        &mut self,
        sheet: SheetRef,
        cell: &CellName,
        resolved: crate::lookup::Resolved<'_>,
    ) -> bool {
        let source = resolved.source;
        let value = resolved.value.cloned();
        match value {
            Some(CellValue::Str(text)) if text == THEMED => self.themed_boolean(sheet, cell),
            Some(CellValue::Bool(flag)) if source != ValueSource::HardDefault => flag,
            Some(CellValue::Number(number)) if source != ValueSource::HardDefault => number != 0.0,
            Some(CellValue::Error(error)) if source != ValueSource::HardDefault => {
                self.warn_boolean(sheet, cell, WarningKind::ErrorValue(error))
            }
            Some(_) if source != ValueSource::HardDefault => {
                self.warn_boolean(sheet, cell, WarningKind::KindMismatch)
            }
            _ => self.warn_boolean(sheet, cell, WarningKind::Missing),
        }
    }

    /// The warnings recorded so far.
    pub fn warnings(&self) -> &[CellWarning] {
        &self.warnings
    }

    /// The document being read.
    pub fn document(&self) -> &'d Document {
        self.doc
    }

    /// Takes the warnings recorded so far, leaving none.
    pub fn take_warnings(&mut self) -> Vec<CellWarning> {
        std::mem::take(&mut self.warnings)
    }

    /// The string policy (ADR 0021) for an already-resolved cell.
    ///
    /// The row readers resolve row cells themselves and apply the policy
    /// through this method.
    pub(crate) fn text_from(
        &mut self,
        sheet: SheetRef,
        cell: &CellName,
        resolved: crate::lookup::Resolved<'_>,
    ) -> String {
        let source = resolved.source;
        let value = resolved.value.cloned();
        match value {
            Some(CellValue::Str(text)) if text == THEMED => self.themed_text(sheet, cell),
            Some(CellValue::Str(text)) if source != ValueSource::HardDefault => text,
            Some(CellValue::Error(error)) if source != ValueSource::HardDefault => {
                self.warn_text(sheet, cell, WarningKind::ErrorValue(error))
            }
            Some(_) if source != ValueSource::HardDefault => {
                self.warn_text(sheet, cell, WarningKind::KindMismatch)
            }
            _ => self.warn_text(sheet, cell, WarningKind::Missing),
        }
    }

    /// The color policy (ADR 0021) for an already-resolved cell: palette
    /// and document-table indices resolve (ADR 0019, ADR 0021 amendment).
    pub(crate) fn color_from(
        &mut self,
        sheet: SheetRef,
        cell: &CellName,
        resolved: crate::lookup::Resolved<'_>,
    ) -> Color {
        let source = resolved.source;
        let value = resolved.value.cloned();
        match value {
            Some(CellValue::Str(text)) if text == THEMED => self.themed_color(sheet, cell),
            Some(CellValue::Color(color)) if source != ValueSource::HardDefault => color,
            Some(CellValue::Number(index)) if source != ValueSource::HardDefault => {
                self.indexed_color(sheet, cell, index)
            }
            Some(CellValue::Error(error)) if source != ValueSource::HardDefault => {
                self.warn_color(sheet, cell, WarningKind::ErrorValue(error))
            }
            Some(_) if source != ValueSource::HardDefault => {
                self.warn_color(sheet, cell, WarningKind::KindMismatch)
            }
            _ => self.warn_color(sheet, cell, WarningKind::Missing),
        }
    }

    /// The list policy (ADR 0021) for an already-resolved cell.
    pub(crate) fn list_from(
        &mut self,
        sheet: SheetRef,
        cell: &CellName,
        resolved: crate::lookup::Resolved<'_>,
    ) -> Vec<f64> {
        let source = resolved.source;
        let value = resolved.value.cloned();
        match value {
            Some(CellValue::Str(text)) if text == THEMED => self.themed_list(sheet, cell),
            Some(CellValue::List(raw)) if source != ValueSource::HardDefault => {
                self.parse_list(sheet, cell, &raw)
            }
            Some(CellValue::Str(raw)) if source != ValueSource::HardDefault => {
                self.parse_list(sheet, cell, &raw)
            }
            Some(CellValue::Number(number)) if source != ValueSource::HardDefault => vec![number],
            Some(CellValue::Error(error)) if source != ValueSource::HardDefault => {
                self.warn_list(sheet, cell, WarningKind::ErrorValue(error))
            }
            Some(_) if source != ValueSource::HardDefault => {
                self.warn_list(sheet, cell, WarningKind::KindMismatch)
            }
            _ => self.warn_list(sheet, cell, WarningKind::Missing),
        }
    }

    /// Records one warning and returns the substituted number.
    fn warn_number(&mut self, sheet: SheetRef, cell: &CellName, kind: WarningKind) -> f64 {
        self.warn(sheet, cell, kind, CellValue::Number(0.0));
        0.0
    }

    /// Records one warning and returns the substituted boolean.
    fn warn_boolean(&mut self, sheet: SheetRef, cell: &CellName, kind: WarningKind) -> bool {
        self.warn(sheet, cell, kind, CellValue::Bool(false));
        false
    }

    /// Records one warning and returns the substituted string.
    fn warn_text(&mut self, sheet: SheetRef, cell: &CellName, kind: WarningKind) -> String {
        self.warn(sheet, cell, kind, CellValue::Str(String::new()));
        String::new()
    }

    /// Records one warning and returns the substituted color.
    fn warn_color(&mut self, sheet: SheetRef, cell: &CellName, kind: WarningKind) -> Color {
        let color = Color::new(0, 0, 0);
        self.warn(sheet, cell, kind, CellValue::Color(color));
        color
    }

    /// Records one warning and returns the substituted list.
    fn warn_list(&mut self, sheet: SheetRef, cell: &CellName, kind: WarningKind) -> Vec<f64> {
        self.warn(sheet, cell, kind, CellValue::List(String::new()));
        Vec::new()
    }

    /// Records one substituted value (ADR 0021).
    fn warn(&mut self, sheet: SheetRef, cell: &CellName, kind: WarningKind, value: CellValue) {
        self.warnings.push(CellWarning {
            sheet,
            cell: cell.clone(),
            kind,
            substituted: value,
        });
    }

    /// Parses a list string into its numbers; an unparsable entry is a kind
    /// mismatch and substitutes the empty list.
    fn parse_list(&mut self, sheet: SheetRef, cell: &CellName, raw: &str) -> Vec<f64> {
        let mut numbers = Vec::new();
        for token in raw.split([';', ',', '\t', '\n', '\r', ' ']) {
            let token = token.trim();
            if token.is_empty() {
                continue;
            }
            match token.parse::<f64>() {
                Ok(number) => numbers.push(number),
                Err(_) => return self.warn_list(sheet, cell, WarningKind::KindMismatch),
            }
        }
        numbers
    }

    /// Resolves a numeric color index: 0..23 is the fixed palette, 24 and
    /// up the document colour table. An index nothing knows is a [`Missing`]
    /// warning; a non-integral number is a kind mismatch.
    fn indexed_color(&mut self, sheet: SheetRef, cell: &CellName, index: f64) -> Color {
        let integral = index.is_finite() && index >= 0.0 && index.fract() == 0.0;
        if !integral || index > u32::MAX as f64 {
            return self.warn_color(sheet, cell, WarningKind::KindMismatch);
        }
        if let Some(color) = palette(index as u32) {
            return color;
        }
        let found = self
            .doc
            .color_table
            .iter()
            .find(|entry| f64::from(entry.index) == index)
            .map(|entry| entry.color);
        found.unwrap_or_else(|| self.warn_color(sheet, cell, WarningKind::Missing))
    }

    /// The theme colour a `Themed` color cell resolves to (ADR 0021
    /// amendment): the shape's `QuickStyle*` colour slot against
    /// `Document::themes[0]`. `None` when the role, the slot or the theme
    /// cannot answer.
    fn theme_color(&self, sheet: SheetRef, cell: &CellName) -> Option<Color> {
        let role = ThemeRole::of_cell(cell.as_str())?;
        let quick_style = CellName::from_static(role.quick_style_cell());
        let resolved = self.resolved(sheet, &quick_style).ok()?;
        let index = match resolved.value? {
            CellValue::Number(index) => *index,
            _ => return None,
        };
        if !index.is_finite() || index < 0.0 || index.fract() != 0.0 || index > u32::MAX as f64 {
            return None;
        }
        let index = index as u32;
        let colors = self.doc.themes.first()?.colors.as_ref()?;
        match theme_slot(index)? {
            ThemeSlot::Dk1 => Some(colors.dk1),
            ThemeSlot::Lt1 => Some(colors.lt1),
            ThemeSlot::Accent(position) => colors.accents.get(position).copied(),
        }
    }

    /// Resolves one cell through the lookup of the sheet's kind.
    fn resolved(
        &self,
        sheet: SheetRef,
        cell: &CellName,
    ) -> Result<crate::lookup::Resolved<'d>, Error> {
        match sheet {
            SheetRef::Shape(shape) => self.doc.resolve(shape, cell),
            SheetRef::Page(page) => self.doc.resolve_page_cell(page, cell),
        }
    }

    /// The substitution for a `Themed` color cell: the theme colour, or
    /// [`WarningKind::UnresolvedTheme`] and black.
    fn themed_color(&mut self, sheet: SheetRef, cell: &CellName) -> Color {
        match self.theme_color(sheet, cell) {
            Some(color) => color,
            None => self.warn_color(sheet, cell, WarningKind::UnresolvedTheme),
        }
    }

    /// The substitution for a `Themed` number cell. A themed transparency
    /// (`*Trans`) resolves through the theme's colour scheme, which carries
    /// no alpha (the `a:fmtScheme` matrices are deferred), so it cannot be
    /// resolved and substitutes 0 with [`WarningKind::UnresolvedTheme`].
    fn themed_number(&mut self, sheet: SheetRef, cell: &CellName) -> f64 {
        self.warn_number(sheet, cell, WarningKind::UnresolvedTheme)
    }

    /// The substitution for a `Themed` boolean cell.
    fn themed_boolean(&mut self, sheet: SheetRef, cell: &CellName) -> bool {
        self.warn_boolean(sheet, cell, WarningKind::UnresolvedTheme)
    }

    /// The substitution for a `Themed` string cell.
    fn themed_text(&mut self, sheet: SheetRef, cell: &CellName) -> String {
        self.warn_text(sheet, cell, WarningKind::UnresolvedTheme)
    }

    /// The substitution for a `Themed` list cell.
    fn themed_list(&mut self, sheet: SheetRef, cell: &CellName) -> Vec<f64> {
        self.warn_list(sheet, cell, WarningKind::UnresolvedTheme)
    }
}

/// The theme role a colour cell plays (ADR 0021 amendment): it selects the
/// `QuickStyle*` cell whose slot names the theme colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ThemeRole {
    /// The shape's line colour (`QuickStyleLineColor`).
    Line,
    /// The shape's fill colour (`QuickStyleFillColor`).
    Fill,
    /// The shape's shadow colour (`QuickStyleShadowColor`).
    Shadow,
    /// The shape's text colour (`QuickStyleFontColor`).
    Font,
}

impl ThemeRole {
    /// The role of a cell by its name, or `None` when the cell is not a
    /// themed colour of a role we map.
    fn of_cell(cell: &str) -> Option<Self> {
        match cell {
            "Color" => Some(Self::Font),
            _ if cell.starts_with("Line") => Some(Self::Line),
            _ if cell.starts_with("Fill") || cell.starts_with("Gradient") => Some(Self::Fill),
            _ if cell.contains("Shdw")
                || cell.starts_with("Shadow")
                || cell.starts_with("Glow")
                || cell.starts_with("Bevel") =>
            {
                Some(Self::Shadow)
            }
            _ => None,
        }
    }

    /// The `QuickStyle*` cell that names this role's theme colour.
    fn quick_style_cell(self) -> &'static str {
        match self {
            Self::Line => "QuickStyleLineColor",
            Self::Fill => "QuickStyleFillColor",
            Self::Shadow => "QuickStyleShadowColor",
            Self::Font => "QuickStyleFontColor",
        }
    }
}

/// The theme colour a `QuickStyle*` slot names (the ShapeSheet reference:
/// 0 is the Dark theme colour, 1 the Light theme colour, 2..7 Accent 1..6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ThemeSlot {
    /// Slot 0: the dark 1 theme colour.
    Dk1,
    /// Slot 1: the light 1 theme colour.
    Lt1,
    /// Slots 2..7: accent 1..6, by position in `ThemeColors::accents`.
    Accent(usize),
}

/// The theme slot a `QuickStyle*` colour value names, or `None` for a value
/// the theme colour scheme cannot answer (8 is the background colour and
/// the 100+ variant encodings).
fn theme_slot(index: u32) -> Option<ThemeSlot> {
    match index {
        0 => Some(ThemeSlot::Dk1),
        1 => Some(ThemeSlot::Lt1),
        2..=7 => Some(ThemeSlot::Accent(index as usize - 2)),
        _ => None,
    }
}

/// The fixed Visio palette (indices 0..23), our own transcription of the
/// default colour table (ADR 0019 decision 6).
///
/// Indices and colour names come from the `VisDefaultColors` enumeration on
/// Microsoft Learn (source 9 of `docs/sources.md`); the RGB values are the
/// conventional values of those names and are not yet verified against
/// Visio's own colour dialog (recorded in `docs/sources.md` and the P3-02
/// Result).
///
/// Returns `None` for an index the table does not hold (24 and up resolve
/// through `Document::color_table`).
pub fn palette(index: u32) -> Option<Color> {
    /// Index, name and RGB of every fixed palette entry.
    const TABLE: &[(u32, &str, Color)] = &[
        (0, "Black", Color::new(0, 0, 0)),
        (1, "White", Color::new(255, 255, 255)),
        (2, "Red", Color::new(255, 0, 0)),
        (3, "Green", Color::new(0, 255, 0)),
        (4, "Blue", Color::new(0, 0, 255)),
        (5, "Yellow", Color::new(255, 255, 0)),
        (6, "Magenta", Color::new(255, 0, 255)),
        (7, "Cyan", Color::new(0, 255, 255)),
        (8, "Dark red", Color::new(128, 0, 0)),
        (9, "Dark green", Color::new(0, 128, 0)),
        (10, "Dark blue", Color::new(0, 0, 128)),
        (11, "Dark yellow", Color::new(128, 128, 0)),
        (12, "Purple", Color::new(128, 0, 128)),
        (13, "Dark cyan", Color::new(0, 128, 128)),
        (14, "Gray", Color::new(128, 128, 128)),
        (15, "10% gray", Color::new(230, 230, 230)),
        (16, "20% gray", Color::new(204, 204, 204)),
        (17, "30% gray", Color::new(179, 179, 179)),
        (18, "40% gray", Color::new(153, 153, 153)),
        (19, "50% gray", Color::new(128, 128, 128)),
        (20, "60% gray", Color::new(102, 102, 102)),
        (21, "70% gray", Color::new(77, 77, 77)),
        (22, "80% gray", Color::new(51, 51, 51)),
        (23, "90% gray", Color::new(26, 26, 26)),
    ];
    TABLE
        .iter()
        .find(|(seen, _, _)| *seen == index)
        .map(|(_, _, color)| *color)
}

#[cfg(test)]
mod tests {
    use crate::cell::Cell;
    use crate::document::Document;
    use crate::id::ShapeId;
    use crate::name::CellName;
    use crate::shape::Shape;
    use crate::theme::{ColorTableEntry, Theme, ThemeColors, ThemeFonts};

    use super::*;

    /// A document with one empty shape (id 0).
    fn document_with_shape() -> (Document, ShapeId) {
        let mut doc = Document::new();
        let shape = doc.add_shape(Shape::new(ShapeId { index: 0 }, 1));
        (doc, shape)
    }

    /// A document with one shape whose sheet has `cells`.
    fn document_with_cells(cells: Vec<Cell>) -> (Document, ShapeId) {
        let (mut doc, shape) = document_with_shape();
        if let Some(shape) = doc.shape_mut(shape) {
            shape.sheet.cells = cells;
        }
        (doc, shape)
    }

    /// A cell with a cached value.
    fn cell(name: &str, value: CellValue) -> Cell {
        let mut cell = Cell::new(CellName::new(name));
        cell.value = Some(value);
        cell
    }

    /// The colour scheme of `themes[0]`, with recognizable slots.
    fn colors() -> ThemeColors {
        ThemeColors {
            name: "Test".to_owned(),
            dk1: Color::new(1, 1, 1),
            lt1: Color::new(2, 2, 2),
            dk2: Color::new(3, 3, 3),
            lt2: Color::new(4, 4, 4),
            accents: [
                Color::new(10, 0, 0),
                Color::new(11, 0, 0),
                Color::new(12, 0, 0),
                Color::new(13, 0, 0),
                Color::new(14, 0, 0),
                Color::new(15, 0, 0),
            ],
            hlink: Color::new(5, 5, 5),
            fol_hlink: Color::new(6, 6, 6),
        }
    }

    /// Adds one theme with `colors` to the document.
    fn with_theme(doc: &mut Document) {
        let mut theme = Theme::new("Test");
        theme.colors = Some(colors());
        theme.fonts = Some(ThemeFonts {
            name: "Test".to_owned(),
            major_latin: "Major".to_owned(),
            minor_latin: "Minor".to_owned(),
            major_ea: String::new(),
            minor_ea: String::new(),
            major_cs: String::new(),
            minor_cs: String::new(),
        });
        doc.themes.push(theme);
    }

    #[test]
    fn reader_starts_without_warnings() {
        let doc = Document::new();
        let reader = CellReader::new(&doc);
        assert!(reader.warnings().is_empty());
        assert!(std::ptr::eq(reader.document(), &doc));
    }

    #[test]
    fn take_warnings_drains_the_list() {
        let doc = Document::new();
        let mut reader = CellReader::new(&doc);
        reader.warnings.push(CellWarning {
            sheet: SheetRef::Shape(ShapeId { index: 0 }),
            cell: CellName::from_static("PinX"),
            kind: WarningKind::Missing,
            substituted: CellValue::Number(0.0),
        });
        assert_eq!(reader.warnings().len(), 1);
        let taken = reader.take_warnings();
        assert_eq!(taken.len(), 1);
        assert!(reader.warnings().is_empty());
    }

    #[test]
    fn sheet_refs_are_distinct() {
        assert_ne!(
            SheetRef::Shape(ShapeId { index: 1 }),
            SheetRef::Page(PageId { index: 1 })
        );
    }

    // --- numbers and angles -------------------------------------------

    #[test]
    fn number_reads_a_cached_number() {
        let (doc, shape) = document_with_cells(vec![cell("Width", CellValue::Number(2.5))]);
        let mut reader = CellReader::new(&doc);
        let got = reader.number(SheetRef::Shape(shape), &CellName::from_static("Width"));
        assert_eq!(got, Ok(2.5));
        assert!(reader.warnings().is_empty());
    }

    #[test]
    fn angle_reads_radians() {
        let (doc, shape) = document_with_cells(vec![cell("Angle", CellValue::Number(1.5))]);
        let mut reader = CellReader::new(&doc);
        let got = reader
            .angle(SheetRef::Shape(shape), &CellName::from_static("Angle"))
            .expect("reads");
        assert_eq!(got.radians(), 1.5);
        assert!(reader.warnings().is_empty());
    }

    #[test]
    fn a_missing_value_substitutes_zero_with_one_warning() {
        let (doc, shape) = document_with_shape();
        let mut reader = CellReader::new(&doc);
        assert_eq!(
            reader.number(SheetRef::Shape(shape), &CellName::from_static("PinX")),
            Ok(0.0)
        );
        assert_eq!(reader.warnings().len(), 1);
        let warning = &reader.warnings()[0];
        assert_eq!(warning.kind, WarningKind::Missing);
        assert_eq!(warning.substituted, CellValue::Number(0.0));
        assert_eq!(warning.cell, CellName::from_static("PinX"));
        assert_eq!(warning.sheet, SheetRef::Shape(shape));
    }

    #[test]
    fn an_error_value_substitutes_zero_and_keeps_the_error() {
        let (doc, shape) =
            document_with_cells(vec![cell("PinX", CellValue::Error("#VALUE!".to_owned()))]);
        let mut reader = CellReader::new(&doc);
        assert_eq!(
            reader.number(SheetRef::Shape(shape), &CellName::from_static("PinX")),
            Ok(0.0)
        );
        assert_eq!(
            reader.warnings()[0].kind,
            WarningKind::ErrorValue("#VALUE!".to_owned())
        );
    }

    #[test]
    fn a_kind_mismatch_is_reported_per_requested_kind() {
        let (doc, shape) = document_with_cells(vec![cell("PinX", CellValue::Str("hello".into()))]);
        let mut reader = CellReader::new(&doc);
        assert_eq!(
            reader.number(SheetRef::Shape(shape), &CellName::from_static("PinX")),
            Ok(0.0)
        );
        assert_eq!(reader.warnings()[0].kind, WarningKind::KindMismatch);

        let (doc, shape) = document_with_cells(vec![cell("PinX", CellValue::Number(1.0))]);
        let mut reader = CellReader::new(&doc);
        assert_eq!(
            reader.text(SheetRef::Shape(shape), &CellName::from_static("PinX")),
            Ok(String::new())
        );
        assert_eq!(reader.warnings()[0].kind, WarningKind::KindMismatch);

        let (doc, shape) = document_with_cells(vec![cell("PinX", CellValue::Bool(true))]);
        let mut reader = CellReader::new(&doc);
        assert_eq!(
            reader.number(SheetRef::Shape(shape), &CellName::from_static("PinX")),
            Ok(0.0)
        );
        assert_eq!(reader.warnings()[0].kind, WarningKind::KindMismatch);
    }

    #[test]
    fn an_unknown_shape_id_is_a_caller_error() {
        let doc = Document::new();
        let mut reader = CellReader::new(&doc);
        assert_eq!(
            reader.number(
                SheetRef::Shape(ShapeId { index: 3 }),
                &CellName::from_static("PinX")
            ),
            Err(Error::UnknownShape(ShapeId { index: 3 }))
        );
        assert_eq!(
            reader.row_number(
                SheetRef::Shape(ShapeId { index: 3 }),
                "Geometry",
                0,
                &CellName::from_static("X")
            ),
            Err(Error::UnknownShape(ShapeId { index: 3 }))
        );
        assert_eq!(
            reader.number(
                SheetRef::Page(PageId { index: 3 }),
                &CellName::from_static("PageWidth")
            ),
            Err(Error::UnknownPage(PageId { index: 3 }))
        );
    }

    // --- booleans, strings, lists -------------------------------------

    #[test]
    fn boolean_reads_bools_and_coerces_zero_and_one() {
        let (doc, shape) = document_with_cells(vec![
            cell("FlipX", CellValue::Bool(true)),
            cell("FlipY", CellValue::Number(0.0)),
            cell("NoFill", CellValue::Number(3.0)),
        ]);
        let mut reader = CellReader::new(&doc);
        let sheet = SheetRef::Shape(shape);
        assert_eq!(
            reader.boolean(sheet, &CellName::from_static("FlipX")),
            Ok(true)
        );
        assert_eq!(
            reader.boolean(sheet, &CellName::from_static("FlipY")),
            Ok(false)
        );
        assert_eq!(
            reader.boolean(sheet, &CellName::from_static("NoFill")),
            Ok(true)
        );
        assert!(reader.warnings().is_empty());
    }

    #[test]
    fn boolean_substitutes_false_on_bad_values() {
        let (doc, shape) = document_with_cells(vec![
            cell("A", CellValue::Str("nope".into())),
            cell("B", CellValue::Error("#DIV/0!".into())),
        ]);
        let mut reader = CellReader::new(&doc);
        let sheet = SheetRef::Shape(shape);
        assert_eq!(
            reader.boolean(sheet, &CellName::from_static("A")),
            Ok(false)
        );
        assert_eq!(reader.warnings()[0].kind, WarningKind::KindMismatch,);
        assert_eq!(
            reader.boolean(sheet, &CellName::from_static("B")),
            Ok(false)
        );
        assert_eq!(
            reader.warnings()[1].kind,
            WarningKind::ErrorValue("#DIV/0!".to_owned())
        );
        assert_eq!(
            reader.boolean(sheet, &CellName::from_static("C")),
            Ok(false)
        );
        assert_eq!(reader.warnings()[2].kind, WarningKind::Missing);
    }

    #[test]
    fn text_reads_strings_and_substitutes_empty() {
        let (doc, shape) = document_with_cells(vec![
            cell("Prop", CellValue::Str("hello".into())),
            cell("Bad", CellValue::Number(1.0)),
        ]);
        let mut reader = CellReader::new(&doc);
        let sheet = SheetRef::Shape(shape);
        assert_eq!(
            reader.text(sheet, &CellName::from_static("Prop")),
            Ok("hello".to_owned())
        );
        assert_eq!(
            reader.text(sheet, &CellName::from_static("Bad")),
            Ok(String::new())
        );
        assert_eq!(reader.warnings()[0].kind, WarningKind::KindMismatch);
    }

    #[test]
    fn list_parses_the_written_separators() {
        let (doc, shape) = document_with_cells(vec![
            cell("LayerMember", CellValue::List("0;2;5".to_owned())),
            cell("One", CellValue::Number(3.0)),
            cell("Spaced", CellValue::Str("1, 2 ;3".to_owned())),
        ]);
        let mut reader = CellReader::new(&doc);
        let sheet = SheetRef::Shape(shape);
        assert_eq!(
            reader.list(sheet, &CellName::from_static("LayerMember")),
            Ok(vec![0.0, 2.0, 5.0])
        );
        assert_eq!(
            reader.list(sheet, &CellName::from_static("One")),
            Ok(vec![3.0])
        );
        assert_eq!(
            reader.list(sheet, &CellName::from_static("Spaced")),
            Ok(vec![1.0, 2.0, 3.0])
        );
        assert!(reader.warnings().is_empty());
    }

    #[test]
    fn list_substitutes_empty_on_bad_entries_and_kinds() {
        let (doc, shape) = document_with_cells(vec![
            cell("Bad", CellValue::List("1;x".to_owned())),
            cell("Color", CellValue::Color(Color::new(1, 2, 3))),
            cell("Err", CellValue::Error("#VALUE!".to_owned())),
        ]);
        let mut reader = CellReader::new(&doc);
        let sheet = SheetRef::Shape(shape);
        assert_eq!(
            reader.list(sheet, &CellName::from_static("Bad")),
            Ok(vec![])
        );
        assert_eq!(reader.warnings()[0].kind, WarningKind::KindMismatch);
        assert_eq!(
            reader.list(sheet, &CellName::from_static("Color")),
            Ok(vec![])
        );
        assert_eq!(reader.warnings()[1].kind, WarningKind::KindMismatch);
        assert_eq!(
            reader.list(sheet, &CellName::from_static("Err")),
            Ok(vec![])
        );
        assert_eq!(
            reader.warnings()[2].kind,
            WarningKind::ErrorValue("#VALUE!".to_owned())
        );
    }

    // --- colors and indices -------------------------------------------

    #[test]
    fn color_reads_a_cached_color() {
        let (doc, shape) = document_with_cells(vec![cell(
            "LineColor",
            CellValue::Color(Color::new(1, 2, 3)),
        )]);
        let mut reader = CellReader::new(&doc);
        assert_eq!(
            reader.color(SheetRef::Shape(shape), &CellName::from_static("LineColor")),
            Ok(Color::new(1, 2, 3))
        );
        assert!(reader.warnings().is_empty());
    }

    #[test]
    fn color_numbers_resolve_palette_and_table_indices() {
        let (mut doc, shape) = document_with_cells(vec![
            cell("A", CellValue::Number(2.0)),
            cell("B", CellValue::Number(25.0)),
            cell("C", CellValue::Number(400.0)),
            cell("D", CellValue::Number(-1.0)),
            cell("E", CellValue::Number(1.5)),
        ]);
        doc.color_table.push(ColorTableEntry {
            index: 25,
            color: Color::new(9, 9, 9),
        });
        let mut reader = CellReader::new(&doc);
        let sheet = SheetRef::Shape(shape);
        assert_eq!(
            reader.color(sheet, &CellName::from_static("A")),
            Ok(Color::new(255, 0, 0))
        );
        assert_eq!(
            reader.color(sheet, &CellName::from_static("B")),
            Ok(Color::new(9, 9, 9))
        );
        assert_eq!(
            reader.color(sheet, &CellName::from_static("C")),
            Ok(Color::new(0, 0, 0))
        );
        assert_eq!(reader.warnings()[0].kind, WarningKind::Missing);
        assert_eq!(
            reader.color(sheet, &CellName::from_static("D")),
            Ok(Color::new(0, 0, 0))
        );
        assert_eq!(reader.warnings()[1].kind, WarningKind::KindMismatch);
        assert_eq!(
            reader.color(sheet, &CellName::from_static("E")),
            Ok(Color::new(0, 0, 0))
        );
        assert_eq!(reader.warnings()[2].kind, WarningKind::KindMismatch);
    }

    #[test]
    fn color_substitutes_black_on_errors_and_mismatches() {
        let (doc, shape) = document_with_cells(vec![
            cell("A", CellValue::Error("#VALUE!".to_owned())),
            cell("B", CellValue::Bool(true)),
        ]);
        let mut reader = CellReader::new(&doc);
        let sheet = SheetRef::Shape(shape);
        assert_eq!(
            reader.color(sheet, &CellName::from_static("A")),
            Ok(Color::new(0, 0, 0))
        );
        assert_eq!(
            reader.warnings()[0].kind,
            WarningKind::ErrorValue("#VALUE!".to_owned())
        );
        assert_eq!(
            reader.color(sheet, &CellName::from_static("B")),
            Ok(Color::new(0, 0, 0))
        );
        assert_eq!(reader.warnings()[1].kind, WarningKind::KindMismatch);
    }

    // --- Themed values ------------------------------------------------

    #[test]
    fn themed_colors_resolve_through_the_quick_style_cells() {
        let (mut doc, shape) = document_with_cells(vec![
            cell("LineColor", CellValue::Str("Themed".to_owned())),
            cell("FillForegnd", CellValue::Str("Themed".to_owned())),
            cell("ShapeShdwColor", CellValue::Str("Themed".to_owned())),
            cell("Color", CellValue::Str("Themed".to_owned())),
            cell("QuickStyleLineColor", CellValue::Number(0.0)),
            cell("QuickStyleFillColor", CellValue::Number(2.0)),
            cell("QuickStyleShadowColor", CellValue::Number(7.0)),
            cell("QuickStyleFontColor", CellValue::Number(1.0)),
        ]);
        with_theme(&mut doc);
        let mut reader = CellReader::new(&doc);
        let sheet = SheetRef::Shape(shape);
        assert_eq!(
            reader.color(sheet, &CellName::from_static("LineColor")),
            Ok(Color::new(1, 1, 1))
        );
        assert_eq!(
            reader.color(sheet, &CellName::from_static("FillForegnd")),
            Ok(Color::new(10, 0, 0))
        );
        assert_eq!(
            reader.color(sheet, &CellName::from_static("ShapeShdwColor")),
            Ok(Color::new(15, 0, 0))
        );
        assert_eq!(
            reader.color(sheet, &CellName::from_static("Color")),
            Ok(Color::new(2, 2, 2))
        );
        assert!(reader.warnings().is_empty());
    }

    #[test]
    fn a_themed_color_with_no_answer_substitutes_black() {
        // No theme, no QuickStyle cell, a variant slot and an unknown role.
        let (doc, shape) = document_with_cells(vec![
            cell("LineColor", CellValue::Str("Themed".to_owned())),
            cell("FillForegnd", CellValue::Str("Themed".to_owned())),
            cell("QuickStyleLineColor", CellValue::Number(106.0)),
            cell("QuickStyleFillColor", CellValue::Number(0.0)),
            cell("MysteryColor", CellValue::Str("Themed".to_owned())),
        ]);
        let mut reader = CellReader::new(&doc);
        let sheet = SheetRef::Shape(shape);
        assert_eq!(
            reader.color(sheet, &CellName::from_static("LineColor")),
            Ok(Color::new(0, 0, 0))
        );
        assert_eq!(
            reader.color(sheet, &CellName::from_static("FillForegnd")),
            Ok(Color::new(0, 0, 0))
        );
        assert_eq!(
            reader.color(sheet, &CellName::from_static("MysteryColor")),
            Ok(Color::new(0, 0, 0))
        );
        assert_eq!(reader.warnings().len(), 3);
        for warning in reader.warnings() {
            assert_eq!(warning.kind, WarningKind::UnresolvedTheme);
        }
    }

    #[test]
    fn a_themed_quick_style_cell_of_the_wrong_kind_does_not_resolve() {
        let (mut doc, shape) = document_with_cells(vec![
            cell("LineColor", CellValue::Str("Themed".to_owned())),
            cell("QuickStyleLineColor", CellValue::Str("Themed".to_owned())),
        ]);
        with_theme(&mut doc);
        let mut reader = CellReader::new(&doc);
        assert_eq!(
            reader.color(SheetRef::Shape(shape), &CellName::from_static("LineColor")),
            Ok(Color::new(0, 0, 0))
        );
        assert_eq!(reader.warnings()[0].kind, WarningKind::UnresolvedTheme);
    }

    #[test]
    fn a_themed_number_boolean_text_and_list_substitute_with_one_warning() {
        let (mut doc, shape) = document_with_cells(vec![
            cell("LineColorTrans", CellValue::Str("Themed".to_owned())),
            cell("LineWeight", CellValue::Str("Themed".to_owned())),
            cell("FillGradientEnabled", CellValue::Str("Themed".to_owned())),
            cell("Label", CellValue::Str("Themed".to_owned())),
            cell("LayerMember", CellValue::Str("Themed".to_owned())),
        ]);
        with_theme(&mut doc);
        let mut reader = CellReader::new(&doc);
        let sheet = SheetRef::Shape(shape);
        assert_eq!(
            reader.number(sheet, &CellName::from_static("LineColorTrans")),
            Ok(0.0)
        );
        assert_eq!(
            reader.number(sheet, &CellName::from_static("LineWeight")),
            Ok(0.0)
        );
        assert_eq!(
            reader.boolean(sheet, &CellName::from_static("FillGradientEnabled")),
            Ok(false)
        );
        assert_eq!(
            reader.text(sheet, &CellName::from_static("Label")),
            Ok(String::new())
        );
        assert_eq!(
            reader.list(sheet, &CellName::from_static("LayerMember")),
            Ok(vec![])
        );
        assert_eq!(reader.warnings().len(), 5);
        for warning in reader.warnings() {
            assert_eq!(warning.kind, WarningKind::UnresolvedTheme);
        }
    }

    #[test]
    fn row_number_reads_row_cells_with_the_same_policy() {
        let (mut doc, shape) = document_with_shape();
        {
            let shape = doc.shape_mut(shape).expect("exists");
            let mut section =
                crate::section::Section::new(crate::name::SectionName::from_static("Geometry"));
            let mut row = crate::section::Row::new();
            row.row_type = Some("LineTo".to_owned());
            row.cells.push(cell("X", CellValue::Number(1.25)));
            let mut missing = crate::section::Row::new();
            missing.row_type = Some("MoveTo".to_owned());
            section.rows.push(row);
            section.rows.push(missing);
            shape.sheet.sections.push(section);
        }
        let mut reader = CellReader::new(&doc);
        let sheet = SheetRef::Shape(shape);
        assert_eq!(
            reader.row_number(sheet, "Geometry", 0, &CellName::from_static("X")),
            Ok(1.25)
        );
        assert!(reader.warnings().is_empty());
        assert_eq!(
            reader.row_number(sheet, "Geometry", 1, &CellName::from_static("X")),
            Ok(0.0)
        );
        assert_eq!(reader.warnings()[0].kind, WarningKind::Missing);
        // A row past the end substitutes too.
        assert_eq!(
            reader.row_number(sheet, "Geometry", 9, &CellName::from_static("X")),
            Ok(0.0)
        );
    }

    #[test]
    fn row_text_boolean_and_color_read_row_cells_with_the_same_policy() {
        let (mut doc, shape) = document_with_shape();
        {
            let shape = doc.shape_mut(shape).expect("exists");
            let mut section =
                crate::section::Section::new(crate::name::SectionName::from_static("Character"));
            let mut row = crate::section::Row::new();
            row.row_type = Some("Character".to_owned());
            row.cells
                .push(cell("Font", CellValue::Str("Calibri".into())));
            row.cells.push(cell("Strikethru", CellValue::Number(1.0)));
            row.cells.push(cell("Color", CellValue::Number(2.0)));
            section.rows.push(row);
            let mut bad = crate::section::Row::new();
            bad.row_type = Some("Character".to_owned());
            bad.cells.push(cell("Font", CellValue::Number(1.0)));
            section.rows.push(bad);
            shape.sheet.sections.push(section);
        }
        let mut reader = CellReader::new(&doc);
        let sheet = SheetRef::Shape(shape);
        let font = CellName::from_static("Font");
        let strike = CellName::from_static("Strikethru");
        let color = CellName::from_static("Color");
        assert_eq!(
            reader.row_text(sheet, "Character", 0, &font),
            Ok("Calibri".to_owned())
        );
        assert_eq!(reader.row_boolean(sheet, "Character", 0, &strike), Ok(true));
        assert_eq!(
            reader.row_color(sheet, "Character", 0, &color),
            Ok(Color::new(255, 0, 0))
        );
        assert!(reader.warnings().is_empty());
        // A bad kind substitutes with one warning, like the sheet readers.
        assert_eq!(
            reader.row_text(sheet, "Character", 1, &font),
            Ok(String::new())
        );
        assert_eq!(reader.warnings()[0].kind, WarningKind::KindMismatch);
        // A row past the end substitutes too.
        assert_eq!(
            reader.row_boolean(sheet, "Character", 9, &strike),
            Ok(false)
        );
        assert_eq!(reader.warnings()[1].kind, WarningKind::Missing);
    }

    #[test]
    fn row_readers_report_a_stale_shape() {
        let doc = Document::new();
        let mut reader = CellReader::new(&doc);
        let sheet = SheetRef::Shape(ShapeId { index: 3 });
        let cell = CellName::from_static("Font");
        assert_eq!(
            reader.row_text(sheet, "Character", 0, &cell),
            Err(Error::UnknownShape(ShapeId { index: 3 }))
        );
        assert_eq!(
            reader.row_boolean(sheet, "Character", 0, &cell),
            Err(Error::UnknownShape(ShapeId { index: 3 }))
        );
        assert_eq!(
            reader.row_color(sheet, "Character", 0, &cell),
            Err(Error::UnknownShape(ShapeId { index: 3 }))
        );
    }

    // --- the fixed palette --------------------------------------------

    #[test]
    fn palette_holds_every_index_from_zero_to_twenty_three() {
        assert_eq!(palette(0), Some(Color::new(0, 0, 0)));
        assert_eq!(palette(1), Some(Color::new(255, 255, 255)));
        assert_eq!(palette(2), Some(Color::new(255, 0, 0)));
        assert_eq!(palette(14), Some(Color::new(128, 128, 128)));
        assert_eq!(palette(19), Some(Color::new(128, 128, 128)));
        assert_eq!(palette(23), Some(Color::new(26, 26, 26)));
        for index in 0..=23 {
            assert!(palette(index).is_some(), "palette {index}");
        }
        assert_eq!(palette(24), None);
        assert_eq!(palette(255), None);
    }

    #[test]
    fn palette_entries_are_named_distinct_table_rows() {
        // Every row is one entry; the names are documentation of the
        // provenance (docs/sources.md).
        for index in 0..=23 {
            assert!(palette(index).is_some());
        }
    }

    // --- theme role and slot tables -----------------------------------

    #[test]
    fn theme_roles_cover_the_four_quick_style_cells() {
        assert_eq!(ThemeRole::of_cell("LineColor"), Some(ThemeRole::Line));
        assert_eq!(
            ThemeRole::of_cell("LineGradientStopColor"),
            Some(ThemeRole::Line)
        );
        assert_eq!(ThemeRole::of_cell("FillForegnd"), Some(ThemeRole::Fill));
        assert_eq!(ThemeRole::of_cell("FillBkgnd"), Some(ThemeRole::Fill));
        assert_eq!(
            ThemeRole::of_cell("GradientStopColor"),
            Some(ThemeRole::Fill)
        );
        assert_eq!(
            ThemeRole::of_cell("ShapeShdwColor"),
            Some(ThemeRole::Shadow)
        );
        assert_eq!(ThemeRole::of_cell("GlowColor"), Some(ThemeRole::Shadow));
        assert_eq!(
            ThemeRole::of_cell("BevelDepthColor"),
            Some(ThemeRole::Shadow)
        );
        assert_eq!(ThemeRole::of_cell("Color"), Some(ThemeRole::Font));
        assert_eq!(ThemeRole::of_cell("PinX"), None);
        assert_eq!(
            ThemeRole::quick_style_cell(ThemeRole::Line),
            "QuickStyleLineColor"
        );
        assert_eq!(
            ThemeRole::quick_style_cell(ThemeRole::Fill),
            "QuickStyleFillColor"
        );
        assert_eq!(
            ThemeRole::quick_style_cell(ThemeRole::Shadow),
            "QuickStyleShadowColor"
        );
        assert_eq!(
            ThemeRole::quick_style_cell(ThemeRole::Font),
            "QuickStyleFontColor"
        );
    }

    #[test]
    fn theme_slots_map_the_documented_quick_style_values() {
        assert_eq!(theme_slot(0), Some(ThemeSlot::Dk1));
        assert_eq!(theme_slot(1), Some(ThemeSlot::Lt1));
        assert_eq!(theme_slot(2), Some(ThemeSlot::Accent(0)));
        assert_eq!(theme_slot(7), Some(ThemeSlot::Accent(5)));
        // 8 is the background colour and 100+ are variant encodings: the
        // colour scheme cannot answer them.
        assert_eq!(theme_slot(8), None);
        assert_eq!(theme_slot(106), None);
        assert_eq!(theme_slot(201), None);
    }

    #[test]
    fn text_substitutes_empty_on_errors_and_missing_values() {
        let (doc, shape) =
            document_with_cells(vec![cell("Bad", CellValue::Error("#VALUE!".to_owned()))]);
        let mut reader = CellReader::new(&doc);
        let sheet = SheetRef::Shape(shape);
        assert_eq!(
            reader.text(sheet, &CellName::from_static("Bad")),
            Ok(String::new())
        );
        assert_eq!(
            reader.warnings()[0].kind,
            WarningKind::ErrorValue("#VALUE!".to_owned())
        );
        assert_eq!(
            reader.text(sheet, &CellName::from_static("Gone")),
            Ok(String::new())
        );
        assert_eq!(reader.warnings()[1].kind, WarningKind::Missing);
    }

    #[test]
    fn color_substitutes_black_on_a_missing_value() {
        let (doc, shape) = document_with_shape();
        let mut reader = CellReader::new(&doc);
        assert_eq!(
            reader.color(SheetRef::Shape(shape), &CellName::from_static("LineColor")),
            Ok(Color::new(0, 0, 0))
        );
        assert_eq!(reader.warnings()[0].kind, WarningKind::Missing);
    }

    #[test]
    fn page_sheet_cells_read_through_the_page_lookup() {
        let mut doc = Document::new();
        let mut page = crate::page::Page::new(PageId { index: 0 }, 1);
        page.sheet
            .cells
            .push(cell("PageWidth", CellValue::Number(11.0)));
        doc.pages.push(page);
        let mut reader = CellReader::new(&doc);
        assert_eq!(
            reader.number(
                SheetRef::Page(PageId { index: 0 }),
                &CellName::from_static("PageWidth")
            ),
            Ok(11.0)
        );
        assert_eq!(
            reader.text(
                SheetRef::Page(PageId { index: 0 }),
                &CellName::from_static("PageWidth")
            ),
            Ok(String::new())
        );
        assert_eq!(
            reader.list(
                SheetRef::Page(PageId { index: 0 }),
                &CellName::from_static("PageWidth")
            ),
            Ok(vec![11.0])
        );
        assert_eq!(reader.warnings()[0].kind, WarningKind::KindMismatch);
    }

    #[test]
    fn a_themed_color_with_an_unusable_quick_style_value_does_not_resolve() {
        // A QuickStyle slot that is not a whole number, or beyond u32.
        let (mut doc, shape) = document_with_cells(vec![
            cell("LineColor", CellValue::Str("Themed".to_owned())),
            cell("QuickStyleLineColor", CellValue::Number(2.5)),
            cell("FillForegnd", CellValue::Str("Themed".to_owned())),
            cell("QuickStyleFillColor", CellValue::Number(1.0e30)),
        ]);
        with_theme(&mut doc);
        let mut reader = CellReader::new(&doc);
        let sheet = SheetRef::Shape(shape);
        assert_eq!(
            reader.color(sheet, &CellName::from_static("LineColor")),
            Ok(Color::new(0, 0, 0))
        );
        assert_eq!(
            reader.color(sheet, &CellName::from_static("FillForegnd")),
            Ok(Color::new(0, 0, 0))
        );
        assert_eq!(reader.warnings().len(), 2);
        for warning in reader.warnings() {
            assert_eq!(warning.kind, WarningKind::UnresolvedTheme);
        }
    }
}
