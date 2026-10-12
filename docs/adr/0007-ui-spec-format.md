# 0007: ui-spec YAML format and JSON Schema

- Status: accepted
- Date: 2026-10-12
- Task: P1-02

## Context

The ribbon and menu layout data must be:

- Human-readable and diffable, so we can review what an extractor changed.
- Independent of the Rust code, so the loader (`crates/ui-spec`) and the
  generators (`xtask/src/extract/*`) agree on shape.
- Stable across the screenshot pass: a later pass enriches data
  (`bounds`, `icon_ref`, `keytip`, `size_variant`, `visual_notes`) without
  schema changes.
- Validated at extractor time and at load time, so a typo in an extractor
  fails the gate instead of crashing the app at runtime.

The data covers three surfaces that have different shapes in Fluent XML but
must be expressed in one schema:

- The **ribbon**: tab → group → control tree, with a tab set and a quick
  access toolbar (QAT) on top.
- **Backstage**: tabs behind the File menu (New, Open, Save, Print, ...).
- **Context menus**: per-canvas and per-shape menus.

P1-01 found the Fluent data is in `.xlsx` files with nine columns: `Control
Name`, `Control Type`, `Tab Set`, `Tab`, `Group/Context Menu Name`, `Parent
Control`, `Secondary Parent Control`, `Tertiary Parent Control`, `Policy
ID`. The 19 control types in column B include `tab`, `tabSet`, `group`,
`button`, `toggleButton`, `splitButton`, `gallery`, `menu`, `checkBox`,
`dropDown`, `comboBox`, `labelControl`, `control`,
`button (dialogBoxLauncher)`, `contextMenu`, `task`, `taskFormGroup`,
`category`, plus three "None (...)" placeholders.

## Decision

Format: **YAML**, parsed by `serde_yaml` (MIT) in the loader and
`serde_yaml` again in the extractors. Validated by a **JSON Schema**
(`ui-spec/schema.json`) using the `jsonschema` crate (MIT) at both ends.
YAML is a superset of JSON, so the schema is a normal JSON Schema; we do
not use YAML-specific keywords.

YAML loader dependency: `serde_yaml` 0.9 (MIT) — already in the
crates.io registry. JSON Schema validator: `jsonschema` 0.18 (MIT) or
`valico`; `jsonschema` is the choice because it is the most-used and the
most standard (Draft 7). Both are permissive-license and pass `deny.toml`.

### Files

- `ui-spec/schema.json` — JSON Schema (Draft 7). One schema validates:
  - `visio-2016.yaml`, `visio-365.yaml` (ribbon + backstage + context menus).
  - `commands.yaml` (command-id ↔ VisUICmds ↔ Fluent control mappings).
  - `shortcuts.yaml` (keyboard shortcuts).
- `ui-spec/visio-2016.yaml` — Office 2016 Fluent control data, extracted
  from `reference/fluent-ui-ids/Office 2016/visiocontrols.xlsx`.
- `ui-spec/visio-365.yaml` — Microsoft 365 Current Channel, from
  `reference/fluent-ui-ids/Microsoft 365/Current Channel/visiocontrols.xlsx`.
- `ui-spec/commands.yaml` — command-id ↔ VisUICmds ↔ Fluent control
  mappings, extracted from sources 1 + 2 in `docs/sources.md`.
- `ui-spec/shortcuts.yaml` — shortcut table, extracted from source 5.
- `ui-spec/diff-2016-vs-365.md` — generated structural diff of the two
  layouts (a markdown table of control ids in only one, only the other, or
  both, with control type and ribbon path). Always regenerated.
- `docs/parity/controls.md` — every control id in both layouts, with a
  status: `implemented`, `stub`, or `out-of-scope`. Initial state is
  `stub` (nothing implemented yet) and `out-of-scope` for Microsoft 365
  control ids we know we will not implement. Always regenerated.

### Top-level shape of a layout file

```yaml
# ui-spec/visio-365.yaml
schema_version: 1                 # schema/format version of the file
product: visio                    # always "visio" for now
source: "Microsoft Fluent UI 365, Current Channel"
source_url: "https://github.com/OfficeDev/office-fluent-ui-command-identifiers"
source_revision: "Current Channel, 2024-03-18 (commit b230a0df45036b2d5e8b49ebf368b3f90ada8d63)"
extracted: "2026-10-12T00:00:00Z"  # timestamp; never used in code

tabs:                             # ordered list of top-level tabs (excl. file/backstage)
  - id: home
    label: Home
    order: 0
    groups:
      - id: home.toolbox
        label: Tool
        order: 0
        controls:
          - id: pointer.tool
            type: button
            label: Pointer Tool
            order: 0
            # Reserved for the screenshot pass (empty by default):
            bounds: null
            icon_ref: null
            keytip: null
            size_variant: null
            visual_notes: null
            # Optional mapping to commands.yaml (the control id IS the command id
            # when the control is actionable; this list is informational):
            alt_ids: []

# Backstage appears under the File tab. Same schema as a tab but with a
# `kind: backstage` discriminator. Backstage tabs are flat (no groups).
backstage:
  kind: backstage
  tabs:
    - id: file.info
      label: Info
      order: 0
      controls:
        - id: file.save
          type: button
          label: Save
          order: 0
          ...

# Context menus are flat: every menu is a list of controls keyed by id.
# The generator derives the menu id from the Fluent data:
# "<context-menu-name>".
context_menus:
  - id: shape.context
    label: Shape shortcut menu
    order: 0
    controls:
      - id: shape.cut
        type: button
        label: Cut
        order: 0
        ...
```

