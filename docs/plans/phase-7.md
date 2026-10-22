# Phase 7 plan: Larrygram, the Panel and Classic layouts, the welcome wizard

- Status: proposed (2026-10-23), for the user's review
- Inputs: the Phase 7 package (`/Volumes/Dara/ferrogram-phase7/`, its
  `docs/phase-7/README.md` and draft tasks P7-01 to P7-14), the user's
  brief of 2026-10-23 and the graphics in `graphics/`

## 1. Decisions taken

| Topic | Decision |
|---|---|
| Phase numbers | Phase 7 is now the Panel and Classic layouts, the rename and the wizard. Binary `.vsd` import becomes Phase 8. Agents (control channel and MCP) becomes Phase 9 and is paused. |
| Name | Ferrogram becomes **Larrygram** everywhere: crates (`larrygram-*`), binaries, `LARRYGRAM_*` variables, docs, and the folders `/Users/dara/larrygram` and `/Volumes/Dara/larrygram`. The app shows **Larrygram v1.0 alpha**; the workspace version becomes `1.0.0-alpha.1`. |
| Layout names | **Panel** (follows the 2013 style) and **Classic** (follows the 2007 style). "Ribbon" is not used in any doc, UI text or code identifier. The only exceptions are verbatim third-party identifiers and titles, such as the Fluent control id `RibbonAndQuickAccess` or a `visCmd…Ribbon…` constant. |
| Side panels | The Shape Data and Size & Position side panels become **panes** (`panels.rs` becomes `panes.rs`), so "Panel" names only the layout. |
| Default layout | Panel. |
| Art licence | `about_screen.svg`, `panel-layout.svg`, `classic-layout.svg` and `icon.svg`: © 2026 Janet Jeffus, made in VectorCraft, all rights reserved, used in Larrygram with permission. `larry.jpg`: used with Larry Jeffus's permission, all rights reserved. Not covered by the MIT licence. `ATTRIBUTION.md` and the README say so, and forks must replace them. |
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
  - a short description (proposed): "Larrygram is a free, open-source
    diagramming app. Draw flowcharts and diagrams with shapes and
    connectors, and open, edit and save `.vsdx` drawings, on the
    desktop or in your browser.";
  - a **Why Larrygram?** button that opens the About window.
- Bottom row: **Cancel** and **Next**.

**Page 2, Choose your layout.**

- Two large buttons side by side, each showing its picture with the name
  under it:
  - **Panel** (`panel-layout.svg`), selected by default;
  - **Classic** (`classic-layout.svg`).
- Clicking a button selects it.
- Bottom row: **Back** and **Start**. Start saves the choice as the
  default layout, applies it at once and closes the wizard.

Cancel on either page closes the wizard and keeps Panel. The wizard is
marked as seen in both cases, so it does not open again on the next
launch.

### 2.2 About window

It opens from **Why Larrygram?** and from **Help > About Larrygram** in
every layout.

- Left half: `larry.jpg`, scaled to fit.
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

1. **`larry.jpg` is 180 × 180 pixels.** At half of a 1000-pixel-wide
   window it is scaled up about 2.8 times and looks soft. A version of at
   least 900 pixels on its short side would stay sharp.
2. **The button label "Watch Larry's story"** and the proposed short
   description in 2.1.
3. **The 2016 and 365 layouts from Phase 4.** They stay available in
   Options, but their names cite Microsoft products. Proposed: list them
   as "Panel (2016 data)" and "Panel (365 data)", or retire them once
   Panel ships.
4. **History rewrite** (R4). It runs only after an explicit "go", at the
   boundary, with a backup bundle kept outside the repo until the first
   push.

## 4. Order of work

Parallel work is the priority. Tasks in one group run at the same time
on disjoint files.

```
now ─┬─ Phase 6 tail: P6-09, P6-10, P6-15, P6-16 ─ P6-11 ─ P6-12/13/14 ─ P6-17 ─ tag phase-6
     └─ P7-01 architect (docs, data, stubs) ─────────── merge before the boundary
boundary (nothing else running):
     R1 master: remove the earlier layout records from the tree
     P7-00 worker: rename to Larrygram, Ribbon to Panel, panels to panes
     R4 master: history rewrite (after "go"), move the folders, re-link
Phase 7 groups:
     A: P7-02  P7-03  P7-04                      (3 in parallel)
     B: P7-05  P7-06  P7-07  P7-08  P7-15  P7-16  (6 in parallel)
     C: P7-09 shell integration
     D: P7-10 tests and golden images
     E: P7-11  P7-12  P7-13                      (3 in parallel)
     F: P7-14 fixes, phase report, tag phase-7
```

