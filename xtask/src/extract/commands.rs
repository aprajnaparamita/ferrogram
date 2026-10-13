//! Command mapping merge (P1-04).
//!
//! Builds `ui-spec/commands.yaml` — the Ferrogram command registry's
//! mapping table (ADR 0006) — by joining:
//!
//! 1. the `VisUICmds` enumeration ([`super::visuicmds`], source 2), and
//! 2. the Fluent control names from the P1-03 layouts
//!    `ui-spec/visio-2016.yaml` / `ui-spec/visio-365.yaml` (source 1).
//!
//! It also fills the `command` field of every layout control that
//! surfaces a mapped command and rewrites both layout files (the
//! "Filling control commands" step of `docs/tasks/P1-04.md`).
//!
//! ## Id derivation
//!
//! For every `visCmd*` constant the Ferrogram command id is derived by
//! splitting the constant (minus the `visCmd` prefix) on CamelCase
//! boundaries and joining the words with dots
//! (`visCmdZoomIn` -> `zoom.in`). The first word is mapped to one of
//! the seventeen functional scopes of ADR 0006 by the hand-written
//! [`scope_for_first_word`] table (e.g. `align` -> `arrange`), and the
//! scope becomes the first id segment
//! (`visCmdZoomIn` -> `view.zoom.in`). When the
//! first word already is a scope it is used as-is
//! (`visCmdFileSave` -> `file.save`). A hand-written [`OVERRIDES`]
//! table replaces the whole id where the mechanical split gives the
//! wrong scope or shape (`visCmdABarAutoHeight` -> `app.bar.auto_height`
//! rather than `a.bar.auto.height`).
//!
//! ## Joining to Fluent names
//!
//! A layout control id joins a VisUICmds constant when either side
//! names the other:
//!
//! - **exact**: the control id equals the constant minus `visCmd`
//!   (`FileSave` / `visCmdFileSave`), or the two derive the same
//!   command id (`Paste` / `visCmdUFEditPaste` -> `edit.paste`).
//! - **inferred**: the control's word set and the constant's word set
//!   match by subset after control-type words are stripped
//!   (`AlignLeftButton` / `visCmdAlignObjectLeft`).
//!
//! Every Fluent name maps to at most one command; a name mapped twice
//! is a hard error. Actionable controls with no VisUICmds constant get
//! their own entry with `visuicmd: null` (ADR 0006's `insert.shape`
//! example). Labels come from [`LABELS`], else the matched Fluent
//! control name (humanised), else the last id segment capitalised.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};
use serde_saphyr::SerializerOptions;

use super::schema::Schema;
use super::util::write_atomic;
use super::visuicmds::VisCmd;
use super::{ExtractError, ExtractResult};

/// The seventeen functional scopes of ADR 0006. Adding one is an ADR
/// change; [`command_id_for_constant`] never invents a scope.
pub const SCOPES: [&str; 17] = [
    "app",
    "file",
    "edit",
    "view",
    "insert",
    "format",
    "text",
    "arrange",
    "shape",
    "connector",
    "page",
    "layer",
    "tools",
    "data",
    "review",
    "developer",
    "help",
];

/// Whole-id overrides for constants whose CamelCase split gives the
/// wrong scope or the wrong word joins. Hand-written; keyed by the
/// `visCmd*` constant. This is the only way (besides the first-word
/// scope table) for a constant to land in a different scope than the
/// mechanical split would give.
pub const OVERRIDES: &[(&str, &str)] = &[
    // Align: ADR 0006 canonical ids (no `object` segment).
    ("visCmdAlignObjectBottom", "arrange.align.bottom"),
    ("visCmdAlignObjectCenter", "arrange.align.center"),
    ("visCmdAlignObjectLeft", "arrange.align.left"),
    ("visCmdAlignObjectMiddle", "arrange.align.middle"),
    ("visCmdAlignObjectRight", "arrange.align.right"),
    ("visCmdAlignObjectTop", "arrange.align.top"),
    // Application chrome (the task's own example family).
    ("visCmdABarAutoHeight", "app.bar.auto_height"),
    ("visCmdABarAutohide", "app.bar.auto_hide"),
    ("visCmdABarHide", "app.bar.hide"),
    ("visCmdABarToggleFloat", "app.bar.toggle_float"),
    // The classic Edit menu (the constants are named `UFEdit*`).
    ("visCmdUFEditClear", "edit.clear"),
    ("visCmdUFEditCopy", "edit.copy"),
    ("visCmdUFEditCut", "edit.cut"),
    ("visCmdUFEditDuplicate", "edit.duplicate"),
    ("visCmdUFEditPaste", "edit.paste"),
    ("visCmdUFEditSelectAll", "edit.select_all"),
    ("visCmdEditUndo", "edit.undo"),
    ("visCmdEditRedo", "edit.redo"),
    ("visCmdDeleteBackWord", "edit.delete_back_word"),
    ("visCmdDeleteForwardWord", "edit.delete_forward_word"),
    ("visCmdMoveObject", "arrange.nudge"),
    // File menu.
    ("visCmdFileNew", "file.new"),
    ("visCmdFileOpen", "file.open"),
    ("visCmdFileClose", "file.close"),
    ("visCmdFileSave", "file.save"),
    ("visCmdFileSaveAs", "file.save_as"),
    ("visCmdFilePrint", "file.print"),
    ("visCmdPrintPreview", "file.print_preview"),
    ("visCmdCloseWindow", "file.close_window"),
    // Text formatting (ADR 0006: `format.text.bold`).
    ("visCmdTextBold", "format.text.bold"),
    ("visCmdTextItalic", "format.text.italic"),
    ("visCmdTextAllCaps", "format.text.all_caps"),
    ("visCmdTextSmallCaps", "format.text.small_caps"),
    ("visCmdTextSubscript", "format.text.subscript"),
    ("visCmdSetCharSizeUp", "format.text.increase_size"),
    ("visCmdSetCharSizeDown", "format.text.decrease_size"),
    ("visCmdTextHAlignLeft", "format.text.align_left"),
    ("visCmdTextHAlignCenter", "format.text.align_center"),
    ("visCmdTextHAlignRight", "format.text.align_right"),
    ("visCmdTextHAlignJustify", "format.text.align_justify"),
    ("visCmdTextVAlignTop", "format.text.align_top"),
    ("visCmdTextVAlignMiddle", "format.text.align_middle"),
    ("visCmdTextVAlignBottom", "format.text.align_bottom"),
    // Arrange (ADR 0006: `arrange.group`, `arrange.bring_forward`, ...).
    ("visCmdObjectGroup", "arrange.group"),
    ("visCmdObjectUngroup", "arrange.ungroup"),
    ("visCmdObjectBringForward", "arrange.bring_forward"),
    ("visCmdObjectBringToFront", "arrange.bring_to_front"),
    ("visCmdObjectSendBackward", "arrange.send_backward"),
    ("visCmdObjectSendToBack", "arrange.send_to_back"),
    ("visCmdObjectFlipHorizontal", "arrange.flip_horizontal"),
    ("visCmdObjectFlipVertical", "arrange.flip_vertical"),
    ("visCmdObjectAlignObjects", "arrange.align_objects"),
    ("visCmdDiagramRotateLeft", "arrange.rotate_left"),
    ("visCmdDiagramRotateRight", "arrange.rotate_right"),
    // Drawing tools (ADR 0006: `shape.rectangle`, `tools.pointer`, ...).
    ("visCmdDRPointerTool", "tools.pointer"),
    ("visCmdDRTextTool", "tools.text"),
    ("visCmdTextBlockTool", "tools.text_block"),
    ("visCmdDRConnectorTool", "tools.connector"),
    ("visCmdDRConnectionTool", "tools.connection"),
    ("visCmdDRPencilTool", "tools.pencil"),
    ("visCmdDRRotateTool", "tools.rotate"),
    ("visCmdDRRectTool", "shape.rectangle"),
    ("visCmdDROvalTool", "shape.ellipse"),
    ("visCmdDRLineTool", "shape.line"),
    ("visCmdDRQtrArcTool", "shape.arc"),
    ("visCmdDRSplineTool", "shape.spline"),
    ("visCmdCropTool", "tools.crop"),
    ("visCmdFormatPainter", "tools.format_painter"),
    ("visCmdOptionsSnapGlueSetup", "tools.snap_glue_setup"),
    // Connectors (ADR 0006: `connector.straight`, ...).
    ("visCmdConnectorEffectStraight", "connector.straight"),
    ("visCmdConnectorEffectRightAngle", "connector.right_angle"),
    ("visCmdConnectorEffectCurved", "connector.curved"),
    ("visCmdAutoConnectToggle", "connector.auto_connect"),
    // View (ADR 0006: `view.zoom.in`, `view.toggle_grid`, ...).
    ("visCmdFullScreenMode", "view.full_screen_mode"),
    ("visCmdViewGrid", "view.toggle_grid"),
    ("visCmdViewGuides", "view.toggle_guides"),
    ("visCmdWindowTileAll", "view.tile_all"),
    ("visCmdWindowCascadeAll", "view.cascade_all"),
    // Page navigation (ADR 0006: `page.next`, ...).
    ("visCmdReOrderPage", "page.reorder"),
    ("visCmdNewForegroundPage", "page.add"),
    ("visCmdOptionsDeletePages", "page.delete"),
];

