# Sources

Committed provenance for every external source Ferrogram uses. The local-only copy with file paths is `reference/SOURCES.md`. Raw downloads are never committed; we commit only derived facts and our own summaries.

| # | Source | URL | Retrieved | Version / commit | License / terms | Committed derived data |
|---|---|---|---|---|---|---|
| 1 | Office Fluent UI control identifiers (ribbon XML IDs) | https://github.com/OfficeDev/office-fluent-ui-command-identifiers | 2026-10-12 | `b230a0df45036b2d5e8b49ebf368b3f90ada8d63` (2024-03-18) | MIT (Microsoft Corporation) | Command IDs, control types, ribbon tree (parent chain), policy IDs |
| 2 | VisUICmds + other Visio enumerations (VBA reference) | https://learn.microsoft.com/en-us/office/vba/api/visio.visuicmds and https://github.com/MicrosoftDocs/VBA-Docs/tree/main/api | 2026-10-12 | `c906726ead4e312b08ec6dd3043d6fb896b8cbc3` (VisUICmds.md, 2024-01-24) | Creative Commons Attribution 4.0 International | Enumeration name + value, description; 679 `visCmdXxx` table rows (664 match the `commands.yaml` pattern; 15 with underscores are skipped); 112 enumerations kept for reference |
| 3 | [MS-VSDX] Visio file format + adjacent Microsoft Open Specifications | https://learn.microsoft.com/en-us/openspecs/sharepoint_protocols/ms-vsdx/ | 2026-10-12 | Office Open Specifications PDFs dated 2025-05-19 (the canonical 2026 H1 release) | Microsoft Open Specifications Promise | Schema element/type name tables, per-cell ShapeSheet reference, OPC ↔ Visio XML relationship |
| 4 | ShapeSheet reference (functions, cells, sections, rows) | https://learn.microsoft.com/en-us/office/client-developer/visio/ (sourced from https://github.com/MicrosoftDocs/office-developer-client-docs) | 2026-10-12 | `f8c5dfed613fafd2fb8c8f9b1e43454f29517831` (2024-04-12) | Creative Commons Attribution 4.0 International | Cell name → section mapping; function name + arg count + arg types; cell type/units; section type enumerations |
| 5 | Keyboard shortcuts for Visio (Microsoft Support) | https://support.microsoft.com/en-us/office/keyboard-shortcuts-for-visio-ee952f31-7e3e-4564-8116-f3ecbb733cc1 (live page now redirects; archived snapshot at https://web.archive.org/web/20251205120214id_/https://support.microsoft.com/en-us/office/keyboard-shortcuts-for-visio-ee952f31-7e3e-4564-8116-f3ecbb733cc1) | 2026-10-12 | Web snapshot 2025-12-05; live page retired 2026 | Microsoft Support terms (informal) | Shortcut key combinations mapped to Visio commands (cross-referenced to source 2) |
| 6 | libvisio (binary .vsd reference) | https://git.libreoffice.org/libvisio | 2026-10-12 | `c7c970775b750a4bf7fd1b4364d69bac9815bfcb` (2026-10-09) | Mozilla Public License Version 2.0 (verified from `COPYING.MPL`; recorded in `NOTICE`) | Commit hash, CLI command list (`vsd2raw`, `vss2raw`, `vsd2xhtml`, `vss2xhtml`, `vsd2text`, `vss2text`), file inventory of test data; **never libvisio source itself** — only `crates/vsd-binary` (MPL-2.0) will hold ported code, in a later task |
| 7 | Visio test corpus (libvisio test data) | https://git.libreoffice.org/libvisio (src/test/data/) | 2026-10-12 | libvisio `c7c970775b750a4bf7fd1b4364d69bac9815bfcb` | MPL-2.0 | Nothing: files are fetched and sha256-verified from the pinned commit (`corpus/manifest.toml`, ADR 0005). LibreOffice core test files were dropped (license unclear, commit not pinned). |
| 8 | Visio XML schema map (the XSD embedded in `docs/visio/schema-mapvisio-xml.md`) | https://github.com/MicrosoftDocs/office-developer-client-docs/blob/f8c5dfed613fafd2fb8c8f9b1e43454f29517831/docs/visio/schema-mapvisio-xml.md | 2026-10-14 | `f8c5dfed613fafd2fb8c8f9b1e43454f29517831` (2024-04-12); the file is byte-identical at the local clone's `eedbd61ca40689e7cfed5e1cfd9440a9dc3ab7a5` | Page: Creative Commons Attribution 4.0 (repository `LICENSE`); code samples: MIT (repository `LICENSE-CODE`); the XSD block carries its own Microsoft notice permitting copy, display and distribution with attribution (Copyright (c) Microsoft Corporation) | The licence finding in the note below and the test-time extractor (`crates/vsdx/tests/schema.rs`); **the XSD itself is never committed** |
| 9 | Visual style enumerations and per-cell value tables (P3-08) | https://learn.microsoft.com/en-us/office/vba/api/visio.viscellvals and [MS-VSDX] (source 3) | 2026-10-14 | `Visio.viscellvals.md` from the source 2 clone; [MS-VSDX] v20250520 | CC-BY-4.0 (VBA docs); Microsoft Open Specifications Promise | Fill/shadow pattern names + indices 0..24 (`visNoFill`..`visThinDiagonalCross`) and arrow size classes 0..6; index ranges and meanings for `LinePattern`, `BeginArrow`/`EndArrow`, `FillPattern`, `ShdwPattern`; dash-rhythm proportions *measured from the spec's table figures* and re-expressed in our own numbers in `crates/render/data/visual-styles.yaml` (the figures themselves are never committed) |
| 10 | Observed behavior of Visio as a black box (P3-08) | — (AGENTS.md section 4: black-box observation) | 2026-10-14 | corpus `vsdx/` (15 files), inspected as file data; no Visio binary touched | Observed behavior; nothing Microsoft-owned copied | Corpus census of visual style indices (`LinePattern` 0/1/23, `BeginArrow`/`EndArrow` 0, `FillPattern` 0/1/24, `ShdwPattern` 0); per-entry `note` intent descriptions in `visual-styles.yaml`. No Visio screenshots were taken; the screenshot pass refines the entries |
| 11 | Carlito font (Calibri substitute) | https://github.com/google/fonts/tree/main/ofl/carlito | 2026-10-14 | Directory at `3dd78844021e948ceb633d1dcee3f7885561b5d9` (last commit touching `ofl/carlito`, 2026-03-03) | SIL Open Font License 1.1 (Copyright 2013 The Carlito Project Authors; Reserved Font Name "Carlito"); licence text committed as `crates/text/fonts/Carlito-OFL.txt` | The four faces and their licence, committed as project assets under `crates/text/fonts/` (AGENTS.md section 4, `ATTRIBUTION.md` rows); font facts used in tests (upem 2048, line metrics ascent 1950 / descent 550 per 2048 em) |
| 12 | Selawik font (Segoe UI substitute) | https://github.com/microsoft/Selawik (release `1.01`, asset `Selawik_Release.zip`) | 2026-10-14 | Release `1.01` (2015-12-08); licence from `microsoft/Selawik` `LICENSE.txt` at `89362e84731d1f5777fa078fd4d5ebbd339e4378` (2023-06-05) | SIL Open Font License 1.1 (Copyright 2015, Microsoft Corporation; Reserved Font Name Selawik); licence text committed as `crates/text/fonts/Selawik-OFL.txt` | The five released faces (Light, Semilight, Regular, Semibold, Bold) and their licence, committed as project assets under `crates/text/fonts/` (`ATTRIBUTION.md` rows). The release ships no italics; see the ADR 0024 amendment |
| 13 | VisDefaultColors enumeration (Visio) | https://learn.microsoft.com/en-us/office/vba/api/visio.visdefaultcolors (part of the source 2 batch) | 2026-10-14 | VBA-Docs commit `c906726ead4e312b08ec6dd3043d6fb896b8cbc3` | Creative Commons Attribution 4.0 International | Fixed Visio palette: the indices 0..23 and their colour names (Black, White, Red, ..., 90% gray), transcribed into `ferrogram_model::values::palette` (P3-02). The RGB values come from [MS-VSDX] section 2.2.7.6 "Color Table" (source 3; corrected on main 2026-10-15: indices 14, 16 and 18 differed from the conventional values) |
| 14 | ShapeSheet reference, Quick Style section cells | https://learn.microsoft.com/en-us/office/client-developer/visio/quickstylelinecolor-cell-quick-style-section.md (plus the `QuickStyleFillColor`, `QuickStyleShadowColor`, `QuickStyleFontColor` pages; part of the source 4 batch) | 2026-10-14 | `f8c5dfed613fafd2fb8c8f9b1e43454f29517831` (2024-04-12) | Creative Commons Attribution 4.0 International | The `QuickStyle*Color` slot meanings (0 = Dark theme colour, 1 = Light, 2..7 = Accent 1..6; `QuickStyleFontColor` 0..1 only) used by the `Themed` resolution of ADR 0021 (P3-02) |

