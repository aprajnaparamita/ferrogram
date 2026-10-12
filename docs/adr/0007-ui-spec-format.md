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
  (`bounds`, `icon_ref`, `keytip`, `size_variant`, `visual_notes`)
  without schema changes.
- Validated at extractor time and at load time, so a typo in an extractor
  fails the gate instead of crashing the app at runtime.
- Keyed by **Fluent control names** (not by Ferrogram command ids), so
  the data round-trips with the Microsoft control-identifier spreadsheets
  review's blocker finding B1: the earlier draft used command ids as
  control ids, which would break every cross-reference.

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

Format: **YAML**, parsed by `serde-saphyr` (MIT, pure Rust, no
`unsafe-libyaml` C dependency) in the loader and extractors.
Validated by a **JSON Schema** (`ui-spec/schema.json`) using
`jsonschema` (MIT, latest 0.x as of Phase 1 start) at both ends.
YAML is a superset of JSON, so the schema is a normal JSON Schema;
we do not use YAML-specific keywords.

The first P1-02 review draft chose `serde_yaml` (archived upstream)
and then `serde_yml` (the maintained `serde_yaml` fork); both were
rejected in the P1-02 review follow-up. `serde_yaml` is published
as `0.9.34+deprecated` and `serde_yml` triggers cargo-deny
advisories RUSTSEC-2025-0068 (unsound) and RUSTSEC-2026-0333
(unmaintained). `serde-saphyr` is a pure-Rust, dependency-light
crate with no advisories at the time of this review; the fallback
if advisories reappear is `serde_norway` (also pure Rust, MIT,
Embark Studios). Both produce identical YAML 1.2 output for our
data, so the schema and the committed YAMLs do not change.

### Canonical source for Microsoft 365

`panel-365.yaml` is generated from the **Current Channel** spreadsheet
at `reference/fluent-ui-ids/Microsoft 365/Current Channel/visiocontrols.xlsx`
(1,161 data rows as of the P1-01 snapshot). Current Channel is the
Microsoft 365 channel that rolls out features first; Semi-Annual and
Monthly Enterprise Channels ship later and are kept in `reference/` for
reference but are not used by the extractor. If a future phase needs
to track a different channel, only the input path in
`xtask/src/extract/fluent.rs` changes; the schema and the file are
unchanged. The other 365 channels are listed in `reference/SOURCES.md`
under source 1.

### Files

- `ui-spec/schema.json` — JSON Schema (Draft 7). One schema validates:
  - `panel-2016.yaml`, `panel-365.yaml` (ribbon + backstage + context
    menus).
  - `commands.yaml` (command-id ↔ VisUICmds ↔ Fluent control mappings).
  - `shortcuts.yaml` (keyboard shortcuts).
- `ui-spec/panel-2016.yaml` — Office 2016 Fluent control data, extracted
  from `reference/fluent-ui-ids/Office 2016/visiocontrols.xlsx`.
- `ui-spec/panel-365.yaml` — Microsoft 365 Current Channel, from
  `reference/fluent-ui-ids/Microsoft 365/Current Channel/visiocontrols.xlsx`.
- `ui-spec/commands.yaml` — command-id ↔ VisUICmds ↔ Fluent control
  mappings, extracted from sources 1 + 2 in `docs/sources.md`.
- `ui-spec/shortcuts.yaml` — shortcut table, extracted from source 5.
- `ui-spec/diff-2016-vs-365.md` — generated structural diff of the two
  layouts (a markdown table of Fluent ids in only one, only the other,
  or both, with control type and ribbon path). Always regenerated.
- `docs/parity/controls.md` — every Fluent control id in both layouts,
  with a status: `implemented`, `stub`, or `out-of-scope`. Initial state
  is `stub` (nothing implemented yet) and `out-of-scope` for Microsoft
  365 Fluent control ids we know we will not implement. Always
  regenerated.

### Top-level shape of a layout file

```yaml
# ui-spec/panel-365.yaml
schema_version: 1                 # schema/format version of the file
product: visio                    # always "visio" for now
source: "Microsoft Fluent UI 365, Current Channel"
source_url: "https://github.com/OfficeDev/office-fluent-ui-command-identifiers"
source_revision: "Current Channel, 2024-03-18 (commit b230a0df45036b2d5e8b49ebf368b3f90ada8d63)"
# No `extracted:` timestamp: it would change on every `cargo xtask
# extract` run and break ADR 0009's deterministic-output rule.
# `source_revision` is the pinned source identifier.

