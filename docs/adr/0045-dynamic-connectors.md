# 0045: Dynamic connectors: glue and routing

- Status: accepted
- Date: 2026-10-18
- Task: P5-01

## Context

Phase 5 needs dynamic connectors (KICKOFF; planning brief decision 8):
glue to connection points and to shape outlines, straight and simple
right-angle routing, re-routing when a glued shape moves, and a connector
master. The model already preserves `Connect` records (the `Connects`
part) and the `Connection` section rows; nothing writes them yet. The
routing is explicitly **ours**, not Visio's.

Formula-less editing (ADR 0038) constrains the design: nothing
recomputes a connector's geometry from glue formulas, so the app must
re-route literally.

## Decision

### 1. Glue = `Connects` records plus glue cells

Glue is data in the model (ADR 0039 primitives): `add_connect`,
`remove_connects`, `connects_of` maintain the page's `Connect` records
(`FromSheet`/`FromCell`/`FromPart`, `ToSheet`/`ToCell`/`ToPart`). Two
glue kinds (the `GlueTarget` the commands pass to the model):

- **Connection point** (the highest snap priority, ADR 0043): the target
  is a `Connection` row of the shape (`ToCell` names the row cell, e.g.
  `Connections.Row_1`, `ToPart` the connection-point part).
- **Shape outline** (dynamic glue): the target is the shape itself
  (`ToCell` names the shape's pin cells, `ToPart` the shape part); the
  re-route picks the nearest outline point.

The exact `FromPart`/`ToPart` constants come from the `VisFromParts` /
`VisToParts` enumerations (Microsoft Learn, allowed source) and the
[MS-VSDX] `Connect` element definition; task P5-02 transcribes them into
`crates/model/src/mutate.rs` as named constants with the source recorded
in `docs/sources.md`. Until that transcription lands, the exact values
are task inputs, not decisions of this ADR.

Each connector endpoint holds at most one glue record; a new glue
replaces the old (ADR 0039 invariant 5). Ungluing removes the record.

### 2. Routing is ours: straight or one elbow

`larrygram_geom::routing::route(start, end, RouteOptions)` (L0):

- `Straight`: one segment.
- `RightAngle`: axis-aligned segments with a single elbow. The elbow
  direction is deterministic: horizontal-first when `|dx| >= |dy|`,
  vertical-first otherwise. No obstacle avoidance in Phase 5 (a follow-up
  ADR if we add it).

The path is written into the connector's own `Geometry` rows as literal
rows (`Document::write_connector_path`, ADR 0038 rule 3), so the file
contains what we draw and round-trips (ADR 0048).

### 3. Re-route on change

`connector.reroute_all` (owned by P5-08) recomputes every glued
connector's path on the page after an edit that moved, resized, rotated
or deleted a shape (the editing commands call it through one hook,
`builtins::connector::reroute_after_edit`), and on undo and redo. End
points come from the glue target (its connection point in page
coordinates, or the nearest outline point of the shape's bounds). A
glue whose target was detached is dropped with one warning line.

### 4. The connector master

"Dynamic connector" is an ordinary master in the Basic Flowchart stencil
(ADR 0034): a straight line shape whose `Connection` rows are its two
ends. The connector tool creates an instance of it; nothing in the model
marks a shape "connector" except glue records naming it. A future
formula engine can give the master formula-driven geometry without
migration (ADR 0038 rule 4).

## Consequences

- Connectors re-route deterministically after any edit, with or without
  formulas; the exit flowchart keeps its arrows.
- Glue is inspectable and writable data (`Connects`), so Phase 8 can
  read and set it over the control channel.
- The routing shape can differ from Visio's (ours is one elbow, no
  obstacles); recorded here and in the parity notes, not hidden.

## Alternatives considered

- **Glue as formulas only (`BeginX = ...` glue cells with `SETF`).**
  Rejected: nothing evaluates them until Phase 6; the `Connects` records
  are the format's own glue representation and survive the round trip.
- **Routing in `crates/commands`.** Rejected: it is geometry (L0), and
  geom already hosts the curve math (ADR 0022).
- **Full obstacle-avoiding routing.** Rejected for the alpha: it is a
  research-sized problem, and KICKOFF asks for "straight or with simple
  right angles".
- **Re-route lazily on render only.** Rejected: the written file would
  carry stale geometry; our exit criterion says saved files re-open
  identically.

## Amendment 2026-10-20 (task P5-19): one routed path per connector

Review F7: `Document::write_connector_path` writes the routed path into
the connector's first `Geometry` section and **drops** the connector's
other `Geometry` sections. A connector is one routed path; a second path
would be stale after every re-route and nothing in Phase 5 draws or
reads it. The collapse is documented in the method's doc comment and
pinned by `write_connector_path_replaces_the_extra_geometry_sections`
(`crates/model/tests/mutate.rs`, P5-11).
