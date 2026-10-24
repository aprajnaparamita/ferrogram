# Phase 5 clean-room and licence audit

- Task: P5-14, role `cleanroom-auditor`
- Diff audited: `git diff phase-4..main` (Phase 5, 171 files, +42,133/-299)
- Date: 2026-10-20, branch `agent/P5-14`

Method: `cargo deny check` in the worktree; `git ls-files` and the diff
`--name-status` scanned for binary and Microsoft-owned file types across the
whole tree; every added line of 40 characters or more in the Phase 5 `.rs`,
`.md`, `.yaml`, `.html` and `.toml` files (and 40-character windows over each
such line) compared against a text corpus built from `reference/` (all
Markdown, TXT, HTML, XML, XSD and JSON downloads, about 8.4M characters);
libvisio identifiers grepped outside `crates/vsd-binary`; `ATTRIBUTION.md`,
`corpus/PROVENANCE.md` and `docs/sources.md` rows checked against the
committed assets and the moved fuzz seeds; the `VisFromParts`/`VisToParts`
constants in `crates/model/src/mutate.rs` checked value by value against the
recorded Microsoft Learn pages; `crates/model/src/stencils/` checked against the
diff.

## Verdict

**Pass with findings: 0 blocker, 0 major, 2 minor** (clean-room and licence
rules). No Microsoft-owned content is committed, no copied long-form text
exists anywhere in the diff, no libvisio-derived code exists outside
`crates/vsd-binary`, the routing (ADR 0045) is our own work with no libvisio
identifier or table in it, and `cargo deny check` is green with
`larrygram-vsd-binary` still the only MPL-2.0 crate. The two minor findings
are a blank line that pushes the new `docs/sources.md` row 18 out of the
sources table (F1), and an `ATTRIBUTION.md` row whose task list does not name
the Phase 5 `proptest-regressions` file (F2).

The hand-off gate for this task is **GREEN** (`cargo xtask gate --quick`:
codegen, parity, fmt, clippy, test, doc, deny, layers all PASS; the task
changes no code, so coverage, wasm and fuzz are not triggered).

## Checklist

