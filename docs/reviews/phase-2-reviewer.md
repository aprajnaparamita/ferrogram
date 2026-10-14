# Phase 2 review — reviewer (P2-11)

Scope: `git diff phase-1..main` (123 files, ~30k lines): `crates/geom`,
`crates/model` (incl. the generated ShapeSheet table), `crates/shapesheet`,
`crates/vsdx` (package, read, write), `xtask/src/codegen.rs`, ADRs 0011–0018
and the Phase 2 test suites. Review contracts: ADRs 0011–0018 and the Phase 2
task files.

Method: full read of the non-test sources of `geom`, `model`, `shapesheet` and
`vsdx` (the generated `crates/model/src/generated/shapesheet.rs` spot-checked
against ADR 0015 and `xtask/src/codegen.rs`; its freshness is gate-enforced);
grep sweeps for `unwrap`/`expect`/`panic!`/indexing outside `#[cfg(test)]`;
dependency and layering check against ADR 0014; `cargo xtask gate` in the
worktree (GREEN, all steps including coverage, wasm and fuzz). Test files read
for structure and failure coverage, not line by line; test *quality* is P2-10's
deliverable and only structural gaps are raised here. P2-12 (coverage) and
P2-13 (clean-room) own their subjects; nothing here overlaps their grading.

Positive findings, verified rather than assumed:

- No `unwrap`/`expect`/`panic!`/`unreachable!` and no indexing outside
  `#[cfg(test)]` in the Phase 2 sources; `#![forbid(unsafe_code)]` is on every
  crate. Input-derived sizes use `saturating_*`/`checked` arithmetic, `get()`
  and capped reads (`read_package` inflates under `max_part_uncompressed`
  before allocating, and caps the total).
- Error handling is uniform: fixed, content-free messages that name the part
  and the attribute or cell but never carry file content
  (`read::bad_value`, `package::read::invalid`). Malformed input degrades where
  the module docs promise it (dangling `Rel`, `Master`, `BackPage`) and errors
  where the model must interpret (`parse_attr_int`, numeric/boolean cached
  values).
- The lookup (ADR 0013) matches the amended contract closely: `Inh` does not
  define, the master position rule and the per-kind style-id chain are as
  P2-06b amended, cycle handling is visited-set bounded in both chains, and
  `Resolved::via` ends at `source`.
- The patch-based writer (ADR 0018) is implemented as designed: identity-keyed
  claiming (`claim`, `sync_children`), spelling-preserving attribute writes
  (`sync_int` accepts any spelling that parses to the same number), removal
  guarded by the entity's `Preserved` fragment, schema-rank insertion, and
  bounded recursion (`patch_shape` caps at `max_xml_depth`).
- Layering and dependencies match ADR 0014 exactly (model: geom only; vsdx:
  model, geom, quick-xml, zip-with-deflate-flate2, flate2 rust_backend;
  shapesheet: no deps).

## Findings

### 1. major — `crates/vsdx/src/package/xml.rs:276`: the default namespace is never resolved, so `QName::uri` is `None` for unprefixed names

`resolve_prefix` returns early on `let prefix = prefix?;`, so an unprefixed
element name resolves to no namespace whatever the scope says. That
contradicts the function's own doc ("Unprefixed names take the default
namespace") and ADR 0012's rule table ("`a:Cell` and `Cell` in the same URI
are equal"; equality is by local name *and namespace URI*). Consequences:

- every corpus part uses `xmlns='http://schemas.microsoft.com/office/visio/2012/main'`
  (all 15 `document.xml` files), so `QName::uri` is `None` for every modeled
  element and the namespace rung of `diff` is dead across the whole exit
  criterion: a rewrite that dropped a default-namespace declaration (or moved
  the vocabulary to no namespace) would compare equal;
