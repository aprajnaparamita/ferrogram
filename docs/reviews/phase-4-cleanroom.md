# Phase 4 clean-room and licence audit

- Task: P4-13, role `cleanroom-auditor`
- Diff audited: `git diff phase-3..main` (Phase 4, 118 files, +32,463/-999)
- Date: 2026-10-17, branch `agent/P4-13`

Method: `cargo deny check` in the worktree; `git ls-files` scanned for
binary and Microsoft-owned file types across the whole tree; every added
line of 40 characters or more in the Phase 4 `.rs`, `.md`, `.yaml` and
`.html` files (and 40-character windows over each such line) compared
against a text corpus built from `reference/` (all Markdown, TXT, HTML,
XML and XSD downloads, about 8.0M characters) and separately against the
shared-string tables of all 190 Fluent UI `.xlsx` control-identifier
workbooks (about 4.3M characters); libvisio identifiers grepped outside
`crates/vsd-binary`; `ATTRIBUTION.md`, `corpus/PROVENANCE.md` and
`docs/sources.md` rows checked against the committed assets; the
`epaint_default_fonts` row checked against the crate's own files in the
cargo registry; the new direct dependencies checked against ADR 0030 and
the manifest diffs.

## Verdict

**Pass with findings: 0 blocker, 0 major, 1 minor** (clean-room and
licence rules). No Microsoft-owned content is committed, no copied
long-form text exists anywhere in the diff, no libvisio-derived code
exists outside `crates/vsd-binary`, the two stencils are original work
with honest `ATTRIBUTION.md` rows, the epaint default font row is
accurate, and `cargo deny check` is green with `ferrogram-vsd-binary`
still the only MPL-2.0 crate. The one minor finding is the missing
`ATTRIBUTION.md` rows for the two Phase 4 `insta` snapshot files (and,
in the same class, the new `proptest-regressions` file).

The gate run for this task is **GREEN** (all 11 steps, including
coverage, wasm and fuzz; see the Result of P4-13).

## Checks that passed

1. **Licences.** `cargo deny check` is green (`advisories ok, bans ok,
   licenses ok, sources ok`). `deny.toml` allows MPL-2.0 only through the
   `ferrogram-vsd-binary` exception and checks our own crates
   (`[licenses.private] ignore = false`), so `ferrogram-vsd-binary` is
   still the only MPL-2.0 crate in the graph (ADR 0003 as amended by
   P4-01, which records that the GUI tree adds no MPL crate). The two
   `deny.toml` additions are justified in comments and in ADR 0030: the
   `epaint_default_fonts` licence exception (per-crate, exactly the four
   licences of that crate) and advisory ignores with reasons
   (RUSTSEC-2026-0192 `ttf-parser` unmaintained, transitive via
   `winit` -> `sctk-adwaita` -> `ab_glyph`; RUSTSEC-2026-0367/0368/0369
   `zip` 8.6 via `calamine` in `xtask` only, whose `extract()` is never
   called — the runtime crates use `zip` 9). No new MPL crate was added.
2. **No Microsoft-owned files.** The Phase 4 diff adds no binary or
   asset file at all: the added files are `.rs`, `.md`, `.yaml`, `.toml`,
   `.html`, `.lock`, `.snap` and one `.proptest-regressions`. The
   committed binaries are exactly the pre-existing Phase 1-3 set (our own
   fixtures, fuzz seeds, golden PNGs, snapshots and the nine Carlito and
   Selawik fonts, all attributed in earlier phases). No Office documents,
   stencils, templates, icons, images, fonts or spec PDFs anywhere in
   `git ls-files`, added or deleted, in this phase. No corpus third-party
   file is committed.
