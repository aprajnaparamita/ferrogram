//! The visual styles we define ourselves (ADR 0023).
//!
//! Visio's line patterns, line ends, fill patterns and shadow styles are
//! not in [MS-VSDX]; we draw our own and commit them as data in
//! `crates/render/data/visual-styles.yaml`. Indices and names come from
//! the Visio VBA enumerations (derived facts); the drawings are our own
//! work. This module resolves an index to a template; the scene builder
//! scales templates by the shape's cells (ADR 0020).
//!
//! Conventions (also documented in the data file header):
//!
//! - dash lengths are multiples of the line width;
//! - a line-end shape lives in the unit box: `x = 0` is the tip at the
//!   shape's vertex, `x = 1` is the tail along the line, `y = 0.5` is the
//!   line centre; `width` scales `y` and `length` scales `x`, both as
//!   multiples of the arrow size class;
//! - a fill tile lives in the unit tile box `(0, 0)` to `(1, 1)` and is
//!   repeated with pitch [`PatternTemplate::spacing`], a multiple of the
//!   line width; `filled` fills the tile shapes and `false` strokes them
//!   with [`PatternTemplate::tile_stroke_width`], a fraction of the tile
//!   box side; `invert` marks tiles drawn in the background colour over a
//!   foreground-coloured box;
//! - shadow offsets and blur are multiples of the line width.
//!
//! Paths in the data file use a small absolute-coordinate dialect of our
//! own: `M x y`, `L x y`, `Q cx cy x y`, `C c1x c1y c2x c2y x y` and `Z`
//! (case-insensitive), separated by whitespace or commas. `none` is the
//! empty path.

use std::collections::BTreeMap;

use larrygram_geom::{Path, Point, Segment, Size};
use serde::Deserialize;

use crate::error::Error;

/// The committed data file (ADR 0023). Parsed at build time, so it ships
/// inside the wasm build and there is no runtime file access.
const DATA: &str = include_str!("../data/visual-styles.yaml");

/// The largest path the data file may hold in one entry. Bounds the
/// parser on input-derived values (AGENTS.md section 6).
const MAX_PATH_SEGMENTS: usize = 4096;

/// The largest dash vector a line pattern may hold.
const MAX_DASHES: usize = 64;

/// The tile stroke width of a template that does not name one, as a
/// fraction of the tile box side (the unit-box convention).
const DEFAULT_TILE_STROKE_WIDTH: f64 = 0.05;

/// One line pattern: a dash rhythm as multiples of the line width
/// (ADR 0023).
#[derive(Debug, Clone, PartialEq)]
pub struct LinePatternTemplate {
    /// The dash lengths as multiples of the line width; empty is solid.
    pub dashes: Vec<f64>,
    /// Whether the pattern hides the line entirely.
    pub invisible: bool,
}

/// One line end (arrowhead) template (ADR 0023).
#[derive(Debug, Clone, PartialEq)]
pub struct LineEndTemplate {
    /// The arrowhead shape in unit coordinates.
    pub shape: Path,
    /// The width as a multiple of the size class.
    pub width: f64,
    /// The length as a multiple of the size class.
    pub length: f64,
    /// Whether the shape is filled.
    pub filled: bool,
}

/// What a fill pattern draws (ADR 0023).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternKind {
    /// Nothing: transparent fill (`visNoFill`).
    None,
    /// A solid foreground fill (`visSolid`).
    Solid,
    /// A repeating tile (every other fill pattern).
    Tile,
}

/// One fill pattern template (ADR 0023).
#[derive(Debug, Clone, PartialEq)]
pub struct PatternTemplate {
    /// Whether the pattern is empty, solid or a repeating tile.
    pub kind: PatternKind,
    /// The tile shape in unit coordinates (a [`PatternKind::Tile`]).
    pub tile: Path,
    /// The tile box in unit coordinates (the `(0, 0)` to `(1, 1)` box).
    pub tile_size: Size,
    /// The tile pitch as a multiple of the line width.
    pub spacing: f64,
    /// Whether the tile shapes are filled or stroked.
    pub filled: bool,
    /// The stroke width of a stroked tile (`filled: false`), as a
    /// fraction of the tile box side (the unit-box convention).
    pub tile_stroke_width: f64,
    /// Whether the tile draws the background colour over a
    /// foreground-coloured box (the `visBackDots` family). The builder
    /// swaps the two colours for these tiles.
    pub invert: bool,
}

