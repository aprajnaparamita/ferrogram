# 0021: Cached values, defaults and warnings

- Status: accepted
- Date: 2026-10-14
- Task: P3-01

## Context

Phase 3 renders from **cached values only**: no formula evaluation until
Phase 6. Files cache values at save time, so this covers everything Visio
last computed. But a cached value can be missing (the cell has only a
formula), an error (`E='#VALUE!'`), of the wrong kind, or the inherit
marker `F='Inh'` whose `V` is only a snapshot (ADR 0013 amendment). The
renderer must do something predictable in each case, tell the caller, and
never fail a render because one cell is bad.

## Decision

### One facade: `ferrogram_model::CellReader`

Every cell the renderer reads goes through `CellReader` (L1, in `model`,
because it needs the lookup of ADR 0013):

```rust
pub enum SheetRef { Shape(ShapeId), Page(PageId) }

pub enum WarningKind {
    Missing,                    // nothing defines the value
    ErrorValue(String),         // cached error, e.g. "#VALUE!"
    KindMismatch,               // cached value has another kind
}

pub struct CellWarning {
    pub sheet: SheetRef,
    pub cell: CellName,
    pub kind: WarningKind,
    pub substituted: CellValue, // what was used instead
}

pub struct CellReader<'d> { /* document + collected warnings */ }
impl<'d> CellReader<'d> {
    pub fn new(doc: &'d Document) -> Self;
    pub fn number(&mut self, sheet: SheetRef, cell: &CellName) -> Result<f64, Error>;
    pub fn angle(&mut self, sheet: SheetRef, cell: &CellName) -> Result<Angle, Error>;
    pub fn boolean(&mut self, sheet: SheetRef, cell: &CellName) -> Result<bool, Error>;
    pub fn text(&mut self, sheet: SheetRef, cell: &CellName) -> Result<String, Error>;
    pub fn color(&mut self, sheet: SheetRef, cell: &CellName) -> Result<Color, Error>;
    pub fn list(&mut self, sheet: SheetRef, cell: &CellName) -> Result<Vec<f64>, Error>;
    pub fn row_number(&mut self, sheet: SheetRef, section: &str, row: usize,
                      cell: &CellName) -> Result<f64, Error>;
    pub fn warnings(&self) -> &[CellWarning];
    pub fn take_warnings(&mut self) -> Vec<CellWarning>;
}
```

### The policy

For each requested value, `CellReader`:

1. Resolves the cell through the lookup (ADR 0013 and its ADR 0019
   extensions). A resolving error (unknown id) is an `Err`, because it is a
   caller bug, not file data.
2. Uses the cached value when it is present, not an error and of the
   requested kind.
3. Otherwise substitutes the **hard default** of the kind (number `0`,
   angle `0`, bool `false`, text `""`, color black, list empty) and records
   a `CellWarning`. The `Inh` snapshot is **not** used (ADR 0013
   amendment): the lookup already continued to the master, style and
   defaults, and a snapshot of those would hide a real resolution bug.

A warning never fails the build: `build_scene`, `layout` and the renderers
return `Ok` with warnings collected in `Scene::warnings` (text warnings in
`Layout::warnings`).

### How warnings reach the caller and the CLI

- `Scene::warnings` and `Layout::warnings` carry them to the caller.
- The CLI prints one line per warning to **stderr** (`warning: shape 12
  LineWeight: no cached value, using 0`), keeps rendering, and exits 0.
- With `--strict`, warnings are errors: the CLI still writes the output
  unless rendering failed, and exits with the `warnings` exit code (4,
  ADR 0026).
- `inspect` reports the warning count in its JSON output.

## Consequences

- Rendering a file with one bad cell still produces an image; the exit
  criterion "every corpus file renders without panics" is achievable.
- Provenance is one step away (`resolve` is still public) for debugging.
- Once Phase 6 evaluates formulas, the same facade can prefer an evaluated
  value over the cached one without changing a single call site.