3. **No copied long-form text.** The scan found **zero** verbatim
   matches in the whole Phase 4 diff against either reference corpus. The
   only strings of 40+ characters that overlap `reference/` are HTML
   boilerplate in `apps/ferrogram-web/index.html` (the `viewport` meta
   pattern), divider comments (`----`), and the OPC relationship content
   type identifier `vnd.openxmlformats-package.relationships`
   (`crates/vsdx/tests/zip_entry_names.rs:1`, a format identifier, not
   prose). The new `ui-spec/commands.yaml` entries carry short command
   labels ("Rulers", "Toggle Grid", "Exit", "Export", "Inspect",
   "Render", "About", "Next Page", "Previous Page", "Show Grid", "Fit to
   Window", "Visio 2016 Ribbon", "Microsoft 365 Ribbon") and Fluent IDs
   and `VisUICmds` names (source 1 and source 2 identifiers); the ribbon
   tooltips are ours by construction (`crates/ui/src/ribbon.rs:53`: "The
   tooltip text (ours, never Microsoft's)").
4. **libvisio confinement.** No libvisio identifiers, comments, struct
   names or constant tables outside `crates/vsd-binary` in the diff. The
   pre-existing mentions outside that crate are documentation and the
   dev-only oracle harness (`crates/render/tests/oracle_render.rs`,
   `crates/vsdx/tests/oracle.rs`, `xtask` corpus handling), which hold no
   libvisio code. Phase 4 touches no file in `crates/vsd-binary`.
   `NOTICE` still records the libvisio commit
   (`c7c970775b750a4bf7fd1b4364d69bac9815bfcb`).
5. **`reference/` untracked.** `git ls-files reference` is empty.
6. **Sources.** The Phase 4 diff adds no `docs/sources.md` row and needs
   none: the new data is either identifiers already covered by rows 1-2
   (Fluent control IDs, `VisUICmds` names), our own command ids and
   labels (ADR 0006 "our own commands" list), our own shortcut dispatch
   logic over the pre-existing `ui-spec/shortcuts.yaml` (source 5 rows
   from Phase 2), or original work (stencils, ADR 0034). Every source
   used is in the AGENTS.md section 4 allowed list. The Phase 3 item F2
   sources row (`DrawNURBS`, source 17) was added on main by P3-18 as
   recommended.
7. **Assets and provenance.** The two new asset-like source files are
   attributed: `crates/model/src/stencils/basic_shapes.rs` (12 masters)
   and `crates/model/src/stencils/basic_flowchart.rs` (9 masters) both
   have `ATTRIBUTION.md` rows reading "Original work: hand-authored for
   Ferrogram ... No third-party source; no Microsoft material", matching
   ADR 0034. The geometry is generated by our own helpers
   (`regular_polygon`, `star_points`, `rounded_rect`, `closed_polygon`,
   `edge_midpoints`) in `crates/model/src/stencils/mod.rs` — simple
   polygons, one ellipse, one arc, one line — nothing traced or copied
   from a Microsoft stencil. The `egui` default chrome font row names
   exactly the five `.ttf` files shipped inside
   `epaint_default_fonts` 0.37 (`egui-icons.ttf`, `emoji-icon-font.ttf`,
   `Hack-Regular.ttf`, `NotoEmoji-Regular.ttf`, `Ubuntu-Light.ttf`),
   matches that crate's declared licence
   `(MIT OR Apache-2.0) AND OFL-1.1 AND Ubuntu-font-1.0` and the
   `deny.toml` exception, and states honestly that the fonts are
   dependency-supplied and not copied into the repository. No Microsoft
   font is involved; Selawik stays in `crates/text` (Phase 3 audit).
   `crates/ui/src/icons.rs` draws placeholder marks programmatically
   (a rounded square plus the id's first letter); no icon set is
   committed, so no icon rows are required (ADR 0030).
8. **New dependencies.** The GUI tree (`egui`, `eframe`, `egui_kittest`,
   `rfd`, `winit`, `serde_json`, `wasm-bindgen`, `wasm-bindgen-futures`,
   `js-sys`, `web-sys` and their transitive tree: winit/glutin/wayland/
   x11, accesskit, epaint, glow, arboard, rfd's dialog backends, the
   `windows`/`objc2` platform crates) is exactly the ADR 0030 table and
   is justified there, all MIT/Apache-2.0 permissive except the
   `epaint_default_fonts` exception above. The other direct additions
   reuse crates already in the tree: `serde-saphyr` (already the ui-spec
   YAML parser) for `commands.yaml` loading in `crates/commands`
   (ADR 0028), `serde`, `proptest` and `png` as dev-dependencies, the
   `zip` 8.6 -> 9 upgrade in `crates/vsdx`/`xtask`, and the
   `libfuzzer-sys` fuzz crate `ferrogram-ui-fuzz` (dev-only). `cargo
   deny` covers them all. Note: `wgpu`/`naga` appear in `Cargo.lock`
   because eframe's optional dependencies are recorded in the lockfile
   even with the `wgpu` feature off; they are not built (ADR 0030's
   `glow` decision) and their licences pass `cargo deny` regardless.

## Findings

### F1 (minor) — Phase 4 test files have no `ATTRIBUTION.md` rows

`ATTRIBUTION.md:1` (header: "Every committed asset (fonts, icons,
images, stencils, test files) has a row here") is not satisfied by three
Phase 4 files:

- `crates/ui/src/snapshots/ferrogram_ui__ribbon__tests__layout_counts.snap:1`
- `crates/ui/tests/phase4/snapshots/phase4__ribbon__phase4_placement_counts.snap:1`
- `crates/commands/tests/undo_properties.proptest-regressions:1`

All three are original work (generated by our own tests; the snapshots
hold ribbon placement counts, the regression file holds proptest
cases), so this is a bookkeeping gap, not a provenance problem. Phase 3
set the stricter precedent: its three `insta` snapshots have an
`ATTRIBUTION.md` row (`crates/render/tests/snapshots/*.snap`). The
`proptest-regressions` class is weaker precedent — Phase 3's
`crates/text/tests/layout_properties.proptest-regressions` has no row
either. Fix: add one row per `.snap` file (or a glob row for
`crates/ui/**/snapshots/*.snap`) and decide once whether
`proptest-regressions` files are assets; record the decision in
`ATTRIBUTION.md`. Docs-only; any fix task or the master can do it.

## Notes (not findings)

- **`docs/sources.md` unchanged in Phase 4** is correct: no new source
  was consulted (check 6). The P3-14 F2/F3 documentation items were
  closed on main by P3-18 before this audit.
- **`deny.toml` runs with `all-features = true`**, so the advisory and
  licence checks also see eframe's unused optional `wgpu` path; the
  checks pass for it as well.