/// One shadow template (ADR 0023).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadowTemplate {
    /// The x offset as a multiple of the line width.
    pub dx: f64,
    /// The y offset as a multiple of the line width.
    pub dy: f64,
    /// The blur radius as a multiple of the line width.
    pub blur: f64,
    /// The shadow alpha, 0 to 1.
    pub alpha: f64,
}

/// The visual style definitions (ADR 0023).
///
/// [`VisualStyles::load`] parses the committed data file; the four
/// lookups resolve a Visio style index to its template. An index the data
/// file does not define returns `None`: the caller substitutes the no-op
/// style and warns (ADR 0021).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct VisualStyles {
    line_patterns: BTreeMap<u32, LinePatternTemplate>,
    line_ends: BTreeMap<u32, LineEndTemplate>,
    fill_patterns: BTreeMap<u32, PatternTemplate>,
    shadows: BTreeMap<u32, ShadowTemplate>,
}

impl VisualStyles {
    /// Loads the committed data file `crates/render/data/visual-styles.yaml`.
    ///
    /// Returns [`Error::InvalidStyleData`] when the committed data does
    /// not parse; that is a build bug, not file input.
    pub fn load() -> Result<VisualStyles, Error> {
        Self::parse(DATA)
    }

    /// The dash pattern of a Visio line pattern index.
    ///
    /// Returns `None` when the data file does not define the index
    /// (ADR 0021: the caller substitutes the no-op style and warns).
    pub fn line_pattern(&self, index: u32) -> Option<LinePatternTemplate> {
        self.line_patterns.get(&index).cloned()
    }

    /// The arrowhead template of a Visio line end index
    /// (`BeginArrow` and `EndArrow` share the index space).
    ///
    /// Returns `None` when the data file does not define the index
    /// (ADR 0021: the caller substitutes the no-op style and warns).
    pub fn line_end(&self, index: u32) -> Option<LineEndTemplate> {
        self.line_ends.get(&index).cloned()
    }

    /// The tile template of a Visio fill pattern index.
    ///
    /// Returns `None` when the data file does not define the index
    /// (ADR 0021: the caller substitutes the no-op style and warns).
    pub fn fill_pattern(&self, index: u32) -> Option<PatternTemplate> {
        self.fill_patterns.get(&index).cloned()
    }

    /// The shadow template of a Visio shadow pattern index
    /// (`ShdwPattern`).
    ///
    /// Returns `None` when the data file does not define the index
    /// (ADR 0021: the caller substitutes the no-op style and warns).
    pub fn shadow(&self, index: u32) -> Option<ShadowTemplate> {
        self.shadows.get(&index).cloned()
    }

    /// Parses a data file in the format of `data/visual-styles.yaml`.
    ///
    /// [`Self::load`] calls this on the committed file; it is separate so
    /// tests can feed malformed data.
    fn parse(text: &str) -> Result<VisualStyles, Error> {
        let raw: RawFile = serde_saphyr::from_str(text).map_err(|_| Error::InvalidStyleData)?;
        raw.build()
    }
}

/// The raw shape of the data file, decoded with serde (ADR 0023).
///
/// Fields the templates do not need (`note`, `meta.note`) are not
/// declared; serde ignores them. Validation happens in [`RawFile::build`].
#[derive(Debug, Deserialize)]
struct RawFile {
    /// The `meta` block.
    #[serde(default)]
    meta: RawMeta,
    /// The `line_patterns` entries.
    #[serde(default)]
    line_patterns: Vec<RawLinePattern>,
    /// The `line_ends` entries.
    #[serde(default)]
    line_ends: Vec<RawLineEnd>,
    /// The `fill_patterns` entries.
    #[serde(default)]
    fill_patterns: Vec<RawFillPattern>,
    /// The `shadows` entries.
    #[serde(default)]
    shadows: Vec<RawShadow>,
}

/// The `meta` block of the data file.
#[derive(Debug, Default, Deserialize)]
struct RawMeta {
    /// The data format version; only 1 exists.
    #[serde(default)]
    version: u32,
}

