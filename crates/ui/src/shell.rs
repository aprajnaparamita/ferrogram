//! The application shell (ADR 0029, ADR 0031): hosts the ribbon, the
//! canvas, the panels and the status bar, and dispatches every [`Intent`]
//! through the command registry.
//!
//! The frame (top to bottom) is: a title strip with the quick access
//! toolbar, the menu bar (File, View, Help), the ribbon, the canvas
//! area (the Shapes window on the left, the Shape Data and Size &
//! Position panels on the right, page tabs and the status bar below),
//! and the File backstage over the canvas when it is open. Widget
//! failures are shown in the status bar and never crash the frame
//! (AGENTS.md section 6).
//!
//! [`Shell::dispatch`] is the interactive dispatcher (ADR 0028): a
//! [`CommandError::NeedsInput`] from an interactive source is answered
//! through the [`FilePicker`](crate::dialogs) seam (ADR 0036) and the
//! same command is dispatched again with the value filled in;
//! programmatic sources get the error back untouched and no dialog
//! ever opens (AGENTS.md section 6).
//!
//! Owned by task P4-03.

use egui::{Context, PointerButton, Ui};
use ferrogram_commands::{
    AppCtx, ArgValue, CommandError, CommandId, Enabled, Invocation, NeededInput, Outcome,
    PathPurpose, Payload, Registry, Source, UiRequest,
};
use ferrogram_ui_spec::{LayoutKind, LayoutSet, LayoutSets, ShortcutScope, Shortcuts};

use crate::canvas::{self, CanvasCache};
use crate::dialogs::{
    FileChoice, FilePicker, NativeFilePicker, OpenRequest, PickState, SaveRequest,
};
use crate::panels;
use crate::ribbon::Ribbon;
use crate::settings;
use crate::shapes_window::ShapesWindow;
use crate::shortcuts::{self, ShortcutMap};
use crate::{Intent, UiError};

/// The File menu items (registry ids, in order).
const FILE_MENU: &[&str] = &[
    "file.new",
    "file.open",
    "file.save",
    "file.save_as",
    "file.close",
    "app.quit",
];

/// The View menu items (registry ids, in order). The two layout ids
/// render as radio items (ADR 0032).
const VIEW_MENU: &[&str] = &[
    "view.zoom.in",
    "view.zoom.out",
    "view.zoom.fit",
    "view.toggle_grid",
    "view.rulers",
    "view.ui_layout.visio_2016",
    "view.ui_layout.visio_365",
    "app.office.center.options",
];

/// The Help menu items (registry ids, in order).
const HELP_MENU: &[&str] = &["help.about"];

/// The window-level commands the shell runs itself (see
/// [`Shell::dispatch`]): quitting the window and opening the
/// View/Options dialog. Their `commands.yaml` rows stay stubs.
const SHELL_HANDLED: &[&str] = &["app.quit", "app.office.center.options"];

/// Quitting the window (the P4-02 note: "`app.quit` stays a stub (the
/// shell quits itself)").
const QUIT: &str = "app.quit";

/// Opening the View/Options dialog (ADR 0032).
const OPTIONS: &str = "app.office.center.options";

/// The file extensions the picker accepts (ADR 0036).
const OPEN_EXTENSIONS: &[&str] = &["vsdx", "vssx", "vstx"];

/// The default file name of a save dialog for a never-saved document.
const DEFAULT_SAVE_NAME: &str = "drawing.vsdx";

/// The application shell (one per window, single window in Phase 4).
pub struct Shell {
    /// The command registry every intent dispatches through (ADR 0028).
    pub registry: Registry,
    /// Both ribbon layouts, loaded from `ui-spec` (ADR 0031).
    pub layouts: LayoutSets,
    /// The application state (ADR 0029).
    pub app: AppCtx,
    /// The ribbon widget (its tab and backstage state, ADR 0031).
    ribbon: Ribbon,
    /// The Shapes window widget (ADR 0034).
    shapes: ShapesWindow,
    /// The canvas raster cache (ADR 0033).
    cache: CanvasCache,
    /// The dialog seam (ADR 0036); tests inject a fake picker.
    picker: Box<dyn FilePicker>,
    /// The keyboard shortcut map from `shortcuts.yaml`, dispatched
    /// before the widgets (ADR 0036, P4-14).
    shortcuts: ShortcutMap,
    /// The surface the keyboard shortcuts apply to (ADR 0036 scopes,
    /// P4-14 focus tracking).
    focus: ShortcutScope,
    /// The canvas area of the previous frame (focus tracking).
    canvas_rect: Option<egui::Rect>,
    /// The ribbon strip of the previous frame (focus tracking).
    ribbon_rect: Option<egui::Rect>,
    /// A command waiting for an asynchronous pick (the wasm upload,
    /// ADR 0036).
    pending: Option<(CommandId, Invocation)>,
    /// The last command message, shown in the title strip.
    status: String,
    /// Whether the View/Options dialog is open.
    options_open: bool,
    /// Whether the About window is open.
    about_open: bool,
    /// The document name awaiting the close confirmation (ADR 0028
    /// `ConfirmClose`).
    confirm_close: Option<String>,
    /// The window was asked to close; sent to the viewport in
    /// [`Shell::show_in`].
    quit_requested: bool,
}

impl Shell {
    /// Builds a shell from its three owned parts.
    ///
    /// The widget state, the canvas cache, the native file picker and
    /// the keyboard shortcut map are created here (ADR 0029:
    /// widget-local state lives in `crates/ui`; ADR 0036: the map is
    /// built from the loader's `shortcuts.yaml`, the same embedded
    /// source the ribbon uses). `Shell::new`'s signature is fixed; a
    /// table that fails to load falls back to an empty map with one
    /// warning, and [`Shell::set_shortcuts`] replaces the map later.
    pub fn new(registry: Registry, layouts: LayoutSets, app: AppCtx) -> Self {
        let mut app = app;
        let kind = app.settings.ui_layout;
        let (shortcuts, warnings) = load_shortcuts(&layouts.get(kind).shortcuts);
        for warning in warnings {
            app.record_warning(warning);
        }
        let ribbon = Ribbon::new(app.settings.ui_layout);
        Self {
            registry,
            layouts,
            app,
            ribbon,
            shapes: ShapesWindow::new(),
            cache: CanvasCache::new(),
            picker: Box::new(NativeFilePicker),
            shortcuts,
            focus: ShortcutScope::Global,
            canvas_rect: None,
            ribbon_rect: None,
            pending: None,
            status: String::new(),
            options_open: false,
            about_open: false,
            confirm_close: None,
            quit_requested: false,
        }
    }

    /// The active ribbon layout (ADR 0032).
    pub fn layout_kind(&self) -> LayoutKind {
        self.app.settings.ui_layout
    }

    /// Switches the ribbon layout without a restart (ADR 0032).
    pub fn set_layout_kind(&mut self, kind: LayoutKind) {
        self.app.settings.ui_layout = kind;
    }

    /// The layout set for the active kind (ADR 0031).
    pub fn layout_set(&self) -> &LayoutSet {
        self.layouts.get(self.layout_kind())
    }

    /// The last command message (the title strip shows it).
    pub fn status(&self) -> &str {
        &self.status
    }

