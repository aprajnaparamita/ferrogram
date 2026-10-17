//! Parity matrix extractor (P1-03, statuses derived per ADR 0037).
//!
//! Reads the two layout files and produces
//! `docs/parity/controls.md`: every Fluent control id in both
//! layouts, with a status of `implemented`, `stub`, or
//! `out-of-scope`.
//!
//! The status is **derived, never hand-edited** (ADR 0037):
//!
//! - `out-of-scope` when the id is listed in `ui-spec/out-of-scope.txt`;
//! - `implemented` when the control's `CommandId` (its `command` link,
//!   else its `commands.yaml` entry) resolves in
//!   `ferrogram_commands::builtins::registry()` to
//!   `CommandStatus::Implemented`;
//! - `stub` otherwise.
//!
//! An id listed in `ui-spec/out-of-scope.txt` keeps its `out-of-scope`
//! status even when a command resolves: the alpha does not implement
//! that control (the pre-Phase 4 rule, kept so the extractor fixtures
//! and the committed matrix agree).
//!
//! `cargo xtask parity` (P4-09, `xtask/src/parity.rs`) is the runner
//! that regenerates the committed file or checks it for freshness.
//!
//! See ADR 0009 for the extractor contract.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::diff::Entry;
use super::util::write_atomic;
use super::{ExtractError, ExtractResult};

/// Per-extractor entry point.
pub fn run(in_dir: &Path, out_dir: &Path) -> ExtractResult<Vec<PathBuf>> {
    let _ = in_dir;
    let text = generate(out_dir)?;
    let out_path = out_dir.join("docs").join("parity").join("controls.md");
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| ExtractError::Io {
            path: out_path.clone(),
            source: format!("create parent {}: {e}", parent.display()),
        })?;
    }
    write_atomic(&out_path, &text).map_err(|msg| ExtractError::Io {
        path: out_path.clone(),
        source: msg,
    })?;
    Ok(vec![out_path])
}

