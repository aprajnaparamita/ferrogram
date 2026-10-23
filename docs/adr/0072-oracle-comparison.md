# 0072: The oracle comparison against libvisio

- Status: accepted
- Date: 2026-10-23
- Task: P8-01

## Context

The exit criteria for Phase 8 require a parity report against libvisio
(KICKOFF.md). The oracle is built and pinned (ADR 0010): six CLI tools
(`vsd2raw`, `vsd2text`, `vsd2xhtml`, `vss2raw`, `vss2text`,
`vss2xhtml`) live in a dev-only build on the external drive and are
found through `LARRYGRAM_LIBVISIO_ORACLE`, with
`LARRYGRAM_REQUIRE_ORACLE=1` in CI on macOS and Linux. The corpus has
five `.vsd` files (`corpus/manifest.toml`).

`vsd2raw` dumps the parsed document as a property-list tree
(document metadata, pages, layers, text objects, styles); `vsd2xhtml`
renders structure and text to XHTML. Neither is byte-comparable to our
model, so the comparison needs explicit normalisation.

## Decision

The harness lives in `crates/vsd-binary/tests/parity/` (task P8-08).
It runs the oracle on a corpus file, parses the dump into a neutral
structure, and compares that against our `Import`:

**Compared against `vsd2raw`:**

- document metadata (title, creator, template, dates);
- the page list: names, sizes, background pages;
- per page: the shape count, the layer list, and per shape the text
  strings, the transform values (position, size, angle, flips) and the
  style properties (fill and stroke colours, weights, patterns);
- geometry: the row kinds and the coordinates.

**Compared against `vsd2xhtml`:** the text strings and their order.

**Normalisation:** lengths compare in inches with a fixed tolerance
(1/1000 inch), angles in degrees with 0.01 degree, colours as RGB
triples, names after Unicode NFC. Trailing whitespace in strings is
ignored. Anything the model drops (ADR 0070) is **not compared** and is
listed as "not compared" in the report.

**Reporting:** one report per corpus file, printed by the harness:
the number of compared fields, the number matching, and one line per
mismatch — `place / field / oracle value / our value`. The parity
number (matching / compared) goes into the phase report. A mismatch is
a bug or a documented limitation; it is never "fixed" by loosening the
tolerance.

**Availability:** the harness follows ADR 0010 exactly — it skips with
a clear message when `LARRYGRAM_LIBVISIO_ORACLE` is unset, and fails
when `LARRYGRAM_REQUIRE_ORACLE=1` and the oracle is missing. The gate
keeps passing without the oracle.

**Corpus census** (task P8-08): a list of further openly licensed
`.vsd` candidates (licence and source per file) is prepared for the
master, who adds the good ones to `corpus/manifest.toml`. The candidate
files themselves are never committed (ADR 0005).

## Consequences

- The comparison is content-level, not pixel-level (pixels are out of
  scope for the alpha).
- The normalisation rules are the contract; a disagreement about
  tolerance is settled in this ADR, not in the harness.
- Any libvisio-derived code stays in `crates/vsd-binary` under MPL
  headers (ADR 0003); the harness itself only calls the CLI tools as
  subprocesses (ADR 0010).

## Alternatives considered

- **Diffing `vsd2xhtml` output against our XHTML export.** Rejected:
  we have no XHTML exporter; the text comparison captures the same
  information.
- **Storing oracle dumps as committed goldens.** Rejected: dumps are
  derived from libvisio output; they are generated fresh each run and
  only the numbers enter the phase report.
