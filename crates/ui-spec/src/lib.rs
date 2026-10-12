//! Loader for ribbon, backstage, context-menu and shortcut layout data.
//!
//! Phase 1 fills this crate's public types; P1-06 implements the
//! loader logic. This file declares the public surface only; no
//! runtime logic is here yet. See ADR 0007 (format), ADR 0008
//! (ShapeSheet table) and the `ui-spec/` data directory for the inputs.
//!
//! The crate is L0 (no workspace deps), so it is usable by any other
//! layer without breaking the layering rules (ADR 0002).

#![forbid(unsafe_code)]
#![allow(clippy::derive_partial_eq_without_eq)] // IDs are content-addressed; equality is fine.

use std::collections::BTreeMap;

/// A Ferrogram command id. See ADR 0006.
///
/// The grammar (enforced by `ui-spec/schema.json`):
///
/// ```text
/// command_id  := scope '.' verb ('.' qualifier)*
/// scope       := 'app' | 'file' | 'edit' | 'view' | 'insert' | 'format'
///              | 'home' | 'design' | 'page' | 'review' | 'developer'
///              | 'data' | 'process' | 'shape' | 'stencils' | 'help'
/// verb        := [a-z][a-z0-9_]*
/// qualifier   := [a-z][a-z0-9_]*
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommandId(pub String);

impl CommandId {
    /// Wraps a string in `CommandId` without validation. The loader
    /// is the right place to enforce the grammar; the type itself is
    /// a newtype.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// The dotted id string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for CommandId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for CommandId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// A Microsoft Fluent UI control name (e.g. `FileSave`).
///
/// Distinct from [`CommandId`] (ADR 0006): the Fluent name is a
/// ribbon-XML identifier; the command id is Ferrogram's own.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FluentId(pub String);

impl FluentId {
    /// Wraps a string without validation.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// The Fluent name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for FluentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Fluent control types. The full vocabulary is enumerated in
/// `ui-spec/schema.json` under `definitions/controlType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControlType {
    /// A top-level ribbon tab.
    Tab,
    /// A tab set inside a ribbon (rarely used in Visio).
    TabSet,
    /// A group of controls inside a tab.
    Group,
    /// A click-only button.
    Button,
    /// A two-state button.
    ToggleButton,
    /// A button with a primary action and a dropdown.
    SplitButton,
    /// A grid of items.
    Gallery,
    /// A drop-down menu.
    Menu,
    /// A checkable item.
    CheckBox,
    /// A drop-down list.
    DropDown,
    /// A combined edit + dropdown.
    ComboBox,
    /// A non-interactive label.
    LabelControl,
    /// A generic container.
    Control,
    /// A small button that opens a dialog from a group's corner.
    ButtonDialogBoxLauncher,
    /// A context menu.
    ContextMenu,
    /// A task.
    Task,
    /// A task form group.
    TaskFormGroup,
    /// A backstage category.
    Category,
    /// A placeholder (used internally; never appears in a real layout).
    None,
}

impl ControlType {
    /// Parses the Fluent type string. Returns `None` for unknown
    /// values; the loader treats unknowns as a schema error.
    pub fn parse(s: &str) -> Option<Self> {
        use ControlType as Ct;
        let v = match s {
            "tab" => Ct::Tab,
            "tabSet" => Ct::TabSet,
            "group" => Ct::Group,
            "button" => Ct::Button,
            "toggleButton" => Ct::ToggleButton,
            "splitButton" => Ct::SplitButton,
            "gallery" => Ct::Gallery,
            "menu" => Ct::Menu,
            "checkBox" => Ct::CheckBox,
            "dropDown" => Ct::DropDown,
            "comboBox" => Ct::ComboBox,
            "labelControl" => Ct::LabelControl,
            "control" => Ct::Control,
            "buttonDialogBoxLauncher" => Ct::ButtonDialogBoxLauncher,
            "contextMenu" => Ct::ContextMenu,
            "task" => Ct::Task,
            "taskFormGroup" => Ct::TaskFormGroup,
            "category" => Ct::Category,
            "none" => Ct::None,
            _ => return None,
        };
        Some(v)
    }

    /// The string form used in YAML.
    pub fn as_str(&self) -> &'static str {
        use ControlType::*;
        match self {
            Tab => "tab",
            TabSet => "tabSet",
            Group => "group",
            Button => "button",
            ToggleButton => "toggleButton",
            SplitButton => "splitButton",
            Gallery => "gallery",
            Menu => "menu",
            CheckBox => "checkBox",
            DropDown => "dropDown",
            ComboBox => "comboBox",
            LabelControl => "labelControl",
            Control => "control",
            ButtonDialogBoxLauncher => "buttonDialogBoxLauncher",
            ContextMenu => "contextMenu",
            Task => "task",
            TaskFormGroup => "taskFormGroup",
            Category => "category",
            None => "none",
        }
    }
}

