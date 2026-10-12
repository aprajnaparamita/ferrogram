//! Structural diff between `panel-2016.yaml` and `panel-365.yaml` (P1-03).
//!
//! Reads the two layout files written by [`super::fluent`] and
//! produces `ui-spec/diff-2016-vs-365.md`. The diff is a markdown
//! table of Fluent control ids in only one, only the other, or both,
//! with control type and ribbon path.
//!
//! See ADR 0009 for the extractor contract.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::util::write_atomic;
use super::{ExtractError, ExtractResult};

/// Per-extractor entry point.
pub fn run(in_dir: &Path, out_dir: &Path) -> ExtractResult<Vec<PathBuf>> {
    let _ = in_dir;
    let path_2016 = out_dir.join("ui-spec").join("panel-2016.yaml");
    let path_365 = out_dir.join("ui-spec").join("panel-365.yaml");
    let out_path = out_dir.join("ui-spec").join("diff-2016-vs-365.md");

    let entries_2016 = collect_entries(&path_2016)?;
    let entries_365 = collect_entries(&path_365)?;

    let mut all_ids: Vec<String> = entries_2016
        .keys()
        .chain(entries_365.keys())
        .cloned()
        .collect();
    all_ids.sort();
    all_ids.dedup();

    // One row per id.
    struct Row {
        id: String,
        control_type: String,
        in_2016: bool,
        in_365: bool,
        path_2016: String,
        path_365: String,
    }
    let mut rows: Vec<Row> = Vec::new();
    for id in &all_ids {
        let mut control_type = String::new();
        let mut path_2016 = String::new();
        let mut path_365 = String::new();
        let in_2016 = entries_2016.contains_key(id);
        let in_365 = entries_365.contains_key(id);
        if let Some(e) = entries_2016.get(id) {
            control_type = e.control_type.clone();
            path_2016 = e.path.clone();
        }
        if let Some(e) = entries_365.get(id) {
            // Prefer the 365 control type when both exist; if a
            // row only exists in 2016, the 2016 type is what we
            // have.
            if control_type.is_empty() {
                control_type = e.control_type.clone();
            }
            path_365 = e.path.clone();
        }
        rows.push(Row {
            id: id.clone(),
            control_type,
            in_2016,
            in_365,
            path_2016,
            path_365,
        });
    }
    rows.sort_by(|a, b| {
        b.in_2016
            .cmp(&a.in_2016)
            .then(b.in_365.cmp(&a.in_365))
            .then(a.id.cmp(&b.id))
    });

    // Counts.
    let in_both = rows.iter().filter(|r| r.in_2016 && r.in_365).count();
    let only_2016 = rows.iter().filter(|r| r.in_2016 && !r.in_365).count();
    let only_365 = rows.iter().filter(|r| !r.in_2016 && r.in_365).count();
    let total_2016: usize = entries_2016.len();
    let total_365: usize = entries_365.len();

    let mut text = String::new();
    text.push_str("# Ribbon diff: Office 2016 vs Microsoft 365\n\n");
    text.push_str(
        "Keyed by Fluent control id (ADR 0007 B1). Rows are sorted by \
         (in 2016, in 365, id) so identical sets are obvious.\n\n",
    );
    text.push_str("| id | type | in 2016 | in 365 | path in 2016 | path in 365 |\n");
    text.push_str("|----|------|---------|--------|--------------|-------------|\n");
    for r in &rows {
        text.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            md_escape(&r.id),
            md_escape(&r.control_type),
            check(r.in_2016),
            check(r.in_365),
            md_escape(&r.path_2016),
            md_escape(&r.path_365),
        ));
    }

    text.push_str("\n## Summary\n\n");
    text.push_str(&format!(
        "- Total control count in 2016: **{total_2016}**\n\
         - Total control count in 365: **{total_365}**\n\
         - In both: **{in_both}**\n\
         - Only in 2016: **{only_2016}**\n\
         - Only in 365: **{only_365}**\n",
    ));

    write_atomic(&out_path, &text).map_err(|msg| ExtractError::Io {
        path: out_path.clone(),
        source: msg,
    })?;
    Ok(vec![out_path])
}

/// One Fluent control id with its type and ribbon path.
#[derive(Debug, Clone)]
pub struct Entry {
    /// Control type (e.g. `button`, `splitButton`).
    pub control_type: String,
    /// Ribbon path (`tab/<tab>/<group>/<control>`,
    /// `backstage/<tab>/<control>`, `menu/<menu>/<control>`, or
    /// `qat/<control>`).
    pub path: String,
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
    visit_layout(&value, &mut out);
    Ok(out)
}

/// Visits every control id in a layout `Value` and records its
/// type and ribbon path. Used by both the diff and parity
/// extractors; public so [`super::parity`] can call it.
pub fn visit_layout(value: &Value, out: &mut BTreeMap<String, Entry>) {
    visit(value, "", out);
}

