# Phase 1 clean-room audit

- Task: P1-11, role `cleanroom-auditor`
- Diff audited: `git diff phase-0..main` (89 files, +54,271)
- Date: 2026-10-13

Method: `cargo deny check` on the worktree; `git ls-files` scanned for
binary and Microsoft-owned file types; every committed string of six
words or more in `ui-spec/*.yaml`, `ui-spec/*.toml`, `ui-spec/schema.json`,
`ui-spec/out-of-scope.txt`, `ui-spec/diff-2016-vs-365.md`,
`docs/vsdx-inventory.md`, `docs/parity/controls.md`, `docs/sources.md`,
ADRs 0005-0010, `docs/tasks/`, `corpus/PROVENANCE.md`, `ATTRIBUTION.md`
and the `xtask/` sources compared against a text corpus built from
`reference/` (all Markdown and HTML downloads, tag-stripped, plus the
text of the ten Open Specifications PDFs via `pdftotext`). Deleted lines
in `git log -p phase-0..main` were compared the same way to catch
committed-then-removed text.

## Findings

### F1 (major) — git history still holds 151 verbatim support-page action descriptions

`docs/tasks/P1-04.md` at commit `7097013` listed 151 action descriptions
quoted from the keyboard-shortcuts support page. The master removed them
at merge (`6b75137`), but `7097013` is reachable from `main`, so the text
is in the branch history. 108 of the deleted strings match the archived
support page verbatim (the rest differ only in case or punctuation).
Other deleted lines that match sources are just OPC MIME types in
`docs/vsdx-inventory.md` (derived facts, fine) — the P1-04 list is the
only committed-then-removed source text in the Phase 1 history.

`path: docs/tasks/P1-04.md @ 7097013` (removed at `6b75137`)

Verdict requested by the task: **history must be rewritten before the
first push.** No remote exists yet, so the fix is cheap now: run
`git filter-repo` (or an equivalent rebase) to replace the `7097013`
blob of `docs/tasks/P1-04.md` with the merged version. After the first
push the text is public forever. The current file is clean; it even
records why the list was dropped (`docs/tasks/P1-04.md:460`).

### F2 (minor) — `ACTION_TO_COMMAND` keys are verbatim support-page phrases

`xtask/src/extract/shortcuts.rs:43-195` holds 81 normalised action
phrases taken word-for-word from the "To do this" column of the support
page (STATUS.md deferred item; the table has grown from the ~61 the task
mentions). 27 keys are six words or longer; the longest is a full
sentence: "cut the selected master shape from the custom stencil and put
it on the clipboard" (`xtask/src/extract/shortcuts.rs:117-120`). The
module doc comment claims "No Microsoft prose is committed"
(`xtask/src/extract/shortcuts.rs:17-21`); with these keys and the test
fixtures below, that sentence is wrong as written.

Risk rating: **minor.** The phrases never ship (they live in the
extractor, and `shortcuts.yaml` keeps only key chords and our own
command ids), the support page carries informal Microsoft Support terms
rather than spec text, and the keys are functional row identifiers. But
they are sentence-length strings matching a source verbatim, which rule 3
tells me to flag, and they are avoidable.

Alternatives, in order of preference:

1. Key the table by a hash: `sha256(normalised phrase)` instead of the
   phrase. Matching stays exact, no source text is committed, and a
   maintainer looks phrases up in `reference/` (never tracked) when
   extending the table. A unit test can pin the count of entries so a
   silent drop is caught.
2. Match on our own unordered keyword sets (e.g. `{"rotate", "left"}` to
   `arrange.rotate_left`). Readable and ours, but rows like "select all
   text in a text block" vs "select all shapes on the page" need care.
3. Match rows by their key chord where the chord is unique, and keep the
   phrase table only for chord collisions. Smallest table, but chord
   reuse across sections makes this incomplete on its own.

If none is adopted, at minimum reword the doc comment at
`xtask/src/extract/shortcuts.rs:17-21` to state what is kept and why.

