//! Tiny markdown helpers for the Phase 1 extractors (P1-05).
//!
//! The Microsoft reference pages under `reference/` are markdown with a
//! small, uniform shape. This module extracts just the two constructs the
//! extractors need, without a full markdown dependency:
//!
//! - [`fenced_blocks`] lifts fenced code blocks out of a page (used to
//!   extract the embedded XSD from `schema-mapvisio-xml.md`).
//! - [`tables`], [`cell_text`] and [`kv_pairs`] read pipe tables and the
//!   `|**Key** <br/> |value <br/> |` metadata blocks of the
//!   `*_type-complextypevisio-xml.md` pages.
//!
//! Every entry point is total: malformed input yields fewer or shorter
//! results and never a panic (AGENTS.md section 6). P1-04 may share this
//! module; the signatures are deliberately generic.
//!
//! Clean-room note (AGENTS.md section 4): [`cell_text`] returns text as it
//! appears in the source. Callers must store only structural fields
//! (names, ids, types, parents) and never prose (ADR 0009).

/// One fenced code block: the info string after the opening fence and the
/// block body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FencedBlock {
    /// Info string of the opening fence (`"XML"` for ```` ```XML ````);
    /// empty when the fence has no info string.
    pub info: String,
    /// Block body: the lines between the fences, joined with `\n`.
    pub body: String,
}

/// One pipe table: the header cells and the body rows.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Table {
    /// Cells of the first row (the column names).
    pub header: Vec<String>,
    /// Body rows, each split into cells. Rows may have fewer cells than
    /// the header; readers must tolerate short rows.
    pub rows: Vec<Vec<String>>,
}

/// Returns every fenced code block in `text`, in document order.
///
/// Supports backtick and tilde fences of three or more characters. An
/// unterminated fence consumes the rest of the input (CommonMark
/// behaviour). Nested fences inside a block are part of the body.
pub fn fenced_blocks(text: &str) -> Vec<FencedBlock> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let Some((marker, run, info)) = opening_fence(lines[i]) else {
            i += 1;
            continue;
        };
        let body_start = i + 1;
        let mut end = lines.len();
        let mut j = body_start;
        while j < lines.len() {
            if is_closing_fence(lines[j], marker, run) {
                end = j;
                break;
            }
            j += 1;
        }
        out.push(FencedBlock {
            info: info.to_string(),
            body: lines[body_start..end].join("\n"),
        });
        i = end + 1;
    }
    out
}

/// Parses an opening fence line. Returns the fence character, the fence
/// length and the info string when the line opens a block.
fn opening_fence(line: &str) -> Option<(char, usize, &str)> {
    let trimmed = line.trim_start();
    let indent = line.len() - trimmed.len();
    if indent > 3 {
        return None;
    }
    let marker = trimmed.chars().next()?;
    if marker != '`' && marker != '~' {
        return None;
    }
    let run = trimmed.chars().take_while(|c| *c == marker).count();
    if run < 3 {
        return None;
    }
    let info = trimmed[run..].trim();
    // CommonMark: backtick fences may not carry backticks in the info.
    if marker == '`' && info.contains('`') {
        return None;
    }
    Some((marker, run, info))
}

/// True when `line` closes a fence opened with `run` copies of `marker`
/// (CommonMark: the closing fence is at least as long as the opening).
fn is_closing_fence(line: &str, marker: char, run: usize) -> bool {
    let trimmed = line.trim_start();
    let indent = line.len() - trimmed.len();
    if indent > 3 {
        return false;
    }
    let closing = trimmed.chars().take_while(|c| *c == marker).count();
    closing >= run && trimmed[closing..].trim().is_empty()
}

/// Returns every pipe table in `text`, in document order.
///
/// A table is a run of consecutive lines that start with `|` whose second
/// row is a delimiter row (`|:---|:---|`). The first row is the header.
pub fn tables(text: &str) -> Vec<Table> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if !is_table_line(lines[i]) {
            i += 1;
            continue;
        }
        let start = i;
        while i < lines.len() && is_table_line(lines[i]) {
            i += 1;
        }
        let block = &lines[start..i];
        if block.len() < 2 || !is_delimiter_row(block[1]) {
            continue;
        }
        out.push(Table {
            header: split_row(block[0]),
            rows: block[2..]
                .iter()
                .filter(|l| !is_delimiter_row(l))
                .map(|l| split_row(l))
                .collect(),
        });
    }
    out
}