/// Generates the parity matrix document from the `ui-spec/` data in
/// `out_dir` and the builtin command registry (ADR 0037).
///
/// The document is deterministic: the same inputs always produce the
/// same bytes (ADR 0009).
pub fn generate(out_dir: &Path) -> ExtractResult<String> {
    let rules = StatusRules::load(out_dir)?;
    let path_2016 = out_dir.join("ui-spec").join("visio-2016.yaml");
    let path_365 = out_dir.join("ui-spec").join("visio-365.yaml");

    let value_2016 = load_value(&path_2016)?;
    let value_365 = load_value(&path_365)?;
    let mut entries_2016: BTreeMap<String, Entry> = BTreeMap::new();
    super::diff::visit_layout(&value_2016, &mut entries_2016);
    let mut entries_365: BTreeMap<String, Entry> = BTreeMap::new();
    super::diff::visit_layout(&value_365, &mut entries_365);
    let mut links_2016 = BTreeMap::new();
    collect_command_links(&value_2016, &mut links_2016);
    let mut links_365 = BTreeMap::new();
    collect_command_links(&value_365, &mut links_365);

    // Union of ids, in (status, id) order. Status order is
    // `implemented`, `stub`, `out-of-scope` per the task.
    struct Row {
        id: String,
        control_type: String,
        label: String,
        path: String,
        status: Status,
    }
    let mut rows: Vec<Row> = Vec::new();
    let mut all_ids: Vec<String> = entries_2016
        .keys()
        .chain(entries_365.keys())
        .cloned()
        .collect();
    all_ids.sort();
    all_ids.dedup();
    for id in &all_ids {
        let entry = entries_365.get(id).or_else(|| entries_2016.get(id));
        let (control_type, path) = match entry {
            Some(e) => (e.control_type.clone(), e.path.clone()),
            // all_ids is the union; either 2016 or 365 must
            // contain it. If neither does, the upstream data is
            // inconsistent; report it as out of scope so the
            // parity row is still emitted.
            None => (String::new(), String::new()),
        };
        // The control's `command` link, 365 first (the row data
        // prefers the 365 placement the same way).
        let linked = links_365
            .get(id)
            .or_else(|| links_2016.get(id))
            .map(String::as_str);
        let status = rules.status(id, linked);
        rows.push(Row {
            id: id.clone(),
            control_type,
            label: id.clone(),
            path,
            status,
        });
    }
    rows.sort_by(|a, b| {
        a.status
            .order()
            .cmp(&b.status.order())
            .then(a.id.cmp(&b.id))
    });

    let mut text = String::new();
    text.push_str("# Parity matrix: ui-spec controls\n\n");
    text.push_str(
        "Every Fluent control id in `ui-spec/visio-2016.yaml` and \
         `ui-spec/visio-365.yaml`, with a status of `implemented`, \
         `stub`, or `out-of-scope`. Sorted by `(status, id)`; the \
         status order is `implemented`, `stub`, `out-of-scope`.\n\n\
         The status is derived, never hand-edited (ADR 0037):\n\n\
         - `out-of-scope` when the id is listed in \
         `ui-spec/out-of-scope.txt` (one Fluent id per line; matched \
         exactly or by a `*` glob);\n\
         - `implemented` when the control's command (its `command` \
         link, else its `commands.yaml` entry) resolves in \
         `ferrogram_commands::builtins::registry()` to \
         `CommandStatus::Implemented`;\n\
         - `stub` otherwise.\n\n\
         A row listed in `ui-spec/out-of-scope.txt` stays \
         `out-of-scope` even when a command resolves: the alpha does \
         not implement that control. Regenerate with \
         `cargo xtask parity`; the gate's `parity` step runs \
         `cargo xtask parity --check` and fails when this file is \
         stale.\n\n",
    );
    text.push_str("| id | type | label | path | status |\n");
    text.push_str("|----|------|-------|------|--------|\n");
    for r in &rows {
        text.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            md_escape(&r.id),
            md_escape(&r.control_type),
            md_escape(&r.label),
            md_escape(&r.path),
            r.status.as_str(),
        ));
    }
    Ok(text)
}

/// The rules that derive one row's status (ADR 0037).
pub struct StatusRules {
    /// Command ids the builtin registry reports as
    /// [`CommandStatus::Implemented`](ferrogram_commands::CommandStatus).
    pub implemented: BTreeSet<String>,
    /// Fluent id -> command id, from `commands.yaml` (the entry that
    /// lists the id in `fluent_ids`; first entry wins).
    pub by_fluent: BTreeMap<String, String>,
    /// The `ui-spec/out-of-scope.txt` marker list.
    pub out_of_scope: OutOfScope,
}

impl StatusRules {
    /// Loads the rules from `out_dir`'s `ui-spec/` and the builtin
    /// registry.
    pub fn load(out_dir: &Path) -> ExtractResult<Self> {
        let ui_spec = out_dir.join("ui-spec");
        Ok(Self {
            implemented: implemented_ids()?,
            by_fluent: load_command_table(&ui_spec.join("commands.yaml"))?,
            out_of_scope: load_out_of_scope(&ui_spec.join("out-of-scope.txt"))?,
        })
    }

    /// One row's status.
    ///
    /// `linked` is the control's `command` link; when there is none,
    /// the id's `commands.yaml` entry decides. An id in
    /// `ui-spec/out-of-scope.txt` is `out-of-scope` first (see the
    /// module docs).
    pub fn status(&self, fluent_id: &str, linked: Option<&str>) -> Status {
        if self.out_of_scope.contains(fluent_id) {
            return Status::OutOfScope;
        }
        let command = linked
            .map(str::to_owned)
            .or_else(|| self.by_fluent.get(fluent_id).cloned());
        match command {
            Some(command) if self.implemented.contains(&command) => Status::Implemented,
            _ => Status::Stub,
        }
    }
}

