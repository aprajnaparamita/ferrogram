# Phase 6 clean-room and licence audit

- Task: P6-14, role `cleanroom-auditor`
- Diff audited: `git diff phase-5..main` (`f368b02..4112a26`), the Phase 6
  part = 133 files, +29,305/-547; the interleaved P7-01 import (317
  files: brand assets, Tabler icons, `ui-spec` layouts) is named where it
  touches a shared file and is otherwise left to P7-13
- Date: 2026-10-23, branch `agent/P6-14`

Method: `cargo deny check` in the worktree; `git ls-files` and the diff
`--name-status` scanned for binary and Microsoft-owned file types across
the whole tree; every added line of the Phase 6 `.rs`, `.md`, `.yaml` and
`.toml` files compared against a text corpus built from `reference/`
(the ShapeSheet reference's formula and 209 function pages, the
VBA/automation reference and the enumeration pages: 4 004 Markdown
files, about 7.8M characters) with 8-word and 6-word shingle matching; the function table
(`crates/shapesheet/src/functions.rs`) and the new panels
(`crates/ui/src/panels.rs`) read string by string; libvisio identifiers
grepped outside `crates/vsd-binary`; `ATTRIBUTION.md`,
`corpus/PROVENANCE.md` and `docs/sources.md` rows checked against the
committed files and the two new fuzz seed directories; the corpus formula
census numbers in ADRs 0053-0054 checked against source 20's recording.

## Verdict

**Pass with findings: 0 blocker, 0 major, 5 minor** in the Phase 6 diff
(clean-room and licence rules), plus **1 minor outside it** (in the P7-01
import, handed to P7-13). No Microsoft-owned content is committed, no
copied long-form text exists in the function table or the panels, no
libvisio-derived code exists outside `crates/vsd-binary`, and
`cargo deny check` is green with `ferrogram-vsd-binary` still the only
MPL-2.0 crate. The five minor findings are two verbatim error-value
glosses (F1), the reference's example-column strings used verbatim as
fixtures and fuzz seeds whose `ATTRIBUTION.md` row calls them "our own
bytes" (F2), two provenance rows that do not name the new derived facts
(F3, F4) and an `ATTRIBUTION.md` note that does not name the Phase 6
`proptest-regressions` files (F5). The outside-diff finding F6 is the
blank line that pushes `docs/sources.md` rows 21-23 out of the sources
table (P7-01; the same defect was P5-14's F1).

The hand-off gate for this task is **GREEN** (`cargo xtask gate --quick`:
codegen, parity, fmt, clippy, test, doc, deny, layers all PASS; the task
changes no code, so coverage, wasm and fuzz are not triggered).

## Findings

| Id | Severity | Location | Rule | Suggested fix |
|---|---|---|---|---|
| F1 | minor | `crates/shapesheet/src/value.rs:54`, `:56` | AGENTS.md 4: write our own descriptions | Two `ErrorCode` glosses are verbatim table-cell text of `about-error-values.md` ("an argument or operand of the wrong type", "a reference to a cell that does not exist"). The other four glosses are already reworded. Reword these two, e.g. "a wrongly typed operand or argument" and "names a cell that is gone". |
| F2 | minor | `crates/shapesheet/fuzz/corpus/parse/reference-forms:1`, `crates/shapesheet/fuzz/corpus/eval_source/reference-forms:1`; also `crates/shapesheet/tests/parser.rs:96-112`, `crates/shapesheet/src/printer.rs:462-532`, `crates/shapesheet/src/parser.rs:872-892`, `docs/adr/0051-formula-grammar-ast-and-printer.md:65`, `docs/adr/0054-reference-resolution-and-inheritance.md:78-86`, `docs/tasks/P6-05.md:94-100` | AGENTS.md 4: commit derived facts, not quoted source; `ATTRIBUTION.md` must state the true source | The whole example column of `about-cell-references.md` (`Star!Angle`, `Executive.2!Height`, `Char.Font[3]`, `Scratch.A5`, `Sheet.8!FillForegnd`, `Masters[Gear]!Shaft!Geometry1.X1`, `ThePage!User.Vanishing_Point`, `Pages[Page-3]!Sheet.4!BeginX`, `Styles!Manager!LineColor`, `TheDoc!PreviewQuality`, `'1-D'!LineColor`) is committed verbatim as test fixtures, doc examples and fuzz seeds, while the seeds' `ATTRIBUTION.md` row says "Our own bytes, no third-party source". These are short syntax examples, not prose, and the reference forms themselves are recorded as facts (source 19), so nothing here is long-form text; but the bytes are the source's, not ours. Either rename the examples to our own (`Masters[Bearing]!Hub!...`, `Pages[Sheet-3]!...`) or keep them and correct the row: "example strings assembled from `about-cell-references.md` (source 19)". |
| F3 | minor | `crates/vsdx/src/read/value.rs:4`, `:19-20`, `:76`; `crates/vsdx/src/write/mod.rs:405-414`; `docs/sources.md` row 3 | AGENTS.md 4: every new fact from a Microsoft source has a `docs/sources.md` row | P6-08 and P6-16 commit new [MS-VSDX]-derived facts (a `U` spelling for every value kind, from the `U attribute value` table 2.2.5.3.3.1 and the Cell `V`/`U`/`E` rule 2.2.11.2.2.2; the contents-`Rel` id recovery rule), but source 3's committed-derived-data cell still says only "Schema element/type name tables, per-cell ShapeSheet reference, OPC ↔ Visio XML relationship". Extend source 3's cell (or add a row) to name those tables and the contents-`Rel` rule. The `LIST` spelling is correctly marked as our own in the code. |
| F5 | minor | `ATTRIBUTION.md`, the `crates/*/tests/*.proptest-regressions` row | AGENTS.md 4: every committed asset has a row, with the source named | The row's glob covers the two new Phase 6 files (`crates/shapesheet/tests/parser.proptest-regressions`, commit `178013c`; `crates/vsdx/tests/properties.proptest-regressions`, commit `749c4aa`) but its note names only P3-11, P4-10 and P5-11, and neither commit touches `ATTRIBUTION.md`. Add the P6 task ids to the note (same fix as P5-14's F2). |
| F6 | minor | `docs/sources.md:27` (outside the Phase 6 diff: P7-01, commit `00b7aba`) | AGENTS.md 4 / P5-14 F1 precedent | A blank line splits the sources table after row 20, so rows 21-23 (Tabler, the brand artwork, the photograph) fall out of the table in every renderer. Delete the blank line. For P7-13 or a master fix. |

