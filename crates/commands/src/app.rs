//! Application state (ADR 0029): open documents, the active page, the
//! selection, view state and settings.
//!
//! Everything a command mutates lives here, in L4 (ADR 0002), so headless
//! callers (the CLI today, the JSON control channel and MCP in Phase 8)
//! share one state type with the UI. The egui crates never appear.

use std::path::PathBuf;

use ferrogram_model::{Document, PageId};
use ferrogram_ui_spec::LayoutKind;
use serde::{Deserialize, Serialize};

use crate::SettingsError;
use crate::undo::UndoStack;

/// How the UI formats lengths at the screen edge (AGENTS.md section 6:
/// internal units stay inches; only presentation converts).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub enum DisplayUnits {
    /// Inches (the internal unit).
    #[default]
    Inches,
    /// Millimeters.
    Millimeters,
    /// Centimeters.
    Centimeters,
    /// Meters.
    Meters,
    /// Feet.
    Feet,
}

impl DisplayUnits {
    /// Every accepted name, in menu order. The `view.units` argument
    /// and the Options dialog use these spellings (P4-15).
    pub const NAMES: [&'static str; 5] = ["Inches", "Millimeters", "Centimeters", "Meters", "Feet"];

    /// The name as [`Self::NAMES`] spells it.
    pub fn name(self) -> &'static str {
        match self {
            DisplayUnits::Inches => "Inches",
            DisplayUnits::Millimeters => "Millimeters",
            DisplayUnits::Centimeters => "Centimeters",
            DisplayUnits::Meters => "Meters",
            DisplayUnits::Feet => "Feet",
        }
    }

    /// Parses a [`Self::NAMES`] spelling, ignoring ASCII case.
    ///
    /// ```
    /// use ferrogram_commands::DisplayUnits;
    /// assert_eq!(DisplayUnits::parse("millimeters"), Some(DisplayUnits::Millimeters));
    /// assert_eq!(DisplayUnits::parse("furlongs"), None);
    /// ```
    pub fn parse(text: &str) -> Option<Self> {
        match text.to_ascii_lowercase().as_str() {
            "inches" => Some(DisplayUnits::Inches),
            "millimeters" => Some(DisplayUnits::Millimeters),
            "centimeters" => Some(DisplayUnits::Centimeters),
            "meters" => Some(DisplayUnits::Meters),
            "feet" => Some(DisplayUnits::Feet),
            _ => None,
        }
    }
}

/// The view state of one open document (ADR 0029).
#[derive(Debug, Clone, PartialEq)]
pub struct ViewState {
    /// Page-space pan offset, in inches.
    pub pan: ferrogram_geom::Point,
    /// View scale multiplier (1.0 = 100%); finite and positive.
    pub zoom: f64,
    /// Whether the grid is drawn (ADR 0033).
    pub show_grid: bool,
    /// Whether the rulers are drawn (ADR 0033).
    pub show_rulers: bool,
}

impl ViewState {
    /// The default view: no pan, 100% zoom, grid and rulers on.
    pub fn new() -> Self {
        Self {
            pan: ferrogram_geom::Point::new(0.0, 0.0),
            zoom: 1.0,
            show_grid: true,
            show_rulers: true,
        }
    }

    /// The default view for a new document session: [`Self::new`],
    /// with the grid and rulers toggles seeded from the persisted
    /// settings (ADR 0032 amendment: `Settings::show_grid` and
    /// `Settings::show_rulers` are the defaults for new sessions).
    pub fn from_settings(settings: &Settings) -> Self {
        Self {
            show_grid: settings.show_grid,
            show_rulers: settings.show_rulers,
            ..Self::new()
        }
    }
}

impl Default for ViewState {
    fn default() -> Self {
        Self::new()
    }
}

/// The selected shapes (ADR 0029).
///
/// Phase 4 has no selection: [`Selection::len`] is always 0. Phase 5
/// adds the shape ids here without changing any signature that takes a
/// `&Selection`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Selection;

impl Selection {
    /// An empty selection.
    pub fn new() -> Self {
        Self
    }

    /// Number of selected shapes. Always 0 until Phase 5.
    pub fn len(&self) -> usize {
        0
    }

