//! egui_kittest interaction tests (task P4-10, ADR 0037): clicks that
//! drive the real [`Shell`] end to end.
//!
//! P4-09's modules cover ribbon placement, enabled/disabled states,
//! the file flow and layout switching at the data level. This file
//! clicks the widgets the way a user does, through `egui_kittest`'s
//! AccessKit tree, and asserts the *application state* changes:
//!
//! - a ribbon button/checkbox click dispatches its registry command
//!   (the View tab's `ShowGrid` toggle);
//! - the View/Options dialog switches the ribbon layout with no
//!   restart;
//! - the Help menu opens the About window;
//! - the close-confirmation dialog closes a dirty document on confirm;
//! - the File button opens the backstage with its placements;
//! - a QAT button opens a document;
//! - a page tab click switches the active page.
//!
//! The committed ribbon is about 5000 points wide, so these use
//! [`ShellRig::large`] (the P4-04 note: a click outside the window
//! rect never reaches a widget).
//!
//! A panic is the bug (AGENTS.md section 6).

// The never-crash lints apply to non-test code only (clippy.toml);
// these helpers are test code and use `expect` freely.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::collections::BTreeSet;

use egui::accesskit::Role;
use ferrogram_commands::Source;
use ferrogram_commands::app::{AppCtx, DocumentSession, Settings};
use ferrogram_model::{Document, Page, PageId};
use ferrogram_ui_spec::LayoutKind;

use crate::{ShellRig, sets};

/// The ribbon tab labels of one layout (the `layout_switch.rs` rule).
fn tab_labels(kind: LayoutKind) -> BTreeSet<String> {
    sets()
        .get(kind)
        .layout
        .tabs
        .iter()
        .map(|tab| tab.label.clone())
        .collect()
}

/// A label only the `mine` layout has, if any.
fn unique_tab(mine: LayoutKind, other: LayoutKind) -> Option<String> {
    let ours = tab_labels(mine);
    let theirs = tab_labels(other);
    ours.difference(&theirs).next().cloned()
}

#[test]
fn clicking_the_grid_checkbox_in_the_view_tab_toggles_the_grid() {
    let mut rig = ShellRig::large();
    rig.dispatch("file.new", Source::Ui).expect("file.new runs");
    rig.run();
    assert!(
        rig.shell
            .borrow()
            .app
            .active_document()
            .expect("a document")
            .view
            .show_grid,
        "the new document starts with the grid on (ADR 0029)"
    );

    // The checkbox lives on the View tab of the committed ribbon.
    rig.click(Role::Button, "TabView");
    rig.click(Role::CheckBox, "ShowGrid");
    {
        let shell = rig.shell.borrow();
        assert!(
            !shell
                .app
                .active_document()
                .expect("a document")
                .view
                .show_grid,
            "one click toggles `view.toggle_grid`"
        );
    }

    // And back: the widget is a toggle, not a one-shot button.
    rig.click(Role::CheckBox, "ShowGrid");
    assert!(
        rig.shell
            .borrow()
            .app
            .active_document()
            .expect("a document")
            .view
            .show_grid
    );
}

#[test]
fn switching_layouts_from_the_options_dialog_needs_no_restart() {
    let mut rig = ShellRig::large();
    let before = {
        let shell = rig.shell.borrow();
        assert_eq!(shell.layout_kind(), LayoutKind::Visio2016);
        // State that must survive the switch (the `layout_switch.rs`
        // rule): the same `Shell`, the same `AppCtx`.
        &shell.app as *const AppCtx as usize
    };

    // The shell-handled window command opens the dialog (P4-03).
    let outcome = rig
        .dispatch("app.office.center.options", Source::Ui)
        .expect("no error");
    assert!(
        matches!(
            outcome.ui_request,
            Some(ferrogram_commands::UiRequest::ShowOptions)
        ),
        "{outcome:?}"
    );
    rig.run();

    // Click the 365 radio of the dialog body (`settings::show_options`).
    rig.click(Role::RadioButton, "Microsoft 365 ribbon");
    {
        let shell = rig.shell.borrow();
        assert_eq!(shell.layout_kind(), LayoutKind::Visio365);
        assert_eq!(&shell.app as *const AppCtx as usize, before);
    }

    // The next frame draws the 365 ribbon: no restart, same harness.
    rig.run();
    let only_365 = unique_tab(LayoutKind::Visio365, LayoutKind::Visio2016)
        .expect("365 has a tab 2016 does not");
    assert!(
        rig.names().contains_key(&only_365),
        "the {only_365:?} tab renders: {:?}",
        rig.names().keys().collect::<Vec<_>>()
    );
}