### P7-01 architect: starts now, beside the Phase 6 tail

It runs alongside the Phase 6 tail because it adds new files and stubs
only. It does not touch the Phase 6 crates (`shapesheet`, `commands`,
`model`, `vsdx`, `render`). In `crates/ui` it adds new modules and their
`mod` lines only.

- **Import the package.** Copy the package into the worktree, converted
  to the decisions in section 1:
  - `ui-spec/layouts/ribbon.yaml` becomes `panel.yaml`, and the theme
    file the same;
  - "Ribbon" becomes "Panel" in the ADRs, guides and task files;
  - prose names Larrygram, while code paths keep today's crate names
    until P7-00.
- **Brand assets.** Move `graphics/` to `crates/ui/assets/brand/`
  (`ibbon-layout.svg` is renamed `panel-layout.svg`). Add the
  `ATTRIBUTION.md` rows with the all-rights-reserved terms, and the
  README licence note.
- **ADRs.**
  - Accept 0061–0065.
  - Add 0066, the welcome wizard and About window: pages, first-run
    flag, settings, the Help menu entries.
  - Add 0067, the brand assets and their licence, and the image loading.
    The proposed loader is `egui_extras` image loaders with the `svg`
    and `jpeg` features; check their licences with `cargo deny`.
- **Stubs.**
  - `crates/ui/src/wizard.rs` and `crates/ui/src/about.rs`: the types
    and the `show` functions, as "not implemented" stubs.
  - The settings fields `welcome_seen: bool` and `layout`.
  - `LayoutKind::{Classic, Panel}`.
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
  2. Ribbon becomes Panel, except verbatim third-party identifiers.
  3. Ferrogram becomes Larrygram in every case form: crate and directory
     names, `use` paths, snapshot file names, environment variables, the
     settings storage key, the web page title and the version.
  4. The full gate is green, and so are the corpus tests.
- **R4 (master, after the user's "go").**
  - Write a backup bundle.
  - Run `git filter-repo` to drop the earlier layout records from every
    commit and commit message.
  - Re-tag `phase-0` to `phase-6`.
  - Move `/Users/dara/ferrogram` to `/Users/dara/larrygram` and
    `/Volumes/Dara/ferrogram` to `/Volumes/Dara/larrygram`.
  - Update the sccache path in `~/.cargo/config.toml`, the `reference`
    symlink and the AGENTS.md paths.
  - Run the full gate.
  - This session ends there. The next master session starts in
    `/Users/dara/larrygram`, and the memory files move to its project
    folder.

### Phase 7 group A: 3 in parallel

As in the package: P7-02 commands, P7-03 `ui-spec` layout format v2, and
P7-04 icons. P7-04 owns `crates/ui/Cargo.toml` and adds the image
loader of ADR 0067. P7-02 adds `help.about` and `help.welcome` if P7-01
did not.

### Phase 7 group B: 6 in parallel

- **P7-05 to P7-08**, as in the package: theme, Classic widgets, Panel
  widgets (the package's "Ribbon widgets") and selection contexts.
- **P7-15 welcome wizard** (`wizard.rs`, medium tier).
  - Both pages, Cancel, Next, Back and Start, and the layout buttons.
  - It sets `welcome_seen` and the default layout through the settings
    API.
  - It calls `about::show` for **Why Larrygram?**.
  - It is tested with egui_kittest: page flow, the default selection,
    Start saves the choice, and Cancel keeps Panel.
- **P7-16 About window and app icon** (`about.rs`, medium tier).
  - The two-column window, the corrected text, the video button
    (through egui's open-URL output, which works on desktop and web) and
    Close.
  - The window icon and the web favicon from `icon.svg`.
  - Tests: the text, the button opening the URL, and Close.

### Group C onwards

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
3. No doc, UI text or identifier says "Ferrogram" or "Ribbon", except
   verbatim third-party identifiers. A gate check enforces this from
   P7-00 on.
4. The brand assets have their `ATTRIBUTION.md` rows and pass the
   clean-room audit.