### F3 (minor) — verbatim source sentences in xtask test fixtures

Sentence-length strings copied from the ShapeSheet reference and the
support page, used as extractor test input. Tests only, never shipped,
but rule 3 covers committed strings regardless of use:

- `xtask/src/extract/shapesheet.rs:684-725` — `CELL_FIXTURE`,
  `CELL_REPEAT_FIXTURE`, `SECTION_FIXTURE` reproduce page snippets,
  e.g. "Represents the  *x*  -coordinate of the shape's pin in relation
  to the origin of its parent." and "The y-offset of the action tag
  button."
- `xtask/src/extract/shapesheet.rs:843-872` — `infer_type_unit` test
  input: six description sentences, e.g. "Determines the width of the
  bottom bevel in points."
- `xtask/src/extract/shortcuts.rs:838-856` — `FIXTURE_HTML` rows "Undo a
  recent action", "Open the submenu for a selected command.", "Alt+H, F,
  S to open the Save As dialog box"; also `xtask/src/extract/shortcuts.rs:889`,
  `:907`, `:936`, `:1000`.
- `xtask/src/extract/fixtures.rs:38-50` — `SHORTCUTS_HTML` with the same
  rows.

Risk rating: **minor.** A few sentences per fixture, developer-facing
only. Paraphrasing weakens the realism the parser tests want; the
cheapest fix is to paraphrase the prose while keeping the markup shape,
or to leave as is and note the deliberate exception in the extractor
docs.

### F4 (minor) — support-page section headings in shipped ui-spec data

Four `section:` values in `ui-spec/shortcuts.yaml` are verbatim section
headings from the support page:

- `ui-spec/shortcuts.yaml:66` — "Move from shape to shape in a drawing page"
- `ui-spec/shortcuts.yaml:151` — "Work with stencils in edit mode"
- `ui-spec/shortcuts.yaml:162` — "Work with master shapes in a stencil"
- `ui-spec/shortcuts.yaml:532` — "Move in the full-screen view mode"

Risk rating: **minor.** Short phrases (four to eight words), used as
grouping labels, in the same league as the UI labels rule 4 allows; but
they are article headings, not command names, and this data does ship.
Acceptable as-is, or rename to our own group names ("Navigation",
"Stencils", ...) when Phase 4 builds the shortcut editor.

### F5 (minor, no action) — miniature XSD fragments in test fixtures

`xtask/src/extract/vsdx_schema.rs:1385-1560` and
`xtask/src/extract/fixtures.rs:52-90` contain hand-written miniature
schemas whose individual element/attribute declaration lines are
identical to corresponding lines of the published Visio XSD (for
example `<xsd:element name="Page" type="Page_Type" minOccurs="0"
maxOccurs="unbounded" />`). These are functional schema declarations —
the derived structural data source 3 explicitly commits — not prose, and
the fixtures are documented as hand-written. The Microsoft copyright
annotation is absent and is actively tested for removal
(`FIXTURE-ANNOTATION-MARKER`). No action needed; just never paste real
schema blocks wholesale.

## Checks that passed

1. **Licences.** `cargo deny check` green (advisories, bans, licenses,
   sources). MPL-2.0 is not in the allow-list
   (`deny.toml:17-31`); the single exception is
   `ferrogram-vsd-binary` (`deny.toml:33-35`), and workspace crates are
   licence-checked (`deny.toml:37-39`), so no other MPL crate can enter
   the graph. The only duplicate in the lock file is `winnow`
   (warn-level, `multiple-versions = "warn"`).
2. **No Microsoft-owned files.** `git ls-files` contains zero binary
   files (`file` reports text/empty for all of them); no stencils,
   templates, drawings, specs, icons, images or fonts are tracked.
   `ATTRIBUTION.md`'s "(none yet)" row is accurate: there are no
   committed assets needing attribution. `corpus/manifest.toml` lists
   20 files; `corpus/PROVENANCE.md` has a row for every one and records
   the licence (MPL-2.0, libvisio pinned commit) for all of them; no
   corpus file is tracked (only the manifest and provenance).