    /// The status line shown in the title strip: the chords typed so
    /// far while a shortcut sequence is half-typed (ADR 0036, P4-14),
    /// else the last command message.
    pub fn status_line(&self) -> String {
        if self.shortcuts.pending() {
            self.shortcuts.pending_display()
        } else {
            self.status.clone()
        }
    }

    /// Replaces the shortcut map (tests, and a future layout switch):
    /// the same `shortcuts.yaml` load rules as in [`Shell::new`], with
    /// the load warnings copied into `AppCtx::warnings` (ADR 0036).
    pub fn set_shortcuts(&mut self, shortcuts: &Shortcuts) {
        let (map, warnings) = load_shortcuts(shortcuts);
        for warning in warnings {
            self.app.record_warning(warning);
        }
        self.shortcuts = map;
    }

    /// Whether the window was asked to close (File > Exit, ADR 0028).
    pub fn quit_requested(&self) -> bool {
        self.quit_requested
    }

    /// Replaces the dialog seam (tests, and the web app's picker).
    pub fn set_picker(&mut self, picker: Box<dyn FilePicker>) {
        self.picker = picker;
    }

    /// Records a widget or dispatch failure for the status bar. A
    /// failure never crashes the frame (AGENTS.md section 6).
    pub fn note_error(&mut self, error: impl Into<String>) {
        let message = error.into();
        if self.app.warnings.last() != Some(&message) {
            self.app.record_warning(message);
        }
    }

    /// Records which surface the pointer pressed last (the ADR 0036
    /// shortcut scopes): the canvas area is `Page`, the ribbon strip
    /// is `Ribbon`, and any other press falls back to `Global`. This
    /// is the smallest focus tracking that gives the three scopes
    /// (P4-14): the rects are the ones the previous frame drew, so a
    /// press takes effect in its own frame, before the widgets draw.
    fn note_pointer_focus(&mut self, egui_ctx: &Context) {
        let pressed = egui_ctx.input(|input| {
            if input.pointer.button_pressed(PointerButton::Primary) {
                input.pointer.interact_pos()
            } else {
                None
            }
        });
        let Some(pos) = pressed else {
            return;
        };
        if self.canvas_rect.is_some_and(|rect| rect.contains(pos)) {
            self.focus = ShortcutScope::Page;
        } else if self.ribbon_rect.is_some_and(|rect| rect.contains(pos)) {
            self.focus = ShortcutScope::Ribbon;
        } else {
            self.focus = ShortcutScope::Global;
        }
    }

    /// Shows the reason of a disabled command in the status bar (ADR
    /// 0036 "a disabled status-bar hint"), for every intent source;
    /// the error still goes back to the caller untouched.
    fn note_disabled(&mut self, error: &CommandError) {
        if matches!(error, CommandError::NotEnabled { .. }) {
            self.note_error(error.to_string());
        }
    }

    /// Draws one frame and returns the intents the widgets produced.
    ///
    /// This is the fixed P4-01 entry point. egui 0.37 hands the root
    /// [`Ui`] to `eframe::App::ui`, and [`Context::root_ui`] panics
    /// while that ui is borrowed, so the apps call [`Shell::show_in`]
    /// with the ui they were given and this wrapper only serves
    /// callers outside a frame pass.
    pub fn show(&mut self, egui_ctx: &Context) -> Result<Vec<Intent>, UiError> {
        egui_ctx.root_ui(|ui| self.show_in(ui))
    }