- Warnings are per read; a caller that reads the same cell twice warns
  twice. Fine for Phase 3; a per-shape cache can come with Phase 6.

## Alternatives considered

- **Fail the render on a missing cached value.** Rejected: KICKOFF's exit
  criterion is rendering without panics on real files, and real files have
  gaps (the corpus has `EllipticalArcTo` rows without `C`/`D`).
- **Use the `Inh` snapshot as a fallback.** Rejected for now (ADR 0013
  amendment keeps it out): snapshots are stale data that masks inheritance
  bugs. If corpus evidence shows worse output without it, amend ADR 0013.
- **Return `Result<Option<f64>>` per cell and let every caller decide.**
  Rejected: 420 cell kinds times N callers, each with its own default
  policy; the policy belongs in one place.
- **Warnings as `log` output.** Rejected: the library must not depend on a
  logging facade, and the CLI needs structured warnings for `--strict` and
  `inspect` counts.

## Amendments

### Amended 2026-10-14, P3-02: `Themed` values and colour indices

Two corpus findings the original decision did not cover. Both change what
counts as "of the requested kind" in the policy above.

**A cached `Themed` value is a marker, not a string.** Colour and
transparency cells whose value comes from the theme cache the literal
`Themed` (`<Cell N='FillForegnd' V='Themed' F='Inh'/>`, cells with
`F='THEMEVAL()'`). The reader keeps it as `Str("Themed")` whatever `U`
declares (`ferrogram-vsdx`, so a themed value in a numeric unit is not a
read error), and `CellReader` resolves it:

- `color` reads the shape's `QuickStyle*` cell for the cell's role
  (`QuickStyleLineColor` for `Line*` cells, `QuickStyleFillColor` for
  `Fill*`/`Gradient*` cells, `QuickStyleShadowColor` for shadow/effect
  colour cells, `QuickStyleFontColor` for the Character `Color` cell) and
  maps that slot against `Document::themes[0]`. The slot values are the
  ShapeSheet reference's: 0 is the Dark theme colour, 1 the Light theme
  colour, 2..7 Accent 1..6. Dark maps to `dk1` and Light to `lt1` (the
  corpus is consistent with it: `THEMEVAL("TextColor")` caches `#feffff`,
  which is `lt1`).
- A slot the theme colour scheme cannot answer — 8 (the background
  colour) and the variant encodings 100+ the corpus also writes
  (`QuickStyleLineColor V='106'`, `V='201'`) — substitutes the hard
  default with the new `WarningKind::UnresolvedTheme`, never a kind
  mismatch.
- A themed transparency (`*Trans`) and any other themed value the theme
  cannot answer substitute the hard default with `UnresolvedTheme` too.
  The colour schemes carry no alpha: the `a:fmtScheme` matrices (fill and
  line styles, effects, gradient stops) are deferred (open issue below).

**A cached numeric colour is an index.** Cells like
`<Cell N='LineColor' V='0' F='GUARD(0)'/>` cache an index, not an RGB:
0..23 is the fixed Visio palette (`ferrogram_model::values::palette`,
whose indices and names come from the `VisDefaultColors` enumeration on
Microsoft Learn; the RGB values are our own transcription pending a
black-box check) and 24 and up is the document colour table
(`Colors`/`ColorEntry`, `Document::color_table`; the corpus stores
`ColorEntry` rows only from `IX='24'` up, checked on all 15 files).
`CellReader::color` resolves an index through the palette first and the
colour table after; an index neither knows substitutes black with a
`Missing` warning. A non-integral or negative number is a kind mismatch.

**Booleans and lists are read the way the files write them.** A boolean
cell with no `U` caches `0`/`1` as a number (`FlipX V='0'`), and a
one-element list caches a plain number (`LayerMember V='0'`);
`CellReader::boolean` accepts numbers (0 is `false`) and
`CellReader::list` accepts one number as a one-element list, without a
warning. Those are coercions of the written form, not kind mismatches.

