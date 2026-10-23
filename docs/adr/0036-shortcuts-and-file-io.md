# 0036: Shortcuts and file I/O

- Status: accepted
- Date: 2026-10-16
- Task: P4-01

## Context

KICKOFF Phase 4: "Keyboard shortcuts come from `shortcuts.yaml`"; "Open
and save use native dialogs"; "the wasm build uses file upload and
download". AGENTS.md section 6: programmatic calls never open dialogs.
`ui-spec/shortcuts.yaml` (ADR 0007) stores each shortcut as a sequence of
chords with a `CommandId` and a `scope`; ADR 0028's `NeedsInput` is the
dialog-free input seam. egui has its own built-in keys (text editing,
Ctrl+C/V in text fields) that must not be stolen.

## Decision

### Shortcut dispatch

`crates/ui::shortcuts` builds a `ShortcutMap` from `ui_spec::Shortcuts` at
startup (one per loaded `LayoutSet`):

```rust
pub struct ShortcutMap { /* chord-sequence trie -> CommandId */ }
impl ShortcutMap {
    pub fn from_shortcuts(shortcuts: &ui_spec::Shortcuts) -> Result<Self, UiError>;
    /// Matches typed chords against the map in the active scopes; one
    /// intent per completed sequence, in order (a chord that mismatches a
    /// half-typed sequence fires the held-back completion and then starts
    /// a new sequence, so one chord can complete two).
    pub fn resolve(
        &mut self,
        chords: &[KeyChord],
        active: &ActiveScopes,
    ) -> Result<Vec<Intent>, UiError>;
    /// True while a multi-chord sequence is half-typed (status bar hint).
    pub fn pending(&self) -> bool;
}
```

- A **chord** is modifiers + one key, normalized from egui key events to
  the `shortcuts.yaml` tokens (`Ctrl`, `Shift`, `Alt`, `A`, `F1`, ...).
  The mapping table (egui `Key`/modifiers to tokens) lives in
  `crates/ui::shortcuts` and is exhaustive over the tokens the file uses;
  an unknown token in the file is a load-time `UiError`, not a silent gap.
- **Multi-chord sequences** (keytips like `Alt+H, F, O`) are stateful: the
  map records the chords typed so far and resets on any mismatch or after
  2 seconds (the buffer lives in `crates/ui`, ADR 0029).
- **Scopes** filter by focus: `Global` always; `Page` when the canvas has
  focus; `Text` when an egui text edit has focus; `Selection` needs a
  selection (Phase 5: always false in Phase 4); `CommandBar` when the command bar
  has focus; `Window` always in the single-window Phase 4 shell. A shortcut
  whose scope does not match is not dispatched.
- **Conflicts:** the conflict key is the chord sequence **and** the
  scope. One sequence may be bound in several scopes (the committed
  table binds `Ctrl+C` in both `page` and `text`, and `Ctrl+Shift+C` to
  `format.text.align_center` in `text` and to `tools.format_painter` in
  `global`); each binding fires when its scope is active. When more
  than one bound scope is active, the most specific wins: `Text`, then
  `Selection`, `Page`, `CommandBar`, `Window`, `Global`. Two entries with the
  same sequence and scope: the first in file order wins and one warning
  is recorded on `AppCtx::warnings` at load, except when both name the
  same command (an exact repeat across sections of the source table is
  dropped silently). Sequences that are prefixes of each other are
  legal (the trie waits for the next chord). (Amended at the P4-08
  merge: the first rule keyed on the sequence alone and dropped 29 of
  the 90 committed entries.)
- **Precedence over egui:** the shell consumes matched chords before
  passing input to widgets (`egui::Context::input_mut` +
  `Event::consume`); egui's own shortcuts apply only inside text edits
  (scope `Text` defers). Inside a text edit egui keeps plain typing,
  the caret keys and its editing chords (`Ctrl` + `A`, `C`, `V`, `X`,
  `Z`, `Y`, `Backspace`, `Delete`; `Ctrl+Shift` + `Z` or an arrow);
  every other bound `Ctrl` or `Alt` chord dispatches (amended at the
  P4-14 merge). A matched chord that the registry reports
  `NotEnabled` for is consumed and shown as a disabled status-bar hint,
  never passed on to a widget.

Dispatch returns the `Intent`s (ADR 0028) with `Source::Shortcut`; the shell
executes it through the registry.

### File open and save

Commands (ADR 0028 argument names):

- `file.open` takes `path` (or nothing: `NeedsInput::Path { purpose: OpenFile }`).
- `file.save` saves the active document to its `path`; a never-saved
  document returns `NeedsInput::Path { purpose: SaveFile }`.
- `file.save_as` takes `path` (or `NeedsInput::Path { purpose: SaveFile }`).
- `file.close`, `file.new` behave the same way.

Reading and writing use `larrygram_vsdx::read_document(bytes, limits)` and
`write_document` (byte-based already, so wasm upload needs no new API).
The bytes come from `std::fs` on desktop and from the file dialog on wasm.

### Dialogs: the `FilePicker` seam (L6 only)

```rust
pub enum FileChoice {
    /// Desktop: a real path.
    Path(PathBuf),
    /// Wasm (or a picker that returns bytes): a name and the bytes.
    Bytes { name: String, data: Vec<u8> },
}
pub trait FilePicker {
    fn open(&self, request: &OpenRequest) -> Result<Option<FileChoice>, UiError>;
    fn save(&self, request: &SaveRequest, data: &[u8]) -> Result<Option<FileChoice>, UiError>;
}
```

`NativeFilePicker` (rfd) is the desktop implementation: `OpenRequest`
carries the `.vsdx/.vssx/.vstx` filters and a suggested name;
`SaveRequest` the default file name. On **wasm**, rfd's async dialog is
bridged: `spawn_local` posts the picked file into a channel the shell
polls, and "save" is a browser download of the written bytes (an object
URL), so `FileChoice::Bytes` is the web app's normal case.

Only the shell calls `FilePicker`, and only in response to
`CommandError::NeedsInput` from an **interactive** `Source`. A programmatic
`file.open` without `path` returns `NeedsInput` to its caller and no
dialog ever appears (AGENTS.md section 6).

## Consequences

- Shortcuts are data-driven; the screenshot pass can extend
  `shortcuts.yaml` without code changes beyond the token table.
- The wasm app gets open (upload) and save (download) without a
  filesystem; the desktop app gets native dialogs.
- The registry sees one dispatch path for keys, command bar, menus, CLI and
  (Phase 8) MCP.

## Alternatives considered

- **egui's built-in shortcut handling.** Rejected: egui binds widget-local
  keys, not command ids, and its bindings are compile-time; our map is
  data-driven and must win outside text edits.
- **`global-hotkey` or OS accelerators.** Rejected: window-level only, no
  chord sequences, no scope model.
- **Opening dialogs inside `file.open`.** Rejected outright: AGENTS.md
  section 6; it would also make Phase 8 untestable.
- **A virtual filesystem layer for wasm.** Rejected: overkill; bytes in
  and out cover upload and download.
- **Chords as bitflags with no sequence support.** Rejected: `shortcuts.yaml`
  is m2 (sequences of chords, ADR 0007); keytips need it.