/// One `line_patterns` entry.
#[derive(Debug, Deserialize)]
struct RawLinePattern {
    /// The Visio `LinePattern` index.
    index: u32,
    /// The dash lengths as multiples of the line width.
    #[serde(default)]
    dashes: Vec<f64>,
    /// Whether the pattern hides the line.
    #[serde(default)]
    invisible: bool,
}

/// One `line_ends` entry.
#[derive(Debug, Deserialize)]
struct RawLineEnd {
    /// The Visio `BeginArrow`/`EndArrow` index.
    index: u32,
    /// The arrowhead shape in the path dialect of the data file.
    shape: Option<String>,
    /// The width as a multiple of the size class (default 1).
    width: Option<f64>,
    /// The length as a multiple of the size class (default 1).
    length: Option<f64>,
    /// Whether the shape is filled.
    #[serde(default)]
    filled: bool,
}

/// One `fill_patterns` entry.
#[derive(Debug, Deserialize)]
struct RawFillPattern {
    /// The Visio `FillPattern` index.
    index: u32,
    /// `none`, `solid` or `tile`.
    kind: String,
    /// The tile shape in the path dialect of the data file.
    tile: Option<String>,
    /// The tile pitch as a multiple of the line width.
    spacing: Option<f64>,
    /// Whether the tile shapes are filled or stroked.
    #[serde(default)]
    filled: bool,
    /// The stroke width of a stroked tile, as a fraction of the tile box
    /// side (default [`DEFAULT_TILE_STROKE_WIDTH`]).
    tile_stroke_width: Option<f64>,
    /// Whether the tile draws the background colour.
    #[serde(default)]
    invert: bool,
}

/// One `shadows` entry.
#[derive(Debug, Deserialize)]
struct RawShadow {
    /// The Visio `ShdwPattern` index.
    index: u32,
    /// `none` or `drop`.
    kind: String,
    /// The x offset as a multiple of the line width.
    dx: Option<f64>,
    /// The y offset as a multiple of the line width.
    dy: Option<f64>,
    /// The blur radius as a multiple of the line width.
    blur: Option<f64>,
    /// The shadow alpha, 0 to 1.
    alpha: Option<f64>,
}

impl RawFile {
    /// Validates and converts the raw file into [`VisualStyles`].
    fn build(self) -> Result<VisualStyles, Error> {
        if self.meta.version != 1 {
            return Err(Error::InvalidStyleData);
        }
        let mut styles = VisualStyles::default();
        for entry in self.line_patterns {
            if entry.dashes.len() > MAX_DASHES {
                return Err(Error::InvalidStyleData);
            }
            if entry.dashes.iter().any(|d| !d.is_finite() || *d <= 0.0) {
                return Err(Error::InvalidStyleData);
            }
            let template = LinePatternTemplate {
                dashes: entry.dashes,
                invisible: entry.invisible,
            };
            if styles.line_patterns.insert(entry.index, template).is_some() {
                return Err(Error::InvalidStyleData);
            }
        }
        for entry in self.line_ends {
            let shape = match entry.shape.as_deref() {
                Some(text) => parse_path(text)?,
                None => Path::new(),
            };
            let width = entry.width.unwrap_or(1.0);
            let length = entry.length.unwrap_or(1.0);
            if !is_size(width) || !is_size(length) {
                return Err(Error::InvalidStyleData);
            }
            let template = LineEndTemplate {
                shape,
                width,
                length,
                filled: entry.filled,
            };
            if styles.line_ends.insert(entry.index, template).is_some() {
                return Err(Error::InvalidStyleData);
            }
        }
        for entry in self.fill_patterns {
            let template = match entry.kind.as_str() {
                "none" => PatternTemplate {
                    kind: PatternKind::None,
                    tile: Path::new(),
                    tile_size: Size::new(1.0, 1.0),
                    spacing: 0.0,
                    filled: false,
                    tile_stroke_width: DEFAULT_TILE_STROKE_WIDTH,
                    invert: false,
                },
                "solid" => PatternTemplate {
                    kind: PatternKind::Solid,
                    tile: Path::new(),
                    tile_size: Size::new(1.0, 1.0),
                    spacing: 0.0,
                    filled: false,
                    tile_stroke_width: DEFAULT_TILE_STROKE_WIDTH,
                    invert: false,
                },
                "tile" => {
                    let tile = match entry.tile.as_deref() {
                        Some(text) => parse_path(text)?,
                        None => return Err(Error::InvalidStyleData),
                    };
                    if tile.is_empty() {
                        return Err(Error::InvalidStyleData);
                    }
                    let spacing = match entry.spacing {
                        Some(spacing) if spacing.is_finite() && spacing > 0.0 => spacing,
                        _ => return Err(Error::InvalidStyleData),
                    };
                    let tile_stroke_width =
                        entry.tile_stroke_width.unwrap_or(DEFAULT_TILE_STROKE_WIDTH);
                    if !tile_stroke_width.is_finite() || tile_stroke_width <= 0.0 {
                        return Err(Error::InvalidStyleData);
                    }
                    PatternTemplate {
                        kind: PatternKind::Tile,
                        tile,
                        tile_size: Size::new(1.0, 1.0),
                        spacing,
                        filled: entry.filled,
                        tile_stroke_width,
                        invert: entry.invert,
                    }
                }
                _ => return Err(Error::InvalidStyleData),
            };
            if styles.fill_patterns.insert(entry.index, template).is_some() {
                return Err(Error::InvalidStyleData);
            }
        }
        for entry in self.shadows {
            let template = match entry.kind.as_str() {
                "none" => ShadowTemplate {
                    dx: 0.0,
                    dy: 0.0,
                    blur: 0.0,
                    alpha: 0.0,
                },
                "drop" => {
                    let (dx, dy, blur, alpha) = match (entry.dx, entry.dy, entry.blur, entry.alpha)
                    {
                        (Some(dx), Some(dy), Some(blur), Some(alpha)) => (dx, dy, blur, alpha),
                        _ => return Err(Error::InvalidStyleData),
                    };
                    if !dx.is_finite() || !dy.is_finite() || !is_size(blur) {
                        return Err(Error::InvalidStyleData);
                    }
                    if !alpha.is_finite() || !(0.0..=1.0).contains(&alpha) {
                        return Err(Error::InvalidStyleData);
                    }
                    ShadowTemplate {
                        dx,
                        dy,
                        blur,
                        alpha,
                    }
                }
                _ => return Err(Error::InvalidStyleData),
            };
            if styles.shadows.insert(entry.index, template).is_some() {
                return Err(Error::InvalidStyleData);
            }
        }
        Ok(styles)
    }
}