3. **Labels.** `ui-spec/commands.yaml` has 935 labels, all short
   control names derived from the MIT-licensed Fluent UI identifier set
   (longest: seven-word noun phrases like "Data Graphic Apply After
   Linking Data"); no sentences. `LABELS` in
   `xtask/src/extract/commands.rs:184` has 79 entries, longest three
   words. `ui-spec/schema.json` descriptions are our own words.
4. **libvisio confinement.** No libvisio-derived code outside
   `crates/vsd-binary`: a workspace-wide grep for libvisio identifiers,
   CLI tool names and struct names finds only corpus source URLs, the
   oracle wrapper's binary-name list and docs. Every file in
   `crates/vsd-binary` carries the MPL-2.0 header
   (`src/lib.rs`, `tests/common/mod.rs`, `tests/oracle_smoke.rs`).
   `tools/libvisio-oracle/build.sh` only clones, configures and builds
   into `/Volumes/Dara/ferrogram/build/libvisio-oracle` (outside the
   repository); nothing from the clone is committed. `NOTICE` records
   libvisio commit `c7c970775b750a4bf7fd1b4364d69bac9815bfcb`, matching
   `build.sh` and `corpus/manifest.toml`.
5. **`reference/` untracked.** `git ls-files reference` is empty; the
   symlink is gitignored in the worktree as well.
6. **Sources.** All seven entries in `docs/sources.md` are in the
   allowed list (Microsoft Open Specifications, Microsoft Learn,
   Microsoft's Fluent UI identifier repository, Microsoft Support,
   libvisio) and each records its terms. The Wayback Machine URL is a
   mirror of the allowed Microsoft Support page, recorded as such.

## New Phase 1 dependencies and licences

| Crate | Version | License | Where |
|---|---|---|---|
| serde | 1 | MIT OR Apache-2.0 | crates/ui-spec |
| serde_json | 1 | MIT OR Apache-2.0 | crates/ui-spec, xtask |
| serde-saphyr | 1.3 | MIT OR Apache-2.0 | crates/ui-spec, xtask |
| jsonschema | 0.58 (no default features) | MIT | crates/ui-spec, xtask |
| thiserror | 2 | MIT OR Apache-2.0 | crates/ui-spec |
| proptest | 1 | MIT OR Apache-2.0 | dev: crates/ui-spec, xtask |
| sha2 | 0.10 | MIT OR Apache-2.0 | xtask |
| rustls-native-certs | 0.8 | Apache-2.0 OR ISC OR MIT | xtask |
| ureq | 3 (rustls-no-provider, _ring) | MIT OR Apache-2.0 | xtask |
| rustls | 0.23 (transitive) | Apache-2.0 OR ISC OR MIT | xtask |
| ring | 0.17 (transitive) | Apache-2.0 AND ISC | xtask |
| roxmltree | 0.21 | MIT OR Apache-2.0 | xtask |
| calamine | 0.36 | MIT | xtask |
| regex-lite | 0.1 | MIT OR Apache-2.0 | xtask |
| zip | 8.6 (no default features) | MIT | dev: xtask |

All permissive and inside the `deny.toml` allow-list. The `ureq`
`_ring` private-feature and webpki-roots licence points are the
deferred items already in `docs/STATUS.md`; I rate them no higher.

## Verdict

**Pass with findings.** The working tree is clean: no blocker, no
Microsoft-owned file, no long-form text shipped to users, MPL confined
to `crates/vsd-binary`. One major item (F1) is history-only and must be
fixed before the first push. The minor items (F2-F4) are borderline
short phrases and test-fixture sentences; F2's table should get one of
the proposed keyings before Phase 4 ships the shortcuts feature.

Counts: **blocker 0 · major 1 · minor 4.**
