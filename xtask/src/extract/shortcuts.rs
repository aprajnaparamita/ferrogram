//! Keyboard shortcuts extractor (P1-04).
//!
//! Reads `reference/support/shortcuts/keyboard-shortcuts-visio.html`
//! (a Wayback Machine snapshot of the Microsoft Support page; see
//! `docs/sources.md` source 5) and produces `ui-spec/shortcuts.yaml`.
//!
//! Each `<table>` row maps a Microsoft action description ("Undo a
//! recent action") to a key sequence. The action -> Ferrogram command
//! mapping is the hand-written [`ACTION_TO_COMMAND`] table; rows that
//! are not in the table are dropped with a warning on `stderr` and
//! recorded in the task's Result section. The 365 layout's command
//! ids are preferred in that table; a 2016-only command is used when
//! the 365 layout has no equivalent (both layouts end up in the same
//! `commands.yaml`, so a shortcut always dispatches the id that
//! survives the merge in [`super::commands`]).
//!
//! Clean-room note (AGENTS.md section 4): the file records the source
//! URL and the extracted key data. No Microsoft prose is committed;
//! the only page text that survives is the short section heading
//! ("Frequently used shortcuts") and the action descriptions listed
//! in [`ACTION_TO_COMMAND`] (short UI labels, allowed by rule 4).

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use super::schema::Schema;
use super::util::{collapse, emit_yaml, unescape_entities, write_and_validate};
use super::{ExtractError, ExtractResult};

/// The Wayback Machine snapshot recorded in the file header.
pub const SNAPSHOT_URL: &str = "https://web.archive.org/web/20251205120214id_/https://support.microsoft.com/en-us/office/keyboard-shortcuts-for-visio-ee952f31-7e3e-4564-8116-f3ecbb733cc1";

/// The pinned source identifier recorded in the file header.
pub const SOURCE_REVISION: &str = "web snapshot 20251205120214 (2025-12-05)";

/// Normalised Microsoft action description -> Ferrogram command id.
/// The key is the row's "To do this" cell after
/// [`normalize_action`]; the value must exist in `ui-spec/commands.yaml`
/// (the unit test `action_targets_exist_in_committed_commands` checks
/// this against the committed file). Rows not listed here are dropped.
pub const ACTION_TO_COMMAND: &[(&str, &str)] = &[
    // Frequently used shortcuts (both articles).
    ("enter the full-screen view", "view.full_screen_mode"),
    ("exit the full-screen view", "view.full_screen_mode"),
    ("group the selected shapes", "arrange.group"),
    ("group the selected objects", "arrange.group"),
    ("ungroup shapes in the selected group", "arrange.ungroup"),
    ("ungroup a group", "arrange.ungroup"),
    (
        "bring the selected shape to the front",
        "arrange.bring_to_front",
    ),
    ("bring to front", "arrange.bring_to_front"),
    (
        "send the selected shape to the back",
        "arrange.send_to_back",
    ),
    ("send to back", "arrange.send_to_back"),
    (
        "rotate the selected shape to the left",
        "arrange.rotate_left",
    ),
    (
        "rotate shape 90 degrees counterclockwise",
        "arrange.rotate_left",
    ),
    (
        "rotate the selected shape to the right",
        "arrange.rotate_right",
    ),
    (
        "rotate a shape 90 degrees clockwise",
        "arrange.rotate_right",
    ),
    (
        "flip the selected shape horizontally",
        "arrange.flip_horizontal",
    ),
    (
        "flip the selected shape vertically",
        "arrange.flip_vertical",
    ),
    (
        "open the align shapes dialog box for the selected shape",
        "arrange.align_objects",
    ),
    ("nudge the selected shape", "arrange.nudge"),
    (
        "nudge the selected shape one pixel at a time",
        "arrange.nudge",
    ),
    ("move a shape", "arrange.nudge"),
    ("move a shape by one pixel", "arrange.nudge"),
    // Edit menu.
    ("undo a recent action", "edit.undo"),
    ("redo a recent action", "edit.redo"),
    ("cut", "edit.cut"),
    ("copy", "edit.copy"),
    ("paste", "edit.paste"),
    ("select all", "edit.select_all"),
    ("select all text in a text block", "edit.select_all"),
    ("select all shapes on the page", "edit.select_all"),
    (
        "select all the master shapes in a stencil",
        "edit.select_all",
    ),
    (
        "copy the selected master shapes to the clipboard",
        "edit.copy",
    ),
    (
        "paste the contents of the clipboard to a new stencil",
        "edit.paste",
    ),
    (
        "cut the selected master shape from the custom stencil and put it on the clipboard",
        "edit.cut",
    ),
    ("delete the selected shape", "edit.clear"),
    ("delete the selected master shape", "edit.clear"),
    ("duplicate a shape", "edit.duplicate"),
    ("delete the previous word", "edit.delete_back_word"),
    // Text formatting.
    ("switch bold formatting on or off", "format.text.bold"),
    ("turn bold formatting on or off", "format.text.bold"),
    ("switch italic formatting on or off", "format.text.italic"),
    ("turn italic formatting on or off", "format.text.italic"),
    ("switch all caps mode on or off", "format.text.all_caps"),
    ("switch small caps mode on or off", "format.text.small_caps"),
    (
        "switch subscript formatting on or off",
        "format.text.subscript",
    ),
    (
        "increase the font size of the selected text",
        "format.text.increase_size",
    ),
    (
        "decrease the font size of the selected text",
        "format.text.decrease_size",
    ),
    // Text alignment (desktop article).
    ("align text left", "format.text.align_left"),
    ("center text horizontally", "format.text.align_center"),
    ("align text right", "format.text.align_right"),
    ("justify text horizontally", "format.text.align_justify"),
    ("top-align text vertically", "format.text.align_top"),
    ("center text vertically", "format.text.align_middle"),
    ("bottom-align text vertically", "format.text.align_bottom"),
    // Text alignment (web article).
    ("align left", "format.text.align_left"),
    ("align center", "format.text.align_center"),
    ("align right", "format.text.align_right"),
    ("align top", "format.text.align_top"),
    ("align middle", "format.text.align_middle"),
    ("align bottom", "format.text.align_bottom"),
    // Tools.
    ("select the pointer tool", "tools.pointer"),
    ("select the text tool", "tools.text"),
    ("select the text block tool", "tools.text_block"),
    ("select the connector tool", "tools.connector"),
    ("select the connection point tool", "tools.connection"),
    ("select the pencil tool", "tools.pencil"),
    ("select the crop tool", "tools.crop"),
    ("select the rectangle tool", "shape.rectangle"),
    ("select the ellipse tool", "shape.ellipse"),
    ("select the line tool", "shape.line"),
    ("select the arc tool", "shape.arc"),
    ("select the freeform tool", "shape.spline"),
    (
        "switch the format painter tool on or off",
        "tools.format_painter",
    ),
    (
        "copy formatting with format painter",
        "tools.format_painter",
    ),
    (
        "paste formatting with format painter",
        "tools.format_painter",
    ),
    ("open the snap & glue dialog box", "tools.snap_glue_setup"),
    // View / connectors / help / file.
    ("zoom in", "view.zoom.in"),
    ("zoom out", "view.zoom.out"),
    (
        "show auto-connection suggestions with quick shapes",
        "connector.auto_connect",
    ),
    ("open the help pane", "help.contents"),
    ("print", "file.print"),
    ("close the active window", "file.close_window"),
];