/// Parses a data file for the builder's tests (the same entry
/// [`VisualStyles::load`] uses on the committed file).
#[cfg(test)]
pub(crate) fn parse_for_test(text: &str) -> Result<VisualStyles, Error> {
    VisualStyles::parse(text)
}

/// Whether a size-like value is finite and not negative.
fn is_size(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

/// Parses the path dialect of the data file into a [`Path`].
///
/// `M x y`, `L x y`, `Q cx cy x y`, `C c1x c1y c2x c2y x y` and `Z`
/// (case-insensitive), separated by whitespace or commas. `none` and the
/// empty string are the empty path. The first command must be `M` and
/// every `Z` must be followed by `M`. Malformed input is
/// [`Error::InvalidStyleData`], never a panic (AGENTS.md section 6).
fn parse_path(text: &str) -> Result<Path, Error> {
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed == "none" {
        return Ok(Path::new());
    }
    let mut tokens = trimmed
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|token| !token.is_empty());
    let mut path = Path::new();
    let mut segments = 0usize;
    let mut needs_move = true;
    while let Some(command) = tokens.next() {
        if needs_move && command != "M" && command != "m" {
            return Err(Error::InvalidStyleData);
        }
        let segment = match command {
            "M" | "m" => {
                needs_move = false;
                Segment::MoveTo(Point::new(
                    next_number(&mut tokens)?,
                    next_number(&mut tokens)?,
                ))
            }
            "L" | "l" => Segment::LineTo(Point::new(
                next_number(&mut tokens)?,
                next_number(&mut tokens)?,
            )),
            "Q" | "q" => Segment::QuadTo {
                ctrl: Point::new(next_number(&mut tokens)?, next_number(&mut tokens)?),
                to: Point::new(next_number(&mut tokens)?, next_number(&mut tokens)?),
            },
            "C" | "c" => Segment::CubicTo {
                ctrl1: Point::new(next_number(&mut tokens)?, next_number(&mut tokens)?),
                ctrl2: Point::new(next_number(&mut tokens)?, next_number(&mut tokens)?),
                to: Point::new(next_number(&mut tokens)?, next_number(&mut tokens)?),
            },
            "Z" | "z" => {
                needs_move = true;
                Segment::Close
            }
            _ => return Err(Error::InvalidStyleData),
        };
        path.push(segment);
        segments += 1;
        if segments > MAX_PATH_SEGMENTS {
            return Err(Error::InvalidStyleData);
        }
    }
    Ok(path)
}

