//! Parity matrix extractor (P1-03).
//!
//! Reads the two layout files and produces
//! `docs/parity/controls.md`: every Fluent control id in both
//! layouts, with a status of `implemented`, `stub`, or
//! `out-of-scope`.
//!
//! Initial state (Phase 1) is `stub` for every control, with the
//! `ui-spec/out-of-scope.txt` file (one Fluent id per line) marking
//! known exclusions (P1-04 will refine the master agent's list).
//!
//! See ADR 0009 for the extractor contract.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::diff::Entry;
use super::util::write_atomic;
use super::{ExtractError, ExtractResult};

/// Per-extractor entry point.
pub fn run(in_dir: &Path, out_dir: &Path) -> ExtractResult<Vec<PathBuf>> {
    let _ = in_dir;
    let path_2016 = out_dir.join("ui-spec").join("visio-2016.yaml");
    let path_365 = out_dir.join("ui-spec").join("visio-365.yaml");
    let oos_path = out_dir.join("ui-spec").join("out-of-scope.txt");
    let out_path = out_dir.join("docs").join("parity").join("controls.md");

    let entries_2016 = collect_entries(&path_2016)?;
    let entries_365 = collect_entries(&path_365)?;
    let oos = load_out_of_scope(&oos_path)?;
    let oos = std::sync::Arc::new(oos);

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
        let status = if oos.contains(id) {
            Status::OutOfScope
        } else {
            Status::Stub
        };
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
         In Phase 1 every row is `stub` (no Ferrogram command is \
         registered yet). Implementers in later phases mark a row \
         `implemented` when the corresponding command lands in \
         `crates/commands`. Rows in `ui-spec/out-of-scope.txt` \
         (one Fluent id per line; matched exactly) become \
         `out-of-scope` automatically.\n\n",
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

fn collect_entries(layout_path: &Path) -> ExtractResult<BTreeMap<String, Entry>> {
    let text = std::fs::read_to_string(layout_path).map_err(|e| ExtractError::Io {
        path: layout_path.to_path_buf(),
        source: e.to_string(),
    })?;
    let value: Value = serde_saphyr::from_str::<Value>(&text).map_err(|e| ExtractError::Parse {
        path: layout_path.to_path_buf(),
        message: format!("YAML parse: {e}"),
    })?;
    let mut out = BTreeMap::new();
    super::diff::visit_layout(&value, &mut out);
    Ok(out)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    /// A Ferrogram command has been registered for this id.
    /// Phase 1 has no commands crate, so this variant is never
    /// constructed yet; later phases set it from the commands
    /// registry.
    #[allow(dead_code)]
    Implemented,
    Stub,
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
    Ok(oos)
}

/// Out-of-scope marker. Each entry is either an exact Fluent id or
/// a glob pattern (only `*` is supported). Matched in O(ids *
/// patterns) which is fine for the alpha (under 2 000 ids).
#[derive(Default)]
struct OutOfScope {
    exact: std::collections::BTreeSet<String>,
    globs: Vec<String>,
}

impl OutOfScope {
    fn contains(&self, id: &str) -> bool {
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

    #[test]
    fn loads_out_of_scope_strips_comments() {
        let tmp = std::env::temp_dir().join("ferrogram-parity-oos");
        std::fs::create_dir_all(&tmp).unwrap();
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
        let _ = std::fs::remove_dir_all(&tmp);
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
        let tmp = std::env::temp_dir().join("ferrogram-parity-test");
        std::fs::create_dir_all(&tmp).unwrap();
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
        let _ = std::fs::remove_dir_all(&tmp);
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