/// Normalised section heading -> shortcut scope. Headings not listed
/// fall back to `global`.
pub const SECTION_SCOPES: &[(&str, &str)] = &[
    ("frequently used shortcuts", "global"),
    ("move in the full-screen view mode", "global"),
    ("move in a web page drawing", "page"),
    ("move in text or cells", "text"),
    ("navigate the ribbon", "ribbon"),
    ("edit and format text", "text"),
    ("align text", "text"),
    ("use the snap & glue features", "global"),
    ("group, rotate, and flip shapes", "selection"),
    ("display drawing windows", "window"),
    ("select tools", "global"),
    ("select drawing tools", "global"),
    ("crop a picture", "selection"),
    ("move from shape to shape in a drawing page", "page"),
    ("work with master shapes in a stencil", "window"),
    ("work with stencils in edit mode", "window"),
    ("work with windows", "window"),
    ("work with task panes", "window"),
    ("work with dialog boxes", "window"),
    ("zoom", "global"),
    ("page navigation", "page"),
    ("ribbon navigation", "ribbon"),
    ("work with shapes", "selection"),
    ("format text", "text"),
    ("use the format painter", "global"),
    ("move the insertion point in text", "text"),
    ("select content or expand selection", "text"),
    ("working with mind maps", "selection"),
];

/// One table row: the action cell and the press cell.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Row {
    /// Nearest heading above the table (e.g. "Zoom").
    section: String,
    /// "To do this" cell, raw text.
    action: String,
    /// "Press" cell, raw text.
    press: String,
}

/// Per-extractor entry point: writes `ui-spec/shortcuts.yaml`.
pub fn run(in_dir: &Path, out_dir: &Path) -> ExtractResult<Vec<PathBuf>> {
    let schema = Schema::load_default()?;
    let html_path = in_dir
        .join("support")
        .join("shortcuts")
        .join("keyboard-shortcuts-visio.html");
    // R2: only a missing file is `Missing`; every other read error
    // keeps the io error text.
    let html = super::util::read_source(&html_path)?;

    let rows = table_rows(&html);
    let mut entries: Vec<Value> = Vec::new();
    let mut dropped: Vec<(String, String)> = Vec::new();
    for row in &rows {
        let action = normalize_action(&row.action);
        if action.is_empty() {
            continue;
        }
        let Some(command) = command_for_action(&action) else {
            dropped.push((action, "unmapped action".into()));
            continue;
        };
        let Some(sequence) = parse_sequence(&row.press) else {
            dropped.push((action, "unparseable key sequence".into()));
            continue;
        };
        entries.push(shortcut_value(command, &sequence, &row.section));
    }

    // Deterministic order: command, then the key sequence, then the
    // section heading.
    entries.sort_by_cached_key(sort_key);

    let mut warnings: Vec<String> = Vec::new();
    warnings.push(format!(
        "shortcuts: {} written, {} dropped of {} data rows",
        entries.len(),
        dropped.len(),
        rows.len()
    ));
    let mut uniq = dropped.clone();
    uniq.sort();
    uniq.dedup();
    for (action, reason) in &uniq {
        warnings.push(format!("shortcuts: dropped ({reason}): {action}"));
    }
    eprintln!("{}", warnings.join("\n"));

    let text = render_shortcuts_yaml(&entries)?;
    let path = out_dir.join("ui-spec").join("shortcuts.yaml");
    write_and_validate(&schema, &path, &text)?;
    Ok(vec![path])
}