## Checklist

1. **No Microsoft-owned content committed; no copied long-form text (function
   descriptions, help strings) in the function table or the panels; the
   grammar facts are derived facts with `docs/sources.md` rows, never quoted
   prose.** Pass, with F1 and F2. The Phase 6 diff adds no binary or asset
   file at all: the only binary entries in `--name-status` are the fuzz
   seed `malformed-junk-bytes` (9 hand-written junk bytes). No Office
   document, stencil, template, icon, image, font, spec PDF, ribbon XML or
   Visio screenshot anywhere in the diff. The function table
   (`crates/shapesheet/src/functions.rs`) holds names, arities and our own
   `FunctionKind` categories and **no description text at all**; its module
   docs state that names and arities come from sources 4/19 and "the
   descriptions are our own", and the two rows the reference does not
   spell (`MOD`, `EXP`) say so in their comments. The evaluator's
   per-function methods carry no copied descriptions (shingle scan over
   `crates/shapesheet/src/eval.rs`: 0 matches). The new panels
   (`crates/ui/src/panels.rs`) carry short labels only ("Size & Position",
   "Shape Data", "Pin X", "Loc Pin X", "Unit cost") plus our own one-line
   notes ("This cell carries a formula: read-only in this release"); scan:
   0 matches. The grammar facts in ADRs 0051-0054 (operator precedence,
   the `_GT_` family, reference forms, unit spellings, the six error
   values) are our own prose with the source recorded as row 19; the
   corpus census numbers are recorded as row 20. Two 7-8 word error-value
   glosses are quoted text (F1) and the source's example column is
   reproduced as fixtures (F2), the only verbatim matches in the phase.
   The unit spelling lists (`SQ IN`, `SQ. IN.`, `IN^2`, `FEET/INCH`, ...) are
   data (the spellings themselves are the fact, row 19), and the
   relationship type URI in `crates/vsdx/src/write/mod.rs` is an
   the separate black-box check) records structure and short labels
   only and keeps the screenshot private (F4 for its provenance row).