- `<Cell xmlns="urn:x"/>` and `<Cell/>` are "equivalent" under `diff`, while
  `<Cell>` and `<a:Cell xmlns:a="urn:x">` (the ADR's own example) are not;
- `emit.rs` builds generated names with `uri: Some(VISIO_NS)` (`emit.rs:84`)
  while the parser yields `None` for the same vocabulary — the two halves of
  the writer disagree about what a name is. Nothing notices today because the
  property-test generators avoid default namespaces (`properties.rs` declares
  only `xmlns:p`), but any Phase 3 test that hand-builds an expected tree
  against parsed output will hit it.

Fix: resolve unprefixed *element* names against the default-namespace binding
(attributes correctly stay in no namespace), keeping the current early return
only for attribute names. Add parse/diff tests with default-namespace input
(`<Cell xmlns="urn:x"/>` vs `<p:Cell xmlns:p="urn:x"/>` are equivalent; vs
`<Cell/>` are not) and update the hand-built expected trees that rely on
`uri: None` for Visio names. Suggested fix task: P2-14 (small, one file plus
tests).

### 2. minor — `crates/vsdx/src/write/patch.rs:222`: `remove_children` deletes blank text even when nothing was dropped

`sync_children` always ends in `remove_children`, and `remove_children` clears
the kept children whenever they are all whitespace text. That is right when
the dropped elements were the only markup (pretty-printing noise), but wrong
when `drop` is empty: an element whose content is blank text only
(`<Shapes>   </Shapes>`, `<Text> </Text>`) has that text as *significant*
content under ADR 0012 (no markup children, so the whitespace rule does not
apply), and the write turns it into an empty element. `diff` then reports a
`Child` difference and the round-trip criterion would fail on such a file. The
corpus has none, so it is latent, but Phase 4 edits can create the shape.

Fix: apply the whitespace cleanup only when `drop` is non-empty (or only when
the removed elements were the kept list's only former markup). Add a test with
blank text content and no element children. The existing
`remove_children_drops_orphaned_whitespace` (patch.rs:1487) covers the
intended case and should stay.

### 3. minor — duplicated helpers across `read/` and `write/`

`main_part`, `is_main_content_type`, `rel_type_local`, `resolve_target`
(`crates/vsdx/src/read/mod.rs:148-240` and `crates/vsdx/src/write/mod.rs:117-182`),
`text_content` (`read/mod.rs:286` and `write/patch.rs:437`) and
`parse_color`/`hex`/`digit` (`read/value.rs:124-167` and
`write/value.rs:96-120`) are copy-pasted. They are the rules "the reader and
the writer must agree on"; a fix to one copy will not reach the other.

Fix: one `package` helper module (part/relationship discovery) and one shared
value-parsing module; keep thin wrappers where the call sites differ.

### 4. minor — `crates/model/src/lookup.rs:464`: `interned_default` leaks per call on a poisoned or contended cache

The fast path and the insert path are fine (bounded by the static table), but
the `Err(_)` arm of the second `cache.lock()` returns a freshly
`Box::leak`ed value per call, and the check-then-lock gap leaks one duplicate
per concurrent first-time parse. A poisoned mutex would leak on every lookup.

Fix: `cache.lock().unwrap_or_else(|poisoned| poisoned.into_inner())` and
insert-or-reuse in one critical section (the `entry` call already does this;
only the lock failure path needs work).

### 5. minor — `crates/vsdx/src/package/read.rs:374`: unknown children of a `Relationship` element are dropped

`parse_relationships` builds `Preserved { attributes, .., children: Vec::new() }`
and `package/write.rs`'s `relationships_xml` emits no preserved children, so
any child element of a `Relationship` is silently lost on round-trip. No
corpus file has one, and `Preserved` has the slot for it.

Fix: keep the children in `Relationship::preserved` and write them back (or
document the drop in ADR 0012 if it is deliberate).

### 6. minor — `crates/model/src/raw.rs:21` and `:432`: doc wording drifts from the implementation

The module doc says attribute values that "parse as a number" compare
numerically, and `values_equal`'s doc says it compares "attribute or text
values"; the implementation is stricter and better (a number literal with an
optional suffix, suffixes must match, units never convert) and text compares
exactly in `compare_child`. ADR 0012's wording ("a value that parses as a
number") has the same looseness. Align the docs with the code (or note the
suffix rule as an ADR amendment).

### 7. minor — `docs/adr/0014-crate-split-and-xml-zip-dependencies.md:21`: stale canonical units

The crate table still says `geom` converts "to canonical units (inches,
degrees, fraction)". ADR 0011 was amended twice since (radians are canonical,
in `geom` too). One-line doc fix; worth doing because the table is the layering
reference.

## Verdict

**approve with fixes** — 0 blocker, 1 major, 6 minor. The Phase 2 code is
careful, consistently documented and matches the ADRs; the one substantive
defect is the namespace resolution gap (finding 1), which weakens the exit
criterion's loss detection without causing wrong behavior on the current
corpus. Findings 1 and 2 are cheap and should go into the first fix task;
3–7 can batch with it or wait for the next cleanup pass.