/// Bounding box in device-independent pixels. Filled in by the
/// screenshot pass (ADR 0007).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Bounds {
    /// X coordinate (left edge).
    pub x: f32,
    /// Y coordinate (top edge).
    pub y: f32,
    /// Width in pixels.
    pub w: f32,
    /// Height in pixels.
    pub h: f32,
}

/// A control in a ribbon, backstage, or context menu. See
/// `ui-spec/schema.json#/definitions/control`.
#[derive(Debug, Clone, PartialEq)]
pub struct Control {
    /// The control's command id. For actionable controls this is the
    /// primary command. For containers (groups, tabs) it is a stable
    /// container id.
    pub id: CommandId,
    /// The Fluent type.
    pub r#type: ControlType,
    /// Short UI label (never a description).
    pub label: String,
    /// Sibling order.
    pub order: u32,
    /// Immediate parent control id, or `None` for top-level.
    pub parent: Option<CommandId>,
    /// Alternate command ids that also dispatch this control
    /// (e.g. when the same action lives on two ribbons).
    pub alt_ids: Vec<CommandId>,
    /// Reserved for the screenshot pass.
    pub bounds: Option<Bounds>,
    /// Reserved for the screenshot pass. Path or id of an icon asset.
    pub icon_ref: Option<String>,
    /// Reserved for the screenshot pass. The keytip shown next to the
    /// label.
    pub keytip: Option<String>,
    /// Reserved for the screenshot pass. The button size variant.
    pub size_variant: Option<SizeVariant>,
    /// Reserved for the screenshot pass. Free-text annotation; never a
    /// description of behaviour.
    pub visual_notes: Option<String>,
}

/// Reserved button size variant. See ADR 0007.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SizeVariant {
    /// Small button.
    Small,
    /// Medium button.
    Medium,
    /// Large button.
    Large,
    /// Extra-large button.
    ExtraLarge,
}

impl SizeVariant {
    /// Parses the YAML string.
    pub fn parse(s: &str) -> Option<Self> {
        let v = match s {
            "small" => Self::Small,
            "medium" => Self::Medium,
            "large" => Self::Large,
            "extra-large" => Self::ExtraLarge,
            _ => return None,
        };
        Some(v)
    }
}

/// A group of controls inside a tab.
#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    /// Group id.
    pub id: CommandId,
    /// Group label.
    pub label: String,
    /// Sibling order.
    pub order: u32,
    /// Controls in display order.
    pub controls: Vec<Control>,
}

/// A top-level ribbon tab.
#[derive(Debug, Clone, PartialEq)]
pub struct Tab {
    /// Tab id.
    pub id: CommandId,
    /// Tab label.
    pub label: String,
    /// Sibling order.
    pub order: u32,
    /// Fluent tab-set name (e.g. `TabSetPrimary`); `None` for tabs
    /// that are not part of a tab set.
    pub tab_set: Option<String>,
    /// Groups in display order.
    pub groups: Vec<Group>,
}

/// Backstage surface. Tabs are flat (no groups).
#[derive(Debug, Clone, PartialEq)]
pub struct Backstage {
    /// Tabs under the File button.
    pub tabs: Vec<BackstageTab>,
}

/// A single backstage tab. Always flat (no groups); controls are
/// shown as a vertical stack.
#[derive(Debug, Clone, PartialEq)]
pub struct BackstageTab {
    /// Tab id.
    pub id: CommandId,
    /// Tab label.
    pub label: String,
    /// Sibling order.
    pub order: u32,
    /// Controls in display order.
    pub controls: Vec<Control>,
}

/// A context menu.
#[derive(Debug, Clone, PartialEq)]
pub struct ContextMenu {
    /// Menu id.
    pub id: CommandId,
    /// Menu label (shown in the menu header; can be empty for
    /// canvas context menus).
    pub label: String,
    /// Sibling order.
    pub order: u32,
    /// Controls in display order.
    pub controls: Vec<Control>,
}

/// The Quick Access Toolbar (QAT).
#[derive(Debug, Clone, PartialEq)]
pub struct QuickAccessToolbar {
    /// Controls in display order.
    pub controls: Vec<Control>,
}

/// A complete layout (e.g. `visio-2016.yaml`).
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    /// Always `1` for now.
    pub schema_version: u32,
    /// Always `visio` for now.
    pub product: String,
    /// Human-readable source description (e.g. "Microsoft Fluent UI
    /// 365, Current Channel").
    pub source: String,
    /// URL of the source dataset.
    pub source_url: String,
    /// Pinning info (commit, channel, date).
    pub source_revision: String,
    /// When the file was generated.
    pub extracted: String,
    /// Top-level ribbon tabs (excludes the File tab's backstage).
    pub tabs: Vec<Tab>,
    /// Backstage tabs under the File button.
    pub backstage: Backstage,
    /// Context menus (canvas + per-shape).
    pub context_menus: Vec<ContextMenu>,
    /// Quick Access Toolbar.
    pub quick_access_toolbar: QuickAccessToolbar,
}

