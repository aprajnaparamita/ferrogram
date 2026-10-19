//! Tool commands, the Shapes-window drag and the in-place editor through
//! the real [`Shell`] (task P5-17, ADR 0049).
//!
//! A shortcut selects the rectangle tool and a canvas drag draws one
//! rectangle; a Shapes-window row drag drops one master instance, and
//! Escape cancels it; a double click edits text in one undo step and a
//! group still enters.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::cell::RefCell;
use std::rc::Rc;

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use ferrogram_commands::app::AppCtx;
use ferrogram_commands::{ActiveTool, builtins};
use ferrogram_ui::shell::Shell;
use ferrogram_ui_spec::Loader;

use crate::{PAGE_HEIGHT, app_with_one_shape};

const RULER: f32 = 18.0;
/// The shell canvas shows the whole page at this zoom.
const SHELL_ZOOM: f64 = 0.25;

/// A shell driven by `egui_kittest` with the real registry and layouts.
struct ShellRig {
    shell: Rc<RefCell<Shell>>,
    harness: Harness<'static>,
}

impl ShellRig {
    fn new(mut app: AppCtx) -> Self {
        if let Some(session) = app.active_document_mut() {
            session.view.zoom = SHELL_ZOOM;
        }
        let shell = Rc::new(RefCell::new(Shell::new(
            builtins::registry().expect("registry"),
            Loader::from_embedded().expect("layouts"),
            app,
        )));
        let harness_shell = Rc::clone(&shell);
        let harness = Harness::builder()
            .with_size(egui::vec2(1400.0, 900.0))
            .with_accessibility_check(false)
            .with_context_setup(|ctx| {
                // The harness advances its clock between internal steps;
                // keep two deliberate clicks in one double-click window.
                ctx.options_mut(|options| {
                    options.input_options.max_double_click_delay = 10.0;
                });
            })
            .build_ui(move |ui| {
                let mut shell = harness_shell.borrow_mut();
                let intents = shell.show_in(ui).expect("the frame renders");
                let _ = shell.dispatch(&intents).expect("the intents dispatch");
            });
        let mut rig = Self { shell, harness };
        rig.run();
        rig
    }

    fn run(&mut self) {
        self.harness.run();
    }

    fn key(&mut self, modifiers: egui::Modifiers, key: egui::Key) {
        self.harness.key_down_modifiers(modifiers, key);
        self.run();
        self.harness.key_up_modifiers(modifiers, key);
        self.run();
    }

    fn press_at(&mut self, pos: egui::Pos2) {
        self.harness.event(egui::Event::PointerMoved(pos));
        self.harness.event(egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::default(),
        });
        self.run();
    }

    fn move_at(&mut self, pos: egui::Pos2) {
        self.harness.event(egui::Event::PointerMoved(pos));
        self.run();
    }

    fn release_at(&mut self, pos: egui::Pos2) {
        self.harness.event(egui::Event::PointerMoved(pos));
        self.harness.event(egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::default(),
        });
        self.run();
    }

    fn drag(&mut self, from: egui::Pos2, to: egui::Pos2) {
        self.press_at(from);
        self.move_at(to);
        self.release_at(to);
    }

    fn double_click(&mut self, pos: egui::Pos2) {
        self.harness.event(egui::Event::PointerMoved(pos));
        for _ in 0..2 {
            for pressed in [true, false] {
                self.harness.event(egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::default(),
                });
            }
        }
        self.run();
        // One more frame: `text.begin` ran after the double click's
        // frame, so the editor requests focus now.
        self.run();
    }

    fn canvas_rect(&self) -> egui::Rect {
        self.shell.borrow().canvas_rect().expect("the canvas drew")
    }

    /// The screen point of a page point in the shell's canvas.
    fn screen(&self, x: f64, y: f64) -> egui::Pos2 {
        let rect = self.canvas_rect();
        let viewport = egui::Rect::from_min_max(rect.min + egui::vec2(RULER, RULER), rect.max);
        let k = 96.0 * SHELL_ZOOM;
        egui::pos2(
            viewport.min.x + (x * k) as f32,
            viewport.min.y + ((PAGE_HEIGHT - y) * k) as f32,
        )
    }

    fn page_shape_count(&self) -> usize {
        let shell = self.shell.borrow();
        let session = shell.app.active_document().expect("session");
        let page = session.active_page.expect("page");
        session
            .document
            .page(page)
            .map(|page| page.shapes.len())
            .unwrap_or(0)
    }

    fn undo_len(&self) -> usize {
        self.shell
            .borrow()
            .app
            .active_document()
            .expect("session")
            .undo
            .len()
    }

    /// The center of the first Shapes-window row named `name` (the row
    /// buttons sit in the left panel).
    fn shapes_row(&self, name: &str) -> egui::Pos2 {
        self.harness
            .root()
            .query_all_by_role_and_label(Role::Button, name)
            .map(|node| node.rect().center())
            .find(|center| center.x < 300.0)
            .unwrap_or_else(|| panic!("no Shapes-window row named {name:?}"))
    }
}