/// True when a line could be a table row: starts with `|` after at most
/// three spaces of indent.
fn is_table_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    line.len() - trimmed.len() <= 3 && trimmed.starts_with('|')
}

/// True when a row is a `|:---|:---|` delimiter row.
fn is_delimiter_row(line: &str) -> bool {
    let cells = split_row(line);
    !cells.is_empty()
        && cells.iter().all(|c| {
            let body = c.trim().trim_start_matches(':').trim_end_matches(':');
            !body.is_empty() && body.chars().all(|ch| ch == '-')
        })
}

/// Splits one table row into cells, honouring `\|` escapes and an
/// optional trailing pipe. The trailing pipe is dropped only when it
/// is a real closing pipe: a `|` preceded by an odd number of
/// backslashes is an escaped pipe, not a delimiter.
fn split_row(line: &str) -> Vec<String> {
    let mut body = line.trim();
    if let Some(stripped) = body.strip_prefix('|') {
        body = stripped;
    }
    // Drop a trailing pipe unless it is escaped.
    if body.ends_with('|') {
        let before = &body[..body.len() - 1];
        let backslashes = before.chars().rev().take_while(|c| *c == '\\').count();
        if backslashes % 2 == 0 {
            body = before;
        }
    }
    let mut cells = Vec::new();
    let mut cur = String::new();
    let mut chars = body.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next() {
                Some(next) if next == '|' || next == '\\' => cur.push(next),
                Some(next) => {
                    cur.push('\\');
                    cur.push(next);
                }
                None => cur.push('\\'),
            },
            '|' => cells.push(std::mem::take(&mut cur)),
            other => cur.push(other),
        }
    }
    cells.push(cur);
    cells.into_iter().map(|c| c.trim().to_string()).collect()
}

/// Normalises one raw table cell to plain text: drops HTML tags
/// (`<br/>`) and markdown emphasis/code markers, keeps the text of
/// links (`[text](url)` becomes `text`), and collapses whitespace.
/// Underscores are kept as-is (type names carry them).
pub fn cell_text(raw: &str) -> String {
    let chars: Vec<char> = raw.chars().collect();
    let mut out = String::with_capacity(raw.len());
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '<' => {
                // Skip an HTML tag; keep a stray '<'.
                if let Some(close) = (i + 1..chars.len()).find(|&j| chars[j] == '>') {
                    let inner: String = chars[i + 1..close].iter().collect();
                    let is_tag = inner
                        .chars()
                        .next()
                        .is_some_and(|f| f.is_ascii_alphabetic() || f == '/' || f == '!');
                    if is_tag {
                        i = close + 1;
                        continue;
                    }
                }
                out.push('<');
                i += 1;
            }
            '[' => {
                // Markdown link: [text](url) becomes text.
                if let Some((text, end)) = link_at(&chars, i) {
                    out.push_str(&text);
                    i = end;
                } else {
                    out.push('[');
                    i += 1;
                }
            }
            // Underscores stay: type names carry them. Escapes are
            // resolved so that raw cells keep their literal text.
            '\\' => match chars.get(i + 1) {
                Some(&next) if next == '|' || next == '\\' => {
                    out.push(next);
                    i += 2;
                }
                _ => {
                    out.push('\\');
                    i += 1;
                }
            },
            '*' | '`' => i += 1,
            other => {
                out.push(other);
                i += 1;
            }
        }
    }
    collapse(&out)
}

/// Parses a markdown link starting at the `[` index. Returns the plain
/// text of the link and the index just past its closing `)`.
fn link_at(chars: &[char], start: usize) -> Option<(String, usize)> {
    let mut j = start + 1;
    while j < chars.len() && chars[j] != ']' {
        j += 1;
    }
    if j + 1 >= chars.len() || chars[j] != ']' || chars[j + 1] != '(' {
        return None;
    }
    let mut k = j + 2;
    let mut depth = 1usize;
    while k < chars.len() {
        match chars[k] {
            '(' => depth = depth.saturating_add(1),
            ')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    break;
                }
            }
            _ => {}
        }
        k += 1;
    }
    if k >= chars.len() {
        return None;
    }
    let text: String = chars[start + 1..j].iter().collect();
    Some((cell_text(&text), k + 1))
}

