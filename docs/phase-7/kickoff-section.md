# KICKOFF.md changes for Phase 7

Replace the current "Phase 7: Binary .vsd import (read-only)" section
with the section below. Renumber the later phases:

- Binary .vsd import becomes Phase 8.
- Agents (control channel and MCP) becomes Phase 9.

Their text does not change. In `docs/STATUS.md`, the deferred note "The
vsd-binary threshold becomes meaningful once it has library code (Phase
7)" becomes Phase 8.

---

### Phase 7: Panel Classic and Panel Modern layouts

Two complete, designed layouts on top of the command registry:

- **Panel Classic**: a menu bar with access keys, docked toolbars (Standard
  and Formatting shown; 14 more under View > Toolbars) and context
  menus. Separate menu bars apply when a ShapeSheet window is active or
  a stencil is being edited.
- **Panel Modern**: tabs whose groups place every control in a stack or rows
  slot at a large, medium or small size. It also has:
  - contextual tabs (Picture Tools, Container Tools, ShapeSheet Tools)
    that follow the selection;
  - a Quick Access Toolbar;
  - a collapsible command bar;
  - a File view.

Both layouts are data (`ui-spec/layouts/*.yaml`, layout format v2, ADR
0061) and are drawn by one set of widgets. The Phase 4 Fluent-data
layouts (2016, 365) are converted to the same model on load. All four
layouts switch at run time without a restart.

Also:

- **Icons:** Tabler outline icons (MIT), chosen by meaning, mapped in
  `ui-spec/icons.yaml` and tinted with theme tokens (ADR 0062). No
  third-party application artwork.
- **Themes:** per-layout colour, font and metric tokens (ADR 0063).
  Selawik is the UI font.
- **Selection contexts:** ten named contexts drive contextual tabs,
  context menus and a target availability table for every command (ADR
  0064).
- **New commands:** the commands the new controls need, added as stubs.
  Controls without an implemented command stay disabled with "Not yet
  implemented".

Tests:

- data validation and cross-references;
- count pins per layout;
- egui_kittest tests that open every menu and dispatch every enabled
  item;
- a command bar placement test;
- golden PNGs of both layouts;
- the context cross-check against the registry predicates;
- the wasm build.

Exit:

- all four layouts load and switch without a restart;
- every icon key draws its icon;
- every control dispatches its command;
- contextual tabs appear exactly in their contexts;
- the phase report shows Larrygram's own renders of both layouts.
