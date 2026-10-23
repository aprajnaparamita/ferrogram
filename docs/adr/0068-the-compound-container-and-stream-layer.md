# 0068: The compound container and the stream layer

- Status: accepted
- Date: 2026-10-23
- Task: P8-01

## Context

A binary `.vsd`, `.vss` or `.vst` file is an OLE compound file (MS-CFB)
with a handful of streams: `VisioDocument` (the header and every chunk
record), `\x05SummaryInformation` and `\x05DocumentSummaryInformation`
(document properties), and sometimes `VisioInformation`. Some blocks
inside the Visio stream are stored compressed with an LZ-style scheme
(a small sliding window, literal and back-reference tokens controlled
by flag bytes).

The crate must never crash on malformed input (AGENTS.md section 6), so
whoever parses the container matters: a panic inside a dependency still
crashes the app.

## Decision

`crates/vsd-binary/src/container.rs` holds an **own MS-CFB reader**,
written clean-room from the Microsoft Open Specifications (MS-CFB is an
allowed source, AGENTS.md section 4), under the crate's own lints: no
`unsafe`, `clippy::indexing_slicing` denied, checked arithmetic, capped
allocations and bounded loops on every input-derived count.

- `read_streams(bytes, limits) -> Result<Streams, Error>` reads the
  directory, follows the FAT and mini-FAT chains and returns every
  stream's bytes. Chain walks are bounded and cycle-safe (a visited
  set), and `Limits` caps the stream count, the per-stream size and the
  total.
- `decompress(data, limits) -> Result<Vec<u8>, Error>` restores one
  compressed block. The output is capped by size and by expansion
  ratio. The algorithm is derived from the libvisio reference and
  therefore lives in this crate with its MPL header (ADR 0003).
- The chunk reader (`chunks.rs`, ADR 0069) decides where the format
  marks a block compressed and calls `decompress` there.
- The two summary streams map to `Document::properties`. A
  `VisioInformation` stream is skipped with an `ImportWarning`.

An input that is not a compound file, or a compound file without a
`VisioDocument` stream, is `Error::NotAVisioFile`; a structurally
broken container is `Error::Invalid` or `Error::LimitExceeded`, never a
panic.

## Consequences

- The MS-CFB specification is a source to record in `docs/sources.md`
  (Open issue for the master: add the row; P6-18 owns that file).
- The reader is small and fully covered by malformed-input tests
  (bad signature, truncated header, FAT and mini-FAT cycles, absurd
  declared sizes).
- P8-02 implements this ADR.

## Alternatives considered

- **The `cfb` crate.** Rejected: it is permissive (MIT OR
  Apache-2.0) and well maintained, but its behaviour on hostile input
  is not under our lints and could not be verified without network
  access at planning time. The never-crash rule is non-negotiable, and
  the container is a small, fully specified format. If the own reader
  grows beyond its budget, revisiting the crate with a fuzz campaign
  behind it is fine.
- **libvisio's container handling (via librevenge).** Rejected: the
  oracle is never linked into Ferrogram (ADR 0010), and the OLE layer
  there is LibreOffice's, not Visio's format.
