# 0069: The chunk and pointer model, and the alpha version scope

- Status: accepted
- Date: 2026-10-23
- Task: P8-01

## Context

The `VisioDocument` stream starts with the magic
`"Visio (TM) Drawing\r\n\0"`, then padding, then a one-byte file
version at offset `0x1A`. The rest of the stream is chunk records.
libvisio reads versions 1 to 11 and keeps two parsers: one for versions
1–5 (an older pointer and list record layout) and one for the chunk
format of versions 6 and 11 (Visio 2000/2002 and 2003). All five corpus
`.vsd` files are version 11.

A chunk record header names the record kind (a small integer), the
record's own id, the id of the list it indexes into, the payload length
and the nesting level; the field widths differ between the version 6
and the version 11 header. Some records are pointers: they redirect to
another offset of the stream (a `Type`, `Id`, `Offset`, `Length`,
`Format` and `ListSize` tuple), and list records (geometry, character,
paragraph, field, shape, name and other lists) group the items other
records reference by id.

## Decision

**Alpha scope: versions 6 and 11.** `FormatVersion::from_byte` accepts
6 and 11; anything else (1–5 and the impossible 7–10) is
`Error::UnsupportedVersion`, a clear error and never a crash. A later
phase can add the version 1–5 layout behind the same `FormatVersion`
without touching the mapping layer.

`crates/vsd-binary/src/chunks.rs` owns the model:

- `Record(pub u16)`: the record-kind id, opaque. The id table is
  derived from the libvisio reference (AGENTS.md section 5): it is a
  list of format identifiers and names, it lives in `chunks.rs` with
  the reference note, and nothing else in the workspace sees the raw
  ids.
- `Chunk<'a> { record, level, list, id, data }`: one record, its
  nesting level preserved, its payload borrowed from the stream.
- `version(stream)`: reads the version byte after the magic.
- `read_chunks(stream, version, limits) -> Vec<Chunk>`: flattens the
  stream. Pointer targets are visited at most once (a visited set of
  offsets), so a pointer cycle or a self-referential list terminates;
  `Limits` caps the record count and the nesting level. A record whose
  declared length runs past the stream ends the walk at that record
  with an `ImportWarning`, unless nothing at all can be read (then
  `Error::Invalid`).

The per-version differences (header widths, payload layouts, text
encoding) stay inside `chunks.rs` and the domain modules; the domain
modules compare `Record` ids against their own local constants and
ignore what they do not consume (with a warning).

## Consequences

- The chunk reader is the single place that knows record ids and
  header layouts; the domain modules only see `Chunk` values.
- Versions 6 and 11 double some header handling but share the model;
  the corpus exercises version 11 and the hand-built tests cover both.
- The record id table is derived data recorded in an MPL-2.0 file
  (`chunks.rs`), which keeps ADR 0003 satisfied without debate.

## Alternatives considered

- **A typed `Record` enum with one variant per record kind.** Rejected:
  a closed enum would have to change for every record a future task
  learns to read, and unknown ids would lose their data. The id
  newtype passes unknown records through.
- **Reading versions 1–5 in the alpha.** Rejected: a different record
  layout (its own parser in the reference), no corpus coverage, and
  the alpha exit criteria do not need it.
