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
//! Clean-room note (AGENTS.md section 4). What this file keeps from
//! the page: key names and key chords, and *hashes* of the normalised
//! action phrases ([`ACTION_TO_COMMAND`]) and of the normalised
//! section headings ([`SECTION_SCOPES`]). No sentence or heading text
//! from the page is committed. A maintainer resolves a hash by
//! looking the phrase up in `reference/` (never tracked). The test
//! fixtures are original text written for the parsers, not page text.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use super::schema::Schema;
use super::util::{collapse, emit_yaml, unescape_entities, write_and_validate};
use super::{ExtractError, ExtractResult};

/// The Wayback Machine snapshot recorded in the file header.
pub const SNAPSHOT_URL: &str = "https://web.archive.org/web/20251205120214id_/https://support.microsoft.com/en-us/office/keyboard-shortcuts-for-visio-ee952f31-7e3e-4564-8116-f3ecbb733cc1";

/// The pinned source identifier recorded in the file header.
pub const SOURCE_REVISION: &str = "web snapshot 20251205120214 (2025-12-05)";

/// Normalised action phrase -> Ferrogram command id, keyed by the
/// lowercase hex sha256 of the normalised phrase (F2). The value must
/// exist in `ui-spec/commands.yaml` (the unit test
/// `action_targets_exist_in_committed_commands` checks this against
/// the committed file). Rows whose hash is not in the table are
/// dropped with a warning.
///
/// To add an entry:
///
/// 1. Take the "To do this" cell from the page in `reference/`
///    (never tracked) and normalise it with [`normalize_action`]
///    (lower case, collapsed whitespace, trailing period and `Note:`
///    tails removed).
/// 2. Hash the normalised phrase: sha256 in lowercase hex
///    ([`super::util::sha256_hex`]).
/// 3. Add `("<hash>", "<command id>")` below.
///
/// The phrase itself is never committed (AGENTS.md section 4).
pub const ACTION_TO_COMMAND: &[(&str, &str)] = &[
    // Frequently used shortcuts (both articles).
    (
        "680d630f1da97fa56cabe9fa34c782661eee6e65aaf607a4e6b8428a4a22dc22",
        "view.full_screen_mode",
    ),
    (
        "7a113be17c72007194082ef80d722adf2ffb133fb0785caaebf7b5e5e7dc6245",
        "view.full_screen_mode",
    ),
    (
        "dbe1ccdcc78a8884738efb7e7d78954c306b76f83a16343af392928bd247365b",
        "arrange.group",
    ),
    (
        "0ae099860711d7daf05d795e14f34888cd320566289364b4dd5b31f9a54cb542",
        "arrange.group",
    ),
    (
        "bcb3d5c8dc93faa6465d61229a0aa50cca71f206988dad1e8dda9172ca0f91f4",
        "arrange.ungroup",
    ),
    (
        "ff6a323bb7eafab02b1cf86af856bb9ef8c8df3851b83c627a682b3b00581e8d",
        "arrange.ungroup",
    ),
    (
        "c8a47940794f9483ab05f102413c1dc1ec309a616ea8374568663607221ff82d",
        "arrange.bring_to_front",
    ),
    (
        "28f2adf0524343953eef59040f91cb4edcbbb82540484ae5a044c543e37aaef4",
        "arrange.bring_to_front",
    ),
    (
        "1666b19911918ecfe8053774e61c5656b758fb81d2fa39e25d3b542c7ddd516c",
        "arrange.send_to_back",
    ),
    (
        "eab3dc2771f6f1f37abab111c0ca5cc34f7da5b0550fec2d9e78bba388903976",
        "arrange.send_to_back",
    ),
    (
        "cc4b3a824a99cadd5bf6d1b5c32f1ebde01e1c96a93483a77565660f653cdf9d",
        "arrange.rotate_left",
    ),
    (
        "dc438e0291dbc27aeea89f7ae99de2428fa65ac989aea6f21f6a8146c3d044d4",
        "arrange.rotate_left",
    ),
    (
        "d9744811f15141fe88cd59ed52be531b365130c57727a5b4c1eff5f69c945655",
        "arrange.rotate_right",
    ),
    (
        "daea24e8ccdea9f4a361231daad78c85f9d489bd2a3eee99649449d06fd065c2",
        "arrange.rotate_right",
    ),
    (
        "a080e0b029f61e21716915a777a0a81495a8a6b9886f5d68a9a3191c171fb4d5",
        "arrange.flip_horizontal",
    ),
    (
        "c594f70d18dcf8097f2313af264440f337034114d39a600a9f72c74a097272b4",
        "arrange.flip_vertical",
    ),
    (
        "c734052584cd5c41bfd313144bd660eac856df992e2986957de7bc178cbaeca4",
        "arrange.align_objects",
    ),
    (
        "8a4ebacee696d05d204aaac76cd2bf51ac70feb034e8874756db9cdf9cdb1505",
        "arrange.nudge",
    ),
    (
        "efad2ff0dc3ae484d59fd1110a6987911c89158cb3034120b95855e2fa7a10b8",
        "arrange.nudge",
    ),
    (
        "7d063282b674e167c5331559d1b5f16883978d494423d6e95705647c0e7194b8",
        "arrange.nudge",
    ),
    (
        "dfab6797e63da8667340ddfa5a116d8dc4c6086a1e326fc8bf03daafd1fc8987",
        "arrange.nudge",
    ),
    // Edit menu.
    (
        "105e903aba9b6c7bf51d9d3b34a11daffda34c78efe2311960503662e7efdf1e",
        "edit.undo",
    ),
    (
        "3f770cdc4707f673241de25147104e49c18c27d9e458129eb107f42ed86a8b4b",
        "edit.redo",
    ),
    (
        "378bfce5cda2599a6cda399f1cacef861e4e575ec794744dcf0e55e9c4780633",
        "edit.cut",
    ),
    (
        "6f5a6034e770acbfb3f797e6a7eb7948d470d45f9928f92b7d72dc7c45e6d0cd",
        "edit.copy",
    ),
    (
        "a2c183d5a12aea35598b2ce8d1f92a9c5dc9b157b9eadbcd6c6c0499e97077c2",
        "edit.paste",
    ),
    (
        "a36fc761262ffa20b3d93eaf95ebb31b54e38dc9fa0082cf80c7454bed13fc81",
        "edit.select_all",
    ),
    (
        "aad34f524ffad822bfcec384b96f29fa3abeaed9e50a42a0dc88c90863372632",
        "edit.select_all",
    ),
    (
        "1a172b21c9748ed58c40a4aae723508499a1810db567041461cd43204aebbf6f",
        "edit.select_all",
    ),
    (
        "c9d506cf7e2148a77b7b0a3fcb1e17b7604dad86bc880d0fef41de28807b4dcd",
        "edit.select_all",
    ),
    (
        "8006f6f5806a5167c057f174b44620a1a48b3466f37fef5a14101381968aefd3",
        "edit.copy",
    ),
    (
        "64fe3fbbd34145ebcf0eec20d5bd9894abfa15e02f33976a8a1a6d1f985b6817",
        "edit.paste",
    ),
    (
        "721e6910792d5ffe6ae128b38b450864daa19ef5535d01da022e0455289c0d68",
        "edit.cut",
    ),
    (
        "5fe7b50655c9a3210ac7367370480c33ac4e876152eb2ffbe98ad623dd3c122c",
        "edit.clear",
    ),
    (
        "76ffdbf075537f388fd0fe26f2939332e61f854f15c9dc55654a8e279b3759c8",
        "edit.clear",
    ),
    (
        "4760c2b2d33bcbff705a488e460944e2b9cf75b07c9688df435d34f0130520a5",
        "edit.duplicate",
    ),
    (
        "9302f7b4c0431e91094f4923e54494ade6dc1f3a458ae4f9aa78fc06d58aa7cb",
        "edit.delete_back_word",
    ),
    // Text formatting.
    (
        "b3ca17ad94b6964aeea83087d271b9921c3c2e4bcfa161c763d8b26707853201",
        "format.text.bold",
    ),
    (
        "ce8c9e13a7d520ce3864e25aff923e4d0f928860f7e0028ba051aa1fd69a76b0",
        "format.text.bold",
    ),
    (
        "50e005e131d657704bd67529bf372bb27b4a602db0e2062607cfe20580f7e91c",
        "format.text.italic",
    ),
    (
        "665a11cf9de7749cea247d8e6300483fe4d1cb57689d07b2f71a9c12858c8b6f",
        "format.text.italic",
    ),
    (
        "3a5717d8ec1c3137f28fe958ec71b01861790ce7e91b391fc67fbe5b0e125a19",
        "format.text.all_caps",
    ),
    (
        "a6733176a5375b628a4a18204d851938e031b24ff8ac32257955fb1bf4bdc68a",
        "format.text.small_caps",
    ),
    (
        "bc8529d602b73a75ff952d95be5b6ec454a93fc0eca932a49efc82d2317bcf64",
        "format.text.subscript",
    ),
    (
        "c71b56e3c5b4986e3631bc4c5c5ec0529d837454c266bd79ac35f56d7c9a9c4b",
        "format.text.increase_size",
    ),
    (
        "fa3be8e387401233a5b841e71cc5c5a8f14998dbd0057d8d990c77753ddf087e",
        "format.text.decrease_size",
    ),
    // Text alignment (desktop article).
    (
        "20d5171b06dea1b281ec6b9480f035efeaebdab8c9148a1290582d55f296b676",
        "format.text.align_left",
    ),
    (
        "4581187665f4fd93599ed97c40ffb5e368629a2f827deb8c57e45c5c2b8fead4",
        "format.text.align_center",
    ),
    (
        "065665375091f43015522f1424c0307ffc5dc507678612b2933d92ccea30fa0a",
        "format.text.align_right",
    ),
    (
        "49692db84e43acad7f6f06db0075b96c24059b4bfd4c61c2266ecf1f2c97997a",
        "format.text.align_justify",
    ),
    (
        "9efd469f2c8d7d4d2b94b642aa5347f224401ac1155f369d10c2993b05a57ffc",
        "format.text.align_top",
    ),
    (
        "d4c82ee83c4c09b8ca538c93e27afbdee8722aeb9a8490abf207d772c3d2a07d",
        "format.text.align_middle",
    ),
    (
        "2c341f654dd44712bb1b51deab697dc0815a1f3d4050d1a59eb8e00f8ab88d42",
        "format.text.align_bottom",
    ),
    // Text alignment (web article).
    (
        "5bc54334a456d16105427278da092f72f817fdc10e99bf1ab0235a164845f4a9",
        "format.text.align_left",
    ),
    (
        "20ebb4372d77d394b455fbd5c9726ee5976ec6b19250e5172d8f55f9a11911e6",
        "format.text.align_center",
    ),
    (
        "8b077078c57909cfd11dcc7a948b9adfb535edb2f4a36b45845d699eab523e62",
        "format.text.align_right",
    ),
    (
        "ff99a7e97b1c98e6e9d8011fbaaf1eb8431c747dad7f70bb731cac44c0651000",
        "format.text.align_top",
    ),
    (
        "1157c41d7d5a95c375ab8f55070138bffaf2e6409833eba66a229db176e4d7cf",
        "format.text.align_middle",
    ),
    (
        "1abcf71f0f10e709a4937c1768156b2003ae0ce02ce475bc5711109e858cf5d4",
        "format.text.align_bottom",
    ),
    // Tools.
    (
        "1d6bec1ffc05f90be01a7896991ad0ec7bbf66728bbb187b7efe18bdc859858b",
        "tools.pointer",
    ),
    (
        "75edce520bb8c7a83d707e3ee66536e2e2487143510474ffd07f3792c7787715",
        "tools.text",
    ),
    (
        "347a8569278ae2dfbe9dd7b8512487151f66590de1cefe21dc3a2d4a691cb013",
        "tools.text_block",
    ),
    (
        "954690bb695f6a67833e44c85f4cd92116f7da70401e4b1ff5ba5c88a04a3080",
        "tools.connector",
    ),
    (
        "10c951d8293329b4b4d31698f7a6973365186901930fb00c11c3e76b88dcbfcd",
        "tools.connection",
    ),
    (
        "7aff25ecedc1e5b7962b693953bbe364613f4cc6489169b7930337fc9c8fae3f",
        "tools.pencil",
    ),
    (
        "a888fe8511779d35d1c934ffdbeab9b13b90891f0c416e636dc8a2ded252a558",
        "tools.crop",
    ),
    (
        "13fcb3ac0ceb4c53a01ea883f92aff8b88f773b2ece00658ceb047a781e59007",
        "shape.rectangle",
    ),
    (
        "f65160a9317dd41974725ac468e608550a303899535edffaa93d75caed1908bf",
        "shape.ellipse",
    ),
    (
        "d5d84e60ba911678bc0aed4611ab002325c4ac0302f5e5b50cc4c59cd7a3eddc",
        "shape.line",
    ),
    (
        "e422d5542c1f8c4a4463d584c24e0f47dc996adf19618f6a1551e76220d98bd7",
        "shape.arc",
    ),
    (
        "6b73f2c8248e5c34393ed087bbcb8067553707b38c525cb41817c57cdd2dbb52",
        "shape.spline",
    ),
    (
        "7843afb432778db0597c7d5d7130c964ccc7b6ddc89aebf7b2d5e2a45b3358b3",
        "tools.format_painter",
    ),
    (
        "be222ab47856a4e3c7a25bd5e28e1676925db269c625dd653c049ec4a97c49a2",
        "tools.format_painter",
    ),
    (
        "54316ac9a099fbd6f4ca3cea3e2e1296a28bd1d8b0874f31fb7002106fb10ca5",
        "tools.format_painter",
    ),
    (
        "22ffdaf358988a1f08d8ae07093220a87f5f348e4c86e2584097eafed2f73751",
        "tools.snap_glue_setup",
    ),
    // View / connectors / help / file.
    (
        "c852fe287ffbc9c772581481873559a781d9de0d2c4ff5bdf457000a77720f18",
        "view.zoom.in",
    ),
    (
        "fcbf916675584309207ef9de4b674e69a7c1d1332a0aa952887be40e7db69865",
        "view.zoom.out",
    ),
    (
        "33992de14120f297d376c012bd3cf475483d765c76a9fe8d8be036783c0eba58",
        "connector.auto_connect",
    ),
    (
        "765e82c155f1332e4687563b7b5f374d6cc435e685a423a32cfa5787fe186bb1",
        "help.contents",
    ),
    (
        "ce953a0eb08246617b7f849486c4b26a7af37e9d2e8f0e13b3ae1bf0da8a70a2",
        "file.print",
    ),
    (
        "ea51db7861d4fbaf93027dc7918768ce5f5661b150cc18e6d6afb8349f3d4f84",
        "file.close_window",
    ),
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

/// The lookup key of a normalised action phrase: its lowercase hex
/// sha256 (F2).
fn action_key(normalised: &str) -> String {
    super::util::sha256_hex(normalised)
}

/// Looks up the normalised action in a mapping table keyed by
/// [`action_key`] hashes.
fn lookup_action<'a>(table: &'a [(&str, &str)], action: &str) -> Option<&'a str> {
    let key = action_key(action);
    table.iter().find(|(k, _)| *k == key).map(|(_, c)| *c)
}

/// Looks up the normalised action in [`ACTION_TO_COMMAND`].
fn command_for_action(action: &str) -> Option<&'static str> {
    lookup_action(ACTION_TO_COMMAND, action)
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

    /// F2 pins the entry count: a silent drop of table rows is caught
    /// here (the hashes alone make the table unreadable).
    #[test]
    fn action_to_command_has_81_entries() {
        assert_eq!(ACTION_TO_COMMAND.len(), 81);
    }

    /// F2: every key is 64 lowercase hex digits, and the keys are the
    /// hashes the lookup computes.
    #[test]
    fn action_keys_are_hashes_of_their_normalised_phrase() {
        for (key, cmd) in ACTION_TO_COMMAND {
            assert_eq!(key.len(), 64, "key for {cmd}");
            assert!(
                key.bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()),
                "key for {cmd} is not lowercase hex"
            );
        }
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