/// Collapses whitespace runs and trims.
fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Collects `(key, value)` pairs from every table in `text`, taking the
/// first two cells of each body row.
///
/// Keys and values are passed through [`cell_text`]. Rows with an empty
/// key are skipped. The pages' metadata blocks (`||Value |` headers)
/// parse as key/value pairs this way.
pub fn kv_pairs(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for table in tables(text) {
        for row in &table.rows {
            let key = row.first().map(|c| cell_text(c)).unwrap_or_default();
            let value = row.get(1).map(|c| cell_text(c)).unwrap_or_default();
            if !key.is_empty() {
                out.push((key, value));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fenced_blocks_simple() {
        let text = "intro\n```XML\n<a>\n<b>\n```\noutro\n";
        let blocks = fenced_blocks(text);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].info, "XML");
        assert_eq!(blocks[0].body, "<a>\n<b>");
    }

    #[test]
    fn fenced_blocks_multiple_and_tilde() {
        let text = "```\none\n```\ntext\n~~~rust\ntwo\n~~~\n";
        let blocks = fenced_blocks(text);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].body, "one");
        assert_eq!(blocks[1].info, "rust");
        assert_eq!(blocks[1].body, "two");
    }

    #[test]
    fn fenced_blocks_unclosed_runs_to_eof() {
        let blocks = fenced_blocks("```\nalpha\nbeta");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].body, "alpha\nbeta");
    }

    #[test]
    fn fenced_blocks_nested_fence_is_body() {
        let blocks = fenced_blocks("````\n```\nx\n```\n````\n");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].body, "```\nx\n```");
    }

    #[test]
    fn fenced_blocks_ignores_indented_and_inline() {
        assert!(fenced_blocks("    ```\nnot a fence\n`code`\n").is_empty());
        assert!(fenced_blocks("`` infoticks ``\n").is_empty());
    }

    #[test]
    fn tables_basic() {
        let text = "\
|**Element**|**Type**|
|:-----|:-----|
|[Row](row.md) <br/> |[ActionsRow_Type](art.md) <br/> |
";
        let parsed = tables(text);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].header, vec!["**Element**", "**Type**"]);
        assert_eq!(parsed[0].rows.len(), 1);
        assert_eq!(parsed[0].rows[0].len(), 2);
    }

    #[test]
    fn tables_skips_runs_without_delimiter() {
        assert!(tables("|only one line\n|a|b|\n|---|\n").is_empty());
        assert!(tables("|lone row\n").is_empty());
        assert!(tables("|a|b|\n").is_empty());
    }

    #[test]
    fn tables_escaped_pipe_and_trailing() {
        let parsed = tables("|a|b|\n|---|---|\n|p \\| q|r\n");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].rows.len(), 1);
        assert_eq!(parsed[0].rows[0], vec!["p | q", "r"]);
    }

    #[test]
    fn tables_trailing_escaped_backslash() {
        // `|a\\|` is one cell `a\` with a closing pipe: the trailing
        // pipe follows two (an even number of) backslashes and is a
        // real delimiter, not an escaped one.
        let parsed = tables("|h|\n|---|\n|a\\\\|\n|b\\\\\\|\n");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].rows[0], vec!["a\\"]);
        // `|b\\\||` is one cell `b\|` (escaped pipe at the end).
        assert_eq!(parsed[0].rows[1], vec!["b\\|"]);
    }

    #[test]
    fn tables_short_row_is_kept() {
        let parsed = tables("|a|b|c|\n|---|---|---|\n|only|\n");
        assert_eq!(parsed[0].rows[0], vec!["only"]);
    }

    #[test]
    fn tables_non_table_line_breaks_a_run() {
        let parsed = tables("|a|\n|---|\n|x|\ntext\n|b|\n|---|\n|y|\n");
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].rows, vec![vec!["x"]]);
        assert_eq!(parsed[1].rows, vec![vec!["y"]]);
    }

    #[test]
    fn cell_text_strips_markup() {
        assert_eq!(cell_text("**Namespace** <br/>"), "Namespace");
        assert_eq!(
            cell_text("[ActionsRow_Type](actionsrow_type-complextypevisio-xml.md) <br/>"),
            "ActionsRow_Type"
        );
        assert_eq!(cell_text("a  b\t c"), "a b c");
        assert_eq!(cell_text("p \\| q"), "p | q");
        assert_eq!(cell_text("`code` and **bold**"), "code and bold");
        assert_eq!(cell_text("ActionsRow_Type"), "ActionsRow_Type");
    }

    #[test]
    fn cell_text_keeps_stray_angle_brackets_and_unlinks() {
        assert_eq!(cell_text("0 < x"), "0 < x");
        assert_eq!(cell_text("[plain text"), "[plain text");
        assert_eq!(cell_text("[a](b(c))"), "a");
    }

    #[test]
    fn kv_pairs_from_metadata_block() {
        let text = "\
||Value |
|:-----|:-----|
|**Extension base** <br/> |[Section_Type](section_type-complextypevisio-xml.md) <br/> |
|**Namespace** <br/> |http://schemas.microsoft.com/office/visio/2011/1/core  <br/> |
";
        let pairs = kv_pairs(text);
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0].0, "Extension base");
        assert_eq!(pairs[0].1, "Section_Type");
        assert_eq!(pairs[1].0, "Namespace");
    }

    #[test]
    fn empty_input_is_total() {
        assert!(fenced_blocks("").is_empty());
        assert!(tables("").is_empty());
        assert!(kv_pairs("").is_empty());
        assert_eq!(cell_text(""), "");
    }
    // ---- P1-08 goal 4: property tests -------------------------------

    use proptest::prelude::*;

    /// Cells whose text survives render + parse unchanged: no line
    /// breaks, no leading/trailing whitespace, and never a
    /// delimiter-row look-alike (the `|---|` alphabet is excluded).
    fn cell_strategy() -> impl Strategy<Value = String> {
        "[A-Za-z0-9 _.,;=+*()|\\\\]{0,12}".prop_map(|s| s.trim().to_string())
    }

    /// Renders one table row with `\|` and `\\` escapes and a closing
    /// pipe.
    fn render_row(cells: &[String]) -> String {
        let mut out = String::from("|");
        for cell in cells {
            for ch in cell.chars() {
                match ch {
                    '\\' => out.push_str("\\\\"),
                    '|' => out.push_str("\\|"),
                    other => out.push(other),
                }
            }
            out.push('|');
        }
        out
    }

    /// Renders a whole pipe table: header row, delimiter row, body
    /// rows.
    fn render_table(rows: &[Vec<String>]) -> String {
        let cols = rows.first().map(Vec::len).unwrap_or(0);
        let mut out = String::new();
        out.push_str(&render_row(&rows[0]));
        out.push('\n');
        out.push_str(&"|---".repeat(cols));
        out.push_str("|\n");
        for row in &rows[1..] {
            out.push_str(&render_row(row));
            out.push('\n');
        }
        out
    }

    proptest! {
        /// Every markdown entry point is total: arbitrary input
        /// yields fewer or shorter results, never a panic.
        #[test]
        fn markdown_parsers_never_panic(text in ".*") {
            let _ = fenced_blocks(&text);
            let _ = tables(&text);
            let _ = cell_text(&text);
            let _ = kv_pairs(&text);
        }

        /// A table that is rendered and parsed again keeps its cells.
        #[test]
        fn rendered_tables_round_trip(rows in proptest::collection::vec(
            proptest::collection::vec(cell_strategy(), 1..5),
            1..5,
        )) {
            let text = render_table(&rows);
            let parsed = tables(&text);
            prop_assert_eq!(parsed.len(), 1, "one table must be parsed from {:?}", text);
            let table = &parsed[0];
            prop_assert_eq!(&table.header, &rows[0]);
            prop_assert_eq!(&table.rows, &rows[1..]);
        }
    }
}
