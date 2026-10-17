//! Enabled, disabled and stub rendering (task P4-09, ADR 0031).
//!
//! KICKOFF: a stub control renders disabled with the "Not yet
//! implemented" tooltip; an implemented, enabled command renders
//! enabled; `Enabled::no(reason)` greys out with the reason as the
//! tooltip. The test drives the real builtin registry over a small
//! synthetic layout (the widget mapping rules are the same the
//! committed layouts use, P4-04).

use std::cell::RefCell;
use std::rc::Rc;

use egui::accesskit::Role;
use egui_kittest::{Harness, kittest::Queryable};
use ferrogram_commands::Source;
use ferrogram_commands::app::{AppCtx, Settings};
use ferrogram_ui::Intent;
use ferrogram_ui::ribbon::Ribbon;
use ferrogram_ui_spec::{Control, ControlType, FluentId, Group, LayoutKind, LayoutSet, Tab};

use crate::{Widget, widgets};

/// The ADR 0031 tooltip texts (mirrors `crates/ui/src/ribbon.rs`;
/// the exact strings are the contract the UI test pins).
const NOT_YET_IMPLEMENTED: &str = "Not yet implemented";
const OUT_OF_SCOPE: &str = "Out of scope for this run";

/// A test control.
fn control(id: &str, label: &str, command: Option<&str>) -> Control {
    Control {
        id: FluentId::new(id),
        r#type: ControlType::Button,
        label: label.to_owned(),
        order: 0,
        parent: None,
        command: command.map(ferrogram_ui_spec::CommandId::new),
        aliases: Vec::new(),
        bounds: None,
        icon_ref: None,
        keytip: None,
        size_variant: None,
        visual_notes: None,
    }
}

/// A one-tab layout with one group of controls.
fn set_with(controls: Vec<Control>) -> LayoutSet {
    let mut layout = ferrogram_ui_spec::Layout::default();
    layout.tabs.push(Tab {
        id: FluentId::new("TabHome"),
        label: "TabHome".to_owned(),
        order: 0,
        tab_set: None,
        bounds: None,
        icon_ref: None,
        keytip: None,
        size_variant: None,
        visual_notes: None,
        groups: vec![Group {
            id: FluentId::new("GroupStates"),
            label: "GroupStates".to_owned(),
            order: 0,
            bounds: None,
            icon_ref: None,
            keytip: None,
            size_variant: None,
            visual_notes: None,
            controls,
        }],
    });
    LayoutSet {
        layout,
        ..LayoutSet::default()
    }
}

/// A ribbon driven headlessly with the real registry.
struct Rig {
    intents: Rc<RefCell<Vec<Intent>>>,
    harness: Harness<'static>,
}

impl Rig {
    fn new(set: LayoutSet) -> Self {
        let ribbon = Rc::new(RefCell::new(Ribbon::new(LayoutKind::Visio2016)));
        let intents = Rc::new(RefCell::new(Vec::new()));
        let set = Rc::new(set);
        let app = AppCtx::new(Settings::default());
        // The real builtin registry: the states come from it, not
        // from a stub resolver.
        let registry = ferrogram_commands::builtins::registry().expect("the registry builds");
        let harness = Harness::builder()
            .with_size(egui::vec2(1600.0, 900.0))
            .build_ui({
                let ribbon = Rc::clone(&ribbon);
                let intents = Rc::clone(&intents);
                let set = Rc::clone(&set);
                move |ui| {
                    let mut ribbon = ribbon.borrow_mut();
                    let produced = ribbon
                        .show(ui, &set, &registry, &app)
                        .expect("the ribbon renders");
                    intents.borrow_mut().extend(produced);
                }
            });
        let mut rig = Self { intents, harness };
        rig.harness.run();
        rig
    }

    /// The widgets of the current frame.
    fn widgets(&self) -> Vec<Widget> {
        widgets(&self.harness)
    }

    /// The one widget named `name`.
    fn widget(&self, name: &str) -> Widget {
        self.widgets()
            .into_iter()
            .find(|w| w.name == name)
            .unwrap_or_else(|| panic!("no widget named {name:?}"))
    }

    /// Clicks the widget named `name` and runs a frame.
    fn click(&mut self, name: &str) {
        {
            let root = self.harness.root();
            let node = root
                .query_all_by_role_and_label(Role::Button, name)
                .next()
                .unwrap_or_else(|| panic!("no button named {name:?}"));
            node.click();
        }
        self.harness.run();
    }

    /// The intents the widgets produced since the last call.
    fn take_intents(&self) -> Vec<Intent> {
        std::mem::take(&mut *self.intents.borrow_mut())
    }
}

/// The three states plus the edge cases, one fixture.
fn rig() -> Rig {
    Rig::new(set_with(vec![
        // Implemented and enabled with no open document.
        control("FileNew", "New", Some("file.new")),
        // Implemented but disabled: `Enabled::no("no open document")`.
        control("FileSave", "Save", Some("file.save")),
        // Registered as a stub (`commands.yaml` only).
        control("FilePrint", "Print", Some("file.print")),
        // No command link at all.
        control("Unmapped", "Unmapped", None),
        // Out of scope for this run (its Fluent id is listed in
        // `ui-spec/out-of-scope.txt`), even though its command is
        // implemented.
        control("CoauthorMenu", "Coauthor", Some("file.new")),
    ]))
}

#[test]
fn an_implemented_enabled_control_renders_enabled_with_no_tooltip() {
    let rig = rig();
    let new = rig.widget("New");
    assert_eq!(new.role, Role::Button);
    assert!(
        !new.disabled,
        "an implemented, enabled command renders enabled"
    );
    assert_eq!(new.description, None, "no tooltip on an enabled control");
}

#[test]
fn a_stub_control_renders_disabled_with_the_not_yet_implemented_tooltip() {
    let rig = rig();
    let print = rig.widget("Print");
    assert!(print.disabled, "a stub renders disabled");
    assert_eq!(print.description.as_deref(), Some(NOT_YET_IMPLEMENTED));
    // A control with no command link at all is the same state.
    let unmapped = rig.widget("Unmapped");
    assert!(unmapped.disabled);
    assert_eq!(unmapped.description.as_deref(), Some(NOT_YET_IMPLEMENTED));
}

#[test]
fn an_enabled_no_reason_control_greys_out_with_the_reason() {
    let rig = rig();
    let save = rig.widget("Save");
    assert!(save.disabled, "Enabled::no greys the control out");
    assert_eq!(
        save.description.as_deref(),
        Some("no open document"),
        "the reason is the tooltip"
    );
}

#[test]
fn an_out_of_scope_control_says_so_even_when_its_command_is_implemented() {
    let rig = rig();
    let coauthor = rig.widget("Coauthor");
    assert!(coauthor.disabled);
    assert_eq!(coauthor.description.as_deref(), Some(OUT_OF_SCOPE));
}

#[test]
fn only_an_enabled_control_dispatches_its_command() {
    let mut rig = rig();
    rig.click("New");
    let intents = rig.take_intents();
    assert_eq!(intents.len(), 1, "the enabled control dispatches");
    assert_eq!(intents[0].command.as_str(), "file.new");
    assert_eq!(intents[0].invocation.source, Source::Ribbon);

    rig.click("Print");
    assert!(rig.take_intents().is_empty(), "a stub dispatches nothing");
    rig.click("Save");
    assert!(
        rig.take_intents().is_empty(),
        "a disabled control dispatches nothing"
    );
}
