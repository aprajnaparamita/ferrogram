# 0012: Lossless preservation and semantic XML equivalence

- Status: accepted
- Date: 2026-10-13
- Task: P2-01

## Context

Phase 2 must round-trip every corpus file "without loss" (KICKOFF). The model
maps only the `supported` surface (`docs/vsdx-inventory.md`); everything else
is `preserved-only` or unknown. A round-trip therefore cannot be byte-identical
(the writer generates `[Content_Types].xml`, relationship parts and attribute
order its own way), and the exit criterion needs a precise equality to test
against. That equality is **semantic XML equivalence**.

## Decision

### Where preserved content lives

1. **Unmapped parts** stay in the `Package` (the OPC layer of
   `ferrogram-vsdx`): `PartData::Xml(XmlNode)` for XML parts,
   `PartData::Binary(Vec<u8>)` otherwise. `write_document(document, preserve)`
   starts from the `preserve` package and replaces the supported parts, so an
   unknown part is copied byte for byte.
2. **Unmapped attributes and children of modeled elements** are kept on the
   model entity in `Preserved { attributes, children }` (shapes, pages,
   masters, styles, sections, rows, cells, the root element).
3. **Unknown elements inside supported parts** (e.g. a future child of
   `Shape`) are `Preserved::children` on the nearest modeled ancestor.
4. `[Content_Types].xml` keeps its `Default`/`Override` spelling in
   `Package::content_types`; relationship elements keep order in
   `Package::relationships`.

A preserved subtree is the `XmlNode` tree of ADR 0011: qualified name
(prefix, URI, local), attributes in document order with the prefix spelling
kept, children in document order (`Element | Text | Comment |
ProcessingInstruction`), and a `self_closing` flag kept for byte-faithful
rewrites.

### Semantic XML equivalence

Two XML trees are **semantically equivalent** when `diff(a, b)` is empty.
The relation is defined by these rules, which the round-trip test uses:

| Aspect | Rule |
|---|---|
| Element name | Equal when local name **and namespace URI** are equal. |
| Namespace prefix | **Ignored.** `a:Cell` and `Cell` in the same URI are equal. Namespace declarations are not attributes for comparison. |
| Attribute set | Compared by (namespace URI, local name). Order is **ignored**. |
| Attribute value | Strings compare exactly, except a value that starts with a number literal (with an optional unit suffix) compares numerically against another such value with the same suffix (`0.5` = `.50`, `0.5 in` = `.50 in`). An omitted attribute is a presence difference, never equal to an empty one. |
| Child elements | Compared **in document order**: same length, pairwise equivalent. Reordering is a difference. |
| Text | Character data compares exactly after entity decoding. Whitespace inside element-only content is ignorable and normalized away; whitespace inside mixed content (text-bearing elements of `docProps`) is significant. |
| Self-closing | `<a/>` and `<a></a>` are equivalent. The flag is kept only for faithful rewrites. |
| Comments and PIs | Significant: compared by target/data exactly. |

Whitespace rule in practice: a text node that is entirely whitespace and sits
between two element children is ignored (pretty-printing); any other text is
significant. This is the standard XML "element-only content" rule and it is
what `diff` implements.

**Amended 2026-10-14, P2-14** (review finding 6): the attribute-value rule is
stated as implemented. Two attribute values compare numerically only when both
start with a number literal and their suffixes match exactly; a suffix is
whatever follows the literal (a unit in practice) and units never convert.
Every other value — all text included — compares exactly; text is never
numeric-normalized.

Number comparison is the only value normalization. Lengths do **not** convert
units for equivalence: `1 in` and `25.4 mm` are different spellings and a
round-trip must keep the original one. The model stores both the canonical
number and the source unit (ADR 0011) so this is possible.

### What the round-trip test checks (exit criterion)

For every corpus `.vsdx` file:

1. `read -> write` produces a package whose XML parts are semantically
   equivalent to the source's (rule table above), and whose part list,
   content types and relationships match;
2. `read -> write -> read -> write` is byte-identical between the two writes
   (writer stability: the writer is a function of the model plus preserved
   content);
3. no step panics.

Byte-identity of the first write is explicitly **not** required: attribute
order, prefix choice, self-closing spelling and the pretty-print whitespace of
our own writer are ours.

## Consequences

- `ferrogram_model::raw::diff` is the one place that defines equivalence; the
  test harness (P2-09) calls it and prints the `XmlDifference` path list on
  failure.
- Parts preserved as bytes (images) compare byte-for-byte, which the part-list
  check covers.
- A source file with two attributes that differ only in prefix spelling is
  equivalent to a rewrite that drops one prefix; our writer keeps prefixes
  anyway because `XmlNode` stores them.

## Alternatives considered

- **Byte-identical round-trip.** Rejected: forces the writer to reproduce
  `zip` entry order, compression settings, XML declaration spelling and
  attribute order exactly; any pretty-print change breaks the exit criterion
  with no information value.
- **Canonical XML (C14N) comparison.** Rejected: C14N normalizes less than we
  need (it keeps attribute order in some profiles, keeps prefixes) and pulls
  in a spec-complexity budget we do not need. The rule table above is C14N's
  useful subset.
- **Compare parsed models only (no XML level).** Rejected: bugs in preserved
  content would be invisible; the exit criterion is about loss.
- **Keep preserved content as raw byte slices of the source XML.** Rejected:
  byte slices pin formatting and make insertion of new elements ambiguous;
  a parsed `XmlNode` tree is re-serializable and comparable.