| 15 | ShapeSheet reference, documented cell defaults (P3-15) | https://learn.microsoft.com/en-us/office/client-developer/visio/ (source 4 batch: `locpinx`/`locpiny`/`txtpinx`/`txtpiny`/`txtwidth`/`txtheight`/`txtlocpinx`/`txtlocpiny`/`txtangle`-cell-*.md, `bottommargin`/`leftmargin`/`rightmargin`/`topmargin`-cell-text-block-format-section.md, `fillforegndtrans`/`fillbkgndtrans`/`shdwforegndtrans`-cell-fill-format-section.md, `linecolortrans`-cell-line-format-section.md, `transparency`-cell-character-section.md) | 2026-10-15 | `f8c5dfed613fafd2fb8c8f9b1e43454f29517831` (2024-04-12) | Creative Commons Attribution 4.0 International | The documented-defaults table `ferrogram_model::defaults` (P3-15): `LocPinX`/`LocPinY` `= Width*0.5`/`= Height*0.5`, the `Txt*` text-transform formulas (`= Width*0.5`, `= Height*1`, `= TxtWidth*0.5`, ...), `TxtAngle` 0, the text block margins (0.1 inch, `TopMargin` 4 point) and the transparency cells 0%; used to resolve an absent cell without a warning |
| 16 | [MS-VSDX] Quick Style slices, dynamic theme variants and cell default values (P3-15), with `VisQuickStyleColors` | https://learn.microsoft.com/en-us/openspecs/sharepoint_protocols/ms-vsdx/ (source 3, sections 2.2.5.3.3.1, 2.2.7.4.3-2.2.7.4.5, 2.2.7.6 and the `QuickStyleFillColor`/`QuickStyleFillMatrix`/`VariationColorIndex`/`VariationStyleIndex` cell pages) and https://learn.microsoft.com/en-us/office/vba/api/visio.visquickstylecolors (source 2 batch) | 2026-10-15 | [MS-VSDX] v20250520; VBA-Docs commit `c906726ead4e312b08ec6dd3043d6fb896b8cbc3` | Microsoft Open Specifications Promise; CC-BY-4.0 (VBA docs) | The `QuickStyle*Color` slot table (0 `dk1`, 1 `lt1`, 2..7 `accent1..6`, 8 the theme background colour, 100..106 the variation colours `varColor1..7` of the `VariationColorIndex` scheme); `VariationColorIndex`/`VariationStyleIndex` selection (65534 defers to the page sheet); `QuickStyleFillMatrix` 1..6 and 100..103 to the `a:fmtScheme` `fillStyleLst` and the `vt:varStyle` `fillIdx` (1-based); the `vt:variationClrSchemeLst`/`vt:variationStyleSchemeLst` structures read into `Theme`; the parse-token cell default values; and the fixed colour table 0..23 RGB values (2.2.7.6), which confirm the `ferrogram_model::values::palette` transcription flagged unverified in source 13 |
| 17 | Visio VBA/automation reference, `Page.DrawNURBS` / `Shape.DrawNURBS` (P3-18) | https://learn.microsoft.com/en-us/office/vba/api/visio.page.drawnurbs and https://learn.microsoft.com/en-us/office/vba/api/visio.shape.drawnurbs (part of the source 2 batch) | 2026-10-15 | `c906726ead4e312b08ec6dd3043d6fb896b8cbc3` (Visio VBA-Docs, 2024-01-24) | Creative Commons Attribution 4.0 International | The knot-vector end-padding rule for ADR 0022 and `curve.rs` ("Knot vectors"): for `n` control points the knot array holds `n + 1` knots, and the application supplies the missing repeated knots at the tail of the conventional `n + degree + 1` sequence (worked example: the degree-2 list `0, 0, 0, 2, 5, 8` reads as `0, 0, 0, 2, 5, 8, 8, 8`); plus the repetition limits and the definition of a periodic spline (the first knot repeated fewer than `degree + 1` times), which we read as clamped (open item) |