/// Looks up the normalised action in [`ACTION_TO_COMMAND`].
fn command_for_action(action: &str) -> Option<&'static str> {
    ACTION_TO_COMMAND
        .iter()
        .find(|(a, _)| *a == action)
        .map(|(_, c)| *c)
}

/// Sort key for one rendered shortcut: command, flattened sequence,
/// section.
fn sort_key(v: &Value) -> String {
    let command = v.get("command").and_then(Value::as_str).unwrap_or("");
    let section = v.get("section").and_then(Value::as_str).unwrap_or("");
    let seq = v
        .get("sequence")
        .and_then(Value::as_array)
        .map(|chords| {
            chords
                .iter()
                .filter_map(Value::as_array)
                .map(|keys| {
                    keys.iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join("+")
                })
                .collect::<Vec<_>>()
                .join(",")
        })
        .unwrap_or_default();
    format!("{command}\u{0}{seq}\u{0}{section}")
}

/// Builds one `shortcut` schema object.
fn shortcut_value(command: &str, sequence: &[Vec<String>], section: &str) -> Value {
    let mut obj = Map::new();
    obj.insert("command".into(), Value::String(command.into()));
    obj.insert(
        "scope".into(),
        Value::String(scope_for_section(section).into()),
    );
    obj.insert("section".into(), Value::String(normalize_heading(section)));
    obj.insert(
        "sequence".into(),
        Value::Array(
            sequence
                .iter()
                .map(|chord| Value::Array(chord.iter().map(|k| Value::String(k.clone())).collect()))
                .collect(),
        ),
    );
    Value::Object(obj)
}

/// Scope for a section heading ([`SECTION_SCOPES`], fallback
/// `global`).
fn scope_for_section(section: &str) -> &'static str {
    let key = section.trim().to_ascii_lowercase();
    SECTION_SCOPES
        .iter()
        .find(|(s, _)| *s == key)
        .map(|(_, scope)| *scope)
        .unwrap_or("global")
}

/// Keeps the heading as a short label, decoding entities.
fn normalize_heading(heading: &str) -> String {
    collapse(&unescape_entities(heading))
}

/// Normalises a "To do this" cell: lower case, collapsed whitespace,
/// trailing period removed, `Note:` tails cut.
pub fn normalize_action(action: &str) -> String {
    let text = collapse(&unescape_entities(action));
    let cut = [" note:", " note that"]
        .iter()
        .filter_map(|m| text.to_ascii_lowercase().find(m))
        .min();
    let text = match cut {
        Some(i) => &text[..i],
        None => text.as_str(),
    };
    text.trim()
        .trim_end_matches('.')
        .trim()
        .to_ascii_lowercase()
}

/// Parses a "Press" cell into a sequence of chords.
///
/// Grammar:
///
/// - Alternatives ("A or B") take the first alternative.
/// - Steps are comma separated (`Alt+H, F, S`); a step that is only a
///   modifier joins the next step (`Alt, H, F, O` -> `Alt+H`, `F`, `O`).
/// - Within a step, `+` separates keys pressed together.
/// - Parenthesised single symbols are the key
///   (`Equal sign ( = )` -> `=`); other parentheticals are dropped.
/// - Prose tails after `to` / `where` / `for` / ... are dropped
///   (`S to open the Save As dialog box` -> `S`).
/// - Every key must pass [`canonical_key`]; otherwise the whole
///   sequence is rejected (the row is dropped with a warning).
pub fn parse_sequence(press: &str) -> Option<Vec<Vec<String>>> {
    let text = collapse(&unescape_entities(press));
    if text.is_empty() {
        return None;
    }
    let alt = match text.to_ascii_lowercase().find(" or ") {
        Some(i) => &text[..i],
        None => text.as_str(),
    };
    let mut steps: Vec<Vec<String>> = Vec::new();
    for raw_step in alt.split(',') {
        let step = cut_prose(raw_step.trim());
        if step.is_empty() {
            continue;
        }
        let mut chord = Vec::new();
        for part in split_keys(step) {
            let part = strip_parenthetical(part.trim());
            if part.is_empty() {
                return None;
            }
            chord.push(canonical_key(&part)?);
        }
        if chord.is_empty() {
            return None;
        }
        steps.push(chord);
    }
    if steps.is_empty() {
        return None;
    }
    // A lone modifier chord is typed together with the next step.
    let mut merged: Vec<Vec<String>> = Vec::new();
    let mut i = 0;
    while i < steps.len() {
        let cur = steps.get(i).cloned().unwrap_or_default();
        let lone_modifier = cur.len() == 1 && is_modifier(cur.first().map(String::as_str));
        if lone_modifier && i + 1 < steps.len() {
            let mut next = steps.get(i + 1).cloned().unwrap_or_default();
            let mut combined = cur.clone();
            combined.append(&mut next);
            merged.push(combined);
            i += 2;
        } else {
            merged.push(cur);
            i += 1;
        }
    }
    Some(merged)
}