/// Hand-written labels for the top commands ("the top 100
/// VisUICmds"). Keyed by the Ferrogram command id so a curated
/// Fluent-only entry (e.g. `insert.shape`) can carry a label too.
/// [`LABEL_TABLE_CAP`] documents the intended bound.
pub const LABELS: &[(&str, &str)] = &[
    ("app.quit", "Exit"),
    ("arrange.align_objects", "Align Shapes"),
    ("arrange.bring_forward", "Bring Forward"),
    ("arrange.bring_to_front", "Bring to Front"),
    ("arrange.flip_horizontal", "Flip Horizontal"),
    ("arrange.flip_vertical", "Flip Vertical"),
    ("arrange.group", "Group"),
    ("arrange.nudge", "Move Shape"),
    ("arrange.rotate_left", "Rotate Left"),
    ("arrange.rotate_right", "Rotate Right"),
    ("arrange.send_backward", "Send Backward"),
    ("arrange.send_to_back", "Send to Back"),
    ("arrange.ungroup", "Ungroup"),
    ("connector.auto_connect", "AutoConnect"),
    ("connector.curved", "Curved Connector"),
    ("connector.right_angle", "Right-Angle Connector"),
    ("connector.straight", "Straight Connector"),
    ("data.link", "Link Data"),
    ("data.refresh", "Refresh Data"),
    ("developer.macro", "Macro"),
    ("edit.clear", "Delete"),
    ("edit.copy", "Copy"),
    ("edit.cut", "Cut"),
    ("edit.delete_back_word", "Delete Word Back"),
    ("edit.duplicate", "Duplicate"),
    ("edit.paste", "Paste"),
    ("edit.redo", "Redo"),
    ("edit.select_all", "Select All"),
    ("edit.undo", "Undo"),
    ("file.close", "Close"),
    ("file.new", "New"),
    ("file.open", "Open"),
    ("file.print", "Print"),
    ("file.print_preview", "Print Preview"),
    ("file.save", "Save"),
    ("file.save_as", "Save As"),
    ("format.text.align_bottom", "Bottom-Align Text"),
    ("format.text.align_center", "Center Text"),
    ("format.text.align_justify", "Justify Text"),
    ("format.text.align_left", "Align Text Left"),
    ("format.text.align_middle", "Center Text Vertically"),
    ("format.text.align_right", "Align Text Right"),
    ("format.text.align_top", "Top-Align Text"),
    ("format.text.bold", "Bold"),
    ("format.text.decrease_size", "Decrease Font Size"),
    ("format.text.increase_size", "Increase Font Size"),
    ("format.text.italic", "Italic"),
    ("help.about", "About"),
    ("insert.picture", "Picture"),
    ("insert.shape", "Insert Shape"),
    ("insert.text", "Text"),
    ("layer.add", "New Layer"),
    ("layer.hide", "Hide Layer"),
    ("layer.show", "Show Layer"),
    ("page.add", "New Page"),
    ("page.delete", "Delete Page"),
    ("page.next", "Next Page"),
    ("page.previous", "Previous Page"),
    ("page.rename", "Rename Page"),
    ("shape.arc", "Arc"),
    ("shape.ellipse", "Ellipse"),
    ("shape.line", "Line"),
    ("shape.rectangle", "Rectangle"),
    ("shape.spline", "Freeform"),
    ("tools.connector", "Connector Tool"),
    ("tools.connection", "Connection Point Tool"),
    ("tools.crop", "Crop Tool"),
    ("tools.format_painter", "Format Painter"),
    ("tools.pencil", "Pencil Tool"),
    ("tools.pointer", "Pointer Tool"),
    ("tools.rotate", "Rotate Tool"),
    ("tools.snap_glue_setup", "Snap & Glue"),
    ("tools.text", "Text Tool"),
    ("tools.text_block", "Text Block Tool"),
    ("view.cascade_all", "Cascade All"),
    ("view.full_screen_mode", "Full Screen"),
    ("view.tile_all", "Tile All"),
    ("view.zoom.in", "Zoom In"),
    ("view.zoom.out", "Zoom Out"),
];

/// Intended bound for [`LABELS`] ("the top 100 VisUICmds"). Asserted
/// by the unit tests; not read by the extractor itself.
#[allow(dead_code)]
pub const LABEL_TABLE_CAP: usize = 100;

/// Fluent-name overrides: whole-id replacements for layout control
/// names whose mechanical derivation would land somewhere else than
/// their VisUICmds twin, or would break the id grammar. Keyed by the
/// Fluent control name.
pub const FLUENT_OVERRIDES: &[(&str, &str)] = &[
    // No `visCmd*Underline*` exists; the control maps onto the
    // `format.text.*` family so it keeps its siblings company.
    ("UnderlineButton", "format.text.underline"),
    ("Underline", "format.text.underline"),
    ("DoubleUnderline", "format.text.double_underline"),
    // `_3D*` controls split into digit-leading words (`3`, `d`).
    ("_3DRotationGallery", "tools.rotation_3d"),
    ("_3DRotationOptionsDialog", "tools.rotation_3d_options"),
    ("_3DBevelOptionsDialog", "tools.bevel_options_3d"),
];

/// Control-type words stripped from a Fluent name before word-set
/// matching and before humanising a label. A trailing type word says
/// how the control is rendered, not what it does
/// (`AlignLeftButton` -> `AlignLeft`).
const CONTROL_TYPE_WORDS: &[&str] = &[
    "bar", "box", "button", "combo", "control", "down", "drop", "dropdown", "editor", "gallery",
    "group", "item", "label", "launcher", "list", "menu", "pane", "split", "tab", "task", "toggle",
    "tool",
];