/// The command ids `ferrogram_commands::builtins::registry()` reports
/// as implemented: the one source of truth for `implemented` rows
/// (ADR 0037).
fn implemented_ids() -> ExtractResult<BTreeSet<String>> {
    let registry =
        ferrogram_commands::builtins::registry().map_err(|error| ExtractError::Parse {
            path: PathBuf::from("crates/commands"),
            message: format!("builtin registry does not build: {error}"),
        })?;
    Ok(registry
        .ids()
        .filter(|id| registry.status(id) == ferrogram_commands::CommandStatus::Implemented)
        .map(|id| id.as_str().to_owned())
        .collect())
}

/// Parses one layout YAML file into a generic [`Value`].
fn load_value(path: &Path) -> ExtractResult<Value> {
    let text = std::fs::read_to_string(path).map_err(|e| ExtractError::Io {
        path: path.to_path_buf(),
        source: e.to_string(),
    })?;
    serde_saphyr::from_str::<Value>(&text).map_err(|e| ExtractError::Parse {
        path: path.to_path_buf(),
        message: format!("YAML parse: {e}"),
    })
}

/// Walks a layout [`Value`] and records every control's `command`
/// link: Fluent id -> Ferrogram command id. First occurrence wins
/// (a repeated id with two links is an extractor artefact).
pub fn collect_command_links(value: &Value, out: &mut BTreeMap<String, String>) {
    match value {
        Value::Array(items) => {
            for item in items {
                collect_command_links(item, out);
            }
        }
        Value::Object(map) => {
            if let (Some(Value::String(id)), Some(Value::String(command))) =
                (map.get("id"), map.get("command"))
                && !id.is_empty()
                && !command.is_empty()
            {
                out.entry(id.clone()).or_insert_with(|| command.clone());
            }
            for child in map.values() {
                collect_command_links(child, out);
            }
        }
        _ => {}
    }
}

/// Builds the Fluent id -> command id reverse map of a
/// `commands.yaml` table (every `fluent_ids` entry of every row; the
/// first row claiming an id wins).
fn load_command_table(path: &Path) -> ExtractResult<BTreeMap<String, String>> {
    let value = load_value(path)?;
    let items = value.as_array().ok_or_else(|| ExtractError::Parse {
        path: path.to_path_buf(),
        message: "commands.yaml is not a list of entries".to_owned(),
    })?;
    let mut out = BTreeMap::new();
    for item in items {
        let Some(id) = item.get("id").and_then(Value::as_str) else {
            continue;
        };
        if let Some(fluent_ids) = item.get("fluent_ids").and_then(Value::as_array) {
            for fluent in fluent_ids.iter().filter_map(Value::as_str) {
                out.entry(fluent.to_owned())
                    .or_insert_with(|| id.to_owned());
            }
        }
    }
    Ok(out)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// The control's command resolves to a real handler in
    /// `ferrogram_commands::builtins::registry()`.
    Implemented,
    /// No handler is registered for the control's command yet.
    Stub,
    /// The Fluent id is listed in `ui-spec/out-of-scope.txt`.
    OutOfScope,
}

impl Status {
    fn order(self) -> u8 {
        match self {
            Self::Implemented => 0,
            Self::Stub => 1,
            Self::OutOfScope => 2,
        }
    }
    fn as_str(self) -> &'static str {
        match self {
            Self::Implemented => "implemented",
            Self::Stub => "stub",
            Self::OutOfScope => "out-of-scope",
        }
    }
}

fn load_out_of_scope(path: &Path) -> ExtractResult<OutOfScope> {
    if !path.is_file() {
        // No file: nothing is out of scope.
        return Ok(OutOfScope::default());
    }
    let text = std::fs::read_to_string(path).map_err(|e| ExtractError::Io {
        path: path.to_path_buf(),
        source: e.to_string(),
    })?;
    Ok(parse_out_of_scope(&text))
}