### Control type vocabulary

Exactly the strings found in the Fluent data, lower-cased and de-spaced.
Three "None (...)" placeholders are folded to `none` and dropped (they
represent empty cells). The vocabulary is enumerated in the JSON Schema
under `ControlType` so an unknown type fails validation.

### Reserved fields

The five reserved fields for the screenshot pass are present on every
control, set to `null` by the extractors. The schema does not require them
(non-required), but a custom lint in `xtask/src/extract` rejects layout
files that omit them entirely, so the screenshot pass can fill them in
without further schema work.

### Multiple parents and ancestors

The Fluent spreadsheet has four parent columns: `Parent Control`,
`Secondary Parent Control`, `Tertiary Parent Control`, plus a
`Group/Context Menu Name`. Each control in the YAML refers to its
**immediate parent** by id (`parent` is a string or `null`). The loader
walks `parent` links to build the tree. Tab membership comes from the
`Tab Set` + `Tab` columns, recorded as `tab` and `tab_set` on the top
tab entry (not on every control).

This avoids the cyclic-parent trap in the Fluent data: the same control
can be a member of two groups in two ribbons (e.g. `ConnectorTool`
appears in the Home ribbon's Tools group and the Insert ribbon's
Shapes gallery). In our format the control appears once with two
parent links collapsed: the **primary** parent is the one in the same
ribbon as the control's `tab` field; alternate paths go in `alt_ids`
or in a sibling `aliases: [parent_in_other_ribbon]` field. P1-03 picks
the primary parent by `tab` match.

### Commands and shortcuts

- `commands.yaml` schema is in the same `ui-spec/schema.json`, under
  `$defs/commands`. Each entry: `id`, `label`, `visuicmd` (string or
  `null`), `fluent_ids` (list of strings), `mapping_confidence`
  (`exact` | `inferred` | `unknown`). All cross-references to control
  ids in `visio-2016.yaml` / `visio-365.yaml` are checked at validation
  time: every `fluent_ids` entry must appear in at least one layout file.
- `shortcuts.yaml` schema is under `$defs/shortcuts`. Each entry:
  `keys` (a list of chord tokens, e.g. `[Ctrl, Z]`), `command`
  (a command id from `commands.yaml`), `scope` (`global` | `ribbon` |
  `page` | `text` | `selection` | `window`). The `keys` field is
  normalised to a sorted, lowercased list of tokens; chord syntax
  (`Ctrl+Shift+Z`) is also accepted for backward-compat in the
  extractor output, but the schema prefers the list form.

### Validation

- **At extract time** (`cargo xtask extract`): each generator loads the
  schema once and validates its output before writing. A failed
  validation aborts the extractor.
- **At load time** (`crates/ui-spec::Loader::load`): the loader validates
  every YAML it reads against the schema, then cross-checks the
  `fluent_ids` references in `commands.yaml` against the layout. A
  mismatch is `Err(LoadError::UnknownFluentId(...))`.
- **In tests** (`crates/ui-spec/tests/schema.rs`): the loader round-trips
  a known-good minimal fixture through validation; an intentionally bad
  fixture is rejected; the committed `visio-2016.yaml` /
  `visio-365.yaml` validate once they exist (a `#[ignore]` test until
  then).

## Consequences

- A new control type or new field can be added without changing the
  schema, as long as it fits into the existing `properties` (the
  screenshot-pass fields are already there).
- Two layouts (2016 and 365) with the same schema, and a diff that
  requires no schema knowledge.
- The loader does not need to know what a "tab" is in Microsoft Office:
  tabs, groups and controls are all controls in Fluent, and the
  `type` field discriminates them.
- Cross-references between layout files, `commands.yaml` and
  `shortcuts.yaml` are validated together, so a typo in one is caught
  immediately.
- A `serde_yaml` regression (it is in maintenance mode) is contained to
  the loader. The schema is the contract.

## Alternatives considered

- **Use a binary format** (RON, bincode, protobuf). Rejected: bad for
  diffs, bad for the screenshot pass, and the data is tiny (one
  thousand rows).
- **Use TOML instead of YAML.** TOML cannot express nested lists of
  mixed records as cleanly, and the Fluent data is tree-shaped.
- **Split backstage, context menus and ribbon into three files.**
  Rejected: harder to keep them in sync (a control can be on a ribbon
  and in a context menu), and the schema is the same.
- **Embed the schema in each YAML file.** Rejected: duplicates the
  schema; the schema is committed once at `ui-spec/schema.json`.