2. **Every new fact derived from a Microsoft source has a
   `docs/sources.md` row; raw downloads stayed in `reference/`.** Pass,
   with F3 and F4. Rows 19 (the ShapeSheet reference's formula pages and
   function pages) and 20 (the corpus formula census) were added in the
   same commit as the ADRs that use them (`4990003`) and cover the phase's
   main transcriptions: operator set and precedence, reference forms,
   unit spellings, error values, function names and arities, the census
   counts. `git ls-files reference` is empty and no diff path touches
   `reference/`. The two gaps are the [MS-VSDX] `U`/`Rel` facts (F3) and
3. **No libvisio-derived code anywhere (Phase 6 touches no `vsd`
   code).** Pass. The Phase 6 diff does not touch `crates/vsd-binary`; a
   grep for libvisio identifiers over the diff matches only the two
   mentions of the rule in this task's own file. A tree-wide grep outside
   `crates/vsd-binary` and `tools/libvisio-oracle` matches documentation
   about the oracle and `crates/render/tests/oracle_render.rs` (the
   dev-only oracle comparison of P3-10, behind `FERROGRAM_LIBVISIO_ORACLE`,
   holding counts and no libvisio code), plus one pre-existing comment in
   `crates/geom/src/curve.rs`. The `VisioDocument` hits are the [MS-VSDX]
   root element name. `NOTICE` still records the libvisio commit.
4. **`cargo deny` passes; every new dependency is license-clean and named
   (expected: none).** Pass. `cargo deny check` output (worktree):
   `advisories ok, bans ok, licenses ok, sources ok` (one
   `multiple-versions` **warning** for `zip` 8.6 via `calamine` in
   `xtask`, pre-existing and allowed by `deny.toml`). The only manifest
   changes in the phase are internal edges (`ferrogram-shapesheet` into
   `commands`/`ui`, `ferrogram-geom`/`ferrogram-model` into
   `shapesheet`), the `proptest` dev-dependency of `crates/shapesheet` and
   `serde` in `crates/ui`, both already in the workspace lock, and the
   standalone fuzz crate (`libfuzzer-sys`, same pattern as the existing
   fuzz crates, its own lockfile). No new external crate, as expected.
5. **Every new committed asset has an `ATTRIBUTION.md` row in the same
   commit (expected: none in Phase 6).** Pass, with F5. Phase 6 adds no
   new asset *type*: the two new fuzz seed directories get their rows in
   the same commits that add the seeds (`cf0d16e` for
   `crates/shapesheet/fuzz/corpus/parse/*`, `461e170` for
   `crates/shapesheet/fuzz/corpus/eval_source/*`), and the row wording is
   accurate for every seed except the two `reference-forms` seeds (F2).
   The master's `7244bce` removed the 541 libFuzzer-generated inputs that
   `461e170` had brought in beside the named seeds; only the named,
   hand-written seeds are committed now. No `corpus/` file was added, so
   `corpus/PROVENANCE.md` needed no row. The two `proptest-regressions`
   files sit under the existing class row but its note does not name them
   (F5). The P7-01 assets on the branch (245 Tabler SVGs, 5 brand files,
   their licences) each have their rows and sources entries; they belong
   to P7-13's audit.

## Gate

`cargo xtask gate --quick` in `/Volumes/Dara/ferrogram/worktrees/P6-14`:

```
==> gate summary
  PASS  codegen       0.0s
  PASS  parity        0.3s
  PASS  fmt           6.0s
  PASS  clippy      106.4s
  PASS  test       1888.3s
  PASS  doc          31.7s
  SKIP  coverage      0.0s  --skip/--quick
  PASS  deny          2.2s
  PASS  layers        0.1s
  SKIP  wasm          0.0s  --skip/--quick
  SKIP  fuzz          0.0s  --skip/--quick
gate: GREEN
```

The task changes no code (this report and the task file only), so
coverage, wasm and fuzz are not triggered (AGENTS.md section 7).