/// Parses an `out-of-scope.txt` document: one Fluent id per line,
/// `#` comments and blank lines ignored, `*` globs allowed.
fn parse_out_of_scope(text: &str) -> OutOfScope {
    let mut oos = OutOfScope::default();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.contains('*') {
            oos.globs.push(glob_to_pattern(trimmed));
        } else {
            oos.exact.insert(trimmed.to_string());
        }
    }
    oos
}

/// Out-of-scope marker. Each entry is either an exact Fluent id or
/// a glob pattern (only `*` is supported). Matched in O(ids *
/// patterns) which is fine for the alpha (under 2 000 ids).
#[derive(Default)]
pub struct OutOfScope {
    exact: BTreeSet<String>,
    globs: Vec<String>,
}

impl OutOfScope {
    /// Whether `id` is marked out of scope.
    pub fn contains(&self, id: &str) -> bool {
        if self.exact.contains(id) {
            return true;
        }
        for g in &self.globs {
            if glob_match(g, id) {
                return true;
            }
        }
        false
    }
}

/// Converts a shell-style glob (`*` only) into a regex anchored at
/// both ends. The escape rules are minimal: `*` matches any run of
/// characters, everything else is literal.
fn glob_to_pattern(glob: &str) -> String {
    let mut out = String::with_capacity(glob.len() + 4);
    out.push('^');
    for ch in glob.chars() {
        match ch {
            // `(?s)` so `*` really matches any string, newlines
            // included (P1-08 goal 4).
            '*' => out.push_str("(?s:.*)"),
            // Regex meta-characters we want literal.
            '.' | '(' | ')' | '[' | ']' | '?' | '+' | '|' | '^' | '$' | '\\' | '{' | '}' => {
                out.push('\\');
                out.push(ch);
            }
            c => out.push(c),
        }
    }
    out.push('$');
    out
}

fn glob_match(pattern: &str, s: &str) -> bool {
    // Compiled per call. Fine for the alpha's data sizes.
    let re = match regex_lite::Regex::new(pattern) {
        Ok(r) => r,
        Err(_) => return false,
    };
    re.is_match(s)
}