fn visit(value: &Value, path: &str, out: &mut BTreeMap<String, Entry>) {
    // Backstage
    if let Some(backstage) = value.get("backstage")
        && let Some(tabs) = backstage.get("tabs").and_then(|v| v.as_array())
    {
        for tab in tabs {
            let tab_id = tab.get("id").and_then(Value::as_str).unwrap_or("");
            let tab_path = if path.is_empty() {
                format!("backstage/{tab_id}")
            } else {
                format!("{path}/backstage/{tab_id}")
            };
            // Record the backstage tab itself.
            if !tab_id.is_empty() {
                let control_type = tab
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("tab")
                    .to_string();
                out.insert(
                    tab_id.to_string(),
                    Entry {
                        control_type,
                        path: tab_path.clone(),
                    },
                );
            }
            if let Some(controls) = tab.get("controls").and_then(|v| v.as_array()) {
                for c in controls {
                    visit_control(c, &tab_path, out);
                }
            }
        }
    }
    // Ribbon tabs
    if let Some(tabs) = value.get("tabs").and_then(|v| v.as_array()) {
        for tab in tabs {
            let tab_id = tab.get("id").and_then(Value::as_str).unwrap_or("");
            let tab_path = if path.is_empty() {
                format!("tab/{tab_id}")
            } else {
                format!("{path}/tab/{tab_id}")
            };
            // Record the tab itself.
            if !tab_id.is_empty() {
                let control_type = tab
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("tab")
                    .to_string();
                out.insert(
                    tab_id.to_string(),
                    Entry {
                        control_type,
                        path: tab_path.clone(),
                    },
                );
            }
            if let Some(groups) = tab.get("groups").and_then(|v| v.as_array()) {
                for g in groups {
                    let g_id = g.get("id").and_then(Value::as_str).unwrap_or("");
                    let g_path = format!("{tab_path}/{g_id}");
                    // Record the group itself.
                    if !g_id.is_empty() {
                        let control_type = g
                            .get("type")
                            .and_then(Value::as_str)
                            .unwrap_or("group")
                            .to_string();
                        out.insert(
                            g_id.to_string(),
                            Entry {
                                control_type,
                                path: g_path.clone(),
                            },
                        );
                    }
                    if let Some(controls) = g.get("controls").and_then(|v| v.as_array()) {
                        for c in controls {
                            visit_control(c, &g_path, out);
                        }
                    }
                }
            }
        }
    }
    // Context menus
    if let Some(menus) = value.get("context_menus").and_then(|v| v.as_array()) {
        for m in menus {
            let m_id = m.get("id").and_then(Value::as_str).unwrap_or("");
            let m_path = if path.is_empty() {
                format!("menu/{m_id}")
            } else {
                format!("{path}/menu/{m_id}")
            };
            if !m_id.is_empty() {
                let control_type = m
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("contextMenu")
                    .to_string();
                out.insert(
                    m_id.to_string(),
                    Entry {
                        control_type,
                        path: m_path.clone(),
                    },
                );
            }
            if let Some(controls) = m.get("controls").and_then(|v| v.as_array()) {
                for c in controls {
                    visit_control(c, &m_path, out);
                }
            }
        }
    }
    // QAT
    if let Some(qat) = value.get("quick_access_toolbar")
        && let Some(controls) = qat.get("controls").and_then(|v| v.as_array())
    {
        for c in controls {
            visit_control(c, "qat", out);
        }
    }
}

fn visit_control(c: &Value, path: &str, out: &mut BTreeMap<String, Entry>) {
    let id = match c.get("id").and_then(Value::as_str) {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => return,
    };
    let control_type = c
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    // For "structural" rows (group, tab) we still record the entry;
    // they appear in the diff and the parity matrix.
    out.insert(
        id,
        Entry {
            control_type,
            path: path.to_string(),
        },
    );
}

fn check(b: bool) -> &'static str {
    if b { "yes" } else { "no" }
}

fn md_escape(s: &str) -> String {
    // Markdown table cells can't contain a literal pipe. Escape
    // it as `\|`. Also strip control characters.
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
    fn md_escape_pipe_and_newlines() {
        assert_eq!(md_escape("a|b"), r"a\|b");
        assert_eq!(md_escape("a\nb"), "a b");
        assert_eq!(md_escape("a\rb"), "a b");
        assert_eq!(md_escape("a\x01b"), "ab");
    }

    #[test]
    fn builds_diff_from_real_layouts() {
        let p2016 = ws().join("ui-spec").join("panel-2016.yaml");
        let p365 = ws().join("ui-spec").join("panel-365.yaml");
        if !p2016.is_file() || !p365.is_file() {
            eprintln!("layout files missing; skipping diff test");
            return;
        }
        let a = collect_entries(&p2016).unwrap();
        let b = collect_entries(&p365).unwrap();
        // TabHome must be in both.
        assert!(a.contains_key("TabHome"));
        assert!(b.contains_key("TabHome"));
        // FileSave is in QAT in both.
        assert!(a.contains_key("FileSave"));
        assert!(b.contains_key("FileSave"));
    }

    #[test]
    fn written_diff_is_byte_stable() {
        let p2016 = ws().join("ui-spec").join("panel-2016.yaml");
        let p365 = ws().join("ui-spec").join("panel-365.yaml");
        if !p2016.is_file() || !p365.is_file() {
            eprintln!("layout files missing; skipping diff test");
            return;
        }
        // Run the extractor pointing at the workspace as `out_dir`,
        // so the input layout files exist. Capture the produced
        // diff into a tmp dir, then re-run.
        let tmp = std::env::temp_dir().join("ferrogram-diff-test");
        std::fs::create_dir_all(&tmp).unwrap();
        let _ = std::fs::remove_file(tmp.join("ui-spec").join("diff-2016-vs-365.md"));
        let out_a = run(Path::new("."), &ws()).unwrap();
        let first = out_a
            .iter()
            .find(|p| p.ends_with("diff-2016-vs-365.md"))
            .expect("diff output path")
            .clone();
        let tmp_diff = tmp.join("diff-2016-vs-365.md");
        std::fs::copy(&first, &tmp_diff).unwrap();
        // Re-run: must produce the same byte content.
        let _ = run(Path::new("."), &ws()).unwrap();
        let ta = std::fs::read_to_string(&tmp_diff).unwrap();
        let tb = std::fs::read_to_string(&first).unwrap();
        assert_eq!(ta, tb);
        assert!(ta.contains("TabHome"));
        assert!(ta.contains("Summary"));
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