    /// Draws one frame into `ui` and returns the intents (ADR 0031).
    ///
    /// The keyboard shortcut pass runs before the widgets (ADR 0036,
    /// P4-14) and its intent joins the widget intents. Widget stubs
    /// report [`UiError::NotImplemented`] in this task's absence;
    /// their error text goes to the status bar and the frame still
    /// renders.
    pub fn show_in(&mut self, ui: &mut Ui) -> Result<Vec<Intent>, UiError> {
        let mut intents = Vec::new();
        self.finish_pick();
        if self.quit_requested {
            self.quit_requested = false;
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }

        // Keyboard shortcuts first, before the widgets draw (ADR 0036
        // "Precedence over egui"): the intent joins the frame's other
        // intents and runs through `Shell::dispatch` like any other
        // (`Source::Shortcut`).
        self.note_pointer_focus(ui.ctx());
        match shortcuts::dispatch(ui.ctx(), &mut self.shortcuts, &self.app, self.focus) {
            Ok(Some(intent)) => intents.push(intent),
            Ok(None) => {}
            Err(error) => self.note_error(error.to_string()),
        }

        // Title strip: document title, status line, quick access
        // toolbar (ADR 0031).
        egui::Panel::top("shell-title").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Ferrogram");
                ui.separator();
                ui.label(self.title_text());
                ui.separator();
                ui.label(self.status_line());
                let kind = self.app.settings.ui_layout;
                let set = self.layouts.get(kind);
                let produced = self
                    .ribbon
                    .show_quick_access(ui, set, &self.registry, &self.app);
                self.absorb(produced, &mut intents);
            });
        });

        // Menu bar (File, View, Help).
        egui::Panel::top("shell-menu").show(ui, |ui| {
            egui::menu::MenuBar::default().ui(ui, |ui| {
                self.show_menu(ui, "File", FILE_MENU, &mut intents);
                self.show_menu(ui, "View", VIEW_MENU, &mut intents);
                self.show_menu(ui, "Help", HELP_MENU, &mut intents);
            });
        });

        // The ribbon (or the backstage over the canvas, ADR 0031).
        let kind = self.app.settings.ui_layout;
        if self.ribbon.kind() != kind {
            self.ribbon.set_kind(kind);
        }
        egui::Panel::top("shell-ribbon").show(ui, |ui| {
            self.ribbon_rect = Some(ui.max_rect());
            let set = self.layouts.get(kind);
            let produced = self.ribbon.show(ui, set, &self.registry, &self.app);
            self.absorb(produced, &mut intents);
        });

        // Shapes window (left) and the two panels (right).
        egui::Panel::left("shell-shapes")
            .resizable(true)
            .default_size(200.0)
            .show(ui, |ui| {
                let produced = self.shapes.show(ui, &self.app);
                if let Some(_picks) = self.keep(produced) {
                    // Picking a stencil inserts a shape in Phase 5;
                    // the Shapes window keeps the selection itself.
                }
            });
        egui::Panel::right("shell-panels")
            .resizable(true)
            .default_size(220.0)
            .show(ui, |ui| {
                let shape_data = panels::show_shape_data(ui, &self.app);
                let _ = self.keep(shape_data);
                let size_position = panels::show_size_position(ui, &self.app);
                let _ = self.keep(size_position);
            });

        // Canvas area: page tabs and status bar below it.
        egui::Panel::bottom("shell-status").show(ui, |ui| {
            let produced = canvas::show_status_bar(ui, &self.app);
            let _ = self.keep(produced);
        });
        egui::Panel::bottom("shell-tabs").show(ui, |ui| {
            let mut produced = Vec::new();
            if self.app.active.is_some() {
                let session = self.app.active_document_mut();
                if let Some(session) = session {
                    match canvas::show_page_tabs(ui, session) {
                        Ok(mut more) => produced.append(&mut more),
                        Err(error) => self.note_error(error.to_string()),
                    }
                }
            }
            intents.extend(produced);
        });

        egui::CentralPanel::default().show(ui, |ui| {
            if self.ribbon.backstage_open() {
                let set = self.layouts.get(kind);
                let produced = self
                    .ribbon
                    .show_backstage(ui, set, &self.registry, &self.app);
                let opened = matches!(&produced, Ok(intents) if !intents.is_empty());
                self.absorb(produced, &mut intents);
                if opened {
                    // A backstage command runs: leave the File view
                    // (ADR 0031).
                    self.ribbon.set_backstage_open(false);
                }
            } else {
                self.show_canvas_area(ui, &mut intents);
            }
        });

        self.show_dialogs(ui, &mut intents);
        Ok(intents)
    }

    /// The canvas area: the page raster with the rulers over its
    /// bands (ADR 0033), or a placeholder with no open document. The
    /// area is remembered for the shortcut focus tracking (P4-14).
    fn show_canvas_area(&mut self, ui: &mut Ui, intents: &mut Vec<Intent>) {
        let units = self.app.settings.units;
        let rect = ui.available_rect_before_wrap();
        self.canvas_rect = Some(rect);
        if self.app.active_document().is_none() {
            ui.centered_and_justified(|ui| {
                ui.label("No document");
            });
            return;
        }
        // The two canvas functions draw into the same area (the
        // rulers paint over the canvas bands, P4-05) and their errors
        // are recorded after the document borrow ends.
        let mut produced = Vec::new();
        let mut errors = Vec::new();
        {
            let session = self.app.active_document_mut();
            if let Some(session) = session {
                ui.scope_builder(
                    egui::UiBuilder::new().max_rect(rect),
                    |ui| match canvas::show(ui, session, &mut self.cache) {
                        Ok(output) => produced.extend(output.intents),
                        Err(error) => errors.push(error.to_string()),
                    },
                );
            }
        }
        {
            let session = self.app.active_document();
            if let Some(session) = session {
                ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                    if let Err(error) = canvas::show_rulers(ui, session, units) {
                        errors.push(error.to_string());
                    }
                });
            }
        }
        for error in errors {
            self.note_error(error);
        }
        intents.extend(produced);
    }

    /// The modal windows: View/Options, About and the close
    /// confirmation (ADR 0028 [`UiRequest`] flow).
    fn show_dialogs(&mut self, ui: &mut Ui, intents: &mut Vec<Intent>) {
        if self.options_open {
            let mut open = true;
            egui::Window::new("Options")
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ui.ctx(), |ui| {
                    if let Err(error) = settings::show_options(ui, &mut self.app.settings) {
                        self.note_error(error.to_string());
                    }
                });
            self.options_open = open;
        }
        if self.about_open {
            let mut open = true;
            egui::Window::new("About Ferrogram")
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ui.ctx(), |ui| {
                    ui.heading("Ferrogram");
                    ui.label("A clean-room, Visio-compatible diagramming app (alpha).");
                    ui.label("Original work; no Microsoft assets (ADR 0030).");
                });
            self.about_open = open;
        }
        if let Some(name) = self.confirm_close.clone() {
            let mut answer: Option<bool> = None;
            egui::Window::new("Unsaved changes")
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    ui.label(format!("Unsaved changes in {name}."));
                    ui.horizontal(|ui| {
                        if ui.button("Close without saving").clicked() {
                            answer = Some(true);
                        }
                        if ui.button("Cancel").clicked() {
                            answer = Some(false);
                        }
                    });
                });
            if let Some(close) = answer {
                self.confirm_close = None;
                if close && let Ok(command) = CommandId::parse("file.close") {
                    let args =
                        ferrogram_commands::Args::new().with("confirm", ArgValue::Flag(true));
                    intents.push(Intent::new(command, Invocation::new(args, Source::Ui)));
                }
            }
        }
    }

    /// One menu of the menu bar (File, View, Help). Every item is a
    /// registry id (ADR 0028); the two layout ids render as radios.
    fn show_menu(
        &mut self,
        ui: &mut Ui,
        title: &str,
        ids: &[&'static str],
        intents: &mut Vec<Intent>,
    ) {
        ui.menu_button(title, |ui| {
            for &id in ids {
                let checked = match id {
                    "view.ui_layout.visio_2016" => {
                        Some(self.app.settings.ui_layout == LayoutKind::Visio2016)
                    }
                    "view.ui_layout.visio_365" => {
                        Some(self.app.settings.ui_layout == LayoutKind::Visio365)
                    }
                    _ => None,
                };
                if let Some(intent) = self.menu_item(ui, id, checked) {
                    intents.push(intent);
                    ui.close();
                }
            }
        });
    }

    /// One menu item for a registry id: `Some(intent)` when clicked.
    ///
    /// The label and the enabled state come from the registry; the
    /// shell-handled window commands ([`SHELL_HANDLED`]) are always
    /// enabled because their registry rows are still stubs.
    fn menu_item(
        &mut self,
        ui: &mut Ui,
        id: &'static str,
        checked: Option<bool>,
    ) -> Option<Intent> {
        let Ok(command) = CommandId::parse(id) else {
            return None;
        };
        let label = self
            .registry
            .meta(&command)
            .map(|meta| meta.label.clone())
            .unwrap_or_else(|| id.to_owned());
        let enabled = self.menu_enabled(&command);
        let response = ui.add_enabled_ui(enabled.value, |ui| match checked {
            Some(selected) => ui.radio(selected, label),
            None => ui.button(label),
        });
        let response = response.inner;
        let response = match &enabled.reason {
            Some(reason) => response.on_hover_text(reason.clone()),
            None => response,
        };
        if response.clicked() {
            Some(Intent::new(command, Invocation::empty(Source::Menu)))
        } else {
            None
        }
    }

    /// The enabled state of a menu item (see [`SHELL_HANDLED`]).
    fn menu_enabled(&self, command: &CommandId) -> Enabled {
        if SHELL_HANDLED.contains(&command.as_str()) {
            Enabled::yes()
        } else {
            self.registry.enabled(command, &self.app)
        }
    }

    /// The title strip's document text.
    fn title_text(&self) -> String {
        match self.app.active_document() {
            Some(session) => session.display_name(),
            None => "No document".to_owned(),
        }
    }

    /// Keeps the value of a widget call and shows its failure in the
    /// status bar (never a crash, AGENTS.md section 6).
    fn keep<T>(&mut self, produced: Result<T, UiError>) -> Option<T> {
        match produced {
            Ok(value) => Some(value),
            Err(error) => {
                self.note_error(error.to_string());
                None
            }
        }
    }

    /// Keeps the intents of a widget and shows its failure in the
    /// status bar.
    fn absorb(&mut self, produced: Result<Vec<Intent>, UiError>, intents: &mut Vec<Intent>) {
        if let Some(mut more) = self.keep(produced) {
            intents.append(&mut more);
        }
    }

    /// Dispatches intents through the registry (ADR 0028), in order.
    ///
    /// One result per intent. A cancelled dialog dispatches nothing
    /// and reports [`Outcome::message`] "Cancelled" (ADR 0036).
    pub fn dispatch(
        &mut self,
        intents: &[Intent],
    ) -> Result<Vec<Result<Outcome, CommandError>>, UiError> {
        let mut results = Vec::with_capacity(intents.len());
        for intent in intents {
            results.push(self.dispatch_intent(intent)?);
        }
        Ok(results)
    }

    /// Dispatches one intent, answering a `NeedsInput` from an
    /// interactive source through the picker (ADR 0036).
    fn dispatch_intent(
        &mut self,
        intent: &Intent,
    ) -> Result<Result<Outcome, CommandError>, UiError> {
        // Window-level commands the shell runs itself (ADR 0028 note
        // from P4-02: `app.quit` stays a stub and the shell quits).
        match intent.command.as_str() {
            QUIT => {
                self.quit_requested = true;
                return Ok(Ok(Outcome::message("Quit")));
            }
            OPTIONS => {
                self.options_open = true;
                let mut outcome = Outcome::message("Options");
                outcome.ui_request = Some(UiRequest::ShowOptions);
                return Ok(Ok(outcome));
            }
            _ => {}
        }

        let source = intent.invocation.source;
        let first = dispatch_one(
            &self.registry,
            &mut self.app,
            &intent.command,
            intent.invocation.clone(),
        );
        let outcome = match first {
            Ok(outcome) => outcome,
            Err(UiError::Command(CommandError::NeedsInput(needed))) if source.is_interactive() => {
                match self.ask_picker(&intent.command, &intent.invocation, &needed)? {
                    Ask::Again(invocation) => {
                        match dispatch_one(
                            &self.registry,
                            &mut self.app,
                            &intent.command,
                            invocation,
                        ) {
                            Ok(outcome) => outcome,
                            Err(UiError::Command(error)) => {
                                self.note_disabled(&error);
                                return Ok(Err(error));
                            }
                            Err(error) => return Err(error),
                        }
                    }
                    Ask::Pending => {
                        self.pending = Some((intent.command, intent.invocation.clone()));
                        return Ok(Ok(Outcome::message("Waiting for the file dialog")));
                    }
                    Ask::Cancelled => return Ok(Ok(Outcome::message("Cancelled"))),
                    Ask::NoDialog(needed) => {
                        return Ok(Err(CommandError::NeedsInput(needed)));
                    }
                }
            }
            Err(UiError::Command(error)) => {
                self.note_disabled(&error);
                return Ok(Err(error));
            }
            Err(error) => return Err(error),
        };
        self.apply_outcome(&outcome)?;
        Ok(Ok(outcome))
    }

    /// Asks the [`FilePicker`] for a `NeedsInput` value (ADR 0036).
    ///
    /// `Ok(None)` from the picker is a cancel ("cancelling a dialog
    /// dispatches nothing") unless an asynchronous pick is still in
    /// flight (the wasm upload).
    fn ask_picker(
        &mut self,
        command: &CommandId,
        invocation: &Invocation,
        needed: &NeededInput,
    ) -> Result<Ask, UiError> {
        match needed {
            NeededInput::Path {
                purpose: PathPurpose::OpenFile,
                suggested,
            } => {
                let request = OpenRequest {
                    title: format!("Open ({command})"),
                    extensions: OPEN_EXTENSIONS.iter().map(|e| (*e).to_owned()).collect(),
                    suggested: suggested.clone(),
                };
                let choice = match self.picker.open(&request)? {
                    Some(choice) => Some(choice),
                    None => match self.picker.poll() {
                        PickState::Picked(choice) => Some(choice),
                        PickState::Pending => return Ok(Ask::Pending),
                        _ => None,
                    },
                };
                Ok(match choice {
                    Some(choice) => Ask::Again(with_choice(invocation, choice)),
                    None => Ask::Cancelled,
                })
            }
            NeededInput::Path {
                purpose: PathPurpose::SaveFile,
                suggested,
            } => {
                let default_name = suggested
                    .as_ref()
                    .and_then(|path| path.file_name())
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| DEFAULT_SAVE_NAME.to_owned());
                let request = SaveRequest {
                    title: format!("Save ({command})"),
                    default_name,
                };
                // Empty data: ask for the target only (the command
                // writes the file when dispatched with `path`).
                Ok(match self.picker.save(&request, &[])? {
                    Some(choice) => Ask::Again(with_choice(invocation, choice)),
                    None => Ask::Cancelled,
                })
            }
            // No dialog kind for folders, text and numbers: the
            // programmatic contract, error back untouched.
            _ => Ok(Ask::NoDialog(needed.clone())),
        }
    }

    /// Acts on the follow-up requests of a completed command (ADR
    /// 0028 [`UiRequest`], ADR 0036 [`Payload`]).
    fn apply_outcome(&mut self, outcome: &Outcome) -> Result<(), UiError> {
        if let Some(message) = &outcome.message {
            self.status = message.clone();
        }
        match &outcome.ui_request {
            Some(UiRequest::ShowAbout) => self.about_open = true,
            Some(UiRequest::ShowOptions) => self.options_open = true,
            Some(UiRequest::ConfirmClose { name }) => self.confirm_close = Some(name.clone()),
            _ => {}
        }
        if let Some(Payload::Bytes { name, data }) = &outcome.payload {
            let request = SaveRequest {
                title: "Save".to_owned(),
                default_name: name.clone(),
            };
            self.picker.save(&request, data)?;
        }
        Ok(())
    }

    /// Completes a pending asynchronous pick (the wasm upload, ADR
    /// 0036): dispatches the waiting command with the picked bytes.
    fn finish_pick(&mut self) {
        let choice = match self.picker.poll() {
            PickState::Picked(choice) => Some(choice),
            PickState::Cancelled => {
                self.pending = None;
                None
            }
            _ => None,
        };
        let Some(choice) = choice else {
            return;
        };
        let Some((command, invocation)) = self.pending.take() else {
            return;
        };
        let intent = Intent::new(command, with_choice(&invocation, choice));
        match self.dispatch_intent(&intent) {
            Ok(Ok(outcome)) => {
                if let Some(message) = outcome.message {
                    self.status = message;
                }
            }
            Ok(Err(error)) => self.note_error(error.to_string()),
            Err(error) => self.note_error(error.to_string()),
        }
    }
}