## Open Specifications Promise (note)

The Microsoft Open Specifications used in source 3 (and the supporting
`MS-ODRAW`, `MS-OGRAPH`, `MS-OSHARED`, `MS-OWEMXML`, `MS-OVBA`,
`MS-OFFCRYPTO`, `MS-ODCFF`, `MS-VGSP` PDFs) are made available under the
Open Specifications Promise
(<https://learn.microsoft.com/en-us/openspecs/dev_center/MS-DEVCENTLP/1c24c7c8-28b0-4ce1-b96a-95fe1ff504bc>):
they may be used to implement the specifications, and short verbatim
quotes are permitted, but we commit only derived structural data (element
names, type tables, schemas of relationships), never the spec text itself.

## XSD licence finding (source 8, P2-09)

Checked 2026-10-14 at the pinned commit `f8c5dfed613fafd2fb8c8f9b1e43454f29517831`
of `MicrosoftDocs/office-developer-client-docs`:

- The repository `LICENSE` is **Creative Commons Attribution 4.0
  International** and covers the documentation pages, including
  `docs/visio/schema-mapvisio-xml.md`.
- The repository `LICENSE-CODE` is **MIT** and covers code samples in the
  documentation.
- The embedded XSD block additionally carries its own Microsoft copyright
  notice with a permission to copy, display and distribute the document,
  provided the notice is reproduced.

**May the XSD ever be committed? No.** Redistribution appears permitted with
attribution, but AGENTS.md section 4 forbids committing Microsoft-owned spec
content, and nothing needs it in git: `crates/vsdx/tests/schema.rs` extracts
the fenced block into a temp file at test time and never writes it anywhere
else. `[MS-VSDX].pdf` is likewise never committed.

## Skipped

- The Microsoft 2016 Visio UI controls download
  (`https://www.microsoft.com/en-us/download/details.aspx?id=50745`) was
  not fetched because the Fluent UI repository (source 1) already ships
  the Office 2016 Visio data.
- The current `support.microsoft.com` page for Visio keyboard shortcuts
  is a redirect stub; we use the Internet Archive snapshot listed in
  source 5.
- Several large Microsoft-generated test files in `libvisio/src/test/data/`
  (`tdf154379-DrawingUnits-type.vsd`, `bitmaps.vsd`, the
  `Visio{NN}*PlanWithDimensions.vsd` set) are kept in `reference/` but
  excluded from the committed corpus; see `corpus/PROVENANCE.md`.