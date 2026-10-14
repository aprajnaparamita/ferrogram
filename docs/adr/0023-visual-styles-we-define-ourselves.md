# 0023: Visual styles we define ourselves

- Status: accepted
- Date: 2026-10-14
- Task: P3-01

## Context

Rendering needs Visio's line patterns (dash rhythms), line-end (arrowhead)
shapes, fill patterns and shadow styles. [MS-VSDX] specifies only the
*indices* in cells like `LinePattern`, `BeginArrow`, `FillPattern` and
`ShapeShdwPattern`; the patterns themselves are Visio's drawing, not
published data. The clean-room rules allow Microsoft's enumerations (names
and values) and observed behavior of Visio as a black box; they forbid
extracting anything from a Microsoft binary. We must therefore draw our own
patterns, commit them as our own work with provenance, and be honest about
what "close enough" means.

## Decision

### One committed data file: `crates/render/data/visual-styles.yaml`

Our own work, authored by hand, structured so the later screenshot pass can
enrich it without schema changes (KICKOFF). It holds:

```yaml
# Header comment: our own definitions, sources and date (provenance).
meta:
  version: 1
  note: "Our own approximations of Visio's visual styles (ADR 0023)."
line_patterns:      # index -> dash rhythm as multiples of line width
  - index: 0        # visLineNone: invisible
    dashes: []
    invisible: true
  - index: 1        # visLineSolid
    dashes: []
  - index: 2        # ...
    dashes: [4, 2]
line_ends:          # index -> arrowhead template
  - index: 0        # visLineNoArrow
    shape: none
  - index: 1
    shape: "M 0 0 L 1 0.5 L 0 1 z"   # own path, unit coordinates
    filled: true
    width: 1.0      # multiples of the line width or the size cell
    length: 2.0
fill_patterns:      # index -> tile
  - index: 0
    kind: none
  - index: 1
    kind: solid
  - index: 27
    kind: tile
    tile: "M 0 0 L 1 1"              # own tile path
    spacing: 4.0                     # multiples of line width
shadows:            # index -> offset, blur, color transform
  - index: 0
    kind: none
  - index: 1
    kind: drop
    dx: 2.0                          # multiples of line width
    dy: -2.0
    blur: 1.0
    alpha: 0.5
```

Indices and names come from the Visio VBA enumerations on Microsoft Learn
(`VisLinePatterns`, `VisArrowSizes`, `VisFillPatterns`, ...) — derived
facts, recorded in `docs/sources.md` rows. The path and tile *drawings* are
ours. The file is parsed at build time (`include_str!` + serde), so it
ships inside the wasm build and there is no runtime file access.

`render::styles::VisualStyles` (P3-08) loads the file and resolves an index
to templates (`LineEndTemplate`, `PatternTemplate`, dash vectors, shadow
settings). The scene builder (P3-05) turns templates into `scene` values
(ADR 0020): dash lengths scaled by the line width, arrowheads scaled by the
`BeginArrowSize`/`EndArrowSize` cells (0..4 size classes), pattern tiles
scaled by the pattern's spacing cell where relevant.

### What "close enough" means for the alpha

- **Family and rhythm**: a dashed pattern dashes where Visio dashes, with a
  similar proportion (dash/gap ratio within roughly 25% by eye). Not
  pixel-exact dash lengths.
- **Silhouette**: an arrowhead has the same silhouette family (filled
  triangle, open V, circle, bar) and roughly the same aspect. Not Visio's
  exact proportions.
- **Fill patterns**: same pattern family (horizontal, vertical, diagonal,
  grid, checker) at similar density.
- **Shadows**: same direction of offset and a soft edge of roughly the same
  weight.
- Every entry carries a `note` field for observations from the later
  screenshot pass to refine.

Nothing here is copied from Visio output; a screenshot may be *observed*
(black-box rule) and the observation described in our own numbers.

## Consequences

- The renderer has its own, reviewable style definitions; a later pass
  edits data, not code.
- Golden images (ADR 0025) pin our own patterns, so they change only when
  we change the file.
- Two sources of truth for "what a Visio pattern looks like": the file and
  future screenshots. The file wins until the screenshot pass revises it.

## Alternatives considered

- **Hard-code the patterns in `render`.** Rejected: harder to review and
  enrich; KICKOFF wants data formats the later pass can enrich without
  schema changes.
- **Trace patterns from screenshots now.** Rejected for the alpha: visual
  fidelity is explicitly not a goal of this run; observation-driven
  refinement belongs to the screenshot pass.
- **Keep patterns in `ui-spec/`.** Rejected: that directory is UI layout
  data (menus, ribbon, shortcuts); these are rendering definitions used by
  the engine and the CLI with no UI involved.
- **Skip patterns in Phase 3 (solid lines only).** Rejected: KICKOFF lists
  line patterns and fill patterns in the Phase 3 rendering list.