/// What to do with a `NeedsInput` value once the picker answered (or
/// could not).
enum Ask {
    /// Dispatch the command again with these arguments.
    Again(Invocation),
    /// An asynchronous pick is in flight; dispatch when it finishes.
    Pending,
    /// The user cancelled; dispatch nothing.
    Cancelled,
    /// No dialog answers this input; hand the error back.
    NoDialog(NeededInput),
}

/// Builds the shortcut map of a `shortcuts.yaml` table with its load
/// warnings (ADR 0036: `ShortcutMap::conflicts` go to
/// `AppCtx::warnings` at load). A table that fails to load becomes an
/// empty map with one warning (AGENTS.md section 6: never crash).
fn load_shortcuts(shortcuts: &Shortcuts) -> (ShortcutMap, Vec<String>) {
    match ShortcutMap::from_shortcuts(shortcuts) {
        Ok(map) => {
            let warnings = map.conflicts().to_vec();
            (map, warnings)
        }
        Err(error) => (
            ShortcutMap::default(),
            vec![format!("shortcuts not loaded: {error}")],
        ),
    }
}

/// Fills the `path` (desktop) or `name` + `data` (upload) arguments of
/// a re-dispatch (ADR 0036).
fn with_choice(invocation: &Invocation, choice: FileChoice) -> Invocation {
    let mut invocation = invocation.clone();
    match choice {
        FileChoice::Path(path) => {
            invocation.args.set("path", ArgValue::Path(path));
        }
        FileChoice::Bytes { name, data } => {
            invocation.args.set("name", ArgValue::Text(name));
            invocation.args.set("data", ArgValue::Bytes(data));
        }
    }
    invocation
}

