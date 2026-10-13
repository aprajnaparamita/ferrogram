# 0014: Phase 2 crate split and XML/zip dependencies

- Status: accepted
- Date: 2026-10-13
- Task: P2-01

## Context

Phase 2 work must land in `geom`, `model`, `vsdx` and `shapesheet` without
breaking the layering (ADR 0002) or the MPL confinement (ADR 0003). Two
external crates are needed (XML and zip), and their licenses must pass
`cargo deny`. `shapesheet`'s Phase 2 scope is open: formulas could be parsed
now or left as text.

## Decision

### Crate split

| Crate | Layer | Phase 2 content |
|---|---|---|
| `geom` | L0 | `Point`, `Size`, `Rect`, `Angle`, `Transform`, `Unit` and the unit conversions to canonical units (inches, degrees, fraction). Path/arc/NURBS/spline types wait for Phase 3 geometry rows. |
| `model` | L1 | ADR 0011 shape: `Document`, `Page`, `Master`, `Shape`, `Stylesheet`/`Style`, `Sheet`/`Section`/`Row`/`Cell`, ids, the preserved-XML tree (`XmlNode`, `Preserved`) and the equivalence `diff`, and the lookup API (ADR 0013). Depends on `geom` only. |
| `vsdx` | L2 | `package/`: pure OPC (zip container, `[Content_Types].xml`, relationships, `Limits` caps). `read/`: package to model. `write/`: model to parts to bytes. Depends on `model`, `geom`, `quick-xml`, `zip`, `flate2`. |
| `shapesheet` | L2 | Types only: `Formula` (source text), `CellRef`, opaque `Expr` and the `parse` signature. Parsing and evaluation are later phases. No dependency on `model` yet. |

`model` knows nothing about OPC or XML parsing; it only stores preserved XML
trees (`XmlNode` is plain data, no parser). `vsdx` does all format work.
Anything `vsdx` and `shapesheet` will both need belongs in `model`.

**Formulas are stored as source strings in Phase 2.** Parsing them now would
drag the formula engine into the round-trip scope; raw text round-trips
losslessly and the parser is its own later task.

### XML crate: `quick-xml` 0.41

- License: MIT/Apache-2.0 (passes `deny.toml`).
- Already in `Cargo.lock` (via `calamine` in xtask), so no new advisories
  surface; RUSTSEC-2024-0001/RUSTSEC-2025-0004 need quick-xml >= 0.41.
- Pull-based, no unsafe (workspace `#![forbid(unsafe_code)]` is crate-level,
  but the dep tree stays clean anyway), and it lets us preserve attribute
  order and prefix spelling, which serde-based DOM crates normalize away.

### Zip crate: `zip` 8.6 with only deflate

```toml
zip = { version = "8.6", default-features = false, features = ["deflate-flate2"] }
flate2 = { version = "1.1", default-features = false, features = ["rust_backend"] }
```

- `zip` is MIT (passes `deny.toml`); 8.6 is already in `Cargo.lock` (xtask's
  fixture builder uses it with stored entries).
- `default-features = false` drops bzip2, zstd, lzma, xz, ppmd, AES and the
  time crates: their licenses are not in `deny.toml`, and Visio writes deflate
  (or stores) only. A non-deflate entry is a readable-but-uncompressed or an
  error case, both fine to reject explicitly.
- `deflate-flate2` plus `flate2`'s `rust_backend` uses **miniz_oxide**
  (MIT/Zlib/Apache-2.0), pure Rust, no C toolchain. The zip crate's `deflate`
  feature instead pulls zopfli (write) and zlib-rs; both are allowed
  (Apache-2.0, Zlib) but zopfli is a slow compressor we do not need.

`cargo deny check` runs with these dependencies in the tree (acceptance
criterion of P2-01).

## Consequences

- `vsdx` compiles `zip`, `quick-xml` and `flate2`; the fuzz target (P2-09)
  exercises them against malformed input.
- If a corpus file ever needs another codec (none does today), that is a new
  ADR, because the codec's license must pass `deny.toml`.
- `shapesheet` stays a leaf until the formula engine task needs `model`; the
  layering table (ADR 0002) is unchanged.

## Alternatives considered

- **`roxmltree`.** Already used by xtask (MIT), but read-only and
  namespace-normalizing; we must write XML. Rejected.
- **`xmltree`.** DOM with `Element` attributes in a `BTreeMap`: attribute
  order and prefix spelling lost on load. Rejected for lossless preservation.
- **serde-xml-rs / quick-xml serde.** Structural mapping, but preserved
  unknown content has no schema. Rejected.
- **`miniz_oxide` directly.** Rejected: the zip container format (central
  directory, data descriptors) is exactly what the `zip` crate owns.
- **zip with the `deflate` feature (zopfli + zlib-rs).** Licenses pass;
  rejected only because zopfli compresses slowly and miniz_oxide is enough
  for both directions.
- **Parse formulas in Phase 2.** Rejected: out of the round-trip scope; raw
  text is lossless and the parser needs its own design task.
