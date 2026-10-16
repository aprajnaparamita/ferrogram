# 0031: Ribbon from ui-spec

- Status: accepted
- Date: 2026-10-16
- Task: P4-01

## Context

KICKOFF Phase 4: "Ribbon: generated from `ui-spec`, covering tabs, groups
and controls. Galleries can start as basic dropdowns." Both layouts bind to
the same command registry. The layout data (`ui-spec/panel-2016.yaml`,
`panel-365.yaml`) is committed, schema-validated YAML keyed by Fluent id
(ADR 0007), with `commands.yaml` mapping ids to Ferrogram command ids
(ADR 0006). The P1-14 note flagged repeated Fluent ids inside one container
(`TabSave` x9 in one backstage tab) as something the ribbon builder must
rule on.

## Decision

### Load at runtime from the committed YAML, embedded at build time

The ribbon is built from the **committed YAML at runtime**, not from
build-time-generated Rust. The two layout files, `commands.yaml` and
`shortcuts.yaml` are embedded with `include_str!` in `crates/ui-spec`
(`ui_spec::embedded`, added by P4-04) and parsed by the same
`Loader` code path the tests use on disk. Rationale:

- One source of truth: the YAML is extractable, diffable and validated
  (ADR 0007); codegen of a whole ribbon tree would duplicate it.
- Wasm has no filesystem; embedding is the only way the web app loads the
  layouts. Parsing ~25,000 lines of YAML at startup costs milliseconds and
  happens once.
- The later screenshot pass enriches the YAML (`bounds`, `icon_ref`, ...)
  and the ribbon picks the data up without a rebuild of generated code.

`Loader::load_all` on disk stays for tests, the extractors and the parity
generator; `Loader::from_embedded` parses the embedded strings through the
same validation minus the schema file read (the schema is checked in tests,
`crates/ui-spec/tests/schema.rs`, at the gate).

### Widget mapping

One rule per control type; all are plain egui widgets in Phase 4:

| ui-spec type | egui rendering |
|---|---|
| `tab` | a tab button in the ribbon tab strip; the active tab shows its groups |
| `group` | a titled vertical panel with a bottom caption; controls flow in rows |
| `button` | `egui::Button` with the control's `label` |
| `toggleButton` | `egui::Button` with `selected` styling |
| `splitButton` | a button plus a small dropdown arrow button in one row |
| `gallery`, `dropDown`, `menu` | one button that opens an `egui::ComboBox`-style dropdown listing the item labels |
| `checkBox` | `egui::Checkbox` |
| `comboBox` | `egui::ComboBox` |
| `labelControl`, `control`, `none` | `egui::Label` (non-actionable) |
| `button (dialogBoxLauncher)` | a small corner button in the group caption |
| backstage `tab` | entries behind a File button; `kind: backstage` shows a full-window vertical stack |
| `contextMenu` | Phase 4 renders it only on right-click on the canvas, as `egui::Button`s in a popup |
| `task`, `taskFormGroup`, `category` | backstage section headings |

Galleries are basic dropdowns (KICKOFF); `size_variant` and `icon_ref` are
ignored until the screenshot pass fills them in.

### Control to command

A control's `command` field (the ADR 0007/0006 link) is looked up in the
registry (`Registry::get`). Dispatch returns an `Intent` (ADR 0028) that
the shell executes; widgets never call `execute` themselves. States:

1. **Implemented** (`CommandStatus::Implemented` and `enabled`): normal
   widget; greyed while `enabled` is `Enabled::no(reason)` with the reason
   as tooltip.
2. **Stub or unmapped**: rendered **disabled** with the tooltip
   "Not yet implemented" (KICKOFF). A control with no `command` link at
   all is disabled the same way.
3. **Out of scope** (`ui-spec/out-of-scope.txt`): rendered disabled with
   the tooltip "Out of scope for this run".

Labels come from the layout's `label` field (short UI labels, allowed by
AGENTS.md section 4). Tooltip text is ours.

### Repeated ids and placements

Adopting the P1-14 note as a rule (recorded as an ADR 0007 amendment): a
Fluent id is **not unique within a layout**. A placement is identified by
its path (`tab/TabHome/GroupClipboard/PasteGallery`, the parity matrix's
`path` column). The ribbon renders every placement it finds, including
repeats inside one container (`TabSave` x9, `LineStylesDialog` twice in one
group): each occurrence is a separate widget sharing one `CommandId`.
Command ids stay unique (enforced by the registry). Extractor artefacts
surfacing as obvious duplicates (same id, same label, same order in one
container) are deduplicated at render time with a documented "first wins"
rule, so a P1-04 extractor quirk does not paint nine Save buttons.

### Quick access toolbar

`Layout::quick_access_toolbar` renders as a small row in the window title
strip with the same control-to-command rules. A Phase 4 QAT button is just
a placement; no customization (that is out of scope for this run).

## Consequences

- The ribbon is data-driven: no ribbon widget code changes when the
  screenshot pass enriches the YAML.
- Both layouts share all widget code; switching is a data swap (ADR 0032).
- Embedded YAML grows the `ferrogram-ui-spec` binary by ~700 KB of text;
  acceptable, and it is the same data the CLI and tests use.
- Deduplication is display-only; the parity matrix still lists every
  placement.

## Alternatives considered

- **Build-time codegen of Rust ribbon tables.** Rejected: a second
  representation of the layout to keep in sync; the screenshot pass would
  regenerate code every time; wasm needs the data embedded anyway.
- **Runtime load from files on disk.** Rejected: no filesystem on wasm;
  shipping loose YAML beside the binary complicates installation.
- **Skip repeated placements.** Rejected: the repeats are part of the
  extracted truth; hiding them hides extractor bugs from review. Display
  dedup only where id + label + order match exactly in one container.
- **Map controls to widgets by Fluent id.** Rejected: ids are names, not
  types; the `type` field is the stable discriminator (ADR 0007).
- **Our own ribbon layout YAML parallel to the Fluent data.** Rejected:
  KICKOFF requires the structure to come from Microsoft's
  control-identifier data.
