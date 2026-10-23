//! Drawing-tool and stencil-drop tests through the real canvas, the real
//! `ToolMachine` and the real command registry (task P5-06, ADR 0049).
//!
//! One rectangle drag creates one shape and one undo step; one stencil
//! drop creates one master instance at the snapped pointer and one undo
//! step.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::cell::RefCell;
use std::rc::Rc;

use egui_kittest::Harness;
use larrygram_commands::app::AppCtx;
use larrygram_commands::builtins;
use larrygram_render::Overlay;
use larrygram_ui::canvas::CanvasCache;
use larrygram_ui::shell::dispatch_one;
use larrygram_ui::tools::{self, Tool, ToolMachine};

use crate::{CanvasRig, app_with_one_shape, screen, shape_number};

fn page_shape_count(app: &AppCtx) -> usize {
    let session = app.active_document().expect("document");
    let page = session.active_page.expect("page");
    session
        .document
        .page(page)
        .map(|page| page.shapes.len())
        .unwrap_or(0)
}

fn undo_len(app: &AppCtx) -> usize {
    app.active_document().expect("document").undo.len()
}

#[test]
fn one_rectangle_drag_creates_one_shape_and_one_undo_step() {
    let (app, existing) = app_with_one_shape();
    let before = page_shape_count(&app);
    let mut rig = CanvasRig::new(app);
    rig.tools.borrow_mut().tool = Tool::Rectangle;
    rig.drag(screen(1.0, 1.0), screen(3.0, 2.0));

    let app = rig.app.borrow();
    assert_eq!(page_shape_count(&app), before + 1);
    let session = app.active_document().expect("document");
    let page = session.active_page.expect("page");
    let shape = *session
        .document
        .page(page)
        .expect("page")
        .shapes
        .last()
        .expect("new shape");
    assert_ne!(shape, existing);
    assert!((shape_number(&app, shape, "Width") - 2.0).abs() < 1.0e-9);
    assert!((shape_number(&app, shape, "Height") - 1.0).abs() < 1.0e-9);
    assert!((shape_number(&app, shape, "PinX") - 2.0).abs() < 1.0e-9);
    assert!((shape_number(&app, shape, "PinY") - 1.5).abs() < 1.0e-9);
    assert_eq!(undo_len(&app), 1, "one drag, one undo step");
    assert_eq!(app.selection.anchor, Some(shape));
    drop(app);

    {
        let app = rig.app.borrow_mut();
        let mut app = app;
        let registry = builtins::registry().expect("registry");
        let id = larrygram_commands::CommandId::parse("edit.undo").expect("id");
        dispatch_one(
            &registry,
            &mut app,
            &id,
            larrygram_commands::Invocation::empty(larrygram_commands::Source::Test),
        )
        .expect("undo");
    }
    rig.run();
    assert_eq!(
        page_shape_count(&rig.app.borrow()),
        before,
        "undo removes it"
    );
}

/// A canvas rig whose events go straight to the stencil tool (no
/// `Tool` variant exists for a Shapes-window drag).
struct StencilRig {
    app: Rc<RefCell<AppCtx>>,
    tools: Rc<RefCell<ToolMachine>>,
    overlay: Rc<RefCell<Overlay>>,
    harness: Harness<'static>,
}

impl StencilRig {
    fn new(app: AppCtx) -> Self {
        let app = Rc::new(RefCell::new(app));
        let cache = Rc::new(RefCell::new(CanvasCache::new()));
        let tools = Rc::new(RefCell::new(ToolMachine::new()));
        let overlay = Rc::new(RefCell::new(Overlay::new()));
        let registry = Rc::new(builtins::registry().expect("registry"));
        let harness = Harness::builder()
            .with_size(egui::vec2(crate::WIDTH, crate::HEIGHT))
            .build_ui({
                let app = Rc::clone(&app);
                let cache = Rc::clone(&cache);
                let tools = Rc::clone(&tools);
                let overlay = Rc::clone(&overlay);
                let registry = Rc::clone(&registry);
                move |ui| {
                    let mut app = app.borrow_mut();
                    let mut cache = cache.borrow_mut();
                    let mut tools = tools.borrow_mut();
                    let mut overlay = overlay.borrow_mut();
                    let events;
                    {
                        let Some(session) = app.active_document_mut() else {
                            return;
                        };
                        let output = larrygram_ui::canvas::show_with_overlay(
                            ui, session, &mut cache, &overlay,
                        )
                        .expect("the canvas draws");
                        events = output.events;
                    }
                    for event in events {
                        let effect = tools::stencil::handle(&mut tools, &event, &app)
                            .expect("stencil event");
                        *overlay = effect.overlay;
                        for intent in effect.intents {
                            let _ = dispatch_one(
                                &registry,
                                &mut app,
                                &intent.command,
                                intent.invocation,
                            );
                        }
                    }
                }
            });
        let mut rig = Self {
            app,
            tools,
            overlay,
            harness,
        };
        rig.run();
        rig
    }

    fn run(&mut self) {
        self.harness.run();
    }

    fn down(&mut self, pos: egui::Pos2) {
        self.harness.drag_at(pos);
        self.harness.run();
    }

    fn move_to(&mut self, pos: egui::Pos2) {
        self.harness.hover_at(pos);
        self.harness.run();
    }

    fn up(&mut self, pos: egui::Pos2) {
        self.harness.drop_at(pos);
        self.harness.run();
    }

    fn drag(&mut self, from: egui::Pos2, to: egui::Pos2) {
        self.down(from);
        self.move_to(to);
        self.up(to);
    }
}

#[test]
fn one_stencil_drop_creates_one_master_instance_and_one_undo_step() {
    let (app, existing) = app_with_one_shape();
    let before = page_shape_count(&app);
    let mut rig = StencilRig::new(app);
    tools::stencil::begin_drag(&mut rig.tools.borrow_mut(), "Basic Shapes", "Rectangle");
    rig.drag(screen(6.0, 8.0), screen(6.0, 8.0));

    let app = rig.app.borrow();
    assert_eq!(page_shape_count(&app), before + 1);
    let session = app.active_document().expect("document");
    let page = session.active_page.expect("page");
    let shape = *session
        .document
        .page(page)
        .expect("page")
        .shapes
        .last()
        .expect("new shape");
    assert_ne!(shape, existing);
    assert!((shape_number(&app, shape, "PinX") - 6.0).abs() < 1.0e-9);
    assert!((shape_number(&app, shape, "PinY") - 8.0).abs() < 1.0e-9);
    assert!((shape_number(&app, shape, "Width") - 1.5).abs() < 1.0e-9);
    assert!((shape_number(&app, shape, "Height") - 1.0).abs() < 1.0e-9);
    assert!(
        session
            .document
            .shape(shape)
            .expect("shape")
            .master
            .is_some()
    );
    assert_eq!(undo_len(&app), 1, "one drop, one undo step");
    assert_eq!(app.selection.anchor, Some(shape));
    drop(app);

    {
        let app = rig.app.borrow_mut();
        let mut app = app;
        let registry = builtins::registry().expect("registry");
        let id = larrygram_commands::CommandId::parse("edit.undo").expect("id");
        dispatch_one(
            &registry,
            &mut app,
            &id,
            larrygram_commands::Invocation::empty(larrygram_commands::Source::Test),
        )
        .expect("undo");
    }
    rig.run();
    assert_eq!(page_shape_count(&rig.app.borrow()), before);
}
