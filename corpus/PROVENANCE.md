# Corpus provenance

Third-party test files are **not committed** (ADR 0005). They are listed in `corpus/manifest.toml` with upstream path, pinned commit and sha256, and fetched into `corpus/vsd/` and `corpus/vsdx/` (gitignored) by `cargo xtask corpus`.

Hand-authored fixtures we create ourselves are committed under `corpus/fixtures/` and listed in the second table.

## Fetched files

All from libvisio `src/test/data/` at commit `c7c970775b750a4bf7fd1b4364d69bac9815bfcb`, licensed MPL-2.0 as part of the libvisio source tree. Many embed master shapes that were created from Microsoft templates, which is why they are fetched, not committed.

| Name | Notes |
|---|---|
| `vsd/dwg.vsd` | Small generic .vsd |
| `vsd/fdo86729-ms1252.vsd` | Codepage MS-1252 text (fdo#86729) |
| `vsd/fdo86729-utf8.vsd` | UTF-8 text (fdo#86729) |
| `vsd/no-bgcolor.vsd` | No background colour |
| `vsd/tdf76829-numeric-format.vsd` | Numeric text-field formats (tdf#76829) |
| `vsdx/bgcolor.vsdx` | Background colour |
| `vsdx/blue-box.vsdx` | Single rectangle |
| `vsdx/color-boxes.vsdx` | Multiple fill colours |
| `vsdx/dwg.vsdx` | Drawing shape data |
| `vsdx/fdo86664.vsdx` | fdo#86664 |
| `vsdx/qs-box.vsdx` | Quick-style boxes |
| `vsdx/recursion-cycle.vsdx` | Malformed: recursion cycle |
| `vsdx/tab-short-prefix.vsdx` | Malformed: short tab prefix |
| `vsdx/tdf136564-WhiteTextBackground.vsdx` | tdf#136564 |
| `vsdx/tdf154379-QuickStyleFillMatrix.vsdx` | tdf#154379, quick-style fill matrix |
| `vsdx/testfile1.vsdx` | Generic, with masters |
| `vsdx/testfile3.vsdx` | Variant |
| `vsdx/testfile4.vsdx` | Variant |
| `vsdx/testfile5.vsdx` | Variant |
| `vsdx/testfile6.vsdx` | Variant |

Not listed (kept in `reference/corpus-candidates/` only): the larger libvisio files (`bitmaps*.vsd`, `Visio*PlanWithDimensions.vsd` and text-field sets, `tdf154379-DrawingUnits-type.vsd`, `tdf76829-datetime-format.vsd`, `office_varient4.vsdx`) and all LibreOffice core test files (license unclear, commit not pinned). They can be added to the manifest later with a pinned commit.

## Committed hand-authored fixtures

| Path | Author | License | Notes |
|---|---|---|---|
| _(none yet)_ | | | |
