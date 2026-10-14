# Phase 2 report: Document model and .vsdx I/O

Date: 2026-10-14

## What was built

| Area | Content | Tasks |
|---|---|---|
| `crates/geom` (L0) | Units and conversions (canonical: inches, radians, fraction), `UnitKind` kind checks, points, rectangles, angles, affine transforms with a checked inverse | P2-03, P2-06b |
| `crates/model` (L1) | Document, pages, masters, shapes (arena with group links), styles, ShapeSheet sheets, sections, rows and cells. Each cell keeps formula, cached value and unit separately. Content the model does not map is kept in `Preserved`. | P2-04, P2-06b |
| Inheritance lookup | `CellLookup::resolve`: local cell, master, style chain (by Visio style ID, per kind), table default, hard default. Provenance in `Resolved::via`; cycles are cut; nothing is flattened on load; `Inh` cells do not define. | P2-04, P2-06b, master |
| Semantic XML equivalence | `raw::diff`, the ADR 0012 rule table: prefixes and attribute order ignored, namespace URIs and child order significant, number literals compare numerically | P2-04, P2-14 |
| Generated ShapeSheet table | `cargo xtask codegen` writes `crates/model/src/generated/shapesheet.rs` from `ui-spec/shapesheet.toml`; a gate step fails when it is stale | P2-02 |
| Name aliases | 17 cell and 8 section aliases between `.vsdx` spellings and the ShapeSheet table | P2-06 |
| `crates/vsdx` package layer (L2) | OPC zip, content types, relationships, part access, with caps on part size, total size, entry count, XML depth and attributes. Accepts packages without `[Content_Types].xml` (two libvisio files). | P2-05 |
| `.vsdx` reader | Pages, masters, shapes, styles, document properties and connections into the model; drawings, stencils and templates (ADR 0016) | P2-06, P2-06b |
| `.vsdx` writer | Patches each part's source tree from the model (ADR 0018): identity-keyed matching, unmapped content kept in place, new entities at the schema position. `from_model` writes new documents. | P2-07, P2-14 |
| Fixtures | Hand-written minimal drawing, stencil and template plus 10 malformed variants (8 committed, 2 generated at test time) | P2-08 |
| Test harness | Corpus round-trip, malformed input and every cap, libvisio oracle comparison, `xmllint` schema check, `read_package` fuzz target, property tests | P2-07, P2-09, P2-10, P2-14 |

## Exit criterion

**Every corpus `.vsdx` file round-trips without loss and without panics: met.**

`FERROGRAM_REQUIRE_CORPUS=1 cargo test -p ferrogram-vsdx --test roundtrip` passes for all 15 corpus files. For each file it checks four things:

- read, write and read again gives no `diff` on any XML part;
- the part lists, content types and relationships are equal;
- binary parts are byte-equal;
- a second write is byte-identical to the first.

## Gate results

```
cargo xtask gate (full, 2026-10-14, main)
  PASS  codegen
  PASS  fmt
  PASS  clippy
  PASS  test      (851 tests; 2 ignored: the fixture regenerator and the oracle smoke test)
  PASS  doc
  PASS  coverage
  PASS  deny
  PASS  layers
  PASS  wasm
  PASS  fuzz      (crates/ui-spec/fuzz, crates/vsdx/fuzz)
gate: GREEN

FERROGRAM_REQUIRE_CORPUS=1 FERROGRAM_REQUIRE_SCHEMA=1 cargo test -p ferrogram-vsdx --test roundtrip --test schema: OK
cargo xtask extract --check: OK
```

## Coverage

| Crate | Lines | Coverage | Threshold | Phase 1 ratchet | New ratchet |
|---|---|---|---|---|---|
| ferrogram-geom | 542 | 100.0% | 85 | — | 100.0 |
| ferrogram-model | 2525 | 99.7% | 85 | — | 99.7 |
| ferrogram-shapesheet | 45 | 100.0% | 85 | — | 100.0 |
| ferrogram-vsdx | 6162 | 97.8% | 85 | — | 97.7 |
| ferrogram-ui-spec | 1738 | 99.3% | 85 | 99.3 | 99.3 |
| xtask | 9232 | 91.6% | — | 91.3 | 91.6 |
| ferrogram, ferrogram-cli | 9 each | 66.7% | 50 | 66.6 | 66.6 |

