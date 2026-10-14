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
| `fixtures/minimal-drawing.vsdx` | P2-08 | MIT | Hand-authored minimal drawing (one page, one rectangle shape). Builder output (`crates/vsdx/tests/common/`). |
| `fixtures/minimal-stencil.vssx` | P2-08 | MIT | Hand-authored minimal stencil (one master, empty pages part). Builder output. |
| `fixtures/minimal-template.vstx` | P2-08 | MIT | Hand-authored minimal template (one page, one master). Builder output. |
| `fixtures/malformed-truncated-zip.vsdx` | P2-08 | MIT | Malformed: valid package cut in half. Builder output. |
| `fixtures/malformed-bad-central-directory.vsdx` | P2-08 | MIT | Malformed: central directory signature destroyed. Builder output. |
| `fixtures/malformed-no-content-types.vsdx` | P2-08 | MIT | Malformed: `[Content_Types].xml` missing. Builder output. |
| `fixtures/malformed-dangling-relationship.vsdx` | P2-08 | MIT | Malformed: relationship target `pages/missing.xml` does not exist. Builder output. |
| `fixtures/malformed-missing-main-part.vsdx` | P2-08 | MIT | Malformed: `/visio/document.xml` missing. Builder output. |
| `fixtures/malformed-huge-declared-size.vsdx` | P2-08 | MIT | Malformed: `page1.xml` declares 100 MiB uncompressed. Builder output. |
| `fixtures/malformed-deep-nesting.vsdx` | P2-08 | MIT | Malformed: 200 nested elements in `document.xml`. Builder output. |
| `fixtures/malformed-bad-content-types.vsdx` | P2-08 | MIT | Malformed: `[Content_Types].xml` is not well-formed XML. Builder output. |

Two more malformed fixtures are generated at test time and never committed:
`malformed-zip-bomb.vsdx` (8 MiB of zeros deflated; the deflate stream is not
stable across `flate2` versions) and `malformed-many-entries.vsdx` (8193
boring zip entries). Same author and license as above.

The files under `crates/vsdx/fuzz/corpus/` (task P2-09) are byte-identical
copies of every committed fixture above, used as the seed corpus of the
`read_package` fuzz target (ADR 0017 section 4). Same author and license
(MIT); `fixtures/` stays the single source of truth and a copy that drifts is
a bug.

All rows above are our own work: hand-written XML and a plain zip container
produced by `crates/vsdx/tests/common/mod.rs` (task P2-08). Element names,
content types and relationship type URIs are structural facts recorded in
`docs/vsdx-inventory.md`; no third-party bytes are included. A test
(`fixtures::committed_fixtures_match_the_builder_output`) proves every
committed file equals the builder output byte for byte.