/// Takes the next token as a finite number, or errors.
fn next_number<'a, I: Iterator<Item = &'a str>>(tokens: &mut I) -> Result<f64, Error> {
    let token = tokens.next().ok_or(Error::InvalidStyleData)?;
    let value: f64 = token.parse().map_err(|_| Error::InvalidStyleData)?;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(Error::InvalidStyleData)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The committed file, with one small change: a test data source.
    fn data_with(old: &str, new: &str) -> String {
        DATA.replace(old, new)
    }

    #[test]
    fn load_parses_the_committed_file() {
        let styles = VisualStyles::load().expect("committed data must parse");
        assert!(!styles.line_patterns.is_empty());
        assert!(!styles.line_ends.is_empty());
        assert!(!styles.fill_patterns.is_empty());
        assert!(!styles.shadows.is_empty());
    }

    #[test]
    fn corpus_indices_have_entries() {
        let styles = VisualStyles::load().expect("committed data must parse");
        // Corpus .vsdx census (data file header): LinePattern 0/1/23,
        // BeginArrow 0, EndArrow 0, FillPattern 0/1/24, ShdwPattern 0.
        assert!(styles.line_pattern(0).is_some());
        assert!(styles.line_pattern(1).is_some());
        assert!(styles.line_pattern(23).is_some());
        assert!(styles.line_end(0).is_some());
        assert!(styles.fill_pattern(0).is_some());
        assert!(styles.fill_pattern(1).is_some());
        assert!(styles.fill_pattern(24).is_some());
        assert!(styles.shadow(0).is_some());
    }

    #[test]
    fn known_indices_return_their_templates() {
        let styles = VisualStyles::load().expect("committed data must parse");
        assert_eq!(
            styles.line_pattern(0),
            Some(LinePatternTemplate {
                dashes: Vec::new(),
                invisible: true
            })
        );
        assert_eq!(
            styles.line_pattern(1),
            Some(LinePatternTemplate {
                dashes: Vec::new(),
                invisible: false
            })
        );
        assert_eq!(
            styles.line_pattern(23),
            Some(LinePatternTemplate {
                dashes: vec![1.5, 1.0],
                invisible: false
            })
        );
        let end = styles.line_end(0).expect("index 0 is defined");
        assert!(end.shape.is_empty());
        assert_eq!(end.width, 0.0);
        assert_eq!(end.length, 0.0);
        assert_eq!(
            styles.fill_pattern(0).map(|p| p.kind),
            Some(PatternKind::None)
        );
        assert_eq!(
            styles.fill_pattern(1).map(|p| p.kind),
            Some(PatternKind::Solid)
        );
        assert_eq!(
            styles.shadow(0),
            Some(ShadowTemplate {
                dx: 0.0,
                dy: 0.0,
                blur: 0.0,
                alpha: 0.0
            })
        );
    }

    #[test]
    fn the_drop_shadow_has_values() {
        let styles = VisualStyles::load().expect("committed data must parse");
        let shadow = styles.shadow(1).expect("index 1 is defined");
        assert_eq!(shadow.alpha, 0.5);
        assert!(shadow.blur > 0.0);
    }

    #[test]
    fn every_line_end_but_zero_draws_something() {
        let styles = VisualStyles::load().expect("committed data must parse");
        for index in 1..=24 {
            let end = styles.line_end(index).expect("indices 1..24 defined");
            assert!(!end.shape.is_empty(), "index {index} must draw");
            assert!(end.width > 0.0 && end.length > 0.0);
        }
    }

    #[test]
    fn every_tile_pattern_draws_a_tile() {
        let styles = VisualStyles::load().expect("committed data must parse");
        for index in 2..=24 {
            let pattern = styles.fill_pattern(index).expect("indices 2..24 defined");
            assert_eq!(pattern.kind, PatternKind::Tile);
            assert!(!pattern.tile.is_empty(), "index {index} must draw a tile");
            assert!(pattern.spacing > 0.0);
        }
    }

    #[test]
    fn unknown_indices_return_none() {
        let styles = VisualStyles::load().expect("committed data must parse");
        assert_eq!(styles.line_pattern(24), None);
        assert_eq!(styles.line_end(25), None);
        assert_eq!(styles.fill_pattern(25), None);
        assert_eq!(styles.shadow(2), None);
    }

    #[test]
    fn lookups_return_clones() {
        let styles = VisualStyles::load().expect("committed data must parse");
        let mut first = styles.line_pattern(23).expect("index 23 is defined");
        first.dashes.clear();
        let second = styles.line_pattern(23).expect("index 23 is defined");
        assert_eq!(second.dashes, vec![1.5, 1.0]);
    }

    #[test]
    fn parse_rejects_broken_yaml() {
        assert_eq!(
            VisualStyles::parse("]not yaml"),
            Err(Error::InvalidStyleData)
        );
        assert_eq!(VisualStyles::parse(""), Err(Error::InvalidStyleData));
    }

    #[test]
    fn parse_rejects_an_unknown_version() {
        let text = data_with("version: 1", "version: 2");
        assert_eq!(VisualStyles::parse(&text), Err(Error::InvalidStyleData));
        let text = data_with("version: 1", "");
        assert_eq!(VisualStyles::parse(&text), Err(Error::InvalidStyleData));
    }

    #[test]
    fn parse_rejects_duplicate_indices() {
        let text = "meta: {version: 1}\nline_patterns:\n  - index: 0\n  - index: 0\n";
        assert_eq!(VisualStyles::parse(text), Err(Error::InvalidStyleData));
    }

    #[test]
    fn parse_rejects_bad_dash_values() {
        let text = "meta: {version: 1}\nline_patterns:\n  - index: 2\n    dashes: [4, -2]\n";
        assert_eq!(VisualStyles::parse(text), Err(Error::InvalidStyleData));
        let text = "meta: {version: 1}\nline_patterns:\n  - index: 2\n    dashes: [inf]\n";
        assert_eq!(VisualStyles::parse(text), Err(Error::InvalidStyleData));
        let text = "meta: {version: 1}\nline_patterns:\n  - index: 2\n    dashes: [nan]\n";
        assert_eq!(VisualStyles::parse(text), Err(Error::InvalidStyleData));
    }

    #[test]
    fn parse_rejects_oversized_dash_vectors() {
        let dashes: Vec<String> = (0..MAX_DASHES + 1).map(|_| "1".to_string()).collect();
        let text = format!(
            "meta: {{version: 1}}\nline_patterns:\n  - index: 2\n    dashes: [{}]\n",
            dashes.join(", ")
        );
        assert_eq!(VisualStyles::parse(&text), Err(Error::InvalidStyleData));
    }

    #[test]
    fn parse_rejects_bad_line_end_values() {
        let text = "meta: {version: 1}\nline_ends:\n  - index: 1\n    width: -1\n";
        assert_eq!(VisualStyles::parse(text), Err(Error::InvalidStyleData));
        let text = "meta: {version: 1}\nline_ends:\n  - index: 1\n    length: inf\n";
        assert_eq!(VisualStyles::parse(text), Err(Error::InvalidStyleData));
    }

    #[test]
    fn parse_rejects_bad_fill_patterns() {
        // Unknown kind.
        let text = "meta: {version: 1}\nfill_patterns:\n  - index: 2\n    kind: wavy\n";
        assert_eq!(VisualStyles::parse(text), Err(Error::InvalidStyleData));
        // Tile without a tile path.
        let text =
            "meta: {version: 1}\nfill_patterns:\n  - index: 2\n    kind: tile\n    spacing: 4\n";
        assert_eq!(VisualStyles::parse(text), Err(Error::InvalidStyleData));
        // Tile with an empty tile path.
        let text = "meta: {version: 1}\nfill_patterns:\n  - index: 2\n    kind: tile\n    tile: none\n    spacing: 4\n";
        assert_eq!(VisualStyles::parse(text), Err(Error::InvalidStyleData));
        // Tile without spacing.
        let text = "meta: {version: 1}\nfill_patterns:\n  - index: 2\n    kind: tile\n    tile: M 0 0 L 1 1\n";
        assert_eq!(VisualStyles::parse(text), Err(Error::InvalidStyleData));
        // Tile with a zero spacing.
        let text = "meta: {version: 1}\nfill_patterns:\n  - index: 2\n    kind: tile\n    tile: M 0 0 L 1 1\n    spacing: 0\n";
        assert_eq!(VisualStyles::parse(text), Err(Error::InvalidStyleData));
        // Tile with a zero, negative or non-finite stroke width.
        for width in ["0", "-1", "inf"] {
            let text = format!(
                "meta: {{version: 1}}\nfill_patterns:\n  - index: 2\n    kind: tile\n    \
                 tile: M 0 0 L 1 1\n    spacing: 4\n    tile_stroke_width: {width}\n"
            );
            assert_eq!(VisualStyles::parse(&text), Err(Error::InvalidStyleData));
        }
    }

    #[test]
    fn a_tile_stroke_width_defaults_and_sticks() {
        // The default of a tile that names none.
        let text = "meta: {version: 1}\nfill_patterns:\n  - index: 2\n    kind: tile\n    tile: M 0 0 L 1 1\n    spacing: 4\n";
        let styles = VisualStyles::parse(text).expect("valid file");
        let tile = styles.fill_pattern(2).expect("tile");
        assert_eq!(tile.tile_stroke_width, 0.05);
        assert!(!tile.filled);
        // An explicit value.
        let text = "meta: {version: 1}\nfill_patterns:\n  - index: 2\n    kind: tile\n    tile: M 0 0 L 1 1\n    spacing: 4\n    filled: true\n    tile_stroke_width: 0.2\n";
        let styles = VisualStyles::parse(text).expect("valid file");
        let tile = styles.fill_pattern(2).expect("tile");
        assert_eq!(tile.tile_stroke_width, 0.2);
        assert!(tile.filled);
    }

    #[test]
    fn parse_rejects_bad_shadows() {
        let text = "meta: {version: 1}\nshadows:\n  - index: 1\n    kind: inner\n";
        assert_eq!(VisualStyles::parse(text), Err(Error::InvalidStyleData));
        // A drop shadow needs all four values.
        let text = "meta: {version: 1}\nshadows:\n  - index: 1\n    kind: drop\n    dx: 1\n";
        assert_eq!(VisualStyles::parse(text), Err(Error::InvalidStyleData));
        // Alpha stays in 0..1.
        let text = "meta: {version: 1}\nshadows:\n  - index: 1\n    kind: drop\n    dx: 1\n    dy: -1\n    blur: 1\n    alpha: 2\n";
        assert_eq!(VisualStyles::parse(text), Err(Error::InvalidStyleData));
        let text = "meta: {version: 1}\nshadows:\n  - index: 1\n    kind: drop\n    dx: 1\n    dy: -1\n    blur: -1\n    alpha: 0.5\n";
        assert_eq!(VisualStyles::parse(text), Err(Error::InvalidStyleData));
    }

    #[test]
    fn parse_accepts_a_minimal_file() {
        let styles = VisualStyles::parse("meta: {version: 1}").expect("minimal file is valid");
        assert_eq!(styles.line_pattern(0), None);
        assert_eq!(styles.line_end(0), None);
        assert_eq!(styles.fill_pattern(0), None);
        assert_eq!(styles.shadow(0), None);
    }

    #[test]
    fn parse_accepts_minimal_entries() {
        let text = concat!(
            "meta: {version: 1}\n",
            "line_patterns:\n  - index: 7\n",
            "line_ends:\n  - index: 3\n",
            "fill_patterns:\n  - index: 5\n    kind: none\n",
            "shadows:\n  - index: 0\n    kind: none\n",
        );
        let styles = VisualStyles::parse(text).expect("defaults are valid");
        assert_eq!(
            styles.line_pattern(7),
            Some(LinePatternTemplate {
                dashes: Vec::new(),
                invisible: false
            })
        );
        let end = styles.line_end(3).expect("index 3 is defined");
        assert!(end.shape.is_empty());
        assert_eq!(end.width, 1.0);
        assert_eq!(end.length, 1.0);
    }

    #[test]
    fn parse_path_handles_the_dialect() {
        let path = parse_path("M 0 0 L 1 0.5 L 2 1 Q 0 0 1 1 C 0 0 1 1 2 2 Z M 3 3 Z")
            .expect("valid path");
        assert_eq!(path.segments().len(), 8);
        assert_eq!(
            path.segments().first(),
            Some(&Segment::MoveTo(Point::new(0.0, 0.0)))
        );
        assert_eq!(path.segments().last(), Some(&Segment::Close));
    }

    #[test]
    fn parse_path_accepts_the_empty_forms() {
        assert!(parse_path("none").expect("none is empty").is_empty());
        assert!(parse_path("").expect("empty is empty").is_empty());
        assert!(parse_path("  ").expect("blank is empty").is_empty());
    }

    #[test]
    fn parse_path_rejects_malformed_input() {
        // Unknown command.
        assert_eq!(parse_path("X 0 0"), Err(Error::InvalidStyleData));
        // The first command must be a move.
        assert_eq!(parse_path("L 0 0"), Err(Error::InvalidStyleData));
        // Missing numbers.
        assert_eq!(parse_path("M 0"), Err(Error::InvalidStyleData));
        assert_eq!(parse_path("M 0 0 L 1"), Err(Error::InvalidStyleData));
        assert_eq!(parse_path("M 0 0 Q 1 1"), Err(Error::InvalidStyleData));
        assert_eq!(parse_path("M 0 0 C 1 1 2 2"), Err(Error::InvalidStyleData));
        // Tokens that are not numbers.
        assert_eq!(parse_path("M zero 0"), Err(Error::InvalidStyleData));
        // Non-finite numbers.
        assert_eq!(parse_path("M inf 0"), Err(Error::InvalidStyleData));
        assert_eq!(parse_path("M nan 0"), Err(Error::InvalidStyleData));
        // A close must be followed by a move.
        assert_eq!(parse_path("M 0 0 Z L 1 1"), Err(Error::InvalidStyleData));
        // A stray token.
        assert_eq!(parse_path("M 0 0 1 1"), Err(Error::InvalidStyleData));
    }

    #[test]
    fn parse_path_caps_the_segment_count() {
        let mut text = String::from("M 0 0");
        for _ in 0..MAX_PATH_SEGMENTS {
            text.push_str(" L 1 1");
        }
        assert_eq!(parse_path(&text), Err(Error::InvalidStyleData));
    }

    #[test]
    fn parse_rejects_a_broken_tile_path() {
        let text = "meta: {version: 1}\nfill_patterns:\n  - index: 2\n    kind: tile\n    tile: oops\n    spacing: 4\n";
        assert_eq!(VisualStyles::parse(text), Err(Error::InvalidStyleData));
        let text = "meta: {version: 1}\nline_ends:\n  - index: 1\n    shape: oops\n";
        assert_eq!(VisualStyles::parse(text), Err(Error::InvalidStyleData));
    }

    #[test]
    fn parse_rejects_an_oversized_tile_path() {
        let mut tile = String::from("M 0 0");
        for _ in 0..MAX_PATH_SEGMENTS {
            tile.push_str(" L 1 1");
        }
        let text = format!(
            "meta: {{version: 1}}\nfill_patterns:\n  - index: 2\n    kind: tile\n    tile: \"{tile}\"\n    spacing: 4\n"
        );
        assert_eq!(VisualStyles::parse(&text), Err(Error::InvalidStyleData));
    }

    #[test]
    fn duplicate_style_indices_are_invalid_data() {
        // One entry per repeated-index arm of the builder: line ends,
        // fill patterns and shadows.
        let line_ends = "meta:\n  version: 1\nline_ends:\n  - index: 1\n  - index: 1\n";
        assert_eq!(
            VisualStyles::parse(line_ends),
            Err(Error::InvalidStyleData),
            "duplicate line_end index parsed"
        );
        let fill_patterns = concat!(
            "meta:\n  version: 1\nfill_patterns:\n",
            "  - index: 1\n    kind: none\n",
            "  - index: 1\n    kind: none\n"
        );
        assert_eq!(
            VisualStyles::parse(fill_patterns),
            Err(Error::InvalidStyleData),
            "duplicate fill_pattern index parsed"
        );
        let shadows = concat!(
            "meta:\n  version: 1\nshadows:\n",
            "  - index: 1\n    kind: none\n",
            "  - index: 1\n    kind: none\n"
        );
        assert_eq!(
            VisualStyles::parse(shadows),
            Err(Error::InvalidStyleData),
            "duplicate shadow index parsed"
        );
    }
}