    /// True while nothing is selected (always in Phase 4).
    pub fn is_empty(&self) -> bool {
        true
    }
}

/// The identity of one [`DocumentSession`] (P4-15).
///
/// The key is assigned once per session and never reused within the
/// process, so an [`Edit`](crate::Edit) whose `apply`/`revert` opens or
/// closes documents can still find the session it belongs to: the
/// document index moves, the identity does not. The wrapped number is
/// opaque; compare ids, do not order by them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SessionId(u64);

/// The next session identity (a process-local counter; wrapping is
/// harmless in practice at 2^64 sessions).
fn next_session_id() -> SessionId {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    SessionId(NEXT.fetch_add(1, Ordering::Relaxed))
}

/// One open document with its per-document state (ADR 0029).
#[derive(Debug)]
pub struct DocumentSession {
    /// Stable identity of this session (P4-15; see [`SessionId`]).
    pub id: SessionId,
    /// The document (model, L1).
    pub document: Document,
    /// Where it was loaded from or last saved to; `None` when never saved.
    pub path: Option<PathBuf>,
    /// Whether it has unsaved changes.
    pub dirty: bool,
    /// The page shown on the canvas, or `None` before the first page.
    pub active_page: Option<PageId>,
    /// Pan, zoom and the grid and rulers toggles.
    pub view: ViewState,
    /// The per-document undo stack (ADR 0028).
    pub undo: UndoStack,
    /// The package the document was read from, kept for the lossless
    /// patch-based writer (ADR 0018: `write_document` patches the
    /// source parts so unmapped content survives a save). `None` for a
    /// document that was never read from a file (task P4-02 extension
    /// of the ADR 0029 field list).
    pub preserve: Option<ferrogram_vsdx::Package>,
}

impl DocumentSession {
    /// Wraps a document. The active page is the document's first page.
    pub fn new(document: Document, path: Option<PathBuf>) -> Self {
        let active_page = document.pages.first().map(|page| page.id);
        Self {
            id: next_session_id(),
            document,
            path,
            dirty: false,
            active_page,
            view: ViewState::new(),
            undo: UndoStack::new(UndoStack::DEFAULT_CAPACITY),
            preserve: None,
        }
    }

    /// Keeps the package the document was read from (ADR 0018).
    pub fn with_preserve(mut self, preserve: ferrogram_vsdx::Package) -> Self {
        self.preserve = Some(preserve);
        self
    }

    /// The display name: the file name, or "Untitled" when never saved.
    pub fn display_name(&self) -> String {
        match &self.path {
            Some(path) => path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Untitled".to_owned()),
            None => "Untitled".to_owned(),
        }
    }
}

/// The persisted user settings (ADR 0029, ADR 0032).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Settings {
    /// The ribbon layout (ADR 0032).
    pub ui_layout: LayoutKind,
    /// Whether the grid is drawn by default.
    pub show_grid: bool,
    /// Whether the rulers are drawn by default.
    pub show_rulers: bool,
    /// How the UI formats lengths.
    pub units: DisplayUnits,
    /// Recently opened files, most recent first.
    pub recent_files: Vec<PathBuf>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            ui_layout: LayoutKind::Visio2016,
            show_grid: true,
            show_rulers: true,
            units: DisplayUnits::Inches,
            recent_files: Vec::new(),
        }
    }
}

/// Loads and saves [`Settings`] (ADR 029 seam, ADR 0032 storage).
///
/// The apps implement this over `eframe::Storage`; the CLI never
/// persists settings. A corrupt store is a [`SettingsError::Format`],
/// never a panic.
pub trait SettingsStore: Send {
    /// Reads the stored settings, or reports why they could not be read.
    fn load(&mut self) -> Result<Settings, SettingsError>;

    /// Writes the settings.
    fn save(&mut self, settings: &Settings) -> Result<(), SettingsError>;
}

