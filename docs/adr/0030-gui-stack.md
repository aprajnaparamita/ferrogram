# 0030: GUI stack

- Status: accepted
- Date: 2026-10-16
- Task: P4-01

## Context

Phase 4 builds the eframe desktop app and keeps the wasm build green.
KICKOFF requires egui/eframe, `egui_kittest` for UI tests and native
dialogs; AGENTS.md section 6 forbids anything below L6 seeing egui, eframe,
winit or rfd (ADR 0002, `cargo xtask layers`), section 4 requires clean-room
fonts and icons and no Microsoft assets. `cargo deny check` must pass on
the whole new dependency tree, and the macOS and wasm builds must work.

## Decision

### Versions and crates

| Crate | Version | Licence | Where | Why |
|---|---|---|---|---|
| `egui` | 0.37 | MIT OR Apache-2.0 | `crates/ui` (L6) | immediate-mode UI; widgets for the ribbon and panels |
| `eframe` | 0.37 | MIT OR Apache-2.0 | `apps/ferrogram`, `apps/ferrogram-web` (L7) | window + event loop + storage, desktop and wasm |
| `egui_kittest` | 0.37 | MIT OR Apache-2.0 | dev-dependency of `crates/ui`, `apps/ferrogram` | headless UI tests (KICKOFF) |
| `rfd` | 0.17 | MIT OR Apache-2.0 | `crates/ui` (L6) | native file dialogs on desktop (KICKOFF) |
| `serde_json` | 1 | MIT OR Apache-2.0 | `crates/ui` (L6) | the settings slot format (ADR 0032); added by P4-03 |
| `wasm-bindgen`, `wasm-bindgen-futures`, `js-sys`, `web-sys` | 0.2 / 0.4 / 0.3 / 0.3 | MIT OR Apache-2.0 | `crates/ui` and `apps/ferrogram-web`, wasm32 only | the async upload bridge and the download save (ADR 0036); added by P4-03 |

`egui`, `eframe` and `egui_kittest` are released in lockstep; the three are
pinned to the same 0.x minor. `rfd` is the only dialog crate; on wasm the
web app uses `rfd`'s async file dialog bridged to the sync `FilePicker`
trait (ADR 0036), not a second dependency.

### Renderer backend: `glow`

`eframe` is built with its `glow` backend (OpenGL), not `wgpu`
(`default-features = false` plus the `glow` feature; see Licences below),
on desktop and wasm:

- Smaller tree: no `wgpu`/`naga`/`wgpu-hal`, faster cold builds in every
  agent worktree (the gate builds the workspace from scratch per task).
- The Phase 4 canvas draws one raster texture plus immediate-mode widgets
  (ADR 0033); `glow` is more than enough for that.
- `wgpu` stays the documented upgrade path if the canvas moves to vector
  rendering or compute (Phase 5/6 revisits).

Everything GUI stays in L6/L7: `crates/ui` depends on `egui` (and `rfd`),
the apps depend on `eframe`. `cargo xtask layers` already encodes this.

### Tests: `egui_kittest`

UI tests run headless through `egui_kittest::Harness` (which drives the
AccessKit tree): ribbon generation for both layouts, layout switching,
enabled/disabled state, shortcut dispatch (KICKOFF Phase 4). The same
harness backs the `ui` and app crates' 50% coverage lines (ADR 0037).
Plain `egui::Context` tests are allowed for pure widget logic.

### Fonts in the UI chrome

Phase 4 chrome text uses egui's `default_fonts` (the egui crate's embedded
UFL-1.0 font, supplied by the dependency, never a Microsoft font; a row in
`ATTRIBUTION.md` records it). Visio's own chrome font (Segoe UI) is
represented by Selawik inside `crates/text` (ADR 0024) and is already used
for canvas text. Making chrome use Selawik needs a public font-bytes
accessor in `crates/text`; that is a Phase 4 open issue and a screenshot-
pass item, not a Phase 4 requirement (fidelity is not a goal of this run).

### Icons: placeholders

