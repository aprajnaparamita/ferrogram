# Phase 7 plan: Larrygram, the Panel Modern and Panel Classic layouts, the welcome wizard

- Status: accepted (2026-10-23); the wizard copy and the button label confirmed by the user
- Inputs: the Phase 7 package (`/Volumes/Dara/ferrogram-phase7/`, its
  `docs/phase-7/README.md` and draft tasks P7-01 to P7-14), the user's
  brief of 2026-10-23 and the graphics in `graphics/`

## 1. Decisions taken

| Topic | Decision |
|---|---|
| Phase numbers | Phase 7 is now the Panel Modern and Panel Classic layouts, the rename and the wizard. Binary `.vsd` import becomes Phase 8. Agents (control channel and MCP) becomes Phase 9 and is paused. |
| Name | Ferrogram becomes **Larrygram** everywhere: crates (`larrygram-*`), binaries, `LARRYGRAM_*` variables, docs, and the folders `/Users/dara/larrygram` and `/Volumes/Dara/larrygram`. The app shows **Larrygram v1.0 alpha**; the workspace version becomes `1.0.0-alpha.1`. |
| Layout names | **Panel Modern** (id `panel-modern`, the 2013 data) and **Panel Classic** (id `panel-classic`, the 2007 data). The Phase 4 layouts become **Panel (2016 data)** and **Panel (365 data)** (ids `panel-2016`, `panel-365`). The Microsoft term for a tabbed command bar ("Ribbon") is a Microsoft feature name and is not used in any doc, UI text, file name or code identifier. The only exceptions are verbatim third-party identifiers and titles, such as the Fluent control id `RibbonAndQuickAccess` or a `visCmd…Ribbon…` constant. |
| Side panels | The Shape Data and Size & Position side panels become **panes** (`panels.rs` becomes `panes.rs`), so "Panel" names only the layouts. |
| Default layout | Panel Modern. |
| Art licence | `about_screen.svg`, `panel-layout.svg`, `classic-layout.svg` and `icon.svg`: © 2026 Janet Jeffus, made in VectorCraft, all rights reserved, used in Larrygram with permission. `larry.png`: used with Larry Jeffus's permission, all rights reserved. Not covered by the MIT licence. `ATTRIBUTION.md` and the README say so, and forks must replace them. |
| Earlier layout records | Removed from the tree, then purged from history with `git filter-repo` before the first push (section 4, step R4). |

## 2. What the user sees

### 2.1 Welcome wizard (first launch)

It opens on the first launch, before any document. It can be opened
again from Help > Welcome (proposed).

**Page 1, Welcome.**

- Left half: `about_screen.svg`, scaled to fit and keeping its 16:10
  ratio.
- Right half, with generous padding, top to bottom:
  - the title **Larrygram v1.0 alpha**;
  - a short description (confirmed): "Larrygram is a free, open-source
    diagramming app. Draw flowcharts and diagrams with shapes and
    connectors, and open, edit and save `.vsdx` drawings, on the
    desktop or in your browser.";
  - a **Why Larrygram?** button that opens the About window.
- Bottom row: **Cancel** and **Next**.

**Page 2, Choose your layout.**

- Two large buttons side by side, each showing its picture with the name
  under it:
  - **Panel Modern** (`panel-layout.svg`), selected by default;
  - **Panel Classic** (`classic-layout.svg`).
- Clicking a button selects it.
- Bottom row: **Back** and **Start**. Start saves the choice as the
  default layout, applies it at once and closes the wizard.

Cancel on either page closes the wizard and keeps Panel Modern. The wizard is
marked as seen in both cases, so it does not open again on the next
launch.

### 2.2 About window

It opens from **Why Larrygram?** and from **Help > About Larrygram** in
every layout.

- Left half: `larry.png`, scaled to fit.
- Right half, with generous padding, this text (corrected from the
  brief):

  > This app is dedicated to Larry Jeffus.
  >
  > Larry is an engineer with a deeply inspiring story. He began life
  > with severe dyslexia, and although he could not read, he went on to
  > write more than 40 books and to teach hundreds of thousands of
  > students around the world.
  >
  > **[ Watch Larry's story ]** (centred; opens the video in the
  > browser)
  >
  > He loves to make all his engineering designs with diagramming
  > applications. That is why his niece, Janet Jeffus, decided to make
  > this app and dedicate it to his inspiring story.

- Bottom: a **Close** button.
- The video link is `https://youtu.be/wpgiRvcGY_U`. The `?si=…` part
  of the link in the brief is a share-tracking id; it is dropped.

### 2.3 App icon

`icon.svg` becomes the window icon (desktop) and the favicon (web).

## 3. Points for the user to confirm

1. **`larry.png` is 360 × 360 pixels** (it replaced the 180-pixel
   `larry.jpg`). At half of a 1000-pixel-wide window it is scaled up
   about 1.4 times; a 900-pixel version would stay fully sharp.