fn md_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '|' => out.push_str(r"\|"),
            '\n' => out.push(' '),
            '\r' => out.push(' '),
            c if (c as u32) < 0x20 => {}
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn ws() -> PathBuf {
        crate::util::root()
    }

    /// Rules over synthetic data: the implemented set and the
    /// `commands.yaml` map are what the tests flip.
    fn rules(implemented: &[&str], by_fluent: &[(&str, &str)], oos: &[&str]) -> StatusRules {
        StatusRules {
            implemented: implemented.iter().map(|s| (*s).to_owned()).collect(),
            by_fluent: by_fluent
                .iter()
                .map(|(f, c)| ((*f).to_owned(), (*c).to_owned()))
                .collect(),
            out_of_scope: parse_out_of_scope(&oos.join("\n")),
        }
    }

    #[test]
    fn loads_out_of_scope_strips_comments() {
        let tmp = crate::util::test_support::TempDir::new("parity-oos");
        let p = tmp.join("oos.txt");
        std::fs::write(
            &p,
            "# coauthoring\nCoauthorMenu\n\n*Macro*\n# trailing comment\n",
        )
        .unwrap();
        let oos = load_out_of_scope(&p).unwrap();
        assert!(oos.contains("CoauthorMenu"));
        assert!(oos.contains("MacroMenu"));
        assert!(oos.contains("MacrosMenu"));
        assert_eq!(oos.exact.len(), 1);
        assert_eq!(oos.globs.len(), 1);
    }

    #[test]
    fn out_of_scope_when_file_missing_is_empty() {
        let p = PathBuf::from("/nonexistent/never/there/oos.txt");
        let oos = load_out_of_scope(&p).unwrap();
        assert!(oos.exact.is_empty());
        assert!(oos.globs.is_empty());
    }

    #[test]
    fn builds_parity_from_real_layouts() {
        let p2016 = ws().join("ui-spec").join("visio-2016.yaml");
        let p365 = ws().join("ui-spec").join("visio-365.yaml");
        if !p2016.is_file() || !p365.is_file() {
            eprintln!("layout files missing; skipping parity test");
            return;
        }
        // Run the extractor pointing at the workspace as `out_dir`
        // so the input layout files exist. Re-run, and compare the
        // produced file content to a captured copy in a tmp dir.
        let tmp = crate::util::test_support::TempDir::new("parity");
        let _ = std::fs::remove_file(ws().join("docs").join("parity").join("controls.md"));
        let out_a = run(Path::new("."), &ws()).unwrap();
        let first = out_a[0].clone();
        let tmp_parity = tmp.join("controls.md");
        std::fs::copy(&first, &tmp_parity).unwrap();
        let _ = run(Path::new("."), &ws()).unwrap();
        let ta = std::fs::read_to_string(&tmp_parity).unwrap();
        let tb = std::fs::read_to_string(&first).unwrap();
        assert_eq!(ta, tb);
        assert!(ta.contains("| status |"));
        assert!(ta.contains("| stub |"));
    }

    // ---- status derivation (ADR 0037) ------------------------------

    /// The row's command link decides when there is one.
    #[test]
    fn the_command_link_decides_the_status() {
        let r = rules(&["file.save"], &[], &[]);
        assert_eq!(r.status("FileSave", Some("file.save")), Status::Implemented);
        assert_eq!(r.status("FileSave", Some("file.print")), Status::Stub);
    }

    /// With no link, the id's `commands.yaml` entry decides.
    #[test]
    fn the_commands_yaml_entry_is_the_fallback() {
        let r = rules(&["file.save"], &[("FileSave", "file.save")], &[]);
        assert_eq!(r.status("FileSave", None), Status::Implemented);
        assert_eq!(r.status("UnknownControl", None), Status::Stub);
    }

    /// The acceptance rule: flipping one handler to stub flips its
    /// rows (nothing else moves).
    #[test]
    fn flipping_one_handler_to_stub_flips_its_rows() {
        let before = rules(
            &["file.save", "file.open"],
            &[("FileSave", "file.save")],
            &[],
        );
        assert_eq!(
            before.status("FileSave", Some("file.save")),
            Status::Implemented
        );
        assert_eq!(before.status("FileSave", None), Status::Implemented);
        assert_eq!(
            before.status("FileOpen", Some("file.open")),
            Status::Implemented
        );

        // The `file.save` handler becomes a stub: only its rows flip.
        let after = rules(&["file.open"], &[("FileSave", "file.save")], &[]);
        assert_eq!(after.status("FileSave", Some("file.save")), Status::Stub);
        assert_eq!(after.status("FileSave", None), Status::Stub);
        assert_eq!(
            after.status("FileOpen", Some("file.open")),
            Status::Implemented
        );
    }

    /// An id the alpha will not implement stays `out-of-scope`, even
    /// when a command resolves.
    #[test]
    fn out_of_scope_beats_an_implemented_command() {
        let r = rules(&["file.save"], &[("FileSave", "file.save")], &["FileSave"]);
        assert_eq!(r.status("FileSave", Some("file.save")), Status::OutOfScope);
    }

    /// A control's `command` link wins over its `commands.yaml` entry.
    #[test]
    fn the_link_wins_over_the_table() {
        let r = rules(&["file.save"], &[("FileSave", "file.print")], &[]);
        assert_eq!(r.status("FileSave", Some("file.save")), Status::Implemented);
        assert_eq!(r.status("FileSave", None), Status::Stub);
    }

    #[test]
    fn command_links_are_collected_from_the_layout_value() {
        let text = r#"
tabs:
  - id: TabHome
    groups:
      - id: GroupClipboard
        controls:
          - id: FileSave
            command: file.save
          - id: SaveLabel
            command: null
          - id: FileSave
            command: file.print
"#;
        let value: Value = serde_saphyr::from_str(text).unwrap();
        let mut links = BTreeMap::new();
        collect_command_links(&value, &mut links);
        assert_eq!(links.get("FileSave").map(String::as_str), Some("file.save"));
        assert!(!links.contains_key("SaveLabel"));
    }

    #[test]
    fn command_table_maps_fluent_ids_and_the_first_row_wins() {
        let tmp = crate::util::test_support::TempDir::new("parity-table");
        let p = tmp.join("commands.yaml");
        std::fs::write(
            &p,
            "- id: file.save\n  fluent_ids: [FileSave, Save]\n\
             - id: file.save_as\n  fluent_ids: [FileSave, SaveAs]\n",
        )
        .unwrap();
        let table = load_command_table(&p).unwrap();
        assert_eq!(table.get("FileSave").map(String::as_str), Some("file.save"));
        assert_eq!(
            table.get("SaveAs").map(String::as_str),
            Some("file.save_as")
        );
        assert_eq!(table.len(), 3);
    }

    #[test]
    fn a_missing_or_broken_command_table_is_an_error() {
        let p = PathBuf::from("/nonexistent/never/there/commands.yaml");
        assert!(load_command_table(&p).is_err());
        let tmp = crate::util::test_support::TempDir::new("parity-bad-table");
        let p = tmp.join("commands.yaml");
        std::fs::write(&p, "a: 1").unwrap();
        assert!(load_command_table(&p).is_err());
        std::fs::write(&p, "{oops").unwrap();
        assert!(load_command_table(&p).is_err());
    }

    /// The committed matrix marks the rows of the builtin registry
    /// `implemented` and everything else `stub` or `out-of-scope`.
    #[test]
    fn the_generated_matrix_agrees_with_the_builtin_registry() {
        let root = ws();
        let text = generate(&root).unwrap();
        let rules = StatusRules::load(&root).unwrap();
        // The links as the generator merges them (365 first).
        let mut links = BTreeMap::new();
        collect_command_links(
            &load_value(&root.join("ui-spec").join("visio-2016.yaml")).unwrap(),
            &mut links,
        );
        let mut links_365 = BTreeMap::new();
        collect_command_links(
            &load_value(&root.join("ui-spec").join("visio-365.yaml")).unwrap(),
            &mut links_365,
        );
        for (id, command) in &links_365 {
            links.insert(id.clone(), command.clone());
        }
        let mut checked = 0usize;
        for fluent in rules.by_fluent.keys() {
            let Some(row) = text
                .lines()
                .find(|line| line.starts_with(&format!("| {fluent} |")))
            else {
                continue;
            };
            let linked = links.get(fluent).map(String::as_str);
            let status = rules.status(fluent, linked);
            assert!(
                row.ends_with(&format!("| {} |", status.as_str())),
                "{fluent} should be {status:?}: {row}"
            );
            checked += 1;
        }
        assert!(checked > 0, "no mapped row was checked");
    }

    #[test]
    fn star_glob_matches_across_newlines() {
        // Regression (P1-08 goal 4): `*` must match *any* string, so
        // the wildcard's dot has to match newlines too.
        assert!(glob_match(&glob_to_pattern("*"), "a\nb"));
        assert!(glob_match(&glob_to_pattern("*"), ""));
        assert!(glob_match(&glob_to_pattern("*Macro*"), "My\nMacro"));
    }
    // ---- P1-08 goal 4: property tests -------------------------------

    use proptest::prelude::*;

    proptest! {
        /// The out-of-scope glob matcher is total: arbitrary patterns
        /// and ids never panic.
        #[test]
        fn glob_matching_never_panics(glob in ".*", s in ".*") {
            let _ = glob_match(&glob_to_pattern(&glob), &s);
        }

        /// `*` matches any string.
        #[test]
        fn star_matches_any_string(s in ".*") {
            prop_assert!(glob_match(&glob_to_pattern("*"), &s));
        }
    }
}
