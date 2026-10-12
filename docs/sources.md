# Sources

Committed provenance for every external source Ferrogram uses. The local-only copy with file paths is `reference/SOURCES.md`. Raw downloads are never committed; we commit only derived facts and our own summaries.

| # | Source | URL | Retrieved | Version / commit | License / terms | Committed derived data |
|---|---|---|---|---|---|---|
| 1 | Office Fluent UI control identifiers (ribbon XML IDs) | https://github.com/OfficeDev/office-fluent-ui-command-identifiers | 2026-10-12 | `b230a0df45036b2d5e8b49ebf368b3f90ada8d63` (2024-03-18) | MIT (Microsoft Corporation) | Command IDs, control types, ribbon tree (parent chain), policy IDs |
| 2 | VisUICmds + other Visio enumerations (VBA reference) | https://learn.microsoft.com/en-us/office/vba/api/visio.visuicmds and https://github.com/MicrosoftDocs/VBA-Docs/tree/main/api | 2026-10-12 | `c906726ead4e312b08ec6dd3043d6fb896b8cbc3` (VisUICmds.md, 2024-01-24) | Creative Commons Attribution 4.0 International | Enumeration name + value, description; 664 `visUICmdXxx` constants; 112 enumerations kept for reference |
| 3 | [MS-VSDX] Visio file format + adjacent Microsoft Open Specifications | https://learn.microsoft.com/en-us/openspecs/sharepoint_protocols/ms-vsdx/ | 2026-10-12 | Office Open Specifications PDFs dated 2025-05-19 (the canonical 2026 H1 release) | Microsoft Open Specifications Promise | Schema element/type name tables, per-cell ShapeSheet reference, OPC ↔ Visio XML relationship |
| 4 | ShapeSheet reference (functions, cells, sections, rows) | https://learn.microsoft.com/en-us/office/client-developer/visio/ (sourced from https://github.com/MicrosoftDocs/office-developer-client-docs) | 2026-10-12 | `f8c5dfed613fafd2fb8c8f9b1e43454f29517831` (2024-04-12) | Creative Commons Attribution 4.0 International | Cell name → section mapping; function name + arg count + arg types; cell type/units; section type enumerations |
| 5 | Keyboard shortcuts for Visio (Microsoft Support) | https://support.microsoft.com/en-us/office/keyboard-shortcuts-for-visio-ee952f31-7e3e-4564-8116-f3ecbb733cc1 (live page now redirects; archived snapshot at https://web.archive.org/web/20251205120214id_/https://support.microsoft.com/en-us/office/keyboard-shortcuts-for-visio-ee952f31-7e3e-4564-8116-f3ecbb733cc1) | 2026-10-12 | Web snapshot 2025-12-05; live page retired 2026 | Microsoft Support terms (informal) | Shortcut key combinations mapped to Visio commands (cross-referenced to source 2) |
| 6 | libvisio (binary .vsd reference) | https://git.libreoffice.org/libvisio | 2026-10-12 | `c7c970775b750a4bf7fd1b4364d69bac9815bfcb` (2026-10-09) | Mozilla Public License Version 2.0 (verified from `COPYING.MPL`; recorded in `NOTICE`) | Commit hash, CLI command list (`vsd2raw`, `vss2raw`, `vsd2xhtml`, `vss2xhtml`, `vsd2text`, `vss2text`), file inventory of test data; **never libvisio source itself** — only `crates/vsd-binary` (MPL-2.0) will hold ported code, in a later task |
| 7 | Visio test corpus (libvisio test data) | https://git.libreoffice.org/libvisio (src/test/data/) | 2026-10-12 | libvisio `c7c970775b750a4bf7fd1b4364d69bac9815bfcb` | MPL-2.0 | Nothing: files are fetched and sha256-verified from the pinned commit (`corpus/manifest.toml`, ADR 0005). LibreOffice core test files were dropped (license unclear, commit not pinned). |

## Open Specifications Promise (note)

The Microsoft Open Specifications used in source 3 (and the supporting
`MS-ODRAW`, `MS-OGRAPH`, `MS-OSHARED`, `MS-OWEMXML`, `MS-OVBA`,
`MS-OFFCRYPTO`, `MS-ODCFF`, `MS-VGSP` PDFs) are made available under the
Open Specifications Promise
(<https://learn.microsoft.com/en-us/openspecs/dev_center/MS-DEVCENTLP/1c24c7c8-28b0-4ce1-b96a-95fe1ff504bc>):
they may be used to implement the specifications, and short verbatim
quotes are permitted, but we commit only derived structural data (element
names, type tables, schemas of relationships), never the spec text itself.

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