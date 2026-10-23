# Phase 8 plan: binary `.vsd`, `.vss` and `.vst` import

- Status: proposed (written by task P8-01, 2026-10-23); the master
  accepts it at merge
- Inputs: KICKOFF.md (Phase 8), AGENTS.md section 5, ADR 0003, ADR 0010,
  `tools/libvisio-oracle/README.md`, `corpus/PROVENANCE.md`,
  `corpus/manifest.toml`, `crates/vsdx/src/read/**`, `crates/model`, and
  the libvisio sources' structure (pinned commit in `NOTICE` and
  `docs/sources.md`)

## 1. Scope and versions

Larrygram imports binary Visio files **read-only**. The workflow is:
open a `.vsd`, `.vss` or `.vst`, edit it, and save it as `.vsdx` through
the existing writer. Writing binary `.vsd` stays out of scope
(KICKOFF.md).

**Versions in scope for the alpha: 6 and 11.** The `VisioDocument`
stream carries a one-byte version at offset `0x1A` after the magic
`"Visio (TM) Drawing\r\n\0"`:

| Version byte | Format | Alpha |
|---|---|---|
| 1–5 | the older pointer and list record layout | out of scope: `Error::UnsupportedVersion`, never a crash |
| 6 | the chunk record format (Visio 2000, 2002) | read |
| 7–10 | do not occur in real files | rejected like 1–5 |
| 11 | the chunk record format (Visio 2003); every later Visio writes this version when it saves `.vsd` | read |

All five corpus `.vsd` files are version 11 (read from their stream
headers, 2026-10-23). Versions 6 and 11 share the chunk model and
differ in the chunk header layout, some record payloads and the text
encoding, so one reader with per-version rules covers both (ADR 0069).
The version 1–5 layout is a different parser; a later phase can add it
behind `FormatVersion` without touching the mapping layer.

`.vss` (stencil) and `.vst` (template) use the same stream and record
formats. The document kind is derived from the records (a file of
stencil pages is a stencil), not from the file name (ADR 0070).

**What is read** (full mapping table in ADR 0070): the compound-file
container and its streams, pages and page properties, masters and
stencils, shapes with transforms and geometry, text with character,
paragraph and field runs (code pages included), styles, colours, fills
and lines, layers, shape data, connects and document properties.

**What is dropped with a warning** (never silently):

| Dropped | Why |
|---|---|
| Embedded OLE objects and pictures (foreign data) | no model place and out of scope (KICKOFF); rendering them is a later task |
| Hyperlinks | no model field; kept out of `Preserved` on purpose |
| Events and macro references | macros are never executed (KICKOFF) |
| Annotations, reviewers, smart tags | out of scope |
| The document font list | font names reach text runs as cells; the list itself has no model place |
| Custom label overrides of the shape-data rows | the rows themselves map; their label plumbing does not |

Everything dropped or degraded produces an `ImportWarning` (ADR 0071).
A file that cannot be structurally read at all is an `Error`, never a
panic and never a partial document with no diagnostics.

## 2. Layering inside `crates/vsd-binary`

The crate is L2 (ADR 0002) and depends only on L0/L1 crates
(`ferrogram-model`, later `ferrogram-geom` if geometry mapping needs
it). MPL-2.0, every file with the MPL header (ADR 0003). The layers:

1. **Container** (`container.rs`): the OLE compound file (MS-CFB), the
   streams, and the compressed-block decompression (ADR 0068).
2. **Chunks** (`chunks.rs`): the `VisioDocument` header, the version
   dispatch, the chunk records and their pointer and list redirection
   (ADR 0069).
3. **Domains** (`style.rs`, `pages.rs`, `shape.rs`, `text.rs`): record
   payloads into the model (ADR 0070).
4. **Pipeline** (`pipeline.rs`): the one entry point `import(bytes,
   limits) -> Result<Import, Error>`, running the layers and collecting
   the warnings (ADR 0071).

The seams between the layers are frozen in the skeleton task P8-01
wrote (`Chunk`, `Record`, `collect`, `import`), so the domain tasks run
in parallel against hand-built values.

## 3. ADRs