Consequences: themed shapes render in their theme colours instead of the
hard default; a shape whose QuickStyle variant the theme model cannot
answer renders in the hard default and says so in one warning per read.

Open issues:

- The variant encodings (`QuickStyle*Color` 100+, 2xx; the corpus shows
  101..106 and 200..206) resolve to colours that are not plain
  `a:clrScheme` slots — Visio derives them from the theme's variation
  colour schemes and the style matrices. Resolving them needs the
  `vt:variationClrSchemeLst` extension and the `a:fmtScheme` matrices
  (also the source of themed transparency, gradient stop colours and
  line weights).
- Whether Dark/Light mean `dk1`/`lt1` or `dk2`/`lt2` is not stated by the
  ShapeSheet reference; the corpus evidence above points at `dk1`/`lt1`.
  A black-box check against Visio settles it.
- A cached colour index of 24 or more that the document colour table does
  not hold (layer rows cache `Color V='255'`) may be a packed RGB value
  ("custom colors have values of 24 and above" in numeric operations,
  ShapeSheet reference) instead of a table index. It substitutes black
  with a `Missing` warning for now.

### Amended 2026-10-15, P3-15: theme variant colours

The variant encodings of the open issue above are now resolved. The mapping
([MS-VSDX] `QuickStyleFillColor` and the `QuickStyleLineColor`,
`QuickStyleShadowColor`, `QuickStyleFontColor` pages that reuse its table;
`VisQuickStyleColors`):

| `QuickStyle*Color` | Resolves to |
|---|---|
| 0 | `dk1` (the Dark theme colour) |
| 1 | `lt1` (the Light theme colour) |
| 2..7 | `accent1`..`accent6` |
| 8 | the theme's background colour (`bkgnd`, the `a:clrScheme` extension; `UnresolvedTheme` when the part has none) |
| 100..106 | `varColor1`..`varColor7` of the theme variation colour scheme the sheet's `VariationColorIndex` cell selects ("Color variant 1..7", `VisQuickStyleColors`) |
| 200..206 | the same `varColor1`..`varColor7` (no published meaning; corpus-derived, below) |
| anything else | `UnresolvedTheme` + the hard default |

- The variation colour schemes are the `vt:variationClrSchemeLst`
  extension of `a:clrScheme`, read into `Theme::variations` (one
  `varColor1`..`varColor7` set per `vt:variationClrScheme`).
- `VariationColorIndex` ([MS-VSDX]) selects the scheme: 0..3 the
  scheme's position, 65534 or a missing cell defers to the page sheet's
  cell, anything else is variation 0. `VariationStyleIndex` works the same
  way for the style schemes below.
- **200..206 have no published meaning.** The corpus shows the same
  variation colours: `color-boxes.vsdx`'s shapes 68..74 carry
  `QuickStyleLineColor`/`QuickStyleFillColor` 200..206 and cache the
  variation colours modulated by the line style matrix
  (`QuickStyleLineMatrix` 6, e.g. `#547495` = 0.73 x `varColor2`
  `#759FCC`), and `tdf154379-QuickStyleFillMatrix.vsdx`'s shape 317
  (variant 200, matrix 2) caches `LineColor V='#5b9bd5'` = `varColor1`
  unshaded. We resolve 200+k exactly like 100+k and flag it
  corpus-derived.
- The **style-matrix modulation of the colour** (`QuickStyle*Matrix` →
  the `a:fmtScheme` `fillStyleLst`/`lnStyleLst` entries, whose `tint` /
  `shade` modifiers work in the HSL colour space per [MS-VSDX] `ThemeCBV`)
  is **not** applied: `CellReader::color` resolves the theme's own colour.
  The corpus snapshots show Visio's value after the modulation; the exact
  transform stays open.

### Amended 2026-10-15, P3-15: themed patterns, weights and transparencies

A themed value other than a colour resolves through the theme's style
lists where a published meaning exists (the P3-15 note from the master,
after the CLI smoke run showed `Themed` patterns resolving to 0 made most
corpus shapes invisible):