/// Trailing words dropped from a Fluent name when they repeat the
/// whole control type (`ButtonGroup` is a group, not a button group
/// action). One pass over [`CONTROL_TYPE_WORDS`] is enough for the
/// observed data; the loop is capped for determinism.
const TYPE_STRIP_PASSES: usize = 2;

/// Confidence of one command mapping (ADR 0006 / schema
/// `mappingConfidence`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confidence {
    /// The Fluent control name names the same action as the constant.
    Exact,
    /// The mapping came out of name matching only.
    Inferred,
    /// No Fluent mapping is known yet (or no VisUICmds constant).
    Unknown,
}

impl Confidence {
    /// The schema string.
    fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Inferred => "inferred",
            Self::Unknown => "unknown",
        }
    }

    /// Sort order of the three groups in `commands.yaml`.
    fn order(self) -> u8 {
        match self {
            Self::Exact => 0,
            Self::Inferred => 1,
            Self::Unknown => 2,
        }
    }
}

/// One row of `commands.yaml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandEntry {
    /// The Ferrogram command id.
    pub id: String,
    /// Short UI label.
    pub label: String,
    /// The `visCmd*` constant, when known.
    pub visuicmd: Option<String>,
    /// Fluent control names that surface this command (sorted).
    pub fluent_ids: Vec<String>,
    /// Mapping confidence.
    pub confidence: Confidence,
}

/// What we know about one layout control (union over both layouts).
#[derive(Debug, Clone)]
pub struct ControlMeta {
    /// Control type from the schema vocabulary (e.g. `button`).
    control_type: String,
    /// Layout `label` (P1-03 stores the Fluent control name there).
    label: String,
}

/// Per-extractor entry point: builds `ui-spec/commands.yaml` and
/// fills the `command` field of both layout files.
pub fn run(in_dir: &Path, out_dir: &Path) -> ExtractResult<Vec<PathBuf>> {
    let schema = Schema::load_default()?;
    let vis_cmds = super::visuicmds::run(in_dir)?;
    let ui_dir = out_dir.join("ui-spec");
    let path_2016 = ui_dir.join("visio-2016.yaml");
    let path_365 = ui_dir.join("visio-365.yaml");
    let path_commands = ui_dir.join("commands.yaml");

    let mut layout_2016 = load_layout(&path_2016)?;
    let mut layout_365 = load_layout(&path_365)?;

    let mut controls: BTreeMap<String, ControlMeta> = BTreeMap::new();
    collect_controls(&layout_2016, &mut controls);
    // 365 (Current Channel) is the canonical surface; it wins when a
    // control differs between the two layouts.
    collect_controls(&layout_365, &mut controls);

    let entries = build_entries(&vis_cmds, &controls)?;

    // Hard cross-check (B1): every fluent_id must be a real Fluent
    // control name in at least one layout.
    let known: BTreeSet<String> = controls.keys().cloned().collect();
    cross_check_fluent_ids(&entries, &known)?;

    let text = render_commands_yaml(&entries)?;
    write_atomic(&path_commands, &text).map_err(|msg| ExtractError::Io {
        path: path_commands.clone(),
        source: msg,
    })?;
    schema.validate_file(&path_commands)?;

    // Fill `command` on the layout controls and rewrite the files.
    let map = fluent_command_map(&entries)?;
    fill_layout_commands(&mut layout_2016, &map)?;
    fill_layout_commands(&mut layout_365, &map)?;
    let options = yaml_options();
    for (path, value) in [(&path_2016, &layout_2016), (&path_365, &layout_365)] {
        let text = serde_saphyr::to_string_with_options(value, options.clone()).map_err(|e| {
            ExtractError::Parse {
                path: path.clone(),
                message: format!("YAML serialize: {e}"),
            }
        })?;
        write_atomic(path, &text).map_err(|msg| ExtractError::Io {
            path: path.clone(),
            source: msg,
        })?;
        schema.validate_file(path)?;
    }

    Ok(vec![path_commands, path_2016, path_365])
}

/// The YAML emitter options P1-03 uses (`xtask/src/extract/fluent.rs`),
/// so the rewritten layout files keep the committed style.
fn yaml_options() -> SerializerOptions {
    let mut options = SerializerOptions::default();
    options.compact_list_indent = false;
    options.prefer_block_scalars = false;
    options
}

/// Reads a layout YAML file into a mutable `serde_json::Value` tree.
fn load_layout(path: &Path) -> ExtractResult<Value> {
    let text = std::fs::read_to_string(path).map_err(|e| ExtractError::Io {
        path: path.to_path_buf(),
        source: e.to_string(),
    })?;
    serde_saphyr::from_str::<Value>(&text).map_err(|e| ExtractError::Parse {
        path: path.to_path_buf(),
        message: format!("YAML parse: {e}"),
    })
}

/// Records every control (and group/tab) id of a layout tree. Only
/// typed nodes are controls; the `quick_access_toolbar` container has
/// an id but no type and is not a command surface.
fn collect_controls(value: &Value, out: &mut BTreeMap<String, ControlMeta>) {
    visit_objects(value, &mut |obj| {
        if let Some(id) = obj.get("id").and_then(Value::as_str)
            && let Some(ty) = obj.get("type").and_then(Value::as_str)
        {
            out.entry(id.to_string()).or_insert_with(|| ControlMeta {
                control_type: ty.to_string(),
                label: obj
                    .get("label")
                    .and_then(Value::as_str)
                    .unwrap_or(id)
                    .to_string(),
            });
        }
    });
}

