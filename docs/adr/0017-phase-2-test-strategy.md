# 0017: Phase 2 test strategy

- Status: accepted
- Date: 2026-10-13
- Task: P2-01

## Context

The Phase 2 exit criterion is: every corpus `.vsdx` file round-trips without
loss and without panics. Around it KICKOFF requires property tests,
malformed-input tests, a fuzz target for the package reader, an oracle
comparison against libvisio's raw dump, and schema validation of written
files. Everything must respect ADR 0005 (corpus fetched, not committed) and
the clean-room rules.

## Decision

### 1. Round-trip tests (the exit criterion)

For each `corpus/vsdx/*.vsdx` (15 files, `corpus/manifest.toml`):

1. `read_document` -> `write_document` -> re-read: every XML part of the
   written package is **semantically equivalent** to the source (ADR 0012
   rule table; `model::raw::diff` is the definition), the part lists, content
   types and relationships match, and binary parts are byte-equal.
2. Second write is byte-identical to the first (writer stability).
3. Nothing panics; every failure is an `Err` with a message.

Tests skip with a clear message when `corpus/vsdx/` is absent; in CI,
`LARRYGRAM_REQUIRE_CORPUS=1` turns a missing corpus into failure (ADR 0005).

### 2. Property tests (`proptest`)

- Random `Package` trees round-trip through `write_package` ->
  `read_package` -> `write_package` unchanged (semantic equivalence).
- Random model documents round-trip through `from_model` -> `to_model`.
- Random `XmlNode` trees survive parse -> serialize -> parse with `diff`
  empty (the preservation tree is faithful).

Generators are bounded (depth, element count, attribute count) so the tests
stay fast; the caps below are what production code enforces.

### 3. Malformed input and caps

Malformed-input tests feed the package reader truncated zips, bad content
types, dangling relationships, huge declared sizes and deeply nested XML, and
assert an `Err` and no panic. The caps (`vsdx::Limits`, enforced before
allocation, ADR 0014):

| Cap | Value |
|---|---|
| `max_entries` | 8192 zip entries |
| `max_part_uncompressed` | 64 MiB per part |
| `max_total_uncompressed` | 512 MiB per package |
| `max_compression_ratio` | 200:1 per entry (zip bomb) |
| `max_xml_depth` | 128 element levels |
| `max_attributes_per_element` | 256 |

Checked arithmetic and capped `Vec` allocation apply to every input-derived
size (AGENTS.md section 6). Malformed-input tests exercise each cap.

### 4. Fuzz target

`crates/vsdx/fuzz/fuzz_targets/read_package.rs`, edition 2024 (P1-08
deferred item), cargo-fuzz (libFuzzer). The target calls `read_document` with
default `Limits` on the raw bytes and asserts no panic; if it reads, it also
round-trips to catch write panics. The gate's `fuzz` step builds it
(`cargo +nightly fuzz build`); seed corpus committed under
`crates/vsdx/fuzz/corpus/` (small, hand-authored).

### 5. Oracle comparison (libvisio, ADR 0010)

Dev-only tests behind `LARRYGRAM_LIBVISIO_ORACLE`; skipped with a message
when unset, mandatory with `LARRYGRAM_REQUIRE_ORACLE=1`. They compare our
parse of each corpus file against `vsd2raw`'s dump: page count, master
count, shape count per page, and a few key cell values (`PinX`, `PinY`,
`Width`, `Height`). Where the oracle does not cover a format (libvisio's
`.vsdx` support is not its purpose), the test records the skip and falls back
to the round-trip corpus. **No libvisio code and no copied output format
specification lives in these tests**; they consume the CLI's stdout as an
independent opinion. The test file notes its oracle-only nature.

### 6. Schema validation of written files

Rust has no mature XSD validator, so validation is a dev-only test that
shells out to `xmllint --schema`:

- The XSD is **not committed**. The test extracts the XSD from
  `reference/ms-vsdx/office-developer-client-docs/docs/visio/schema-mapvisio-xml.md`
  (a fenced code block) into a temp file at test time. `reference/` is
  gitignored (AGENTS.md section 4); the extractor's clean-room rules apply
  (no annotation text leaves the source).
- The test **skips with a message** when `reference/` is missing or
  `xmllint` is not installed; CI installs `libxml2` and sets
  `LARRYGRAM_REQUIRE_SCHEMA=1` to make it mandatory.
- What is validated: each part the model regenerates (`document.xml`,
  `pages/pages.xml`, `page{N}.xml`, `masters/masters.xml`, `master{N}.xml`)
  after a corpus round-trip.
- **Licence note (to record at implementation, P2-09):** the docs repo
  `MicrosoftDocs/office-developer-client-docs` states its licence in its
  `LICENSE` file at the pinned commit; P2-09 checks it and records the
  finding in `docs/sources.md`. Until that check is recorded, the XSD stays
  out of git. `[MS-VSDX].pdf` (Open Specifications) is likewise never
  committed.

### 7. Hand-authored fixtures

- Location: `corpus/fixtures/` (committed) for stable minimal/malformed
  files; test helpers under `crates/vsdx/tests/common/` generate variants at
  run time.
- Provenance: every committed fixture has a `corpus/PROVENANCE.md` row
  ("hand-authored, our own work, MIT"), added in the same commit (AGENTS.md
  section 4).
- Content: a minimal drawing (one page, one shape), a minimal stencil, a
  minimal template (ADR 0016), and one malformed file per failure mode
  (truncated zip, dangling relationship, missing `document.xml`, huge
  declared size, deep nesting, bad content types).

## Consequences

- The gate runs everything except the oracle and schema tests, which skip
  cleanly locally and are mandatory in CI once CI exists (Phase 1 known
  issue: there is no remote yet).
- The round-trip test is the exit criterion's single point of truth; its
  failure output is the `diff` path list.
- Caps live in one `Limits` struct; changing them changes one constant and
  one ADR row.

## Alternatives considered

- **Validate with a Rust XSD validator** (`xdr`, `xsd-validator`...): none is
  mature or maintained; rejected. `xmllint` is the reference implementation.
- **Commit the XSD for offline validation.** Rejected pending the licence
  check (AGENTS.md section 4: no spec documents in the repo); `reference/`
  is the standing home and the extractor rule ("names and structure only")
  keeps us clean.
- **Byte-level round-trip equality.** Rejected (ADR 0012).
- **Oracle tests mandatory in the default `cargo test` run.** Rejected
  (ADR 0010): libvisio's build is heavy and platform-specific; the oracle is
  opt-in locally, mandatory in CI.
- **A single fuzz target over the XML reader instead of the package
  reader.** Rejected: the package reader is the entry point for untrusted
  bytes; the XML reader is covered through it.
