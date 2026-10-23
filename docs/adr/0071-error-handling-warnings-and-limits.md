# 0071: Error handling, warnings and limits

- Status: accepted
- Date: 2026-10-23
- Task: P8-01

## Context

Binary Visio files in the wild are old, sometimes damaged and sometimes
hostile. The rules are fixed: never crash (AGENTS.md section 6),
capped allocations on input-derived values, bounded recursion, and a
fuzz target for every parser crate. The `.vsdx` reader already has the
pattern: `Limits` caps, an `Error` that names the place but never the
content, and a warning stream for degradations (ADR 0021).

## Decision

**Errors** (`Error` in `error.rs`) are structural failures only:

- `NotAVisioFile`: not a compound file, or no `VisioDocument` stream,
  or the magic is wrong.
- `UnsupportedVersion`: the version byte is out of alpha scope (ADR 0069).
- `Invalid`: a record or structure is unusable (a header cut short, a
  length field that lies beyond the stream when nothing more can be
  read).
- `LimitExceeded`: a cap in `Limits` was hit; the message names the
  cap, the value and the maximum.
- `Model`: a `ferrogram_model` operation rejected the mapping.

**Warnings** (`ImportWarning` in `warning.rs`) record every degradation:
skipped records, dropped data, unresolved references and values that
did not parse. `import` returns the document and the warnings together;
the UI and the CLI surface them as a list.

The boundary: when a document can be built at all, `import` returns
`Ok` with warnings. When the container, the version or the stream
header cannot be read, it returns `Err`. A record that is cut short or
nonsensical is skipped with a warning and the walk continues at the
next recoverable boundary.

**Limits** (`limits.rs`) cap the stream count, per-stream and total
uncompressed size, the decompression ratio, the record count, the
nesting level, the shape count and the total text size. Defaults live
in `Limits::DEFAULT`; callers may lower them. A cap is checked before
the allocation it guards.

**Never crash in practice:**

- no `unwrap`, `expect`, `panic!`, `unreachable!`, `todo!` outside
  tests (workspace clippy lints), no `unsafe`, and
  `clippy::indexing_slicing` denied in the crate root — the parser
  rules;
- every input-derived loop and recursion is bounded (FAT chains,
  pointer graphs, record nesting, group nesting);
- the fuzz target (P8-10) drives `import` with mutated bytes; every
  crash fix gets a regression test next to it.

The `Error::NotImplemented` variant exists only while the interface
stubs of P8-01 are in place. The last Phase 8 task removes it and its
tests.

## Consequences

- Callers see one error type and one warning type, both
  `#[non_exhaustive]`, so later phases can extend them.
- The degradations are visible: a phase report can say what was dropped
  per corpus file instead of hiding it.
- Malformed-input tests (truncated containers, FAT cycles, lying
  lengths, pointer cycles) are part of each implementing task's
  acceptance criteria.

## Alternatives considered

- **Fail hard on any anomaly.** Rejected: old files are full of
  harmless anomalies; refusing them would make the importer useless.
- **Silent degradation.** Rejected: the exit criteria demand an honest
  report.