/// Applies `map` (Fluent id or alias -> command id) to every object in
/// the tree that carries an `id` and a `command` field. A control
/// whose id and alias map to different commands is a hard error.
fn fill_layout_commands(value: &mut Value, map: &BTreeMap<String, String>) -> ExtractResult<()> {
    match value {
        Value::Array(items) => {
            for item in items {
                fill_layout_commands(item, map)?;
            }
            Ok(())
        }
        Value::Object(obj) => {
            let is_control = obj.contains_key("command") && obj.contains_key("id");
            if is_control {
                let id = obj
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                let mut matched: Option<&String> = map.get(&id);
                if let Some(aliases) = obj.get("aliases").and_then(Value::as_array) {
                    for a in aliases.iter().filter_map(Value::as_str) {
                        if let Some(cmd) = map.get(a) {
                            match matched {
                                None => matched = Some(cmd),
                                Some(existing) if existing == cmd => {}
                                Some(existing) => {
                                    return Err(ExtractError::Schema {
                                        path: PathBuf::from("<layout>"),
                                        message: format!(
                                            "control {id}: id maps to {existing:?} but alias {a} maps to {cmd:?}"
                                        ),
                                    });
                                }
                            }
                        }
                    }
                }
                if let Some(cmd) = matched {
                    obj.insert("command".into(), Value::String(cmd.clone()));
                }
            }
            for (_, v) in obj.iter_mut() {
                fill_layout_commands(v, map)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

/// Visits every object node in a JSON tree (depth-first).
fn visit_objects(value: &Value, f: &mut dyn FnMut(&Map<String, Value>)) {
    match value {
        Value::Array(items) => {
            for item in items {
                visit_objects(item, f);
            }
        }
        Value::Object(obj) => {
            f(obj);
            for (_, v) in obj {
                visit_objects(v, f);
            }
        }
        _ => {}
    }
}

/// Builds the Fluent id -> command id map used to fill the layouts.
/// A Fluent id mapped by two different commands is a hard error.
fn fluent_command_map(entries: &[CommandEntry]) -> ExtractResult<BTreeMap<String, String>> {
    let mut map: BTreeMap<String, String> = BTreeMap::new();
    for e in entries {
        for f in &e.fluent_ids {
            match map.get(f) {
                Some(prev) if prev != &e.id => {
                    return Err(ExtractError::Schema {
                        path: PathBuf::from("ui-spec/commands.yaml"),
                        message: format!(
                            "fluent id {f} mapped by both {prev} and {}; a Fluent id may map to only one command",
                            e.id
                        ),
                    });
                }
                _ => {
                    map.insert(f.clone(), e.id.clone());
                }
            }
        }
    }
    Ok(map)
}

/// Cross-checks every `fluent_ids` entry against the layout names.
/// A name in neither layout is [`ExtractError::Schema`] (the P1-04
/// task's "hard error").
pub fn cross_check_fluent_ids(
    entries: &[CommandEntry],
    known: &BTreeSet<String>,
) -> ExtractResult<()> {
    for e in entries {
        for f in &e.fluent_ids {
            if !known.contains(f) {
                return Err(ExtractError::Schema {
                    path: PathBuf::from("ui-spec/commands.yaml"),
                    message: format!(
                        "command {}: fluent id {f} appears in neither ui-spec/visio-2016.yaml nor ui-spec/visio-365.yaml",
                        e.id
                    ),
                });
            }
        }
    }
    Ok(())
}

/// Joins the VisUICmds constants with the layout control names.
pub fn build_entries(
    vis_cmds: &[VisCmd],
    controls: &BTreeMap<String, ControlMeta>,
) -> ExtractResult<Vec<CommandEntry>> {
    // Derive one id per constant; duplicates mean OVERRIDES or the
    // mechanical split collapsed two commands. Report every
    // collision in one error so the OVERRIDES table can be fixed in
    // one pass.
    let mut constant_ids: BTreeMap<String, String> = BTreeMap::new();
    let mut seen_ids: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for c in vis_cmds {
        let id = command_id_for_constant(&c.constant);
        seen_ids
            .entry(id.clone())
            .or_default()
            .push(c.constant.clone());
        constant_ids.insert(c.constant.clone(), id);
    }
    let collisions: Vec<String> = seen_ids
        .iter()
        .filter(|(_, cs)| cs.len() > 1)
        .map(|(id, cs)| format!("{id}: {}", cs.join(", ")))
        .collect();
    if !collisions.is_empty() {
        return Err(ExtractError::Parse {
            path: PathBuf::from("reference/learn/visuicmds/visuicmds.md"),
            message: format!(
                "constants share a command id (add OVERRIDES entries): {}",
                collisions.join("; ")
            ),
        });
    }

    // Pass 1: exact names (`FileSave` / `visCmdFileSave`).
    // Pass 2: equal derived ids (`Paste` / `visCmdUFEditPaste`).
    // Pass 3: word-subset (`AlignLeftButton` / `visCmdAlignObjectLeft`).
    // Each control id joins at most one constant.
    let mut joined: BTreeMap<String, Vec<(String, bool)>> = BTreeMap::new(); // constant -> (control, exact)
    let mut claimed: BTreeSet<String> = BTreeSet::new();

    let actionable = |m: &ControlMeta| -> bool {
        matches!(
            m.control_type.as_str(),
            "button"
                | "toggleButton"
                | "splitButton"
                | "gallery"
                | "menu"
                | "checkBox"
                | "dropDown"
                | "comboBox"
                | "control"
        )
    };

    for (name, meta) in controls {
        if !actionable(meta) {
            continue;
        }
        if let Some(c) = vis_cmds.iter().find(|c| {
            c.constant
                .get("visCmd".len()..)
                .map(|s| s == name)
                .unwrap_or(false)
        }) {
            joined
                .entry(c.constant.clone())
                .or_default()
                .push((name.clone(), true));
            claimed.insert(name.clone());
        }
    }
    for (name, meta) in controls {
        if !actionable(meta) || claimed.contains(name) {
            continue;
        }
        let derived = fluent_command_id(name);
        if let Some(id) = seen_ids.get(&derived).and_then(|v| v.first()) {
            joined
                .entry(id.clone())
                .or_default()
                .push((name.clone(), true));
            claimed.insert(name.clone());
        }
    }
    for (name, meta) in controls {
        if !actionable(meta) || claimed.contains(name) {
            continue;
        }
        let f_words: BTreeSet<String> = fluent_words(name);
        if f_words.is_empty() {
            continue;
        }
        let mut best: Option<(String, usize, usize)> = None; // (constant, overlap, |C|)
        for c in vis_cmds {
            let c_words: BTreeSet<String> = camel_words(c.constant.trim_start_matches("visCmd"))
                .into_iter()
                .collect();
            if c_words.is_empty() {
                continue;
            }
            let subset = f_words.is_subset(&c_words) || c_words.is_subset(&f_words);
            if !subset {
                continue;
            }
            let overlap = f_words.intersection(&c_words).count();
            let candidate = (c.constant.clone(), overlap, c_words.len());
            // Best match: most shared words, then the smallest
            // constant word set, then the alphabetically first name.
            let better = match &best {
                None => true,
                Some(b) => {
                    (
                        candidate.1,
                        std::cmp::Reverse(candidate.2),
                        std::cmp::Reverse(&candidate.0),
                    ) > (b.1, std::cmp::Reverse(b.2), std::cmp::Reverse(&b.0))
                }
            };
            if better {
                best = Some(candidate);
            }
        }
        if let Some((constant, _, _)) = best {
            joined
                .entry(constant)
                .or_default()
                .push((name.clone(), false));
            claimed.insert(name.clone());
        }
    }

    // Build the constant-backed entries.
    let mut entries: Vec<CommandEntry> = Vec::new();
    for c in vis_cmds {
        let id = constant_ids
            .get(&c.constant)
            .ok_or_else(|| ExtractError::Parse {
                path: PathBuf::from("<in-memory>"),
                message: format!("no derived id for {}", c.constant),
            })?
            .clone();
        let matches = joined.get(&c.constant).cloned().unwrap_or_default();
        let confidence = if matches.iter().any(|(_, exact)| *exact) {
            Confidence::Exact
        } else if !matches.is_empty() {
            Confidence::Inferred
        } else {
            Confidence::Unknown
        };
        let mut fluent_ids: Vec<String> = matches.iter().map(|(n, _)| n.clone()).collect();
        fluent_ids.sort();
        let label = label_for(&id, &fluent_ids, controls);
        entries.push(CommandEntry {
            id,
            label,
            visuicmd: Some(c.constant.clone()),
            fluent_ids,
            confidence,
        });
    }

    // Fluent-only entries: actionable controls with no constant.
    let mut fluent_only: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (name, meta) in controls {
        if !actionable(meta) || claimed.contains(name) {
            continue;
        }
        fluent_only
            .entry(fluent_command_id(name))
            .or_default()
            .push(name.clone());
    }
    for (id, mut names) in fluent_only {
        if seen_ids.contains_key(&id) {
            // Unreachable via the passes above; keep the data honest.
            return Err(ExtractError::Schema {
                path: PathBuf::from("ui-spec/commands.yaml"),
                message: format!("fluent-only id {id} collides with a VisUICmds-derived id"),
            });
        }
        names.sort();
        let label = label_for(&id, &names, controls);
        entries.push(CommandEntry {
            id,
            label,
            visuicmd: None,
            fluent_ids: names,
            confidence: Confidence::Unknown,
        });
    }

    entries.sort_by(|a, b| {
        a.confidence
            .order()
            .cmp(&b.confidence.order())
            .then_with(|| a.id.cmp(&b.id))
    });
    Ok(entries)
}

/// Label resolution: [`LABELS`], else the matched Fluent control name
/// humanised (type word and leading scope word stripped), else the
/// command id's last segment capitalised.
fn label_for(id: &str, fluent_ids: &[String], controls: &BTreeMap<String, ControlMeta>) -> String {
    if let Some((_, label)) = LABELS.iter().find(|(k, _)| *k == id) {
        return (*label).to_string();
    }
    if let Some(first) = fluent_ids.first() {
        let scope = id.split('.').next().unwrap_or("tools");
        // The layout `label` is the Fluent `Control Name` (P1-03).
        let name = controls
            .get(first)
            .map(|m| m.label.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or(first.as_str());
        return humanize_fluent_name(name, scope);
    }
    let tail = id.rsplit('.').next().unwrap_or(id);
    tail.split('_')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// `FileSave` (scope `file`) -> `Save`; `AlignLeftButton` ->
/// `Align Left`; `InsertShapeGallery` -> `Insert Shape`.
fn humanize_fluent_name(name: &str, scope: &str) -> String {
    let mut words = camel_words(name);
    let last_is_type = words
        .last()
        .map(|w| CONTROL_TYPE_WORDS.contains(&w.as_str()))
        .unwrap_or(false);
    if words.len() > 1 && last_is_type {
        words.pop();
    }
    if words.len() > 1 && words.first().map(String::as_str) == Some(scope) {
        words.remove(0);
    }
    words
        .iter()
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Splits a CamelCase name into lowercase words. Uppercase runs are
/// kept together (`INETAdd` -> `inet`, `add`); a single trailing
/// capital starts a new word (`Move1D` -> `move1`, `d`).
pub fn camel_words(name: &str) -> Vec<String> {
    let chars: Vec<char> = name.chars().collect();
    let mut words: Vec<String> = Vec::new();
    let mut cur = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if c.is_ascii_digit() {
            cur.push(c);
            continue;
        }
        if !c.is_ascii_alphabetic() {
            if !cur.is_empty() {
                words.push(std::mem::take(&mut cur));
            }
            continue;
        }
        if !cur.is_empty()
            && let Some(&prev) = chars.get(i.wrapping_sub(1))
        {
            let prev_upper = prev.is_ascii_uppercase();
            let next_lower = chars
                .get(i + 1)
                .map(|n| n.is_ascii_lowercase())
                .unwrap_or(false);
            let boundary = if c.is_ascii_uppercase() {
                (!prev_upper && (prev.is_ascii_lowercase() || prev.is_ascii_digit()))
                    || (prev_upper && next_lower && cur.chars().all(|ch| ch.is_ascii_uppercase()))
            } else {
                false
            };
            if boundary {
                words.push(std::mem::take(&mut cur));
            }
        }
        cur.push(c);
    }
    if !cur.is_empty() {
        words.push(cur);
    }
    words.iter().map(|w| w.to_ascii_lowercase()).collect()
}

/// The words of a Fluent control name used for matching: the
/// CamelCase words with trailing control-type words stripped.
fn fluent_words(name: &str) -> BTreeSet<String> {
    let mut words = camel_words(name);
    for _ in 0..TYPE_STRIP_PASSES {
        if words.len() > 1
            && words
                .last()
                .map(|w| CONTROL_TYPE_WORDS.contains(&w.as_str()))
                .unwrap_or(false)
        {
            words.pop();
        } else {
            break;
        }
    }
    words.into_iter().collect()
}

/// Maps the first word of a constant to one of the seventeen ADR 0006
/// scopes. This table is the hand-written heart of the id scheme; the
/// fallback (`tools`) keeps unknown prefixes inside the closed scope
/// list rather than inventing one.
pub fn scope_for_first_word(word: &str) -> &'static str {
    match word {
        // Scopes that name themselves (returned as literals so the
        // result is `'static`).
        "app" => "app",
        "file" => "file",
        "edit" => "edit",
        "view" => "view",
        "insert" => "insert",
        "format" => "format",
        "text" => "text",
        "arrange" => "arrange",
        "shape" => "shape",
        "connector" => "connector",
        "page" => "page",

        "tools" => "tools",
        "data" => "data",
        "review" => "review",
        "developer" => "developer",
        "help" => "help",
        // Application / environment.
        "a" | "mdi" | "office" | "coauth" | "trust" | "minimize" => "app",
        // File operations.
        "save" | "open" | "close" | "new" | "print" | "publish" | "export" | "send" | "email"
        | "upgrade" | "create" | "browse" | "header" => "file",
        // Editing.
        "del" | "delete" | "cut" | "copy" | "paste" | "undo" | "redo" | "select" | "deselect"
        | "duplicate" | "remove" | "mod" | "mov" | "move" | "move1" | "move2" | "drag"
        | "replace" | "recalc" | "reinstate" | "cancel" | "break" | "right" | "sw" | "uf" => "edit",
        // Viewing and navigation.
        "view50" | "view75" | "view100" | "view150" | "view200" | "view400" | "zoom" | "grid"
        | "guides" | "ruler" | "rul" | "pan" | "fit" | "full" | "show" | "hide" | "toggle"
        | "collapse" | "expand" | "window" | "next" | "previous" | "last" | "first" | "task"
        | "center" | "go" | "ss" | "toolbars" | "list" => "view",
        // Inserting objects.
        "inser" | "add" | "drop" | "stamp" | "image" | "acquire" => "insert",
        // Formatting.
        "set" | "apply" | "style" | "increase" | "decrease" => "format",
        // Text.
        "bullets" | "font" => "text",
        // Arranging.
        "align" | "distribute" | "bring" | "rotate" | "rotate90" | "flip" | "ungroup" | "group"
        | "nudge" | "offset" | "space" | "order" | "reorder" | "relayout" | "layout"
        | "disband" | "position" | "object" | "size" | "size1" | "size2" | "smart" | "auto"
        | "diagram" | "combine" | "fragment" | "intersect" | "join" | "trim" | "subtract"
        | "convert" => "arrange",
        // Shapes, stencils, masters.
        "shapes" | "sten" | "master" | "container" | "draw" | "drawing" | "crop" | "icon"
        | "lock" | "associate" => "shape",
        // Connectors.
        "conn" | "dyn" | "dynamic" => "connector",
        // Pages.
        "pages" | "multiple" => "page",
        // Layers.

        // Data.
        "link" | "update" => "data",
        // Review / validation.
        "reviewer" | "validation" | "validate" | "check" | "ignore" | "spelling" => "review",
        // Help and research.
        "research" | "rhi" => "help",
        // Developer / macro plumbing.
        "record" | "run" | "prog" => "developer",
        // Everything else is tool-shaped: dialogs, options, ink,
        // language, animation, the ShapeSheet window spellings.
        _ => "tools",
    }
}

/// True when `word` is one of the seventeen scopes.
fn is_scope(word: &str) -> bool {
    SCOPES.contains(&word)
}

/// Joins words into a command id under the scope rules shared by the
/// constant and Fluent derivations. A word that starts with a digit
/// (`2pt` in `visCmdSetDynConnLineJumpStyle_2pt`) is merged into the
/// previous word with an underscore, so every id segment matches the
/// ADR 0006 grammar (`[a-z][a-z0-9_]*`).
fn join_id(words: &[String]) -> String {
    let mut merged: Vec<String> = Vec::with_capacity(words.len());
    for w in words {
        let digit_start = w
            .chars()
            .next()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false);
        if digit_start && let Some(prev) = merged.last_mut() {
            prev.push('_');
            prev.push_str(w);
        } else {
            merged.push(w.clone());
        }
    }
    let words = &merged;
    if words.is_empty() {
        return "tools.untitled".to_string();
    }
    // Defensive: a segment may still start with a digit (`3d` from a
    // `_3D*` name); the ADR 0006 grammar needs `[a-z]` there.
    let sanitized: Vec<String> = words
        .iter()
        .map(|w| {
            if w.chars()
                .next()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
            {
                format!("x{w}")
            } else {
                w.clone()
            }
        })
        .collect();
    let words = &sanitized;
    if words.len() == 1 && is_scope(&words[0]) {
        // A lone scope word (`Connector`) still needs a verb.
        return format!("tools.{}", words[0]);
    }
    let scope = scope_for_first_word(&words[0]);
    if is_scope(&words[0]) {
        words.join(".")
    } else {
        format!("{scope}.{}", words.join("."))
    }
}

/// Derives the Ferrogram command id of a `visCmd*` constant
/// ([`OVERRIDES`] first, then the CamelCase split).
pub fn command_id_for_constant(constant: &str) -> String {
    if let Some((_, id)) = OVERRIDES.iter().find(|(k, _)| *k == constant) {
        return (*id).to_string();
    }
    join_id(&camel_words(constant.trim_start_matches("visCmd")))
}

/// Derives the Ferrogram command id of a Fluent control name
/// ([`FLUENT_OVERRIDES`] first, then the CamelCase split with
/// control-type words stripped).
pub fn fluent_command_id(name: &str) -> String {
    if let Some((_, id)) = FLUENT_OVERRIDES.iter().find(|(k, _)| *k == name) {
        return (*id).to_string();
    }
    let mut words = camel_words(name);
    for _ in 0..TYPE_STRIP_PASSES {
        if words.len() > 1
            && words
                .last()
                .map(|w| CONTROL_TYPE_WORDS.contains(&w.as_str()))
                .unwrap_or(false)
        {
            words.pop();
        } else {
            break;
        }
    }
    join_id(&words)
}

/// Renders `ui-spec/commands.yaml` (header comment + YAML list).
fn render_commands_yaml(entries: &[CommandEntry]) -> ExtractResult<String> {
    let list: Vec<Value> = entries
        .iter()
        .map(|e| {
            let mut obj = Map::new();
            obj.insert(
                "fluent_ids".into(),
                Value::Array(
                    e.fluent_ids
                        .iter()
                        .map(|f| Value::String(f.clone()))
                        .collect(),
                ),
            );
            obj.insert("id".into(), Value::String(e.id.clone()));
            obj.insert("label".into(), Value::String(e.label.clone()));
            obj.insert(
                "mapping_confidence".into(),
                Value::String(e.confidence.as_str().into()),
            );
            match &e.visuicmd {
                Some(v) => {
                    obj.insert("visuicmd".into(), Value::String(v.clone()));
                }
                None => {
                    obj.insert("visuicmd".into(), Value::Null);
                }
            }
            Value::Object(obj)
        })
        .collect();
    let text =
        serde_saphyr::to_string_with_options(&Value::Array(list), yaml_options()).map_err(|e| {
            ExtractError::Parse {
                path: PathBuf::from("ui-spec/commands.yaml"),
                message: format!("YAML serialize: {e}"),
            }
        })?;
    let mut out = String::new();
    out.push_str("# Generated by `cargo xtask extract` (xtask/src/extract/commands.rs).\n");
    out.push_str("# Do not edit by hand: edit the extractor rules instead (ADR 0006).\n");
    out.push_str("# Ferrogram command id -> VisUICmds constant (docs/sources.md source 2)\n");
    out.push_str("# and Fluent control names (source 1).\n");
    out.push_str(&text);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::super::schema::SchemaKind;
    use super::*;

    fn meta(t: &str) -> ControlMeta {
        ControlMeta {
            control_type: t.into(),
            label: String::new(),
        }
    }

    fn controls(pairs: &[(&str, &str)]) -> BTreeMap<String, ControlMeta> {
        pairs
            .iter()
            .map(|(id, t)| (id.to_string(), meta(t)))
            .collect()
    }

    fn vis(pairs: &[(&str, i64)]) -> Vec<VisCmd> {
        pairs
            .iter()
            .map(|(c, v)| VisCmd {
                constant: (*c).to_string(),
                value: *v,
            })
            .collect()
    }

    #[test]
    fn camel_words_splits_acronyms_and_digits() {
        assert_eq!(camel_words("FileSave"), vec!["file", "save"]);
        assert_eq!(
            camel_words("ABarAutoHeight"),
            vec!["a", "bar", "auto", "height"]
        );
        assert_eq!(
            camel_words("INETAddToFavorites"),
            vec!["inet", "add", "to", "favorites"]
        );
        assert_eq!(
            camel_words("MSOInsertSymbolDlg"),
            vec!["mso", "insert", "symbol", "dlg"]
        );
        assert_eq!(camel_words("RHIDlg"), vec!["rhi", "dlg"]);
        assert_eq!(camel_words("Move1D"), vec!["move1", "d"]);
        assert_eq!(
            camel_words("Rotate90Clockwise"),
            vec!["rotate90", "clockwise"]
        );
        assert_eq!(camel_words("ReOrderPage"), vec!["re", "order", "page"]);
        assert_eq!(camel_words("UFEditCut"), vec!["uf", "edit", "cut"]);
        assert_eq!(camel_words(""), Vec::<String>::new());
        assert_eq!(camel_words("Cut"), vec!["cut"]);
    }

    #[test]
    fn ids_use_the_functional_scope_list() {
        // The task's own examples.
        assert_eq!(
            command_id_for_constant("visCmdABarAutoHeight"),
            "app.bar.auto_height"
        );
        assert_eq!(
            command_id_for_constant("visCmdAlignObjectLeft"),
            "arrange.align.left"
        );
        assert_eq!(command_id_for_constant("visCmdFileSave"), "file.save");
        assert_eq!(command_id_for_constant("visCmdZoomIn"), "view.zoom.in");
        // Overrides.
        assert_eq!(command_id_for_constant("visCmdUFEditCut"), "edit.cut");
        assert_eq!(
            command_id_for_constant("visCmdTextBold"),
            "format.text.bold"
        );
        assert_eq!(
            command_id_for_constant("visCmdDRPointerTool"),
            "tools.pointer"
        );
        // Everything stays inside the closed scope list.
        for c in [
            "visCmdAcquireImages",
            "visCmdStenActivate",
            "visCmdSWAccept",
            "visCmdINETGoBack",
            "visCmdValidationIssuesWindowToggle",
            "visCmdMsoMediaGallery",
        ] {
            let id = command_id_for_constant(c);
            let scope = id.split('.').next().unwrap_or("");
            assert!(SCOPES.contains(&scope), "{c} -> {id}");
            assert!(id.split('.').count() >= 2, "{c} -> {id} needs a verb");
        }
    }

    #[test]
    fn override_table_targets_are_valid_ids() {
        for (constant, id) in OVERRIDES {
            assert!(constant.starts_with("visCmd"), "{constant}");
            let scope = id.split('.').next().unwrap_or("");
            assert!(SCOPES.contains(&scope), "{constant} -> {id}");
            assert!(id.split('.').count() >= 2, "{constant} -> {id}");
            for seg in id.split('.') {
                assert!(!seg.is_empty(), "{constant} -> {id}");
                assert!(
                    seg.chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
                    "{constant} -> {id}"
                );
            }
        }
    }

    #[test]
    fn label_table_is_bounded_and_short() {
        assert!(LABELS.len() <= LABEL_TABLE_CAP, "{}", LABELS.len());
        for (id, label) in LABELS {
            assert!(!label.is_empty(), "{id}");
            assert!(label.len() <= 32, "{id}: {label}");
        }
    }

    #[test]
    fn label_resolution_order() {
        let mut ctrls = controls(&[("FileSave", "button"), ("AlignLeftButton", "button")]);
        ctrls.get_mut("FileSave").unwrap().label = "FileSave".into();
        ctrls.get_mut("AlignLeftButton").unwrap().label = "AlignLeftButton".into();
        // LABELS wins for curated ids.
        assert_eq!(label_for("file.save", &["FileSave".into()], &ctrls), "Save");
        // Humanised Fluent name otherwise.
        assert_eq!(
            label_for("arrange.align.left", &["AlignLeftButton".into()], &ctrls),
            "Align Left"
        );
        // Last id segment capitalised as the final fallback.
        assert_eq!(
            label_for("page.reorder_pages", &[], &ctrls),
            "Reorder Pages"
        );
    }

    #[test]
    fn humanize_strips_type_words_and_scope() {
        assert_eq!(humanize_fluent_name("FileSave", "file"), "Save");
        assert_eq!(
            humanize_fluent_name("AlignLeftButton", "arrange"),
            "Align Left"
        );
        assert_eq!(humanize_fluent_name("ConnectorTool", "tools"), "Connector");
        assert_eq!(
            humanize_fluent_name("InsertShapeGallery", "insert"),
            "Shape"
        );
    }

    #[test]
    fn fluent_derivation_joins_the_classics() {
        assert_eq!(fluent_command_id("FileSave"), "file.save");
        assert_eq!(fluent_command_id("Paste"), "edit.paste");
        assert_eq!(fluent_command_id("AlignLeftButton"), "arrange.align.left");
        assert_eq!(fluent_command_id("ConnectorTool"), "tools.connector");
        assert_eq!(
            fluent_command_id("UnderlineButton"),
            "format.text.underline"
        );
    }

    #[test]
    fn exact_and_inferred_joins() {
        let v = vis(&[
            ("visCmdFileSave", 1004),
            ("visCmdUFEditPaste", 1022),
            ("visCmdAlignObjectLeft", 1196),
            ("visCmdObjectFlipHorizontalShape", 1300),
        ]);
        let ctrls = controls(&[
            ("FileSave", "button"),
            ("Paste", "button"),
            ("AlignLeftButton", "gallery"),
            ("FlipShapeHorizontal", "button"),
            ("TabHome", "tab"),
            ("GroupClipboard", "group"),
        ]);
        let entries = build_entries(&v, &ctrls).unwrap();
        assert_eq!(entries.len(), 4);
        let save = entries.iter().find(|e| e.id == "file.save").unwrap();
        assert_eq!(save.fluent_ids, vec!["FileSave"]);
        assert_eq!(save.confidence, Confidence::Exact);
        let paste = entries.iter().find(|e| e.id == "edit.paste").unwrap();
        assert_eq!(paste.fluent_ids, vec!["Paste"]);
        assert_eq!(paste.confidence, Confidence::Exact);
        let align = entries
            .iter()
            .find(|e| e.id == "arrange.align.left")
            .unwrap();
        assert_eq!(align.fluent_ids, vec!["AlignLeftButton"]);
        // Pass 2: the OVERRIDES id `arrange.align.left` equals the id
        // derived from the Fluent name, so the join counts as exact.
        assert_eq!(align.confidence, Confidence::Exact);
        // Pass 3: word subset of a constant without an override.
        let flip = entries
            .iter()
            .find(|e| e.visuicmd.as_deref() == Some("visCmdObjectFlipHorizontalShape"))
            .unwrap();
        assert_eq!(flip.fluent_ids, vec!["FlipShapeHorizontal"]);
        assert_eq!(flip.confidence, Confidence::Inferred);
        // Tabs and groups never become commands.
        assert!(entries.iter().all(|e| {
            !e.fluent_ids
                .iter()
                .any(|f| f.starts_with("Tab") || f.starts_with("Group"))
        }));
    }

    #[test]
    fn fluent_only_entry_gets_unknown_confidence() {
        let v = vis(&[("visCmdFileSave", 1004)]);
        let ctrls = controls(&[("FileSave", "button"), ("InsertShapeGallery", "gallery")]);
        let entries = build_entries(&v, &ctrls).unwrap();
        assert_eq!(entries.len(), 2);
        let insert = entries.iter().find(|e| e.id == "insert.shape").unwrap();
        assert_eq!(insert.visuicmd, None);
        assert_eq!(insert.fluent_ids, vec!["InsertShapeGallery"]);
        assert_eq!(insert.confidence, Confidence::Unknown);
        assert_eq!(insert.label, "Insert Shape");
        // Grouping order: exact first, then inferred, then unknown.
        assert_eq!(entries[0].id, "file.save");
        assert_eq!(entries[1].id, "insert.shape");
    }

    #[test]
    fn duplicate_derived_ids_are_an_error() {
        // Two constants, same mechanical id (`mso.insert.symbol`).
        let v = vis(&[("visCmdMsoInsertSymbol", 1), ("visCmdMSOInsertSymbol", 2)]);
        let err = build_entries(&v, &BTreeMap::new()).unwrap_err();
        assert!(err.to_string().contains("share a command id"), "{err}");
        assert!(err.to_string().contains("visCmdMsoInsertSymbol"), "{err}");
    }

    #[test]
    fn cross_check_rejects_a_fake_fluent_id() {
        let entries = vec![CommandEntry {
            id: "file.save".into(),
            label: "Save".into(),
            visuicmd: Some("visCmdFileSave".into()),
            fluent_ids: vec!["FileSave".into(), "TotallyFakeId".into()],
            confidence: Confidence::Exact,
        }];
        let mut known = BTreeSet::new();
        known.insert("FileSave".to_string());
        let err = cross_check_fluent_ids(&entries, &known).unwrap_err();
        assert!(err.to_string().contains("TotallyFakeId"), "{err}");
        // And it passes with the fake id known.
        known.insert("TotallyFakeId".to_string());
        cross_check_fluent_ids(&entries, &known).unwrap();
    }

    #[test]
    fn one_fluent_id_two_commands_is_an_error() {
        let entries = vec![
            CommandEntry {
                id: "edit.cut".into(),
                label: "Cut".into(),
                visuicmd: None,
                fluent_ids: vec!["Cut".into()],
                confidence: Confidence::Unknown,
            },
            CommandEntry {
                id: "edit.cut2".into(),
                label: "Cut2".into(),
                visuicmd: None,
                fluent_ids: vec!["Cut".into()],
                confidence: Confidence::Unknown,
            },
        ];
        let err = fluent_command_map(&entries).unwrap_err();
        assert!(err.to_string().contains("Cut"), "{err}");
    }

    #[test]
    fn fill_layout_sets_command_on_controls() {
        let yaml = r#"
tabs:
  - id: TabHome
    groups:
      - id: GroupClipboard
        controls:
          - id: Cut
            type: button
            command: null
            aliases: []
          - id: Other
            type: button
            command: null
            aliases: []
backstage:
  kind: backstage
  tabs: []
context_menus: []
quick_access_toolbar:
  id: qat
  controls: []
"#;
        let mut value: Value = serde_saphyr::from_str(yaml).unwrap();
        let mut map = BTreeMap::new();
        map.insert("Cut".to_string(), "edit.cut".to_string());
        fill_layout_commands(&mut value, &map).unwrap();
        let controls = value
            .pointer("/tabs/0/groups/0/controls")
            .and_then(Value::as_array)
            .unwrap();
        assert_eq!(
            controls[0].get("command").and_then(Value::as_str),
            Some("edit.cut")
        );
        assert!(controls[1].get("command").unwrap().is_null());
    }

    #[test]
    fn fill_layout_alias_conflict_is_an_error() {
        let yaml = r#"
controls:
  - id: Cut
    type: button
    command: null
    aliases: [GroupIconEditClipboard]
"#;
        let mut value: Value = serde_saphyr::from_str(yaml).unwrap();
        let mut map = BTreeMap::new();
        map.insert("Cut".to_string(), "edit.cut".to_string());
        map.insert(
            "GroupIconEditClipboard".to_string(),
            "edit.copy".to_string(),
        );
        let err = fill_layout_commands(&mut value, &map).unwrap_err();
        assert!(err.to_string().contains("alias"), "{err}");
    }

    #[test]
    fn visuicmd_null_string_is_rejected_by_the_schema() {
        let path = crate::util::root().join("ui-spec").join("schema.json");
        if !path.is_file() {
            eprintln!("schema.json not present; skipping");
            return;
        }
        let schema = Schema::load(&path).expect("schema loads");
        let bad = r#"
- id: file.save
  label: Save
  visuicmd: "null"
  fluent_ids: [FileSave]
  mapping_confidence: exact
"#;
        let err = schema
            .validate_str(bad, SchemaKind::Commands)
            .expect_err("literal \"null\" must fail");
        assert!(err.to_string().contains("schema"), "{err}");

        let good = r#"
- id: file.save
  label: Save
  visuicmd: null
  fluent_ids: [FileSave]
  mapping_confidence: exact
"#;
        schema
            .validate_str(good, SchemaKind::Commands)
            .expect("null visuicmd is valid");

        let good_str = r#"
- id: file.save
  label: Save
  visuicmd: visCmdFileSave
  fluent_ids: [FileSave]
  mapping_confidence: exact
"#;
        schema
            .validate_str(good_str, SchemaKind::Commands)
            .expect("string visuicmd is valid");
    }

    /// The top 50 VisUICmds (first 50 in source order) land in the
    /// ADR 0006 functional scope list, never in a ribbon-tab scope.
    /// Skipped without `reference/`.
    #[test]
    fn top_50_viscmds_use_the_functional_scope_list() {
        let root = crate::util::root();
        let ref_dir = root.join("reference");
        if !ref_dir.join("learn").join("visuicmds").is_dir() {
            eprintln!("reference/ not present; skipping");
            return;
        }
        let cmds = crate::extract::visuicmds::run(&ref_dir).expect("VisUICmds parse");
        let mut scoped = 0;
        for c in cmds.iter().take(50) {
            let id = command_id_for_constant(&c.constant);
            let scope = id.split('.').next().unwrap_or("");
            assert!(
                SCOPES.contains(&scope),
                "{} -> {id} is not in the functional scope list",
                c.constant
            );
            assert!(
                !matches!(scope, "home" | "design" | "process" | "stencils"),
                "{} -> {id} uses a ribbon-tab scope",
                c.constant
            );
            assert!(id.split('.').count() >= 2, "{} -> {id}", c.constant);
            scoped += 1;
        }
        assert_eq!(scoped, 50.min(cmds.len()));
    }

    #[test]
    fn derived_ids_obey_the_command_grammar() {
        // Digit-led words (after a `_` separator) merge into the
        // previous word (`style_2pt`).
        let id = command_id_for_constant("visCmdSetStyle_2pt");
        assert_eq!(id, "format.set.style_2pt");
        // A digit-led first word is prefixed so every segment matches
        // `[a-z][a-z0-9_]*` (the `_3D*` Fluent names).
        assert_eq!(fluent_command_id("_3DRotationGallery"), "tools.rotation_3d");
        let raw = fluent_command_id("3DRotationGallery");
        assert!(
            raw.split('.').all(|seg| {
                seg.starts_with(|c: char| c.is_ascii_lowercase())
                    && seg
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
            }),
            "{raw}"
        );
    }

    #[test]
    fn rendered_yaml_parses_back_to_the_same_entries() {
        let entries = vec![
            CommandEntry {
                id: "file.save".into(),
                label: "Save".into(),
                visuicmd: Some("visCmdFileSave".into()),
                fluent_ids: vec!["FileSave".into()],
                confidence: Confidence::Exact,
            },
            CommandEntry {
                id: "insert.shape".into(),
                label: "Insert Shape".into(),
                visuicmd: None,
                fluent_ids: vec!["InsertShapeGallery".into()],
                confidence: Confidence::Unknown,
            },
        ];
        let text = render_commands_yaml(&entries).unwrap();
        assert!(text.starts_with("# Generated"));
        let body: Value = serde_saphyr::from_str(&text).unwrap();
        let arr = body.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0].get("id").unwrap(), "file.save");
        assert!(arr[0].get("visuicmd").unwrap().is_string());
        assert!(arr[1].get("visuicmd").unwrap().is_null());
    }

    // ---- P1-08 goal 4: property tests -------------------------------

    use proptest::prelude::*;

    /// The committed schema, compiled once per test run.
    fn schema() -> Schema {
        use std::sync::OnceLock;
        static SCHEMA: OnceLock<Schema> = OnceLock::new();
        SCHEMA
            .get_or_init(|| Schema::load_default().unwrap())
            .clone()
    }

    /// A minimal schema-valid `commands.yaml` entry carrying `id`.
    fn entry_with_id(id: &str) -> Value {
        serde_json::json!([{
            "fluent_ids": [],
            "id": id,
            "label": "Label",
            "mapping_confidence": "unknown",
            "visuicmd": null
        }])
    }

    proptest! {
        /// The id derivation for any `visCmd[A-Za-z0-9]+` constant
        /// yields a schema-valid command id; it never yields an
        /// invalid one (the other entry fields are constants, so a
        /// validation failure can only come from the id).
        #[test]
        fn derived_ids_are_schema_valid(constant in "visCmd[A-Za-z0-9]{0,24}") {
            let id = command_id_for_constant(&constant);
            let doc = entry_with_id(&id);
            let err = schema().validate_json(&doc, SchemaKind::Commands).err();
            prop_assert!(err.is_none(), "invalid id {id:?} for {constant:?}: {err:?}");
        }
    }

    #[test]
    fn override_ids_are_schema_valid() {
        for (constant, id) in OVERRIDES {
            let doc = entry_with_id(id);
            schema()
                .validate_json(&doc, SchemaKind::Commands)
                .unwrap_or_else(|e| panic!("OVERRIDES {constant} -> {id}: {e}"));
        }
        for (name, id) in FLUENT_OVERRIDES {
            let doc = entry_with_id(id);
            schema()
                .validate_json(&doc, SchemaKind::Commands)
                .unwrap_or_else(|e| panic!("FLUENT_OVERRIDES {name} -> {id}: {e}"));
        }
    }
}