| ADR | Topic |
|---|---|
| 0068 | the compound container and the stream layer (own MS-CFB reader; decompression) |
| 0069 | the chunk and pointer model per version; the alpha version scope |
| 0070 | the mapping into `model`, and the unit rule |
| 0071 | error handling, warnings and limits |
| 0072 | the oracle comparison against `vsd2raw` and `vsd2xhtml` |

## 4. Order of work

Parallel work is the priority. A task starts as soon as the tasks it
depends on are merged; tasks that run at the same time own disjoint
files. Phase 8 runs beside Phase 7 after the rename boundary, in
`crates/vsd-binary` and its own test directories; it touches no Phase 7
file except through P8-09.

```
now ─┬─ Phase 6 tail: P6-17 ‖ P6-18 ─ phase report ─ tag phase-6
     ├─ P7-01 architect (merged)
     └─ P8-01 architect (this plan, the ADRs, the vsd-binary skeleton,
        the task files) ─ merge before the boundary
boundary (Phase 7 alone: R1, P7-00, R4, R5)
after the boundary, two tracks at once:
  Phase 7: wave 1 (8 workers), then P7-08, P7-09 … P7-14
  Phase 8:
    wave A (5 parallel, disjoint modules):
      P8-02 container and streams ‖ P8-03 chunks and pointers ‖
      P8-04 shapes and geometry ‖ P8-05 text ‖ P8-06 styles
    wave B (the integration point):
      P8-07 pages, masters and the pipeline (after wave A)
    wave C (2 parallel):
      P8-08 oracle harness and corpus census ‖
      P8-09 open path (also after P7-09 merges)
    wave D:
      P8-10 tests, fuzz target and golden images
    wave E (3 parallel):
      P8-11 reviewer ‖ P8-12 coverage ‖ P8-13 clean-room
    wave F:
      P8-14 fixes, phase report, tag phase-8
```

Notes:

- **Wave A codes against the frozen seams** (the P7-01 trick): each
  module is tested with hand-built `Chunk` values and byte payloads.
  The corpus round is P8-07's job, when the pipeline is assembled.
- **P8-07 is the only task that integrates.** It runs `import` over
  the five corpus `.vsd` files and owns the smoke test.
- **P8-09 waits for P7-09.** Phase 7 tasks do not touch
  `crates/commands/src/builtins/file.rs`, so the command side is safe,
  but the file dialog filter and the CLI arguments live in `crates/ui`
  and `apps/*`, which P7-09 (shell integration) edits. Run P8-09 after
  P7-09 merges; it is not parallel-safe with it.
- **The three audits** follow the Phase 6 pattern: review, coverage and
  clean-room reports under `docs/reviews/`, then one fix task.

## 5. Machine load

The user caps the total at twelve workers at once across Phase 7 and
Phase 8 (2026-10-23), and Phase 7 wave 1 takes eight. So:

- Start at most **four** Phase 8 workers beside the full Phase 7 wave
  1. Wave A has five tasks; start the large tiers first
  (P8-03 chunks, P8-04 shapes, P8-05 text) plus P8-02, and start P8-06
  when a Phase 7 slot frees.
- After Phase 7 wave 1 thins out, all of wave A may run at once.
- Later waves are small: wave C two workers, wave E three. They never
  pressure the machine.
- As in the Phase 7 plan: check for orphaned processes before blaming
  a slow worker.

## 6. Exit criteria

1. Every corpus `.vsd` file opens without a panic and produces a
   document that renders recognisably (the Phase 8 golden images).
2. The phase report includes the parity numbers against the libvisio
   oracle (`vsd2raw`, `vsd2xhtml`) per corpus file, with every
   mismatch listed (ADR 0072).
3. A `.vsd` file saved as `.vsdx` goes through the existing writer
   unchanged, and reopening the saved file gives the same model
   (round-trip test).
4. No malformed input crashes: the fuzz target runs, the limits are
   enforced, and every crash fix has a regression test.
5. The open path works end to end: the file dialog, the open command
   and the CLI accept `.vsd`, `.vss` and `.vst` read-only, and Save
   defaults to `.vsdx`.
6. The clean-room audit is clean: only derived facts are committed, the
   libvisio-derived files carry MPL headers, and `NOTICE`,
   `ATTRIBUTION.md` and `docs/sources.md` have the rows this phase
   needs (the master adds them; the tasks list them under Open issues).