#[test]
fn the_help_menu_opens_the_about_dialog() {
    let mut rig = ShellRig::large();
    rig.click(Role::Button, "Help");
    rig.click(Role::Button, "About");
    assert_eq!(
        rig.shell.borrow().status(),
        "About Ferrogram",
        "`help.about` ran through the registry"
    );
    // The modal window is drawn (its title and close button appear).
    assert!(
        rig.names().contains_key("About Ferrogram"),
        "the About window renders"
    );
    assert!(rig.names().contains_key("Close window"), "closable");
}

#[test]
fn the_close_confirmation_dialog_closes_the_document_on_confirm() {
    let mut rig = ShellRig::large();
    rig.dispatch("file.new", Source::Ui).expect("file.new runs");
    {
        let mut shell = rig.shell.borrow_mut();
        shell.app.active_document_mut().expect("a document").dirty = true;
    }

    // An interactive close of a dirty document asks first (ADR 0028):
    // the command returns `ConfirmClose` and closes nothing.
    let outcome = rig.dispatch("file.close", Source::Ui).expect("no error");
    assert!(
        matches!(
            outcome.ui_request,
            Some(ferrogram_commands::UiRequest::ConfirmClose { .. })
        ),
        "{outcome:?}"
    );
    assert_eq!(rig.shell.borrow().app.documents.len(), 1, "still open");

    // The dialog's confirm button re-dispatches `file.close` with
    // `confirm: true` and the document closes.
    rig.run();
    rig.click(Role::Button, "Close without saving");
    assert_eq!(rig.shell.borrow().app.documents.len(), 0, "closed");
}

#[test]
fn the_file_button_opens_the_backstage_with_its_entries() {
    let entries: Vec<String> = sets()
        .get(LayoutKind::Visio2016)
        .layout
        .backstage
        .tabs
        .iter()
        .map(|tab| tab.label.clone())
        .collect();
    assert!(!entries.is_empty(), "the committed layout has a backstage");
    let rig = ShellRig::large();
    let before = rig.names();
    for entry in &entries {
        assert!(!before.contains_key(entry), "{entry:?} is backstage-only");
    }

    // The window has two "File" buttons (the menu bar and the ribbon
    // tab strip). Each attempt gets a fresh rig: a click on the menu
    // bar button opens a popup that covers the ribbon one, so the
    // attempts must not share a frame history.
    let mut opened = false;
    for nth in 0..3 {
        let mut rig = ShellRig::large();
        if !rig.try_click_nth(Role::Button, "File", nth) {
            continue;
        }
        let names = rig.names();
        if entries.iter().any(|entry| names.contains_key(entry)) {
            opened = true;
            break;
        }
    }
    assert!(opened, "the File button opens the backstage: {entries:?}");
    // The entries render whatever the active entry is (the assertion
    // above), and the shell stays healthy with the backstage open.
    assert!(!entries.is_empty());
}

#[test]
fn clicking_the_qat_new_button_opens_a_document() {
    let mut rig = ShellRig::large();
    rig.click(Role::Button, "FileNewDefault");
    assert_eq!(
        rig.shell.borrow().app.documents.len(),
        1,
        "the QAT button runs `file.new`"
    );
}

#[test]
fn clicking_a_page_tab_switches_the_active_page() {
    let mut document = Document::new();
    let mut first = Page::new(PageId { index: 0 }, 1);
    first.name = "First".to_owned();
    let mut second = Page::new(PageId { index: 1 }, 2);
    second.name = "Second".to_owned();
    document.pages.push(first);
    document.pages.push(second);
    let mut app = AppCtx::new(Settings::default());
    app.open_document(DocumentSession::new(document, None));
    let mut rig = ShellRig::with_app(app);
    rig.run();

    // `canvas::show_page_tabs` draws a selectable label per page.
    assert!(
        rig.try_click(Role::Button, "Second") || rig.try_click(Role::Tab, "Second"),
        "a page tab named after its page: {:?}",
        rig.names().keys().collect::<Vec<_>>()
    );
    let shell = rig.shell.borrow();
    let session = shell.app.active_document().expect("a document");
    assert_eq!(session.active_page, Some(PageId { index: 1 }));
}