/// Mapping from Ferrogram command id to VisUICmds and to one or more
/// Fluent control names. See `ui-spec/commands.yaml` and ADR 0006.
#[derive(Debug, Clone, PartialEq)]
pub struct CommandMapping {
    /// The Ferrogram command id.
    pub id: CommandId,
    /// Short UI label.
    pub label: String,
    /// The VisUICmds constant (e.g. `visCmdFileSave`), or `None` when
    /// no mapping is known.
    pub visuicmd: Option<String>,
    /// Fluent control names that surface this command.
    pub fluent_ids: Vec<FluentId>,
    /// How confident the mapping is.
    pub mapping_confidence: MappingConfidence,
}

/// Confidence in a command mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MappingConfidence {
    /// The VisUICmds name and the Fluent control name are both known
    /// and point at the same action.
    Exact,
    /// At least one side is inferred by name match.
    Inferred,
    /// Neither side is known; the command is registered but unimplemented.
    Unknown,
}

impl MappingConfidence {
    /// Parses the YAML string.
    pub fn parse(s: &str) -> Option<Self> {
        let v = match s {
            "exact" => Self::Exact,
            "inferred" => Self::Inferred,
            "unknown" => Self::Unknown,
            _ => return None,
        };
        Some(v)
    }

    /// The YAML string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Inferred => "inferred",
            Self::Unknown => "unknown",
        }
    }
}

/// A keyboard shortcut. See `ui-spec/shortcuts.yaml` and ADR 0007.
#[derive(Debug, Clone, PartialEq)]
pub struct Shortcut {
    /// Chord tokens, in canonical order (modifier-first, then key).
    /// E.g. `[Ctrl, Shift, Z]`.
    pub keys: Vec<String>,
    /// The command the shortcut dispatches.
    pub command: CommandId,
    /// Where the shortcut applies.
    pub scope: ShortcutScope,
    /// Optional human-readable section from the source page.
    pub section: Option<String>,
}

/// Where a shortcut applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShortcutScope {
    /// Always available.
    Global,
    /// Only when the ribbon has focus.
    Ribbon,
    /// Only when a drawing page has focus.
    Page,
    /// Only when a text editor has focus.
    Text,
    /// Only when one or more shapes are selected.
    Selection,
    /// Only when a particular window is active.
    Window,
}

impl ShortcutScope {
    /// Parses the YAML string.
    pub fn parse(s: &str) -> Option<Self> {
        let v = match s {
            "global" => Self::Global,
            "ribbon" => Self::Ribbon,
            "page" => Self::Page,
            "text" => Self::Text,
            "selection" => Self::Selection,
            "window" => Self::Window,
            _ => return None,
        };
        Some(v)
    }
}

/// A `ui-spec/commands.yaml` document.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Commands {
    /// One entry per Ferrogram command.
    pub entries: Vec<CommandMapping>,
}

/// A `ui-spec/shortcuts.yaml` document.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Shortcuts {
    /// One entry per keyboard shortcut.
    pub entries: Vec<Shortcut>,
}

/// The full set of data loaded for a single layout (e.g. 2016).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LayoutSet {
    /// The ribbon + backstage + context menus + QAT.
    pub layout: Layout,
    /// The commands referenced by the layout.
    pub commands: Commands,
    /// The shortcuts dispatched on this layout.
    pub shortcuts: Shortcuts,
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            schema_version: 1,
            product: "visio".to_owned(),
            source: String::new(),
            source_url: String::new(),
            source_revision: String::new(),
            extracted: String::new(),
            tabs: Vec::new(),
            backstage: Backstage { tabs: Vec::new() },
            context_menus: Vec::new(),
            quick_access_toolbar: QuickAccessToolbar {
                controls: Vec::new(),
            },
        }
    }
}

/// A lookup table built from a [`LayoutSet`]. Maps every command id
/// to the surfaces on which it appears. P1-06 builds this once at
/// load time; the rest of Ferrogram (ribbon, menu, shortcut dispatch)
/// queries it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CommandLookup {
    /// `command id -> label`.
    pub labels: BTreeMap<CommandId, String>,
    /// `command id -> one of the control paths (tab/group/control)`
    /// for menus and tooltips. P1-06 picks the ribbon path when there
    /// are several.
    pub primary_path: BTreeMap<CommandId, String>,
}

impl CommandLookup {
    /// An empty lookup. P1-06 will provide a real constructor.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Number of commands in the lookup.
    pub fn len(&self) -> usize {
        self.labels.len()
    }

