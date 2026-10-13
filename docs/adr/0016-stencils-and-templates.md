# 0016: Stencils (.vssx) and templates (.vstx)

- Status: accepted
- Date: 2026-10-13
- Task: P2-01

## Context

Phase 2 reads and writes `.vsdx`, `.vssx` and `.vstx` (KICKOFF). All three
are OPC packages with the same Visio part structure. They differ in the main
document part's content type and in what a user expects from them: a stencil
is a master library, a template is a starting drawing with masters and
settings. The corpus has no `.vssx` or `.vstx` files, and Microsoft's own
stencils and templates must never be committed (AGENTS.md section 4).

## Decision

### Read/write

One code path, parameterized by `DocumentKind`:

| Kind | Extension | Main part content type | Content |
|---|---|---|---|
| `DocumentKind::Drawing` | `.vsdx` | `application/vnd.ms-visio.drawing.main+xml` | pages, masters, settings |
| `DocumentKind::Stencil` | `.vssx` | `application/vnd.ms-visio.stencil.main+xml` | masters (usually no pages), settings |
| `DocumentKind::Template` | `.vstx` | `application/vnd.ms-visio.template.main+xml` | pages, masters, settings |

- `read_document` sets `Document::kind` from the main part's content type
  (not from the file extension); an unknown main content type is
  `Error::InvalidPackage`.
- `write_document` picks the content type and the part layout from
  `Document::kind`. A stencil writes `masters.xml` + `master{N}.xml` and an
  empty or absent `pages.xml`, matching what Visio writes.
- Everything else (parts, relationships, preservation, equivalence) is
  shared, because the schemas are shared.

### Testing

Hand-authored fixtures only (AGENTS.md section 4):

- The fixture builder (task P2-08) constructs minimal `.vssx` and `.vstx`
  packages **programmatically** — our own tiny zip with our own XML — and
  writes them into `corpus/fixtures/` or directly from the test helper.
- Every committed fixture has a row in `corpus/PROVENANCE.md` recording
  "hand-authored by P2-08, our own work, MIT".
- No Microsoft stencil or template is committed or downloaded; if a real
  Microsoft file is ever needed for testing, it is fetched at test time by
  `cargo xtask corpus` from a pinned, license-clear source (ADR 0005), and
  the corpus manifest records the license.

Round-trip tests for stencils and templates use only these fixtures; the
corpus round-trip (the exit criterion) covers drawings (`.vsdx`), which is
what the corpus has.

## Consequences

- `DocumentKind` is on the model (ADR 0011) so `render`, `commands` and the
  UI can branch on it without depending on `vsdx`.
- Phase 4's "New Stencil" and "New Template" commands can create empty
  documents of the right kind with no further format work.
- Fixture-based coverage of `.vssx`/`.vstx` is necessarily smaller than the
  drawing coverage; the shared code path limits the risk.

## Alternatives considered

- **One content type for all three.** Rejected: Visio distinguishes them and
  validates the main part's content type.
- **Commit hand-authored fixtures as binary `.vssx`/`.vstx` files.** Partially
  adopted: tiny fixtures may be committed (with provenance), but the builder
  generates them so diffs are readable and no opaque blobs accumulate.
- **Obtain real Microsoft stencils for tests.** Rejected outright
  (AGENTS.md section 4: no Microsoft stencils, templates or drawings in the
  repo or its history).
- **Defer `.vssx`/`.vstx` to Phase 4.** Rejected: KICKOFF lists them in the
  Phase 2 build scope, and the format differences are content types, which
  are cheap now and expensive to retrofit later.