/// True for chord keys that are modifiers.
fn is_modifier(key: Option<&str>) -> bool {
    matches!(key, Some("Ctrl") | Some("Shift") | Some("Alt"))
}

/// Cuts explanatory prose from a step ("... to open the dialog").
fn cut_prose(step: &str) -> &str {
    let lower = step.to_ascii_lowercase();
    const MARKERS: [&str; 7] = [
        " to ", " where ", " for ", " that ", " with ", " and ", " then ",
    ];
    let cut = MARKERS
        .iter()
        .filter_map(|m| lower.find(m))
        .min()
        .unwrap_or(usize::MAX);
    if cut == usize::MAX {
        step
    } else {
        step[..cut].trim()
    }
}

/// Splits a step into `+`-separated key parts, ignoring `+` inside
/// parentheses (`Plus sign (+)` is one part).
fn split_keys(step: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (i, c) in step.char_indices() {
        match c {
            '(' => depth = depth.saturating_add(1),
            ')' => depth = depth.saturating_sub(1),
            '+' if depth == 0 => {
                out.push(&step[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(&step[start..]);
    out
}

/// Drops a parenthetical tail. A parenthesised single symbol is the
/// key itself (`Equal sign ( = )` -> `=`, `Right angle bracket (>)`
/// -> `>`); anything else is commentary (`F6 (You might...)` ->
/// `F6`).
fn strip_parenthetical(part: &str) -> String {
    let Some(open) = part.find('(') else {
        return part.trim().to_string();
    };
    let Some(close) = part.rfind(')') else {
        return part[..open].trim().to_string();
    };
    if close < open {
        return part.trim().to_string();
    }
    let inner = part[open + 1..close].trim();
    let before = part[..open].trim();
    let inner_is_key = !inner.is_empty()
        && !inner.contains(' ')
        && inner.chars().count() <= 3
        && (inner.chars().all(|c| !c.is_ascii_alphanumeric()) || inner.chars().count() == 1);
    if inner_is_key {
        // The parenthesised symbol *is* the key
        // (`Equal sign ( = )` -> `=`, `Right angle bracket (>)` -> `>`).
        return inner.to_string();
    }
    before.to_string()
}

/// Canonicalises one key token, rejecting prose. Returns `None` when
/// the token is not a key we know.
pub fn canonical_key(token: &str) -> Option<String> {
    let t = token.trim();
    if t.is_empty() || t.chars().count() > 20 {
        return None;
    }
    if t.chars().count() == 1 {
        let c = t.chars().next()?;
        if c.is_ascii_alphanumeric() {
            return Some(c.to_ascii_uppercase().to_string());
        }
        if !c.is_whitespace() {
            return Some(c.to_string());
        }
        return None;
    }
    let lower = t.to_ascii_lowercase();
    let base = lower.strip_suffix(" key").unwrap_or(&lower).trim();
    let name = match base {
        "ctrl" | "control" => "Ctrl",
        "shift" => "Shift",
        "alt" => "Alt",
        "esc" | "escape" => "Esc",
        "enter" | "return" => "Enter",
        "tab" => "Tab",
        "spacebar" | "space" => "Spacebar",
        "backspace" => "Backspace",
        "delete" | "del" => "Delete",
        "home" => "Home",
        "end" => "End",
        "insert" | "ins" => "Insert",
        "page up" | "pageup" | "pgup" => "Page Up",
        "page down" | "pagedown" | "pgdn" => "Page Down",
        "up arrow" | "up" => "Up Arrow",
        "down arrow" | "down" => "Down Arrow",
        "left arrow" | "left" => "Left Arrow",
        "right arrow" | "right" => "Right Arrow",
        "print screen" | "printscreen" | "prtsc" => "Print Screen",
        "arrow keys" | "arrow key" | "arrows" => "Arrow Keys",
        "windows menu key" | "menu key" | "menu" => "Menu",
        "f1" => "F1",
        "f2" => "F2",
        "f3" => "F3",
        "f4" => "F4",
        "f5" => "F5",
        "f6" => "F6",
        "f7" => "F7",
        "f8" => "F8",
        "f9" => "F9",
        "f10" => "F10",
        "f11" => "F11",
        "f12" => "F12",
        _ => return None,
    };
    Some(name.to_string())
}

/// Extracts every `<table>` row of the page together with the nearest
/// heading above it. Header rows (`<th>`) and short rows are skipped.
fn table_rows(html: &str) -> Vec<Row> {
    let mut rows = Vec::new();
    let mut section = String::new();
    let mut pos = 0usize;
    while let Some(next) = find_from(html, pos) {
        match next {
            Hit::Heading(at, end) => {
                let heading = strip_tags(&html[at..end]);
                if !heading.trim().is_empty() {
                    section = heading;
                }
                pos = end;
            }
            Hit::Table(start, end) => {
                let body = &html[start..end];
                for tr in find_blocks(body, "tr") {
                    let cells = find_cells(tr);
                    if cells.iter().any(|(is_th, _)| *is_th) {
                        continue;
                    }
                    let texts: Vec<String> = cells
                        .iter()
                        .map(|(_, raw)| collapse(&strip_tags(raw)))
                        .collect();
                    let (Some(action), Some(press)) = (texts.first(), texts.get(1)) else {
                        continue;
                    };
                    if action.trim().is_empty() {
                        continue;
                    }
                    rows.push(Row {
                        section: section.clone(),
                        action: action.clone(),
                        press: press.clone(),
                    });
                }
                pos = end;
            }
        }
    }
    rows
}

/// A heading or table hit in the page.
enum Hit {
    /// `<h2>`/`<h3>` inner text span.
    Heading(usize, usize),
    /// `<table>` inner span.
    Table(usize, usize),
}

/// Finds the next heading or table at or after `pos`.
fn find_from(html: &str, pos: usize) -> Option<Hit> {
    let bytes = html.as_bytes();
    let mut i = pos;
    while i < bytes.len() {
        if bytes.get(i) == Some(&b'<') {
            let rest = &html[i..];
            let lower_head: String = rest
                .chars()
                .take(6)
                .collect::<String>()
                .to_ascii_lowercase();
            if lower_head.starts_with("<h2") || lower_head.starts_with("<h3") {
                let tag_end = rest.find('>').map(|e| i + e + 1)?;
                let close = if lower_head.starts_with("<h2") {
                    "</h2>"
                } else {
                    "</h3>"
                };
                let end = rest.find(close).map(|e| i + e).unwrap_or(html.len());
                return Some(Hit::Heading(tag_end, end));
            }
            if lower_head.starts_with("<table") {
                let tag_end = rest.find('>').map(|e| i + e + 1)?;
                let end = rest.find("</table>").map(|e| i + e).unwrap_or(html.len());
                return Some(Hit::Table(tag_end, end));
            }
        }
        i += 1;
    }
    None
}

/// Finds every `<tr>...</tr>` block inside `html`.
fn find_blocks<'a>(html: &'a str, tag: &str) -> Vec<&'a str> {
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let mut out = Vec::new();
    let mut pos = 0usize;
    while let Some(start) = html[pos..].find(&open).map(|e| pos + e) {
        let Some(open_end) = html[start..].find('>').map(|e| start + e + 1) else {
            break;
        };
        let end = html[open_end..]
            .find(&close)
            .map(|e| open_end + e)
            .unwrap_or(html.len());
        out.push(&html[open_end..end]);
        pos = end + close.len();
        if pos >= html.len() {
            break;
        }
    }
    out
}

/// Splits a `<tr>` block into `(is_header, raw)` cells.
fn find_cells(tr: &str) -> Vec<(bool, &str)> {
    let mut out = Vec::new();
    let mut pos = 0usize;
    while pos < tr.len() {
        let Some(lt) = tr[pos..].find('<').map(|e| pos + e) else {
            break;
        };
        let lower: String = tr[lt..]
            .chars()
            .take(4)
            .collect::<String>()
            .to_ascii_lowercase();
        let (is_th, close) = if lower.starts_with("<th") {
            (true, "</th>")
        } else if lower.starts_with("<td") {
            (false, "</td>")
        } else {
            pos = lt + 1;
            continue;
        };
        let Some(open_end) = tr[lt..].find('>').map(|e| lt + e + 1) else {
            break;
        };
        let end = tr[open_end..]
            .find(close)
            .map(|e| open_end + e)
            .unwrap_or(tr.len());
        out.push((is_th, &tr[open_end..end]));
        pos = end + close.len();
    }
    out
}

/// Strips HTML tags (keeping `<img alt="...">` alt text) and decodes
/// entities, leaving raw text.
fn strip_tags(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(lt) = rest.find('<') {
        out.push_str(&rest[..lt]);
        let Some(gt) = rest[lt..].find('>') else {
            break;
        };
        let tag = &rest[lt..lt + gt + 1];
        let lower = tag.to_ascii_lowercase();
        if lower.starts_with("<img")
            && let Some(a) = lower.find("alt=\"")
        {
            let alt_region = &rest[lt + a + 5..];
            if let Some(end) = alt_region.find('"') {
                out.push(' ');
                out.push_str(&alt_region[..end]);
                out.push(' ');
            }
        }
        out.push(' ');
        rest = &rest[lt + gt + 1..];
    }
    out.push_str(rest);
    unescape_entities(&out)
}

/// Renders `ui-spec/shortcuts.yaml` (header comment + YAML list).
fn render_shortcuts_yaml(entries: &[Value]) -> ExtractResult<String> {
    let text = emit_yaml(&Value::Array(entries.to_vec())).map_err(|msg| ExtractError::Parse {
        path: PathBuf::from("ui-spec/shortcuts.yaml"),
        message: msg,
    })?;
    let mut out = String::new();
    out.push_str("# Generated by `cargo xtask extract` (xtask/src/extract/shortcuts.rs).\n");
    out.push_str("# Do not edit by hand: edit the extractor rules instead (ADR 0007 m2).\n");
    out.push_str(
        "# Key sequences extracted from the Microsoft Support page \"Keyboard shortcuts\n",
    );
    out.push_str("# for Visio\" (docs/sources.md source 5). Only the URL and the key data are\n");
    out.push_str("# recorded here; no page prose is committed.\n");
    out.push_str(&format!("# Snapshot URL: {SNAPSHOT_URL}\n"));
    out.push_str(&format!("# Source commit: {SOURCE_REVISION}\n"));
    out.push_str(&text);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE_HTML: &str = r#"<!DOCTYPE html>
<html><body>
<h2>Frequently used shortcuts</h2>
<table class="banded">
  <thead><tr><th><p>To do this</p></th><th><p>Press</p></th></tr></thead>
  <tbody>
    <tr><td><p>Undo a recent action</p></td><td><p>Ctrl+Z</p></td></tr>
    <tr><td><p>Group the selected shapes.</p></td><td><p>Ctrl+G or Ctrl+Shift+G</p></td></tr>
  </tbody>
</table>
<h2>Zoom</h2>
<table class="banded">
  <tr><td><p>Zoom in.</p></td><td><p>Alt+F6</p></td></tr>
  <tr><td><p>Not a command we know.</p></td><td><p>F12</p></td></tr>
</table>
<h2>Ribbon navigation</h2>
<table>
  <tr><td><p>Open the submenu for a selected command.</p></td><td><p>Alt+H, F, S to open the Save As dialog box</p></td></tr>
</table>
</body></html>
"#;

    #[test]
    fn walker_finds_tables_rows_and_sections() {
        let rows = table_rows(FIXTURE_HTML);
        assert_eq!(rows.len(), 5, "{rows:?}");
        assert_eq!(rows[0].section, "Frequently used shortcuts");
        assert_eq!(rows[0].action, "Undo a recent action");
        assert_eq!(rows[0].press, "Ctrl+Z");
        assert_eq!(rows[2].section, "Zoom");
        assert_eq!(rows[4].section, "Ribbon navigation");
    }

    #[test]
    fn single_chord_sequences() {
        assert_eq!(
            parse_sequence("Ctrl+Z"),
            Some(vec![vec!["Ctrl".into(), "Z".into()]])
        );
        assert_eq!(parse_sequence("F5"), Some(vec![vec!["F5".into()]]));
        assert_eq!(parse_sequence("Esc"), Some(vec![vec!["Esc".into()]]));
        assert_eq!(parse_sequence("Tab key"), Some(vec![vec!["Tab".into()]]));
    }

    #[test]
    fn alternatives_take_the_first() {
        assert_eq!(
            parse_sequence("Ctrl+G or Ctrl+Shift+G"),
            Some(vec![vec!["Ctrl".into(), "G".into()]])
        );
        assert_eq!(
            parse_sequence("Tab key (forward) or Shift+Tab (backward)"),
            Some(vec![vec!["Tab".into()]])
        );
    }

    #[test]
    fn multi_step_keytips() {
        // The task's example: `Alt, H, F, O`.
        assert_eq!(
            parse_sequence("Alt, H, F, O"),
            Some(vec![
                vec!["Alt".into(), "H".into()],
                vec!["F".into()],
                vec!["O".into()]
            ])
        );
        // The page's own spelling of the same idea.
        assert_eq!(
            parse_sequence("Alt+H, F, S to open the Save As dialog box"),
            Some(vec![
                vec!["Alt".into(), "H".into()],
                vec!["F".into()],
                vec!["S".into()]
            ])
        );
    }

    #[test]
    fn parenthesised_symbols_are_keys() {
        assert_eq!(
            parse_sequence("Ctrl+Equal sign ( = )"),
            Some(vec![vec!["Ctrl".into(), "=".into()]])
        );
        assert_eq!(
            parse_sequence("Ctrl+Shift+Right angle bracket (>)"),
            Some(vec![vec!["Ctrl".into(), "Shift".into(), ">".into()]])
        );
        assert_eq!(
            parse_sequence("F6 (You might have to press F6 more than once.)"),
            Some(vec![vec!["F6".into()]])
        );
    }

    #[test]
    fn prose_sequences_are_rejected() {
        assert_eq!(parse_sequence(""), None);
        assert_eq!(
            parse_sequence("The first letter of an option in a dropdown list"),
            None
        );
        assert_eq!(parse_sequence("Ctrl+This is not a key"), None);
    }

    #[test]
    fn canonical_keys_reject_prose() {
        assert_eq!(canonical_key("ctrl").as_deref(), Some("Ctrl"));
        assert_eq!(
            canonical_key("Left arrow key").as_deref(),
            Some("Left Arrow")
        );
        assert_eq!(canonical_key("Y").as_deref(), Some("Y"));
        assert_eq!(canonical_key("=").as_deref(), Some("="));
        assert_eq!(canonical_key("something else entirely"), None);
    }

    #[test]
    fn ctrl_z_row_keeps_the_sequence_shape() {
        let rows = table_rows(FIXTURE_HTML);
        let undo = rows.iter().find(|r| r.action.contains("Undo")).unwrap();
        let cmd = command_for_action(&normalize_action(&undo.action)).unwrap();
        assert_eq!(cmd, "edit.undo");
        let seq = parse_sequence(&undo.press).unwrap();
        // m2: a single-chord shortcut is a one-element sequence.
        assert_eq!(seq, vec![vec!["Ctrl".to_string(), "Z".to_string()]]);
    }

    #[test]
    fn yaml_11_boolean_keys_survive_a_round_trip() {
        // Y/N/On/Off must be emitted as quoted strings, or a YAML 1.1
        // reader turns the chord into booleans (P1-06 open issue 3).
        let sequence = vec![
            vec!["Ctrl".to_string(), "Y".to_string()],
            vec!["N".to_string()],
            vec!["On".to_string()],
            vec!["Off".to_string()],
        ];
        let entry = shortcut_value("edit.undo", &sequence, "Frequently used shortcuts");
        let text = render_shortcuts_yaml(&[entry]).unwrap();
        let back: Value = serde_saphyr::from_str(&text).unwrap();
        let seq = back
            .get(0)
            .and_then(|v| v.get("sequence"))
            .and_then(Value::as_array)
            .unwrap();
        for (i, chord) in seq.iter().enumerate() {
            let keys = chord.as_array().unwrap();
            for (j, key) in keys.iter().enumerate() {
                assert!(key.is_string(), "key {i}/{j} lost its string type: {key:?}");
                assert_eq!(key.as_str().unwrap(), sequence[i][j]);
            }
        }
    }

    #[test]
    fn normalize_action_is_stable() {
        assert_eq!(
            normalize_action("Select the Pointer Tool ."),
            "select the pointer tool"
        );
        assert_eq!(
            normalize_action(
                "Nudge the selected shape one pixel at a time. Note: Scroll lock must be turned off."
            ),
            "nudge the selected shape one pixel at a time"
        );
    }

    #[test]
    fn output_is_deterministic() {
        let mut entries = Vec::new();
        for row in table_rows(FIXTURE_HTML) {
            let action = normalize_action(&row.action);
            if let Some(cmd) = command_for_action(&action)
                && let Some(seq) = parse_sequence(&row.press)
            {
                entries.push(shortcut_value(cmd, &seq, &row.section));
            }
        }
        entries.sort_by_cached_key(sort_key);
        let a = render_shortcuts_yaml(&entries).unwrap();
        let b = render_shortcuts_yaml(&entries).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn run_on_fixture_writes_and_validates() {
        let tmp = crate::util::test_support::TempDir::new("shortcuts");
        let in_dir = tmp.join("in");
        let out_dir = tmp.join("out");
        let src = in_dir.join("support").join("shortcuts");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(out_dir.join("ui-spec")).unwrap();
        std::fs::write(src.join("keyboard-shortcuts-visio.html"), FIXTURE_HTML).unwrap();
        let outputs = run(&in_dir, &out_dir).unwrap();
        assert_eq!(outputs.len(), 1);
        let text = std::fs::read_to_string(&outputs[0]).unwrap();
        assert!(text.contains("Snapshot URL:"), "{text}");
        // The unmapped row is dropped; the rest are written.
        assert!(!text.contains("Not a command we know"));
        assert!(text.contains("edit.undo"));
    }

    #[test]
    fn run_without_input_is_a_missing_error() {
        let tmp = crate::util::test_support::TempDir::new("shortcuts2");
        let err = run(tmp.path(), tmp.path()).unwrap_err();
        assert!(matches!(err, ExtractError::Missing { .. }), "{err}");
    }

    /// R2: a directory in the way is an Io error naming the file, not
    /// a `Missing` error.
    #[test]
    fn run_with_a_directory_in_the_way_is_an_io_error() {
        let tmp = crate::util::test_support::TempDir::new("shortcuts-dir");
        let src = tmp.join("support/shortcuts");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(src.join("keyboard-shortcuts-visio.html")).unwrap();
        let err = run(tmp.path(), tmp.path()).unwrap_err();
        assert!(matches!(err, ExtractError::Io { .. }), "{err}");
        assert!(
            err.to_string().contains("keyboard-shortcuts-visio.html"),
            "{err}"
        );
    }

    /// R2: a non-UTF-8 source is an Io error keeping the io error
    /// text, not a `Missing` error.
    #[test]
    fn run_with_non_utf8_input_is_an_io_error() {
        let tmp = crate::util::test_support::TempDir::new("shortcuts-bin");
        let src = tmp.join("support/shortcuts");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(
            src.join("keyboard-shortcuts-visio.html"),
            [0xff, 0xfe, 0xfd],
        )
        .unwrap();
        let err = run(tmp.path(), tmp.path()).unwrap_err();
        assert!(matches!(err, ExtractError::Io { .. }), "{err}");
        assert!(
            err.to_string()
                .contains("stream did not contain valid UTF-8"),
            "{err}"
        );
    }

    /// Every command id in [`ACTION_TO_COMMAND`] must exist in the
    /// committed `ui-spec/commands.yaml`, or the loader's cross-check
    /// fails. Skipped when the file is not present.
    #[test]
    fn action_targets_exist_in_committed_commands() {
        let path = crate::util::root().join("ui-spec").join("commands.yaml");
        if !path.is_file() {
            eprintln!("commands.yaml not present; skipping");
            return;
        }
        let text = std::fs::read_to_string(&path).unwrap();
        let value: Value = serde_saphyr::from_str(&text).unwrap();
        let Some(entries) = value.as_array() else {
            eprintln!("commands.yaml empty; skipping");
            return;
        };
        let ids: std::collections::BTreeSet<&str> = entries
            .iter()
            .filter_map(|e| e.get("id").and_then(Value::as_str))
            .collect();
        for (action, cmd) in ACTION_TO_COMMAND {
            assert!(
                ids.contains(cmd),
                "action {action:?} -> unknown command {cmd}"
            );
        }
    }

    #[test]
    fn every_mapped_row_in_the_fixture_survives_the_run() {
        let rows = table_rows(FIXTURE_HTML);
        let mapped: Vec<&Row> = rows
            .iter()
            .filter(|r| command_for_action(&normalize_action(&r.action)).is_some())
            .collect();
        assert_eq!(mapped.len(), 3, "{mapped:?}");
    }

    // ---- P1-08 goal 4: property tests -------------------------------

    use proptest::prelude::*;

    /// Key tokens the writer must quote: YAML 1.1 boolean scalars and
    /// other ambiguous plain scalars, plus ordinary canonical keys.
    const KEY_TOKENS: &[&str] = &[
        "Y",
        "N",
        "On",
        "Off",
        "y",
        "n",
        "on",
        "off",
        "Yes",
        "No",
        "True",
        "false",
        "null",
        "~",
        "0x1F",
        "0123",
        "1.5",
        "-",
        "+",
        "*",
        "=",
        "Ctrl",
        "Shift",
        "Z",
        "F5",
        "Spacebar",
        "Left Arrow",
    ];

    fn key_token() -> impl Strategy<Value = String> {
        proptest::sample::select(KEY_TOKENS.to_vec()).prop_map(|s| s.to_string())
    }

    proptest! {
        /// The key-sequence parser is total: arbitrary "Press" text
        /// yields a sequence or `None`, never a panic.
        #[test]
        fn sequence_parser_never_panics(press in ".*") {
            let _ = parse_sequence(&press);
        }

        /// Every chord the parser emits passes the schema's `keyChord`
        /// rule: a string of 1 to 32 characters.
        #[test]
        fn emitted_chords_pass_the_keychord_rule(press in ".*") {
            if let Some(sequence) = parse_sequence(&press) {
                for chord in &sequence {
                    prop_assert!(!chord.is_empty(), "empty chord for {press:?}");
                    for key in chord {
                        prop_assert!(!key.is_empty(), "empty key for {press:?}");
                        prop_assert!(
                            key.chars().count() <= 32,
                            "key {key:?} longer than keyChord maxLength"
                        );
                    }
                }
            }
        }

        /// Quoted YAML 1.1 boolean tokens (and other ambiguous plain
        /// scalars) survive a write-then-read round trip through
        /// `ui-spec/shortcuts.yaml`.
        #[test]
        fn key_tokens_survive_a_yaml_round_trip(
            tokens in proptest::collection::vec(key_token(), 1..8),
        ) {
            let sequence: Vec<Vec<String>> = tokens.iter().map(|t| vec![t.clone()]).collect();
            let entries = vec![shortcut_value("edit.undo", &sequence, "Frequently used shortcuts")];
            let text = render_shortcuts_yaml(&entries).unwrap();
            let parsed: Value = serde_saphyr::from_str(&text).unwrap();
            let back: Vec<Vec<String>> = parsed[0]
                .get("sequence")
                .and_then(Value::as_array)
                .unwrap()
                .iter()
                .map(|chord| {
                    chord
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|key| key.as_str().unwrap_or_default().to_string())
                        .collect()
                })
                .collect();
            prop_assert_eq!(back, sequence, "tokens lost in the YAML round trip");
        }
    }
}