- **`FillPattern`** resolves through the fill style its
  `QuickStyleFillMatrix` names ([MS-VSDX] `QuickStyleFillMatrix`): 1..6
  the `a:fmtScheme` `fillStyleLst` fills in order, 100..103 the `fillIdx`
  of the first..fourth `vt:varStyle` of the variation style scheme the
  `VariationStyleIndex` cell selects (`vt:variationStyleSchemeLst`; the
  `fillIdx` values are 1-based). The fill kind maps to the pattern value:
  `noFill` 0, `solidFill` 1, `gradFill` 25 (the renderer's gradient
  range; the stops still come from the shape's own `FillGradient` rows).
- **`LinePattern`** resolves the same way through `lnStyleLst` and
  `QuickStyleLineMatrix`/`lineIdx`: `a:noFill` 0 and a solid line (no
  `a:prstDash`, or `solid`) 1. A **dash preset** names no Visio
  `LinePattern` index (no published meaning): it falls back like an
  unresolvable pattern below.
- **`LineWeight`** resolves from the line style's `a:ln/@w` width (EMU,
  1/914400 inch per [ISO/IEC29500-1]).
- **Transparencies** (`FillForegndTrans`, `FillBkgndTrans`,
  `LineColorTrans`) resolve from the style's colour `a:alpha` (opacity
  0..100000): transparency = 1 - alpha/100000. No `a:alpha` is the
  DrawingML default, opaque (transparency 0).
- Matrix 0 names the root style sheet, not the theme.

**Fallbacks (visible, never 0 for fill and line).** Where the theme cannot
answer, a `Themed` pattern falls back to the visible value —
`FillPattern` 1, `LinePattern` 1, `ShdwPattern` 0 — and a `Themed`
`LineWeight` to a hairline default, each substituting with the
`UnresolvedTheme` warning. The hairline default is **0.7 point**, the
value the `THEMEVAL` reference documents for a theme-less document ("the
function returns '0.7 pt.'"); the P3-15 note said "a hairline default" and
we took the documented value instead of inventing one. Any other themed
number (a transparency the style lists cannot answer, `FillGradientDir`
and friends) still substitutes 0 with `UnresolvedTheme`.

### Amended 2026-10-15, P3-15: quiet documented defaults

A cell that is **absent at every rung** and has a default documented on its
ShapeSheet reference page resolves to that default with **no** warning
(`ferrogram_model::defaults`). The table transcribes the documented
defaults of the cells the scene builder reads: `LocPinX`/`LocPinY`
(`= Width*0.5`, `= Height*0.5`), `TxtPinX`/`TxtPinY`, `TxtWidth`/
`TxtHeight` (`= Width*1`, `= Height*1`), `TxtLocPinX`/`TxtLocPinY`
(flattened to `Width`/`Height` halves), `TxtAngle` (0), the four text
block margins (0.1 inch; `TopMargin` 4 point) and the transparency cells
(`FillForegndTrans`, `FillBkgndTrans`, `LineColorTrans`,
`ShdwForegndTrans`, `ColorTrans`: 0). A formula default evaluates one
multiplication from the named cell's cached value (0 when it has none):
no formula engine until Phase 6.

- A cell that something **defines** but that caches no value still warns
  `Missing`: it is not "absent at every rung".
- A cell with **no documented default** keeps the `Missing` warning and
  the hard default. The ShapeSheet reference documents no default for
  `FlipX`, `FlipY`, `Angle`, `Width`, `PinX`, `FillPattern`, `Size`,
  `ShapeShdwShow` and most other cells, so the warnings for those remain;
  extending the table needs another source (a black-box observation pass)
  or a CLI filter.
- `Font`'s documented default is the font *number* 0 ("the default font");
  the model cannot resolve a number to a family without `FaceNames`, and
  substituting the string `"0"` would break the renderer's theme-font
  fallback, so `Font` has no table entry yet.