tabs:                             # ordered list of top-level tabs (excl. file/backstage)
  - id: TabHome                   # Fluent name, verbatim (see B1)
    label: Home
    order: 0
    groups:
      - id: GroupClipboard
        label: Clipboard
        order: 0
        controls:
          - id: PasteGallery
            type: gallery
            label: Paste
            order: 0
            command: edit.paste   # optional Ferrogram command id
            # The five fields below are REQUIRED in the schema
            # (with type [..., "null"]) so the screenshot pass can
            # fill them in without further schema work:
            bounds: null
            icon_ref: null
            keytip: null
            size_variant: null
            visual_notes: null

# Backstage appears under the File tab. Same shape as a tab but with a
# `kind: backstage` discriminator. Backstage tabs are flat (no groups).
backstage:
  kind: backstage
  tabs:
    - id: FileNew
      label: Info
      order: 0
      controls:
        - id: FileSave
          type: button
          label: Save
          order: 0
          command: file.save
          bounds: null
          icon_ref: null
          keytip: null
          size_variant: null
          visual_notes: null

# Context menus are flat: every menu is a list of controls keyed by id.
# The generator derives the menu id from the Fluent data:
# "<context-menu-name>".
context_menus:
  - id: ShapeShortcutMenu
    label: Shape shortcut menu
    order: 0
    controls:
      - id: Cut
        type: button
        label: Cut
        order: 0
        command: edit.cut
        bounds: null
        icon_ref: null
        keytip: null
        size_variant: null
        visual_notes: null