The coverage audit notes that inline `#[cfg(test)]` code counts as instrumented lines, so these numbers are "src coverage", a little above product-only coverage.

## Reviewer findings and resolution

| Report | Verdict | Blocker / major / minor | Resolution |
|---|---|---|---|
| [Code review](../reviews/phase-2-reviewer.md) (P2-11) | approve with fixes | 0 / 1 / 6 | P2-14; finding 7 fixed on main |
| [Coverage](../reviews/phase-2-coverage.md) (P2-12) | pass | 0 / 3 / 11 | P2-14 (majors and the listed minors) |
| [Clean-room](../reviews/phase-2-cleanroom.md) (P2-13) | pass with findings | 0 / 0 / 2 | F1 comment fixed on main; F2 (attribution row one commit late) needs no change |

The major review finding was that the default XML namespace was never resolved, so `diff` could not see a lost namespace. P2-14 fixed it and the corpus round-trip still passes.

## Decisions made during the phase

- **Angles:** cached values in `.vsdx` are already in canonical units, so the reader does not convert. The canonical angle unit is radians (ADR 0011 amendment, P2-06/P2-06b).
- **Writer:** patches the source trees instead of regenerating them (ADR 0018), because unmapped content interleaves with mapped content in real files.
- **Style references:** they are Visio style IDs, and a style's chain is its own `LineStyle`/`FillStyle`/`TextStyle` (ADR 0013 amendment).
- **`Inh` cells:** a cell with formula `Inh` does not define the cell for the lookup (ADR 0013 amendment).
- **Missing parts:** a package without `[Content_Types].xml` is read with inferred types, and a dangling relationship is recorded, not an error.
- **Schema:** the public XSD is XSD 1.1, uses an older namespace and is out of date. Only 3 of 106 corpus parts validate against a derived copy. The schema test therefore checks that the writer adds no new validation errors. The XSD is never committed.

## ADRs added

- 0011 model shape (amended: radians; section-level cells; several master shapes; style IDs)
- 0012 lossless preservation and semantic XML equivalence (amended: number-literal rule)
- 0013 inheritance lookup (amended: master position rule, style-ID chain, `Inh`)
- 0014 crate split and XML/zip dependencies (`quick-xml`, `zip` with pure-Rust deflate)
- 0015 ShapeSheet codegen
- 0016 stencils and templates
- 0017 Phase 2 test strategy
- 0018 patch-based writer

## Known issues

- **CI has still never run** (no remote). When it exists, CI should install `libxml2`, provide `reference/`, and set `FERROGRAM_REQUIRE_SCHEMA=1` and `FERROGRAM_REQUIRE_CORPUS=1`.
- **Oracle:** libvisio's raw dump of `.vsdx` has no masters or shapes, so the oracle compares only page counts and page sizes. The full comparison comes with Phase 7.
- **Deferred to Phase 4 (editing)**, all listed in `docs/STATUS.md`:
  - a deleted page or master leaves its part;
  - new document properties go into `DocumentProperties`;
  - edits to `Preserved` content are not written back;
  - the arena holds nested sub-shapes in wrapper order;
  - the sub-shape generator stops at 128 levels.
- **Models built in memory:** a `Color` or `List` value with a unit, or a `Bool`/`Str` with a real unit, does not read back the same. Files read from disk are not affected. This belongs with the formula engine.
- **ShapeSheet table gaps:**
  - no default values;
  - some cell, section and row indices are missing;
  - 12 corpus cell names have no table entry (listed in the P2-06 Result).
- **Cached defaults:** table defaults are parsed lazily and interned. Move this into codegen once the table records defaults.