/// The application state every command sees (ADR 0029).
#[derive(Debug)]
pub struct AppCtx {
    /// Open documents, in open order.
    pub documents: Vec<DocumentSession>,
    /// Index of the active document in `documents`, if any.
    pub active: Option<usize>,
    /// The selection (empty in Phase 4).
    pub selection: Selection,
    /// The user settings.
    pub settings: Settings,
    /// Recent warnings, capped at 200 entries (ADR 0021 style).
    pub warnings: Vec<String>,
}

impl AppCtx {
    /// Maximum number of warnings kept.
    pub const WARNING_CAP: usize = 200;

    /// An empty application state with the given settings.
    pub fn new(settings: Settings) -> Self {
        Self {
            documents: Vec::new(),
            active: None,
            selection: Selection::new(),
            settings,
            warnings: Vec::new(),
        }
    }

    /// The active document.
    pub fn active_document(&self) -> Option<&DocumentSession> {
        self.active.and_then(|index| self.documents.get(index))
    }

    /// The active document, mutably.
    pub fn active_document_mut(&mut self) -> Option<&mut DocumentSession> {
        self.active.and_then(|index| self.documents.get_mut(index))
    }

    /// Opens a session and makes it active; returns its index.
    pub fn open_document(&mut self, session: DocumentSession) -> usize {
        self.documents.push(session);
        let index = self.documents.len() - 1;
        self.active = Some(index);
        index
    }

    /// Closes the document at `index`, keeping the active index valid.
    ///
    /// The active document is kept when a *different* document closes:
    /// `active` only shifts down when it sat after `index` (P4-15
    /// review minor 5), and only moves when the active document itself
    /// closes (then the next document takes over, or the previous one
    /// when the last one closed).
    ///
    /// Returns the closed session, or `None` when `index` is stale.
    pub fn close_document(&mut self, index: usize) -> Option<DocumentSession> {
        if index >= self.documents.len() {
            return None;
        }
        let closed = self.documents.remove(index);
        self.active = match self.active {
            None => None,
            Some(active) if active > index => Some(active - 1),
            Some(active) if active == index => {
                if self.documents.is_empty() {
                    None
                } else {
                    Some(index.min(self.documents.len() - 1))
                }
            }
            Some(active) => Some(active),
        };
        Some(closed)
    }