Controls render with simple placeholder icons drawn by `crates/ui`
(squares, letters, egui built-in glyphs). No icon set is committed in
Phase 4, so `ATTRIBUTION.md` gains no icon rows. If a later pass picks an
open-licensed set, its licence goes in `ATTRIBUTION.md` in the same commit
(AGENTS.md section 4). No Office icons, ever.

### Licences in `deny.toml`

The GUI tree needed two allowances; each is in `deny.toml` and justified
here:

- **`epaint_default_fonts`** (egui's dependency-supplied chrome fonts) is
  `(MIT OR Apache-2.0) AND OFL-1.1 AND Ubuntu-font-1.0`. The Ubuntu Font
  Licence v1.0 is permissive (free use, modification and redistribution
  with the notice kept) and no Microsoft font is involved. A per-crate
  `[[licenses.exceptions]]` entry allows exactly those four licenses for
  that crate, and P4-03 records the font set in `ATTRIBUTION.md`.
- **`RUSTSEC-2026-0192`** (`ttf-parser` unmaintained) is ignored with a
  reason: it reaches us only through `winit` -> `sctk-adwaita` ->
  `ab_glyph` (Wayland window decoration font parsing), there is no
  vulnerability and no fixed release. Revisit when winit replaces it.

**No MPL allowance is needed.** The wasm binding crates (`wasm-bindgen`,
`js-sys`, `web-sys`) are `MIT OR Apache-2.0`, checked in this task; the
ADR 0003 rule ("`ferrogram-vsd-binary` is the only MPL crate") stands
unchanged and `cargo deny` keeps enforcing it.

`eframe` is built with `default-features = false` plus `accesskit`,
`clipboard`, `default_fonts`, `glow`, `persistence`, `wayland`, `x11` on
desktop and `default_fonts`, `glow`, `persistence`, `web_screen_reader`
on wasm: the `wgpu` feature is off (this ADR's backend decision) and
`persistence` backs the settings store (ADR 0032). eframe's own default
list enables `winit/default`; a dependency feature list cannot contain
slashes, so `apps/ferrogram` depends on `winit = "0.30"` directly (same
line eframe uses) to keep winit's default features (`rwh_06`, `x11`,
`wayland`, `wayland-dlopen`, `wayland-csd-adwaita`).

## Consequences

- `cargo deny check` runs over the full GUI tree at every gate.
- The wasm build (`cargo build -p ferrogram-web --target wasm32-unknown-unknown`)
  now compiles eframe for `wasm32-unknown-unknown`; it stays a gate step.
- macOS build uses `winit`/`glutin` transitively through eframe; nothing in
  L0..L5 changes (layers step stays green).
- Cold builds get heavier (eframe tree); `CARGO_TARGET_DIR` per task
  (AGENTS.md section 2) keeps them isolated.

## Alternatives considered

- **`wgpu` backend.** Rejected for Phase 4: large tree (naga, wgpu-hal),
  slow cold builds, no Phase 4 feature needs it. Revisit for a vector
  canvas.
- **`iced`, `slint`, `gtk-rs`, `egui` + `baseview`.** Rejected: KICKOFF
  names eframe/egui; the others change the whole UI architecture.
- **`native-dialog` instead of `rfd`.** Rejected: `rfd` supports wasm and
  native from one API and is the well-maintained default in the egui
  ecosystem.
- **Bundling an icon set now.** Rejected: no set chosen yet, and
  placeholders satisfy the alpha; the later screenshot pass picks one with
  its licence.
- **Chrome font = Selawik now.** Deferred: needs a `crates/text` API
  change outside this task's ownership; no fidelity goal in this run.
- **Banning MPL-2.0 transitively (no wasm).** Not needed: the wasm
  binding crates are `MIT OR Apache-2.0` (checked in this task), so the
  wasm build adds no MPL crate at all.
- **`wgpu` as the eframe backend feature set.** Rejected for Phase 4:
  eframe's default features pull `egui-wgpu`/`naga`; `default-features =
  false` with `glow` keeps the tree to the OpenGL path.