/// Dispatches one already-built invocation; the seam the CLI and the
/// Phase 8 control channel reuse (ADR 0028).
///
/// The command's undoable edit (when it produced one) is pushed onto
/// the active document's undo stack after a successful run (ADR 0028).
pub fn dispatch_one(
    registry: &Registry,
    app: &mut AppCtx,
    command: &CommandId,
    invocation: Invocation,
) -> Result<Outcome, UiError> {
    let mut outcome = registry
        .execute(command, app, invocation)
        .map_err(UiError::Command)?;
    if let Some(edit) = outcome.edit.take() {
        match app.active_document_mut() {
            Some(session) => {
                session.undo.push(edit);
            }
            None => app.record_warning("an undoable edit with no open document was dropped"),
        }
    }
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use crate::dialogs::{OpenRequest, SaveRequest};
    use ferrogram_commands::{
        Command, CommandMeta, CommandStatus, DocumentSession, Edit, Settings, StubCommand,
    };
    use ferrogram_model::Document;
    use ferrogram_ui_spec::Loader;

    /// The committed layouts (the app loads the same way).
    fn sets() -> LayoutSets {
        Loader::from_embedded().expect("the embedded layouts load")
    }

    fn shell() -> Shell {
        Shell::new(
            Registry::new(),
            LayoutSets::default(),
            AppCtx::new(Settings::default()),
        )
    }

    /// A shell over the real builtin registry and layouts.
    fn full_shell() -> Shell {
        Shell::new(
            ferrogram_commands::builtins::registry().expect("registry"),
            sets(),
            AppCtx::new(Settings::default()),
        )
    }

    /// A picker that records its calls and answers with a fixed
    /// choice.
    #[derive(Default)]
    struct FakePicker {
        choice: Option<FileChoice>,
        open_calls: Rc<RefCell<usize>>,
        save_calls: Rc<RefCell<Vec<Vec<u8>>>>,
    }

    impl FilePicker for FakePicker {
        fn open(&self, _request: &OpenRequest) -> Result<Option<FileChoice>, UiError> {
            *self.open_calls.borrow_mut() += 1;
            Ok(self.choice.clone())
        }
        fn save(&self, _request: &SaveRequest, data: &[u8]) -> Result<Option<FileChoice>, UiError> {
            self.save_calls.borrow_mut().push(data.to_vec());
            Ok(self.choice.clone())
        }
    }

    /// A command that demands a path once and succeeds when it gets
    /// one; counts its runs.
    struct NeedsPath {
        runs: Arc<AtomicUsize>,
    }

    impl Command for NeedsPath {
        fn meta(&self) -> &CommandMeta {
            use std::sync::OnceLock;
            static META: OnceLock<CommandMeta> = OnceLock::new();
            META.get_or_init(|| {
                CommandMeta::new(
                    CommandId::parse("file.needs_path").expect("id"),
                    "Needs Path",
                    CommandStatus::Implemented,
                )
            })
        }
        fn enabled(&self, _ctx: &AppCtx) -> Enabled {
            Enabled::yes()
        }
        fn execute(
            &self,
            _ctx: &mut AppCtx,
            invocation: &Invocation,
        ) -> Result<Outcome, CommandError> {
            self.runs.fetch_add(1, Ordering::SeqCst);
            match (
                invocation.args.path("path"),
                invocation.args.text("name"),
                invocation.args.bytes("data"),
            ) {
                (Some(path), _, _) => Ok(Outcome::message(format!("got {}", path.display()))),
                (None, Some(name), Some(data)) => Ok(Outcome::message(format!(
                    "got {name} ({} bytes)",
                    data.len()
                ))),
                _ => Err(CommandError::NeedsInput(NeededInput::Path {
                    purpose: PathPurpose::OpenFile,
                    suggested: None,
                })),
            }
        }
    }

    /// A command whose outcome carries a payload (the wasm download).
    struct MakesPayload;

    impl Command for MakesPayload {
        fn meta(&self) -> &CommandMeta {
            use std::sync::OnceLock;
            static META: OnceLock<CommandMeta> = OnceLock::new();
            META.get_or_init(|| {
                CommandMeta::new(
                    CommandId::parse("file.makes_payload").expect("id"),
                    "Makes Payload",
                    CommandStatus::Implemented,
                )
            })
        }
        fn enabled(&self, _ctx: &AppCtx) -> Enabled {
            Enabled::yes()
        }
        fn execute(
            &self,
            _ctx: &mut AppCtx,
            _invocation: &Invocation,
        ) -> Result<Outcome, CommandError> {
            let mut outcome = Outcome::message("saved");
            outcome.payload = Some(Payload::Bytes {
                name: "drawing.vsdx".to_owned(),
                data: vec![1, 2, 3],
            });
            Ok(outcome)
        }
    }

    /// A command whose outcome carries one undoable edit.
    struct NoopEdit;

    impl Edit for NoopEdit {
        fn label(&self) -> &str {
            "noop"
        }
        fn apply(&mut self, _ctx: &mut AppCtx) -> Result<(), CommandError> {
            Ok(())
        }
        fn revert(&mut self, _ctx: &mut AppCtx) -> Result<(), CommandError> {
            Ok(())
        }
    }

    struct MakesEdit;

    impl Command for MakesEdit {
        fn meta(&self) -> &CommandMeta {
            use std::sync::OnceLock;
            static META: OnceLock<CommandMeta> = OnceLock::new();
            META.get_or_init(|| {
                CommandMeta::new(
                    CommandId::parse("file.makes_edit").expect("id"),
                    "Makes Edit",
                    CommandStatus::Implemented,
                )
            })
        }
        fn enabled(&self, _ctx: &AppCtx) -> Enabled {
            Enabled::yes()
        }
        fn execute(
            &self,
            _ctx: &mut AppCtx,
            _invocation: &Invocation,
        ) -> Result<Outcome, CommandError> {
            let mut outcome = Outcome::message("edited");
            outcome.edit = Some(Box::new(NoopEdit));
            Ok(outcome)
        }
    }

    fn intent(id: &str, source: Source) -> Intent {
        let command = CommandId::intern(id).expect("id");
        Intent::new(command, Invocation::empty(source))
    }

    #[test]
    fn shell_tracks_the_layout_kind() {
        let mut shell = shell();
        assert_eq!(shell.layout_kind(), LayoutKind::Visio2016);
        shell.set_layout_kind(LayoutKind::Visio365);
        assert_eq!(shell.layout_kind(), LayoutKind::Visio365);
        assert_eq!(shell.layout_set(), &shell.layouts.visio_365);
    }

    #[test]
    fn dispatch_one_runs_a_command() {
        let registry = ferrogram_commands::builtins::registry().expect("registry");
        let mut app = AppCtx::new(Settings::default());
        let id = CommandId::parse("file.new").expect("id");
        let outcome = dispatch_one(&registry, &mut app, &id, Invocation::empty(Source::Test))
            .expect("file.new runs");
        assert_eq!(outcome.message.as_deref(), Some("New drawing"));
        assert_eq!(app.documents.len(), 1);
    }

    #[test]
    fn dispatch_one_wraps_command_errors() {
        let registry = Registry::new();
        let mut app = AppCtx::new(Settings::default());
        let id = CommandId::parse("file.new").expect("id");
        let error = dispatch_one(&registry, &mut app, &id, Invocation::empty(Source::Test))
            .expect_err("unknown");
        assert!(
            matches!(error, UiError::Command(CommandError::UnknownCommand { .. })),
            "{error:?}"
        );
    }

    #[test]
    fn dispatch_one_pushes_the_undo_edit() {
        let mut registry = Registry::new();
        registry.register(Box::new(MakesEdit)).expect("register");
        let mut app = AppCtx::new(Settings::default());
        app.open_document(DocumentSession::new(Document::new(), None));
        let id = CommandId::parse("file.makes_edit").expect("id");
        dispatch_one(&registry, &mut app, &id, Invocation::empty(Source::Test)).expect("edit");
        let session = app.active_document().expect("document");
        assert!(session.undo.can_undo());
    }

    #[test]
    fn an_edit_without_a_document_is_dropped_with_a_warning() {
        let mut registry = Registry::new();
        registry.register(Box::new(MakesEdit)).expect("register");
        let mut app = AppCtx::new(Settings::default());
        let id = CommandId::parse("file.makes_edit").expect("id");
        dispatch_one(&registry, &mut app, &id, Invocation::empty(Source::Test)).expect("edit");
        assert_eq!(app.warnings.len(), 1);
    }

    #[test]
    fn the_window_commands_are_shell_handled() {
        let mut shell = full_shell();
        let quit = intent("app.quit", Source::Menu);
        let results = shell.dispatch(&[quit]).expect("dispatch");
        assert_eq!(results.len(), 1);
        assert!(shell.quit_requested());
        assert_eq!(
            results[0].as_ref().expect("quit").message.as_deref(),
            Some("Quit")
        );

        let options = intent("app.office.center.options", Source::Menu);
        let results = shell.dispatch(&[options]).expect("dispatch");
        assert!(matches!(
            results[0].as_ref().expect("options").ui_request,
            Some(UiRequest::ShowOptions)
        ));
    }

    #[test]
    fn needs_input_picks_a_path_and_dispatches_again() {
        let mut registry = Registry::new();
        let runs = Arc::new(AtomicUsize::new(0));
        registry
            .register(Box::new(NeedsPath {
                runs: Arc::clone(&runs),
            }))
            .expect("register");
        let mut shell = Shell::new(
            registry,
            LayoutSets::default(),
            AppCtx::new(Settings::default()),
        );
        shell.set_picker(Box::new(FakePicker {
            choice: Some(FileChoice::Path("/tmp/a.vsdx".into())),
            ..FakePicker::default()
        }));
        let results = shell
            .dispatch(&[intent("file.needs_path", Source::Ui)])
            .expect("dispatch");
        assert_eq!(
            runs.load(Ordering::SeqCst),
            2,
            "once without input, once with"
        );
        let outcome = results[0].as_ref().expect("picked");
        assert_eq!(outcome.message.as_deref(), Some("got /tmp/a.vsdx"));
    }

    #[test]
    fn needs_input_takes_the_upload_bytes() {
        let mut registry = Registry::new();
        let runs = Arc::new(AtomicUsize::new(0));
        registry
            .register(Box::new(NeedsPath {
                runs: Arc::clone(&runs),
            }))
            .expect("register");
        let mut shell = Shell::new(
            registry,
            LayoutSets::default(),
            AppCtx::new(Settings::default()),
        );
        shell.set_picker(Box::new(FakePicker {
            choice: Some(FileChoice::Bytes {
                name: "a.vsdx".into(),
                data: vec![7, 7],
            }),
            ..FakePicker::default()
        }));
        let results = shell
            .dispatch(&[intent("file.needs_path", Source::Ribbon)])
            .expect("dispatch");
        assert_eq!(runs.load(Ordering::SeqCst), 2);
        let outcome = results[0].as_ref().expect("uploaded");
        assert_eq!(outcome.message.as_deref(), Some("got a.vsdx (2 bytes)"));
    }

    #[test]
    fn cancelling_a_dialog_dispatches_nothing() {
        let mut registry = Registry::new();
        let runs = Arc::new(AtomicUsize::new(0));
        registry
            .register(Box::new(NeedsPath {
                runs: Arc::clone(&runs),
            }))
            .expect("register");
        let mut shell = Shell::new(
            registry,
            LayoutSets::default(),
            AppCtx::new(Settings::default()),
        );
        shell.set_picker(Box::new(FakePicker::default()));
        let results = shell
            .dispatch(&[intent("file.needs_path", Source::Menu)])
            .expect("dispatch");
        assert_eq!(
            runs.load(Ordering::SeqCst),
            1,
            "only the unanswered dispatch ran"
        );
        let outcome = results[0].as_ref().expect("cancelled");
        assert_eq!(outcome.message.as_deref(), Some("Cancelled"));
    }

    #[test]
    fn programmatic_sources_get_the_error_back_untouched() {
        let mut registry = Registry::new();
        let runs = Arc::new(AtomicUsize::new(0));
        let opens = Rc::new(RefCell::new(0));
        registry
            .register(Box::new(NeedsPath {
                runs: Arc::clone(&runs),
            }))
            .expect("register");
        let mut shell = Shell::new(
            registry,
            LayoutSets::default(),
            AppCtx::new(Settings::default()),
        );
        shell.set_picker(Box::new(FakePicker {
            choice: Some(FileChoice::Path("/tmp/a.vsdx".into())),
            open_calls: Rc::clone(&opens),
            save_calls: Rc::new(RefCell::new(Vec::new())),
        }));
        let results = shell
            .dispatch(&[intent("file.needs_path", Source::Cli)])
            .expect("dispatch");
        assert_eq!(runs.load(Ordering::SeqCst), 1);
        assert_eq!(*opens.borrow(), 0, "no dialog for a programmatic call");
        assert!(matches!(
            results[0],
            Err(CommandError::NeedsInput(NeededInput::Path { .. }))
        ));
    }

    #[test]
    fn a_payload_is_handed_to_the_picker() {
        let mut registry = Registry::new();
        registry.register(Box::new(MakesPayload)).expect("register");
        let saves = Rc::new(RefCell::new(Vec::new()));
        let mut shell = Shell::new(
            registry,
            LayoutSets::default(),
            AppCtx::new(Settings::default()),
        );
        shell.set_picker(Box::new(FakePicker {
            choice: None,
            open_calls: Rc::new(RefCell::new(0)),
            save_calls: Rc::clone(&saves),
        }));
        let results = shell
            .dispatch(&[intent("file.makes_payload", Source::Ui)])
            .expect("dispatch");
        assert_eq!(
            results[0].as_ref().expect("payload").message.as_deref(),
            Some("saved")
        );
        assert_eq!(saves.borrow().as_slice(), [vec![1, 2, 3]]);
    }

    #[test]
    fn input_no_dialog_kind_answers_keeps_the_error() {
        /// A command that demands a number: no dialog answers that.
        struct NeedsNumber;

        impl Command for NeedsNumber {
            fn meta(&self) -> &CommandMeta {
                use std::sync::OnceLock;
                static META: OnceLock<CommandMeta> = OnceLock::new();
                META.get_or_init(|| {
                    CommandMeta::new(
                        CommandId::parse("file.needs_number").expect("id"),
                        "Needs Number",
                        CommandStatus::Implemented,
                    )
                })
            }
            fn enabled(&self, _ctx: &AppCtx) -> Enabled {
                Enabled::yes()
            }
            fn execute(
                &self,
                _ctx: &mut AppCtx,
                _invocation: &Invocation,
            ) -> Result<Outcome, CommandError> {
                Err(CommandError::NeedsInput(NeededInput::Number {
                    prompt: "a scale".to_owned(),
                    default: None,
                }))
            }
        }

        let mut registry = Registry::new();
        registry.register(Box::new(NeedsNumber)).expect("register");
        let mut shell = Shell::new(
            registry,
            LayoutSets::default(),
            AppCtx::new(Settings::default()),
        );
        shell.set_picker(Box::new(FakePicker::default()));
        let results = shell
            .dispatch(&[intent("file.needs_number", Source::Ui)])
            .expect("dispatch");
        assert!(matches!(
            results[0],
            Err(CommandError::NeedsInput(NeededInput::Number { .. }))
        ));
    }

    #[test]
    fn a_stub_command_reports_unsupported() {
        let mut registry = Registry::new();
        registry
            .register(Box::new(StubCommand::new(CommandMeta::new(
                CommandId::parse("file.stub").expect("id"),
                "Stub",
                CommandStatus::Stub,
            ))))
            .expect("register");
        let mut shell = Shell::new(
            registry,
            LayoutSets::default(),
            AppCtx::new(Settings::default()),
        );
        let results = shell
            .dispatch(&[intent("file.stub", Source::Ui)])
            .expect("dispatch");
        assert!(matches!(results[0], Err(CommandError::Unsupported { .. })));
    }

    #[test]
    fn a_real_frame_renders_every_widget_without_errors() {
        let mut shell = full_shell();
        shell
            .app
            .open_document(DocumentSession::new(Document::new(), None));
        let ctx = Context::default();
        for _ in 0..2 {
            let mut output = ctx.run_ui(Default::default(), |ui| {
                let intents = shell.show_in(ui).expect("the frame renders");
                let results = shell.dispatch(&intents).expect("dispatch");
                assert!(results.iter().all(|result| result.is_ok()));
            });
            output.textures_delta.clear();
        }
        // Every widget is implemented (P4-07 filled the last stub), so a
        // plain frame over an empty document records no widget error.
        // `note_error` reaching the status bar is tested separately.
        assert!(
            !shell
                .app
                .warnings
                .iter()
                .any(|warning| warning.contains("not implemented")),
            "no widget stub errors: {:?}",
            shell.app.warnings
        );
    }

    #[test]
    fn show_through_the_context_draws_the_frame() {
        let mut shell = full_shell();
        let ctx = Context::default();
        ctx.begin_pass(Default::default());
        let intents = shell.show(&ctx).expect("the frame renders");
        let mut output = ctx.end_pass();
        output.textures_delta.clear();
        assert!(intents.is_empty());
    }

    #[test]
    fn with_choice_fills_path_or_upload_arguments() {
        let invocation = Invocation::empty(Source::Ui);
        let path = with_choice(&invocation, FileChoice::Path("/tmp/a.vsdx".into()));
        assert_eq!(
            path.args.path("path"),
            Some(std::path::Path::new("/tmp/a.vsdx"))
        );
        let upload = with_choice(
            &invocation,
            FileChoice::Bytes {
                name: "a.vsdx".into(),
                data: vec![1],
            },
        );
        assert_eq!(upload.args.text("name"), Some("a.vsdx"));
        assert_eq!(upload.args.bytes("data"), Some(&[1u8][..]));
    }

    #[test]
    fn the_title_text_names_the_document() {
        let mut shell = shell();
        assert_eq!(shell.title_text(), "No document");
        shell
            .app
            .open_document(DocumentSession::new(Document::new(), None));
        assert_eq!(shell.title_text(), "Untitled");
    }

    #[test]
    fn repeated_errors_are_recorded_once() {
        let mut shell = shell();
        shell.note_error("ribbon is not implemented yet");
        shell.note_error("ribbon is not implemented yet");
        assert_eq!(shell.app.warnings.len(), 1);
    }

    /// One shell frame with the given input events (the shortcut pass
    /// runs inside `show_in`, before the widgets).
    #[allow(clippy::expect_used)] // test helper: a failure here is the bug
    fn shell_frame(shell: &mut Shell, events: Vec<egui::Event>, time: f64) {
        let ctx = Context::default();
        let mut output = ctx.run_ui(
            egui::RawInput {
                events,
                time: Some(time),
                ..Default::default()
            },
            |ui| {
                let intents = shell.show_in(ui).expect("the frame renders");
                let _ = shell.dispatch(&intents).expect("dispatch");
            },
        );
        output.textures_delta.clear();
    }

    #[allow(clippy::expect_used)] // test helper: a failure here is the bug
    fn key_event(key: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }
    }

    #[allow(clippy::expect_used)] // test helper: a failure here is the bug
    fn press_event(pos: egui::Pos2) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::default(),
        }
    }

    /// The committed `shortcuts.yaml` table (the loader's).
    #[allow(clippy::expect_used)] // test helper: a failure here is the bug
    fn committed_shortcuts() -> Shortcuts {
        sets().get(LayoutKind::Visio2016).shortcuts.clone()
    }

    /// A two-chord sequence (`Alt+H, F`): the committed table has
    /// none (P4-14 Result).
    #[allow(clippy::expect_used)] // test helper: a failure here is the bug
    fn keytip_table() -> Shortcuts {
        Shortcuts {
            entries: vec![ferrogram_ui_spec::Shortcut {
                sequence: vec![vec!["Alt".to_owned(), "H".to_owned()], vec!["F".to_owned()]],
                command: ferrogram_ui_spec::CommandId::parse("arrange.group").expect("id"),
                scope: ShortcutScope::Global,
                section: None,
            }],
        }
    }

    #[test]
    fn the_shell_loads_the_committed_table_without_conflicts() {
        // The same embedded `shortcuts.yaml` the ribbon uses; the
        // committed table has no (sequence, scope) conflict (P4-14).
        let mut shell = full_shell();
        shell.set_shortcuts(&committed_shortcuts());
        assert!(shell.app.warnings.is_empty(), "{:?}", shell.app.warnings);
    }

    #[test]
    fn alt_f6_dispatches_view_zoom_in_through_the_registry() {
        // `Alt+F6` is `view.zoom.in` (`global` in shortcuts.yaml): a
        // synthesized key event in a real frame runs the command
        // through the registry and changes the zoom, and the key event
        // never reaches the widgets (P4-14 acceptance).
        let mut shell = full_shell();
        shell
            .app
            .open_document(DocumentSession::new(Document::new(), None));
        let zoom_before = shell.app.active_document().expect("document").view.zoom;
        let ctx = Context::default();
        // egui may run the closure for several passes (`run_ui`
        // multipass); the key event arrives in the first one. Collect
        // the intents of every pass, like the app does per pass.
        let mut events_after: Vec<usize> = Vec::new();
        let mut intents_seen: Vec<Intent> = Vec::new();
        let mut output = ctx.run_ui(
            egui::RawInput {
                events: vec![key_event(
                    egui::Key::F6,
                    egui::Modifiers {
                        alt: true,
                        ..Default::default()
                    },
                )],
                time: Some(0.0),
                ..Default::default()
            },
            |ui| {
                let intents = shell.show_in(ui).expect("the frame renders");
                events_after.push(ui.input(|input| input.events.len()));
                let results = shell.dispatch(&intents).expect("dispatch");
                assert!(results.iter().all(|result| result.is_ok()));
                intents_seen.extend(intents);
            },
        );
        output.textures_delta.clear();
        let fired: Vec<&Intent> = intents_seen
            .iter()
            .filter(|intent| intent.invocation.source == Source::Shortcut)
            .collect();
        assert_eq!(fired.len(), 1, "{intents_seen:?}");
        assert_eq!(fired[0].command.as_str(), "view.zoom.in");
        assert!(
            events_after.iter().all(|left| *left == 0),
            "the key event does not reach the widgets: {events_after:?}"
        );
        let zoom_after = shell.app.active_document().expect("document").view.zoom;
        assert!(zoom_after > zoom_before, "view.zoom.in changed the zoom");
    }

    #[test]
    fn a_half_typed_sequence_shows_in_the_status_line() {
        let mut shell = full_shell();
        shell.set_shortcuts(&keytip_table());
        shell_frame(&mut shell, Vec::new(), 0.0);
        assert_eq!(shell.status_line(), "");

        // `Alt+H` starts the sequence: the chords typed so far show.
        shell_frame(
            &mut shell,
            vec![key_event(
                egui::Key::H,
                egui::Modifiers {
                    alt: true,
                    ..Default::default()
                },
            )],
            1.0,
        );
        assert!(shell.shortcuts.pending(), "half-typed");
        assert_eq!(shell.status_line(), "Alt+H");

        // `F` completes it: the status line is the message again.
        shell_frame(
            &mut shell,
            vec![key_event(egui::Key::F, egui::Modifiers::default())],
            2.0,
        );
        assert!(!shell.shortcuts.pending());
        assert_eq!(shell.status_line(), shell.status());
    }

    #[test]
    fn clicking_the_canvas_or_the_ribbon_sets_the_focus_scope() {
        let mut shell = full_shell();
        shell_frame(&mut shell, Vec::new(), 0.0);
        let canvas = shell.canvas_rect.expect("the canvas rect is recorded");
        let ribbon = shell.ribbon_rect.expect("the ribbon rect is recorded");
        assert_eq!(shell.focus, ShortcutScope::Global);

        shell_frame(&mut shell, vec![press_event(canvas.center())], 1.0);
        assert_eq!(shell.focus, ShortcutScope::Page);

        shell_frame(&mut shell, vec![press_event(ribbon.center())], 2.0);
        assert_eq!(shell.focus, ShortcutScope::Ribbon);

        shell_frame(&mut shell, vec![press_event(egui::pos2(0.0, 0.0))], 3.0);
        assert_eq!(shell.focus, ShortcutScope::Global);
    }

    #[test]
    fn shortcut_conflicts_reach_the_warnings_at_load() {
        let mut shell = shell();
        let table = Shortcuts {
            entries: vec![
                ferrogram_ui_spec::Shortcut {
                    sequence: vec![vec!["Ctrl".to_owned(), "G".to_owned()]],
                    command: ferrogram_ui_spec::CommandId::parse("arrange.group").expect("id"),
                    scope: ShortcutScope::Global,
                    section: None,
                },
                ferrogram_ui_spec::Shortcut {
                    sequence: vec![vec!["Ctrl".to_owned(), "G".to_owned()]],
                    command: ferrogram_ui_spec::CommandId::parse("arrange.ungroup").expect("id"),
                    scope: ShortcutScope::Global,
                    section: None,
                },
                ferrogram_ui_spec::Shortcut {
                    sequence: vec![vec!["Ctrl".to_owned(), "H".to_owned()]],
                    command: ferrogram_ui_spec::CommandId::parse("arrange.group").expect("id"),
                    scope: ShortcutScope::Global,
                    section: None,
                },
            ],
        };
        shell.set_shortcuts(&table);
        assert_eq!(
            shell.app.warnings.len(),
            1,
            "one real conflict, the rest is legal"
        );
        assert!(shell.app.warnings[0].contains("conflicting shortcut"));
    }

    #[test]
    fn a_table_that_fails_to_load_records_one_warning() {
        let mut shell = shell();
        let bad = Shortcuts {
            entries: vec![ferrogram_ui_spec::Shortcut {
                sequence: vec![vec!["Warp".to_owned()]],
                command: ferrogram_ui_spec::CommandId::parse("edit.undo").expect("id"),
                scope: ShortcutScope::Global,
                section: None,
            }],
        };
        shell.set_shortcuts(&bad);
        assert_eq!(shell.app.warnings.len(), 1);
        assert!(shell.app.warnings[0].contains("shortcuts not loaded"));
    }

    #[test]
    fn a_disabled_command_shows_its_reason_in_the_status_bar() {
        /// A command whose enabled-state says no, with a reason.
        struct Disabled;

        impl Command for Disabled {
            fn meta(&self) -> &CommandMeta {
                use std::sync::OnceLock;
                static META: OnceLock<CommandMeta> = OnceLock::new();
                META.get_or_init(|| {
                    CommandMeta::new(
                        CommandId::parse("file.disabled").expect("id"),
                        "Disabled",
                        CommandStatus::Implemented,
                    )
                })
            }
            fn enabled(&self, _ctx: &AppCtx) -> Enabled {
                Enabled::no("nothing to close")
            }
            fn execute(
                &self,
                _ctx: &mut AppCtx,
                _invocation: &Invocation,
            ) -> Result<Outcome, CommandError> {
                Ok(Outcome::message("ran"))
            }
        }

        let mut registry = Registry::new();
        registry.register(Box::new(Disabled)).expect("register");
        let mut shell = Shell::new(
            registry,
            LayoutSets::default(),
            AppCtx::new(Settings::default()),
        );
        let results = shell
            .dispatch(&[intent("file.disabled", Source::Shortcut)])
            .expect("dispatch");
        assert!(matches!(results[0], Err(CommandError::NotEnabled { .. })));
        // The reason reaches the status bar for any intent source.
        assert!(
            shell
                .app
                .warnings
                .last()
                .is_some_and(|warning| warning.contains("nothing to close")),
            "{:?}",
            shell.app.warnings
        );
    }
}