```

### Control type vocabulary

Exactly the strings found in the Fluent data, lower-cased and de-spaced.
Three "None (...)" placeholders are folded to `none` and dropped (they
represent empty cells). The vocabulary is enumerated in the JSON Schema
under `controlType` so an unknown type fails validation.

### Reserved fields

The five reserved fields for the screenshot pass are **required** on
every control, group, tab, backstage tab, context menu entry, and
context menu control, with the type `[<shape>, "null"]`. The schema
enforces this directly; no custom lint is needed. The P1-02 review
finding M4 removed the "non-required + custom lint" rule from an
earlier draft. The five fields are:

- `bounds` — `{x, y, w, h}` in device-independent pixels, or `null`.
- `icon_ref` — path or id of an icon asset, or `null`.
- `keytip` — the keyboard keytip shown next to the label, or `null`.
- `size_variant` — `null`, `"small"`, `"medium"`, `"large"`, or
  `"extra-large"`.
- `visual_notes` — free-text annotation, never a description of
  behaviour, or `null`.

### Multiple parents and ancestors

The Fluent spreadsheet has four parent columns: `Parent Control`,
`Secondary Parent Control`, `Tertiary Parent Control`, plus a
`Group/Context Menu Name`. Each control in the YAML refers to its
**immediate parent** by Fluent id (`parent` is a `FluentId` or `null`).
The loader walks `parent` links to build the tree. Tab membership
comes from the `Tab Set` + `Tab` columns, recorded as `tab` and
`tab_set` on the top tab entry (not on every control).

This avoids the cyclic-parent trap in the Fluent data: the same
control can be a member of two groups in two ribbons (e.g.
`ConnectorTool` appears in the Home ribbon's Tools group and the
Insert ribbon's Shapes gallery). In our format the control appears
once with two parent links collapsed: the **primary** parent is the
one in the same ribbon as the control's `tab` field; alternate paths
go in a sibling `aliases: [parent_in_other_ribbon]` field. P1-03 picks
the primary parent by `tab` match.

### Commands and shortcuts

- `commands.yaml` schema is in the same `ui-spec/schema.json`, under
  `$defs/command`. Each entry: `id` (a Ferrogram `CommandId`,
  pattern `^[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*)+$`),
  `label`, `visuicmd` (a `visCmd*` string or `null`,
  pattern `^visCmd[A-Za-z0-9]+$` and `type: ["string", "null"]`),
  `fluent_ids` (list of Fluent control names, pattern
  `^[A-Za-z][A-Za-z0-9_]*$`), `mapping_confidence` (`exact` |
  `inferred` | `unknown`). Every `fluent_ids` entry must appear in
  at least one layout file; the loader checks this with
  `LoadError::CrossReference { kind: "fluent_id", .. }`.
- `shortcuts.yaml` schema is under `$defs/shortcut`. A shortcut is a
  **sequence of chords** (m2): the user types one chord (a set of
  modifiers plus one key, e.g. `[Ctrl, Z]`), then the next, until the
  sequence is complete. Visio's ribbon keytips (Alt + H, F, O) are
  naturally represented as a two-chord sequence
  `[[Alt, H], [F, O]]`. Each chord is an unordered set of keys
  (modifiers + one key); the sequence is the order the user types
  them. Each entry: `sequence` (a list of chords, each chord is a
  list of key tokens), `command` (a `CommandId` from `commands.yaml`),
  `scope` (`global` | `ribbon` | `page` | `text` | `selection` |
  `window`). An optional `section` records the human-readable section
  from the source page (e.g. "Frequently used shortcuts").

### Cross-references

The schema validates per file. Three cross-references are checked by
the loader (`crates/ui-spec`) and by the dispatcher
(`xtask/src/extract/mod.rs`):

1. Every `fluent_ids` entry in `commands.yaml` must appear in at
   least one layout (`Layout.fluent_ids`).
2. Every `command` value on a control in any layout must appear in
   `commands.yaml`.
3. Every `command` value in `shortcuts.yaml` must appear in
   `commands.yaml`.

A failure is `LoadError::CrossReference { kind, id, referenced_by }`
(the dispatcher surfaces it as `extract[*]: FAIL: ...`).

### Validation

- **At extract time** (`cargo xtask extract`): each generator loads the
  schema once and validates its output before writing. A failed
  validation aborts the extractor.
- **At load time** (`crates/ui-spec::Loader::load`): the loader
  validates every YAML it reads against the schema, then performs the
  three cross-references. A mismatch is `Err(LoadError::..)`.
- **In tests** (`crates/ui-spec/tests/schema.rs`): the loader
  round-trips a known-good minimal fixture through validation; an
  intentionally bad fixture is rejected; the committed
  `panel-2016.yaml` / `panel-365.yaml` validate once they exist (a
  `#[ignore]` test until then).

## Consequences

- A new control type or new field can be added without changing the
  schema, as long as it fits into the existing `properties` (the
  screenshot-pass fields are already required-but-null).
- Two layouts (2016 and 365) with the same schema, and a diff that
  requires no schema knowledge.
- The loader does not need to know what a "tab" is in Microsoft Office:
  tabs, groups and controls are all controls in Fluent, and the
  `type` field discriminates them.
- Cross-references between layout files, `commands.yaml` and
  `shortcuts.yaml` are validated together, so a typo in one is caught
  immediately.
- A `serde_yml` regression is contained to the loader. The schema is
  the contract.
- Layout control ids are Fluent names; commands are Ferrogram ids. The
  two namespaces are explicitly distinct in the schema (`fluentId` vs
  `commandId` patterns) and in `crates/ui-spec` (`FluentId` vs
  `CommandId` newtypes).

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
- **Key control ids by command ids** (the earlier draft). Rejected by
  the P1-02 review (B1): it would break the parity matrix, the
  layouts and `commands.yaml`.
- **Stay on `serde_yaml`.** Rejected (M5): it is archived upstream
  (`serde_yaml = "0.9.34+deprecated"`) and is flagged as unmaintained
  in `cargo deny`'s advisory database. `serde_yml` is the maintained
  drop-in successor.
- **Use `serde_yml`.** Rejected by the P1-02 review follow-up:
  `serde_yml` triggers `cargo deny check advisories` (RUSTSEC-2025-0068
  and RUSTSEC-2026-0333). `serde-saphyr` is the chosen replacement;
  `serde_norway` is the documented fallback.
- **Pin `jsonschema` to 0.18.** Rejected (M5): the 0.18 line is no
  longer the latest. We use the latest 0.x as of Phase 1 start.