2. **The button label "Watch Larry's story"** and the short
   description in 2.1: confirmed by the user on 2026-10-23.
3. **The 2016 and 365 layouts from Phase 4** (decided). They are renamed
   **Panel (2016 data)** and **Panel (365 data)**: the files
   `ui-spec/panel-2016.yaml` and `panel-365.yaml` become
   `ui-spec/panel-2016.yaml` and `panel-365.yaml`, and the layout ids,
   CLI values, settings values and labels follow (P7-00). The old names
   are culled from the git history in R4 as well.
4. **History rewrite** (R4). It runs only after an explicit "go", at the
   boundary, with a backup bundle kept outside the repo until the first
   push.

## 4. Order of work

Parallel work is the priority. A task starts as soon as the tasks it
depends on are merged; there are no whole-group waits. Tasks that run
at the same time own disjoint files. Phase 8 (`.vsd` import) runs beside
Phase 7 on its own track, because it lives in `crates/vsd-binary`.

```
now ─┬─ Phase 6 tail: P6-17 ‖ P6-18 ─ phase report ─ tag phase-6
     ├─ P7-01 architect (merged)
     └─ P8-01 architect (docs, ADRs, stubs, task files) ─ merge before the boundary
boundary (nothing else running):
     R1 master: remove the earlier layout records from the tree
     P7-00 worker: rename to Larrygram, the old layout term to Panel, panels to panes
     R4 master: history rewrite (after "go"), move the folders, re-link
     R5 master: the image loaders of ADR 0067 in crates/ui/Cargo.toml
after the boundary, two tracks at once:
  Phase 7 wave 1: P7-02 P7-03 P7-04 P7-05 P7-06 P7-07 P7-15 P7-16  (8 in parallel)
          P7-08 starts when P7-02 merges
          P7-09 shell when P7-05..P7-08 merge ─ P7-10 tests ─ P7-11 ‖ P7-12 ‖ P7-13 ─ P7-14, tag phase-7
  Phase 8: the parallel tasks P8-01 defines (vsd-binary modules, the oracle
          harness, the open-path integration) ─ tests ─ audits ─ fixes, tag phase-8
```

**What made the Phase 7 groups collapse.** P7-01 froze the
`ui_spec::v2` types and the `icons::icon` call, and R5 installs the
image loader before any worker starts. So:

- P7-15 and P7-16 depend only on P7-01 and R5.
- P7-05, P7-06 and P7-07 code against the frozen `ui_spec::v2` types
  and today's `icons` API, and test with hand-built values. They do not
  wait for P7-03's loader or P7-04's icon table; P7-09 wires the loaded
  layouts and themes in. A type change they need is an Open issue for
  P7-03, never an edit to `crates/ui-spec`.
- P7-08 still waits for P7-02, because both touch command predicates.

**Machine load.** Eight Phase 7 workers plus the Phase 8 workers are
more builds than one machine runs well (the P6-13 audit found it at 0%
idle). Run at most six workers at once; start the large-tier tasks
first (P7-03, P7-06, P7-07 and the biggest Phase 8 tasks).

**Merging.** The master merges in arrival order. When several branches
wait, it merges up to three rebased branches and runs one full gate
over the batch; a red batch is bisected.

### P7-01 architect: starts now, beside the Phase 6 tail

It runs alongside the Phase 6 tail because it adds new files and stubs
only. It does not touch the Phase 6 crates (`shapesheet`, `commands`,
`model`, `vsdx`, `render`). In `crates/ui` it adds new modules and their
`mod` lines only.

- **Import the package.** Copy the package into the worktree, converted
  to the decisions in section 1:
  - the package's `layouts/ribbon.yaml` and `layouts/classic.yaml` become
    `ui-spec/layouts/panel-modern.yaml` and `panel-classic.yaml`, and the
    theme files the same;
  - the old layout term becomes "Panel" (or "Panel Modern") in the ADRs,
    guides and task files;
  - prose names Larrygram, while code paths keep today's crate names
    until P7-00.
- **Brand assets.** Move `graphics/` to `crates/ui/assets/brand/`
  (the user has already named the Panel Modern picture `panel-layout.svg`; `.DS_Store` is not copied). Add the
  `ATTRIBUTION.md` rows with the all-rights-reserved terms, and the
  README licence note.
- **ADRs.**
  - Accept 0061–0065.
  - Add 0066, the welcome wizard and About window: pages, first-run
    flag, settings, the Help menu entries.
  - Add 0067, the brand assets and their licence, and the image loading.
    The proposed loader is `egui_extras` image loaders with the `svg`
    and `png` features; check their licences with `cargo deny`.
- **Stubs.**
  - `crates/ui/src/wizard.rs` and `crates/ui/src/about.rs`: the types
    and the `show` functions, as "not implemented" stubs.
  - The settings fields `welcome_seen: bool` and `layout`.
  - `LayoutKind::{PanelModern, PanelClassic, Panel2016, Panel365}`.
  - The commands `help.about` and `help.welcome`, if the registry lacks
    them.