#[test]
fn a_shortcut_picks_the_rectangle_tool_and_a_drag_draws_one_rectangle() {
    let (app, _shape) = app_with_one_shape();
    let before = {
        let session = app.active_document().expect("session");
        session
            .document
            .page(session.active_page.expect("page"))
            .expect("page")
            .shapes
            .len()
    };
    let mut rig = ShellRig::new(app);
    rig.key(egui::Modifiers::CTRL, egui::Key::Num8);
    assert_eq!(
        rig.shell.borrow().active_tool(),
        ActiveTool::Rectangle,
        "Ctrl+8 selects the rectangle tool through the registry"
    );
    let from = rig.screen(1.0, 1.0);
    let to = rig.screen(3.0, 2.0);
    rig.drag(from, to);
    assert_eq!(rig.page_shape_count(), before + 1);
    assert_eq!(rig.undo_len(), 1, "one drag, one undo step");
    assert!(rig.shell.borrow().app.selection.anchor.is_some());
}

#[test]
fn a_shapes_window_drag_drops_one_master_and_escape_cancels() {
    let (app, _shape) = app_with_one_shape();
    let mut rig = ShellRig::new(app);
    let before = rig.page_shape_count();

    let row = rig.shapes_row("Rectangle");
    let target = rig.screen(6.0, 8.0);
    rig.press_at(row);
    rig.move_at(target);
    rig.release_at(target);
    assert_eq!(rig.page_shape_count(), before + 1, "one master dropped");

    let row = rig.shapes_row("Rectangle");
    let target = rig.screen(6.0, 6.0);
    rig.press_at(row);
    rig.move_at(target);
    rig.key(egui::Modifiers::default(), egui::Key::Escape);
    assert_eq!(
        rig.page_shape_count(),
        before + 1,
        "Escape cancels before the release"
    );
    rig.release_at(target);
    assert_eq!(rig.page_shape_count(), before + 1, "Escape drops nothing");
}

fn shape_text(app: &AppCtx, shape: ferrogram_model::ShapeId) -> Option<String> {
    let session = app.active_document().expect("session");
    let entry = session.document.shape(shape)?;
    let text = entry.text.as_ref()?;
    Some(
        text.parts
            .iter()
            .filter_map(|part| match part {
                ferrogram_model::TextPart::Text(text) => Some(text.as_str()),
                _ => None,
            })
            .collect(),
    )
}

#[test]
fn a_double_click_types_text_in_one_undo_step() {
    let (app, shape) = app_with_one_shape();
    let mut rig = ShellRig::new(app);
    let pos = rig.screen(3.0, 3.0);
    rig.double_click(pos);
    assert!(
        rig.shell.borrow().app.selection.anchor == Some(shape),
        "the double click selects the shape"
    );
    rig.harness.event(egui::Event::Text("Hello".to_owned()));
    rig.run();
    rig.key(egui::Modifiers::default(), egui::Key::Escape);
    rig.run();

    let shell = rig.shell.borrow();
    assert_eq!(
        shape_text(&shell.app, shape).as_deref(),
        Some("Hello"),
        "Escape commits the buffer"
    );
    let session = shell.app.active_document().expect("session");
    assert_eq!(session.undo.len(), 1, "one commit, one undo step");
}

#[test]
fn a_double_click_on_a_group_still_enters_it() {
    let (mut app, _shape) = app_with_one_shape();
    let group = {
        let session = app.active_document_mut().expect("session");
        let child = session.document.add_shape(crate::shape(3.0, 3.0, 5));
        let group = session.document.add_shape(crate::shape(3.0, 3.0, 6));
        if let Some(entry) = session.document.shape_mut(group) {
            entry.children.push(child);
        }
        if let Some(entry) = session.document.shape_mut(child) {
            entry.parent = Some(group);
        }
        let page = session.active_page.expect("page");
        if let Some(page) = session.document.pages.get_mut(page.index as usize) {
            page.shapes.push(group);
        }
        group
    };
    let mut rig = ShellRig::new(app);
    let pos = rig.screen(3.0, 3.0);
    rig.double_click(pos);
    let shell = rig.shell.borrow();
    assert_eq!(
        shell.app.selection.entered,
        Some(group),
        "a group double click still enters it"
    );
    assert!(!shell.app.selection.is_empty(), "the group is selected");
}