1. **No Microsoft-owned content committed; no copied long-form text in the
   new commands' labels or tooltips.** Pass. The Phase 5 diff adds no binary
   or asset file: the only binary entries in `--name-status` are 26 pure
   renames (R100) of our own fuzz seeds into per-target directories. No
   Office document, stencil, template, icon, image, font, spec PDF or command bar
   XML anywhere in the diff. The new `ui-spec/commands.yaml` rows and the
   `add_our_commands` table in `xtask/src/extract/commands.rs` carry short
   command labels only ("Draw Connector", "Glue Begin", "Distribute
   Horizontally"), all our own wording; the new commands have no
   super-tooltips or help strings. The reference-corpus scan of every added
   40-character window found one match: the licence name "Creative Commons
   Attribution 4.0 International" in the new `docs/sources.md` row, which is
   a licence name (identical wording appears in the pre-existing rows), not
   copied prose. The ADRs 0038-0049, the plan and the task files are our own
   words (scan: 0 matches).
2. **Every new derived fact has a `docs/sources.md` row; raw downloads stayed
   in `reference/`.** Pass, with F1 (formatting). The only new transcription
   is the `VisFromParts`/`VisToParts` glue constants in
   `crates/model/src/mutate.rs`, recorded as row 18 with both Learn URLs, the
   VBA-Docs commit and the CC-BY-4.0 licence; every constant value in the
   code matches the recorded table (`visFromNone` 0 .. `visFromPin` 14,
   `visControlPoint` 100 + row index, `visToNone` 0 .. `visToAngle` 7,
   `visConnectionPoint` 100 + row index, `visConnectToError` -1), and the
   doc comments paraphrase the descriptions rather than copying them. The
   local pages exist under `reference/learn/enums/`. The `Connect`/`Connects`
   facts the phase uses were already derived and recorded in Phase 1/2
   (`docs/vsdx-inventory.md`, sources rows 3 and 4); Phase 5 needed no new
   row for them. `git ls-files reference` is empty.
3. **No libvisio-derived code outside `crates/vsd-binary`; the routing is our
   own.** Pass. The Phase 5 diff does not touch `crates/vsd-binary` at all. A
   grep for libvisio identifiers (`libvisio`, `VSD*List`, `VSDParser`,
   `VSDXCollector`, libwpg/libwpd) over the tree outside `crates/vsd-binary`
   matches only documentation that talks about the oracle and the rules. The
   new routing code (`crates/geom/src/routing.rs`, `crates/commands` glue and
   connector code) cites ADR 0045 and Microsoft enumerations, not libvisio;
   nothing resembling libvisio structure was found to flag. `NOTICE` still
   records the libvisio commit and the containment rule; the one file in
   `crates/vsd-binary/src/` keeps its MPL-2.0 header.
4. **`cargo deny` passes; every new dependency is license-clean and named.**
   Pass. `cargo deny check` output: `advisories ok, bans ok, licenses ok,
   sources ok`. The only manifest change in the phase is the
   `larrygram-model` dev-dependency of `apps/larrygram` (an internal crate,
   MIT; already recorded in the P5-10 Result as accepted by the master). No
   external crate was added, so the deny graph is unchanged apart from the
   internal edge.
5. **Every new committed asset has an `ATTRIBUTION.md` row in the same
   commit; expected none in Phase 5.** Pass, with F2. No new asset type
   appeared: the moved fuzz seeds are renames of already-attributed files,
   and commit `5de3786` updates both `ATTRIBUTION.md` and
   `corpus/PROVENANCE.md` in the same commit as the moves (rows now name
   `crates/vsdx/fuzz/corpus/read_package/` and
   `crates/render/fuzz/corpus/render_document/`). Every moved file still has
   its row: the `read_package/` set is byte-identical to `corpus/fixtures/`
   (checked file by file) and covered by the fixtures row, and the seven
   `render_document/` files are four byte-identical fixture copies plus the
   three named junk files, exactly as `corpus/PROVENANCE.md` describes. The
   one new asset file, `crates/commands/tests/undo_prop.proptest-regressions`
   (P5-11, commit `f2dfc70`), is covered by the pre-existing glob row but its
   note names no Phase 5 task: F2.

Master notes answered: the whole phase was reviewed as `git diff
phase-4..main`; the deferred findings in `docs/STATUS.md` are not re-reported
(cited only where noted in the Result); the fuzz-seed moves keep every row
(above); the diff adds no image or JSON data file; `crates/model/src/stencils/` is untouched by the phase, so
no master was added, and nothing in the diff resembles a traced Visio
stencil.

## Findings

| id | severity | file:line | rule | suggested fix |
|---|---|---|---|---|
| F1 | minor | `docs/sources.md:25` | AGENTS.md section 4: record every source in `docs/sources.md` (a row that leaves the table is not recorded visibly) | Delete the blank line between row 17 and the new row 18 so the row stays inside the Markdown table. The same defect sits at line 21 (before row 15, pre-existing, not Phase 5); fix both while there. |
| F2 | minor | `ATTRIBUTION.md:11` | AGENTS.md section 4: every committed asset file has a row in `ATTRIBUTION.md` (P4-13 F3 class) | The row `crates/*/tests/*.proptest-regressions` covers the path of `crates/commands/tests/undo_prop.proptest-regressions` (P5-11, `f2dfc70`), but its note lists only tasks P3-11 and P4-10. Add P5-11 to the note so the row stays traceable per file. |

Counts: **blocker 0, major 0, minor 2**.

## Gate output

`cargo deny check`:

```
advisories ok, bans ok, licenses ok, sources ok
```

`cargo xtask gate --quick` in `/Volumes/Dara/larrygram/worktrees/P5-14`:

```
  PASS  codegen       0.0s
  PASS  parity        0.2s
  PASS  fmt           2.0s
  PASS  clippy       70.6s
  PASS  test        580.4s
  PASS  doc          18.3s
  SKIP  coverage      0.0s  --skip/--quick
  PASS  deny          2.3s
  PASS  layers        0.1s
  SKIP  wasm          0.0s  --skip/--quick
  SKIP  fuzz          0.0s  --skip/--quick
gate: GREEN
```

Coverage, wasm and fuzz are skipped as untriggered: this task changes docs
only (`docs/reviews/phase-5-cleanroom.md`, the P5-14 Result), and AGENTS.md
section 7 runs those steps only for code, manifest or fuzz-directory
changes. `sccache` was present and used through the shared cache.