- **KICKOFF.md.** Apply the Phase 7 section and the renumbering
  (8 = `.vsd`, 9 = agents, paused).
- **Task files.** Finalise P7-02 to P7-14, and write P7-00, P7-15 and
  P7-16 from this plan.

### Boundary: P7-00 and the master steps

Nothing else runs during the boundary, because every file changes.

- **R1 (master).** Remove the earlier layout records from the tree.
  - Reword AGENTS.md section 4 so that "record every source" covers data
    compiled by the project (the cleanroom auditor needs a rule to
    apply).
  - Clean up the matching STATUS entries and notes.
- **P7-00 (worker, large tier).** The mechanical rename, as one task:
  1. Side panels become panes.
  2. The old layout term becomes Panel, except verbatim third-party
     identifiers.
  3. Ferrogram becomes Larrygram in every case form: crate and directory
     names, `use` paths, snapshot file names, environment variables, the
     settings storage key, the web page title and the version.
  4. The 2016 and 365 layouts become Panel (2016 data) and Panel (365
     data): `ui-spec/panel-2016.yaml` and `panel-365.yaml`, with the
     layout ids, CLI values (`--ui-layout panel-2016|panel-365`),
     settings values, labels and the extractor output paths.
  5. The full gate is green, and so are the corpus tests.
- **R4 (master, after the user's "go").**
  - Write a backup bundle.
  - Run `git filter-repo` to drop the earlier layout records from every
    commit and commit message, and to cull the old layout file names
    (`panel-2016.yaml`, `panel-365.yaml`) from history: their paths are
    renamed to `panel-2016.yaml` and `panel-365.yaml` in every commit.
  - Re-tag `phase-0` to `phase-6`.
  - Move `/Users/dara/ferrogram` to `/Users/dara/larrygram` and
    `/Volumes/Dara/ferrogram` to `/Volumes/Dara/larrygram`.
  - Update the sccache path in `~/.cargo/config.toml`, the `reference`
    symlink and the AGENTS.md paths.
  - Run the full gate.
  - This session ends there. The next master session starts in
    `/Users/dara/larrygram`, and the memory files move to its project
    folder.

- **R5 (master, after R4).** Add the `egui_extras` image loaders with
  the `svg` and `png` features to `crates/ui/Cargo.toml` and install
  them at start-up (ADR 0067), with `cargo deny` and the wasm build
  green. One commit, so that P7-04, P7-15 and P7-16 do not share the
  manifest.

### Phase 7 wave 1: 8 in parallel

As in the package: P7-02 commands, P7-03 `ui-spec` layout format v2, and
P7-04 icons. P7-04 still owns `crates/ui/Cargo.toml` for the icon
pipeline, but the image loader is already in (R5). P7-02 adds
`help.about` and `help.welcome` if P7-01 did not, and does not touch
`crates/commands/src/builtins/file.rs` (Phase 8 owns the open path).

- **P7-05 to P7-07**, as in the package: theme, Panel Classic widgets
  (menus and toolbars) and Panel Modern widgets (tabs and groups), against
  the frozen types (see above).
- **P7-08** selection contexts, as in the package, as soon as P7-02 merges.
- **P7-15 welcome wizard** (`wizard.rs`, medium tier).
  - Both pages, Cancel, Next, Back and Start, and the layout buttons.
  - It sets `welcome_seen` and the default layout through the settings
    API.
  - It calls `about::show` for **Why Larrygram?**.
  - It is tested with egui_kittest: page flow, the default selection,
    Start saves the choice, and Cancel keeps Panel Modern.
- **P7-16 About window and app icon** (`about.rs`, medium tier).
  - The two-column window, the corrected text, the video button
    (through egui's open-URL output, which works on desktop and web) and
    Close.
  - The window icon and the web favicon from `icon.svg`.
  - Tests: the text, the button opening the URL, and Close.

### After wave 1

- **P7-09 shell.** As in the package, plus:
  - the wizard opens on the first launch;
  - Help > About Larrygram and Help > Welcome in all layouts;
  - Options gets "Show the welcome wizard on next start".
- **P7-10 tests.** Adds golden PNGs of both wizard pages and the About
  window.
- **P7-13 clean-room audit.** Checks the brand-asset rows and that no
  third-party artwork entered the repo.
- **P7-14.** Fixes from the audits, the phase report and the
  `phase-7` tag.

## 5. Exit criteria added to the package's list

1. A fresh install opens the wizard. Start applies the chosen layout
   without a restart, and the next launch skips the wizard.
2. **Why Larrygram?** and **Help > About Larrygram** open the same About
   window, and its button opens the video link.
3. No doc, UI text, file name or identifier says "Ferrogram" or the old
   layout term, except verbatim third-party identifiers. A gate check enforces this from
   P7-00 on.
4. The brand assets have their `ATTRIBUTION.md` rows and pass the
   clean-room audit.