    /// True if there are no commands.
    pub fn is_empty(&self) -> bool {
        self.labels.is_empty()
    }
}

/// Layout variant. Phase 1 ships two: 2016 and 365.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LayoutKind {
    /// Office 2016 Fluent control data.
    Visio2016,
    /// Microsoft 365 Current Channel.
    Visio365,
}

impl LayoutKind {
    /// The file name under `ui-spec/`.
    pub fn file_name(self) -> &'static str {
        match self {
            Self::Visio2016 => "visio-2016.yaml",
            Self::Visio365 => "visio-365.yaml",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_type_round_trip() {
        for s in [
            "tab",
            "tabSet",
            "group",
            "button",
            "toggleButton",
            "splitButton",
            "gallery",
            "menu",
            "checkBox",
            "dropDown",
            "comboBox",
            "labelControl",
            "control",
            "buttonDialogBoxLauncher",
            "contextMenu",
            "task",
            "taskFormGroup",
            "category",
            "none",
        ] {
            let t = ControlType::parse(s).unwrap_or_else(|| panic!("missing {s}"));
            assert_eq!(t.as_str(), s, "round trip failed for {s}");
        }
    }

    #[test]
    fn control_type_rejects_unknown() {
        assert!(ControlType::parse("notAType").is_none());
        assert!(ControlType::parse("Button").is_none()); // case-sensitive
    }

    #[test]
    fn mapping_confidence_round_trip() {
        for s in ["exact", "inferred", "unknown"] {
            let c = MappingConfidence::parse(s).unwrap();
            assert_eq!(c.as_str(), s);
        }
        assert!(MappingConfidence::parse("maybe").is_none());
    }

    #[test]
    fn shortcut_scope_round_trip() {
        for s in ["global", "ribbon", "page", "text", "selection", "window"] {
            assert!(ShortcutScope::parse(s).is_some());
        }
        assert!(ShortcutScope::parse("everywhere").is_none());
    }

    #[test]
    fn layout_kind_file_names() {
        assert_eq!(LayoutKind::Visio2016.file_name(), "visio-2016.yaml");
        assert_eq!(LayoutKind::Visio365.file_name(), "visio-365.yaml");
    }

    #[test]
    fn command_id_display() {
        let id = CommandId::new("home.align.left");
        assert_eq!(id.to_string(), "home.align.left");
        assert_eq!(id.as_str(), "home.align.left");
    }

    #[test]
    fn empty_lookup_is_empty() {
        let l = CommandLookup::empty();
        assert_eq!(l.len(), 0);
        assert!(l.is_empty());
    }

    #[test]
    fn default_layout_has_empty_surfaces() {
        let l = Layout::default();
        assert_eq!(l.schema_version, 1);
        assert_eq!(l.product, "visio");
        assert!(l.tabs.is_empty());
        assert!(l.backstage.tabs.is_empty());
        assert!(l.context_menus.is_empty());
        assert!(l.quick_access_toolbar.controls.is_empty());
    }

    #[test]
    fn default_layout_set_is_empty() {
        let s = LayoutSet::default();
        assert!(s.layout.tabs.is_empty());
        assert!(s.commands.entries.is_empty());
        assert!(s.shortcuts.entries.is_empty());
    }

    #[test]
    fn bounds_default_is_zero() {
        let b = Bounds::default();
        assert_eq!(b.x, 0.0);
        assert_eq!(b.y, 0.0);
        assert_eq!(b.w, 0.0);
        assert_eq!(b.h, 0.0);
    }

    #[test]
    fn size_variant_round_trip() {
        for s in ["small", "medium", "large", "extra-large"] {
            let v = SizeVariant::parse(s).unwrap();
            let back = match v {
                SizeVariant::Small => "small",
                SizeVariant::Medium => "medium",
                SizeVariant::Large => "large",
                SizeVariant::ExtraLarge => "extra-large",
            };
            assert_eq!(back, s);
        }
        assert!(SizeVariant::parse("huge").is_none());
    }

    #[test]
    fn command_id_ord_and_eq() {
        let a = CommandId::new("file.save");
        let b = CommandId::new("file.save");
        let c = CommandId::new("file.save_as");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert!(a < c);
    }

    #[test]
    fn fluent_id_display_and_ref() {
        let f = FluentId::new("FileSave");
        assert_eq!(f.to_string(), "FileSave");
        assert_eq!(f.as_str(), "FileSave");
    }

    #[test]
    fn layout_kind_eq_and_hash() {
        use std::collections::HashSet;
        let mut s = HashSet::new();
        s.insert(LayoutKind::Visio2016);
        s.insert(LayoutKind::Visio365);
        s.insert(LayoutKind::Visio2016);
        assert_eq!(s.len(), 2);
        assert_ne!(LayoutKind::Visio2016, LayoutKind::Visio365);
    }
}
