# Phase 2 clean-room audit

- Task: P2-13, role `cleanroom-auditor`
- Diff audited: `git diff 83169c9..main` (Phase 2, 123 files, +30,367)
- Date: 2026-10-14

Method: `cargo deny check` in the worktree; `git ls-files` scanned for
binary and Microsoft-owned file types across the whole tree and the whole
Phase 2 history; every committed string of eight words or more in the
Phase 2 `.rs`, `.md`, `.toml` and `.yaml` files (and every deleted line in
`git log -p 83169c9..main`, to catch committed-then-removed text) compared
as 12-word shingles against a text corpus built from `reference/` (all
Markdown, HTML, XML and XSD downloads plus the text of the Open
Specifications PDFs via `pdftotext`); the fixture bytes compared against
`crates/vsdx/fuzz/corpus/` and against the builder output test; libvisio
identifiers grepped outside `crates/vsd-binary`.

## Verdict

**Pass with findings: 0 blocker, 0 major, 2 minor.** No Microsoft content
is committed, no libvisio-derived code exists outside `crates/vsd-binary`,
all sources are recorded, and every Phase 2 asset has its attribution and
provenance rows. The two minor findings are a value-level resemblance
between the fixture shape XML and an [MS-VSDX] example, and attribution
rows that landed one commit after their assets.

## Checks that passed

1. **Licences.** `cargo deny check` is green (`advisories ok, bans ok,
   licenses ok, sources ok`). `deny.toml` allows MPL-2.0 only through the
   `larrygram-vsd-binary` exception (`deny.toml:43-45`) and checks our own
   crates (`licenses.private.ignore = false`), so the graph is
   MPL-confined as ADR 0003 requires. `crates/vsd-binary` is untouched by
   Phase 2 and its files carry the MPL-2.0 header.
2. **No Microsoft-owned files.** No images, fonts, icons, Office documents,
   stencils, templates or spec PDFs anywhere in `git ls-files`, in this
   phase or its history (added or deleted). The only committed binaries are
   the 11 hand-authored fixtures under `corpus/fixtures/` and their 11
   byte-identical copies under `crates/vsdx/fuzz/corpus/` (verified with
   `cmp`, and by the test
   `fixtures::committed_fixtures_match_the_builder_output`). No corpus
   third-party file is committed; the fetched corpus rows in
   `corpus/PROVENANCE.md` carry their MPL-2.0 licence and pinned commit.
3. **Assets and provenance.** Every Phase 2 asset has a `corpus/PROVENANCE.md`
   row added in the same commit (`f0724cb` fixtures, `6991de6` fuzz
   corpus), and an `ATTRIBUTION.md` row (see F2 for the timing).
4. **No copied long-form text.** No sentence of eight words or more in the
   Phase 2 sources matches any `reference/` source (the only shingle
   overlaps are XML markup, URLs and structural name lists; see F1). No
   tooltips, help text or descriptions were added in Phase 2 at all. The
   generated `crates/model/src/generated/shapesheet.rs` carries names,
   indices and unit enums (derived facts) and our own doc comments. The
   schema harness (`crates/vsdx/tests/schema.rs`) extracts the XSD to a
   temp file at run time and never copies its text; the oracle test
   (`crates/vsdx/tests/oracle.rs`) holds no libvisio code and only the
   parsing its comparison needs. Deleted lines in the Phase 2 history
   match no source at all (P1-11's F1 does not recur).
5. **libvisio confinement.** No libvisio-derived code outside
   `crates/vsd-binary` (Phase 2 adds none anywhere; `vsd-binary` still
   holds only the stub with MPL headers). `NOTICE` records the libvisio
   commit `c7c970775b750a4bf7fd1b4364d69bac9815bfcb`, matching
   `corpus/manifest.toml` and `docs/sources.md` sources 6 and 7. Mentions
   of "libvisio" outside the crate are URLs, the corpus manifest and the
   oracle test's prose.
6. **`reference/` untracked.** `git ls-files reference` is empty.
7. **Sources recorded.** All eight sources in `docs/sources.md` are on the
   allowed list (Microsoft Open Specifications, Microsoft Learn, Microsoft
   support, libvisio). The XSD licence finding of ADR 0017 section 6 is
   recorded (`docs/sources.md`, "XSD licence finding"), and it concludes,
   correctly, that the XSD is never committed.

## Findings

### F1 (minor) — fixture shape values coincide with the [MS-VSDX] example

`crates/vsdx/tests/common/mod.rs:444-453` (`rect_shape`) writes a shape
whose `Width=2`, `Height=1`, `LocPinX=1`, `LocPinY=0.5` and five geometry
rows (`RelMoveTo` 0,0 then `RelLineTo` (1,0), (1,1), (0,1), (0,0)) match
the example Page XML in [MS-VSDX] section 3.1.3 exactly. The values do
differ where it matters: `PinX`/`PinY` are 2,2 in the builder and 4,5.5 in
the example, and the builder omits `Angle`, `FillForegnd` and `Text` and
adds `NoFill`. The overlapping content is element and attribute markup
plus generic numbers, not prose, so it is structural fact territory, not
rule 3 text copying. But the comment "Our own values"
(`crates/vsdx/tests/common/mod.rs:442`) does not describe this overlap.

Fix options, cheapest first: note the coincidence in
`corpus/PROVENANCE.md`, or vary the geometry (say, a 3x1.5 rectangle with
different row values) so nothing matches the example. This is a hygiene
finding; no history rewrite is warranted for XML markup.

### F2 (minor) — ATTRIBUTION.md rows landed one commit after their assets

AGENTS.md section 4 requires the attribution row in the same commit as the
asset. The fixture files went in at `f0724cb` and their `ATTRIBUTION.md`
row at `d8dbd92`; the fuzz corpus copies went in at `6991de6` and the row
was widened to cover them at `1c822e5`. The rows are complete and correct
at `main`, so this is process only. The `corpus/PROVENANCE.md` rows did
land in the same commit as their assets, which is the row the clean-room
risk actually turns on.

## Not in scope

Phase 1 content (`ui-spec/`, `xtask/src/extract/`, `docs/parity/`) was
audited by P1-11 and is only touched by Phase 2 where noted in the task
board. The Phase 1 history-rewrite finding (P1-11 F1) is recorded as done
in `docs/STATUS.md`; this audit re-checked that Phase 2 adds no new
committed-then-removed source text.