    /// Records a warning for the status bar (capped ring, ADR 0029).
    pub fn record_warning(&mut self, message: impl Into<String>) {
        if self.warnings.len() >= Self::WARNING_CAP {
            self.warnings.remove(0);
        }
        self.warnings.push(message.into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferrogram_model::Document;

    #[test]
    fn view_state_defaults() {
        let view = ViewState::new();
        assert_eq!(view.zoom, 1.0);
        assert_eq!(view.pan, ferrogram_geom::Point::new(0.0, 0.0));
        assert!(view.show_grid);
        assert!(view.show_rulers);
        assert_eq!(view, ViewState::default());
    }

    /// The ADR 0032 amendment: a new session's toggles come from the
    /// settings defaults (P4-15 review major 1).
    #[test]
    fn view_state_seeds_the_toggles_from_the_settings() {
        let settings = Settings {
            show_grid: false,
            show_rulers: true,
            ..Settings::default()
        };
        let view = ViewState::from_settings(&settings);
        assert!(!view.show_grid);
        assert!(view.show_rulers);
        assert_eq!(view.zoom, 1.0);
        assert_eq!(view.pan, ferrogram_geom::Point::new(0.0, 0.0));
    }

    #[test]
    fn display_units_names_round_trip() {
        for name in DisplayUnits::NAMES {
            let units = DisplayUnits::parse(name).unwrap_or_else(|| panic!("{name} parses"));
            assert_eq!(units.name(), name);
            let lower = name.to_ascii_lowercase();
            assert_eq!(DisplayUnits::parse(&lower), Some(units), "{lower}");
        }
        assert_eq!(DisplayUnits::parse("furlongs"), None);
        assert_eq!(DisplayUnits::parse(""), None);
    }

    #[test]
    fn selection_is_empty_in_phase_four() {
        let selection = Selection::new();
        assert!(selection.is_empty());
        assert_eq!(selection.len(), 0);
    }

    #[test]
    fn settings_default_to_the_2016_layout() {
        let settings = Settings::default();
        assert_eq!(settings.ui_layout, LayoutKind::Visio2016);
        assert_eq!(settings.units, DisplayUnits::Inches);
        assert!(settings.recent_files.is_empty());
    }

    #[test]
    fn document_session_takes_the_first_page_and_names_itself() {
        let mut document = Document::new();
        document
            .pages
            .push(ferrogram_model::Page::new(PageId { index: 0 }, 0));
        let session = DocumentSession::new(document, None);
        assert_eq!(session.active_page, Some(PageId { index: 0 }));
        assert_eq!(session.display_name(), "Untitled");
        assert!(!session.dirty);
        assert!(!session.undo.can_undo());

        let mut document = Document::new();
        document
            .pages
            .push(ferrogram_model::Page::new(PageId { index: 0 }, 0));
        let session = DocumentSession::new(document, Some(PathBuf::from("/tmp/a.vsdx")));
        assert_eq!(session.display_name(), "a.vsdx");
    }

    #[test]
    fn document_session_keeps_the_source_package() {
        let mut document = Document::new();
        document
            .pages
            .push(ferrogram_model::Page::new(PageId { index: 0 }, 0));
        let session = DocumentSession::new(document, None);
        assert!(session.preserve.is_none());
        let session = session.with_preserve(ferrogram_vsdx::Package::new());
        assert!(session.preserve.is_some());
    }

    #[test]
    fn app_ctx_tracks_the_active_document() {
        let mut ctx = AppCtx::new(Settings::default());
        assert!(ctx.active_document().is_none());
        assert!(ctx.active_document_mut().is_none());

        let first = ctx.open_document(DocumentSession::new(Document::new(), None));
        assert_eq!(first, 0);
        assert!(ctx.active_document().is_some());

        let second = ctx.open_document(DocumentSession::new(Document::new(), None));
        assert_eq!(second, 1);

        assert!(ctx.close_document(9).is_none());
        assert!(ctx.close_document(0).is_some());
        assert_eq!(ctx.active, Some(0));
        assert!(ctx.close_document(0).is_some());
        assert_eq!(ctx.active, None);
    }

    /// P4-15 review minor 5: closing a *different* document keeps the
    /// user where they are; `active` only shifts when it sat after the
    /// closed index and only moves when the active document closes.
    #[test]
    fn closing_another_document_keeps_the_active_one() {
        let mut ctx = AppCtx::new(Settings::default());
        ctx.open_document(DocumentSession::new(Document::new(), None));
        ctx.open_document(DocumentSession::new(Document::new(), None));
        ctx.open_document(DocumentSession::new(Document::new(), None));
        let third = ctx.active_document().expect("active").id;

        // Close the first while the third is active: the third stays
        // active and shifts down to index 1.
        assert!(ctx.close_document(0).is_some());
        assert_eq!(ctx.active, Some(1));
        assert_eq!(ctx.active_document().map(|s| s.id), Some(third));

        // Close the active one: the next document takes over.
        assert!(ctx.close_document(1).is_some());
        assert_eq!(ctx.active, Some(0));

        // A close before the active index shifts it down.
        ctx.open_document(DocumentSession::new(Document::new(), None));
        assert_eq!(ctx.active, Some(1));
        assert!(ctx.close_document(0).is_some());
        assert_eq!(ctx.active, Some(0));
    }

    /// P4-15 review minor 3: sessions carry their own identity.
    #[test]
    fn session_ids_identify_sessions_not_indices() {
        let one = DocumentSession::new(Document::new(), None);
        let two = DocumentSession::new(Document::new(), None);
        assert_ne!(one.id, two.id, "ids are unique per session");
        assert_eq!(one.id, one.id, "ids are stable");
    }

    #[test]
    fn warnings_are_capped() {
        let mut ctx = AppCtx::new(Settings::default());
        for i in 0..(AppCtx::WARNING_CAP + 5) {
            ctx.record_warning(format!("w{i}"));
        }
        assert_eq!(ctx.warnings.len(), AppCtx::WARNING_CAP);
        assert_eq!(ctx.warnings.first().map(String::as_str), Some("w5"));
        assert_eq!(ctx.warnings.last().map(String::as_str), Some("w204"));
    }
}
