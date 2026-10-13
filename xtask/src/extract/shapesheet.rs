//! ShapeSheet reference extractor (P1-04).
//!
//! Walks the markdown pages under
//! `reference/ms-vsdx/office-developer-client-docs/docs/visio/` (the
//! ShapeSheet reference; `docs/sources.md` source 4) and produces
//! `ui-spec/shapesheet.toml`, the canonical catalogue of every
//! ShapeSheet section, row and cell Ferrogram knows about (ADR 0008).
//!
//! Page classes:
//!
//! - `<name>-cell-<section>-section.md` (H1 `"<Cell> Cell (<Section>
//!   Section)"`) is a cell. The cell name is the first word of the
//!   title (multi-word names are joined: `Y Justify` -> `YJustify`);
//!   `cell_index`, `section_index` and the owning row come from the
//!   page's `Cell index` / `Section index` / `Row index` metadata,
//!   looked up in the `VisCellIndices` / `VisSectionIndices` /
//!   `VisRowIndices` enumerations.
//! - `<name>-section.md` without `cell-` and with H1 `"<Name>
//!   Section"` is a section page (`description_source`). Row pages
//!   (H1 `"<Name> Row (<Section> Section)"`) and how-to pages are not
//!   sections.
//! - `reference/learn/enums/*.md` are the 112 Visio enumerations.
//!   All are read; the four required ones (`visio.vissectionindices`,
//!   `visio.visrowindices`, `visio.viscellindices`,
//!   `visio.visunitcodes`) are written into the TOML's
//!   `[enumerations]` tables and are the source of every index.
//!
//! A cell's `type` and `default_unit` are inferred from the cell's
//! remark text with the small pattern table in [`infer_type_unit`].
//!
//! Clean-room note (AGENTS.md section 4): only names, indices and
//! type/unit tokens are copied. `description_source` stores the
//! source file name, never its prose.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use toml::Value as Toml;

use super::markdown::{cell_text, kv_pairs, tables};
use super::util::write_atomic;
use super::{ExtractError, ExtractResult};

/// ShapeSheet reference revision recorded in the file header
/// (`docs/sources.md` source 4).
pub const SOURCE: &str =
    "Microsoft Learn ShapeSheet reference (office-developer-client-docs f8c5dfed)";

/// Pinned upstream commit of the ShapeSheet reference.
pub const SOURCE_REVISION: &str = "2024-04-12 (commit f8c5dfed613fafd2fb8c8f9b1e43454f29517831)";

/// The four enumerations written into `ui-spec/shapesheet.toml`.
/// The other 108 pages under `reference/learn/enums/` are read but
/// never committed.
pub const REQUIRED_ENUMS: [&str; 4] = [
    "visio.vissectionindices.md",
    "visio.visrowindices.md",
    "visio.viscellindices.md",
    "visio.visunitcodes.md",
];

/// The TOML `[enumerations]` sub-table names, aligned with
/// [`REQUIRED_ENUMS`].
pub const ENUM_TABLE_NAMES: [&str; 4] = [
    "vis_section_indices",
    "vis_row_indices",
    "vis_cell_indices",
    "vis_unit_codes",
];

/// One cell page.
#[derive(Debug, Clone)]
struct CellPage {
    /// File name (the `description_source`).
    file: String,
    /// Cell name (`PinX`, `YJustify`).
    name: String,
    /// Logical section name from the title (`Shape Transform`).
    section: String,
    /// Normalised section key used for grouping.
    section_key: String,
    /// The first paragraph of the page (no prose is committed; only
    /// used for the type/unit inference).
    summary: String,
    /// `Section index` constant (`visSectionObject`).
    section_const: Option<String>,
    /// `Row index` constants (`visRowXFormOut`).
    row_consts: Vec<String>,
    /// `Cell index` constant (`visXFormPinX`).
    cell_const: Option<String>,
    /// True when any `Row index` value repeats (`+ i`).
    row_repeats: bool,
    /// True when the page carries a `|Value|Description|` table.
    has_value_table: bool,
}

/// One section page.
#[derive(Debug, Clone)]
struct SectionPage {
    /// File name (the `description_source`).
    file: String,
    /// Section name (`Shape Transform`).
    name: String,
}

/// The four enumeration tables (`constant` -> integer), plus the
/// count of every enum page read.
#[derive(Debug, Clone, Default)]
struct Enums {
    /// `VisSectionIndices`.
    section_indices: BTreeMap<String, i64>,
    /// `VisRowIndices`.
    row_indices: BTreeMap<String, i64>,
    /// `VisCellIndices`.
    cell_indices: BTreeMap<String, i64>,
    /// `VisUnitCodes`.
    unit_codes: BTreeMap<String, i64>,
    /// How many enum pages parsed (112 in the P1-01 snapshot).
    pages_read: usize,
}

/// Per-extractor entry point: writes `ui-spec/shapesheet.toml`.
pub fn run(in_dir: &Path, out_dir: &Path) -> ExtractResult<Vec<PathBuf>> {
    let enums_dir = in_dir.join("learn").join("enums");
    let docs_dir = in_dir
        .join("ms-vsdx")
        .join("office-developer-client-docs")
        .join("docs")
        .join("visio");
    if !docs_dir.is_dir() {
        return Err(ExtractError::Missing {
            path: docs_dir.clone(),
        });
    }
    let enums = load_enums(&enums_dir)?;
    let (cells, sections) = load_pages(&docs_dir)?;
    let text = render_toml(&cells, &sections, &enums)?;
    let path = out_dir.join("ui-spec").join("shapesheet.toml");
    write_atomic(&path, &text).map_err(|msg| ExtractError::Io {
        path: path.clone(),
        source: msg,
    })?;
    Ok(vec![path])
}

/// Parses every enumeration page under `in_dir`, keeping the four
/// required tables.
fn load_enums(in_dir: &Path) -> ExtractResult<Enums> {
    if !in_dir.is_dir() {
        return Err(ExtractError::Missing {
            path: in_dir.to_path_buf(),
        });
    }
    let mut names: Vec<String> = std::fs::read_dir(in_dir)
        .map_err(|e| ExtractError::Io {
            path: in_dir.to_path_buf(),
            source: e.to_string(),
        })?
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
        .filter(|n| n.to_ascii_lowercase().ends_with(".md"))
        .collect();
    names.sort();
    let mut enums = Enums::default();
    for name in &names {
        let path = in_dir.join(name);
        let text = std::fs::read_to_string(&path).map_err(|e| ExtractError::Io {
            path: path.clone(),
            source: e.to_string(),
        })?;
        let table = parse_enum_table(&text).map_err(|message| ExtractError::Parse {
            path: path.clone(),
            message,
        })?;
        enums.pages_read += 1;
        let lower = name.to_ascii_lowercase();
        if lower == REQUIRED_ENUMS[0] {
            enums.section_indices = table;
        } else if lower == REQUIRED_ENUMS[1] {
            enums.row_indices = table;
        } else if lower == REQUIRED_ENUMS[2] {
            enums.cell_indices = table;
        } else if lower == REQUIRED_ENUMS[3] {
            enums.unit_codes = table;
        }
    }
    for required in REQUIRED_ENUMS {
        let empty = match required {
            "visio.vissectionindices.md" => enums.section_indices.is_empty(),
            "visio.visrowindices.md" => enums.row_indices.is_empty(),
            "visio.viscellindices.md" => enums.cell_indices.is_empty(),
            _ => enums.unit_codes.is_empty(),
        };
        if empty {
            return Err(ExtractError::Missing {
                path: in_dir.join(required),
            });
        }
    }
    Ok(enums)
}

/// Parses a `|Constant|Value|Description|` enumeration page into a
/// `constant -> value` map. Rows whose value is not a plain integer
/// (a few of the 108 unused pages carry hex values like `&H2000`)
/// are skipped: only the four required tables are committed and
/// those are integer-only.
fn parse_enum_table(text: &str) -> Result<BTreeMap<String, i64>, String> {
    let mut out = BTreeMap::new();
    for table in tables(text) {
        let header: Vec<String> = table.header.iter().map(|c| cell_text(c)).collect();
        if header.len() < 2 || !header[0].eq_ignore_ascii_case("constant") {
            continue;
        }
        for row in &table.rows {
            let constant = row.first().map(|c| cell_text(c)).unwrap_or_default();
            if constant.is_empty() {
                continue;
            }
            let value_raw = row.get(1).map(|c| cell_text(c)).unwrap_or_default();
            if let Ok(value) = value_raw.parse::<i64>() {
                out.insert(constant, value);
            }
        }
    }
    Ok(out)
}

/// Classifies and parses every page under `docs_dir`.
fn load_pages(docs_dir: &Path) -> ExtractResult<(Vec<CellPage>, BTreeMap<String, SectionPage>)> {
    let mut files: Vec<String> = std::fs::read_dir(docs_dir)
        .map_err(|e| ExtractError::Io {
            path: docs_dir.to_path_buf(),
            source: e.to_string(),
        })?
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
        .filter(|n| n.ends_with(".md"))
        .collect();
    files.sort();

    let mut cells = Vec::new();
    let mut sections = BTreeMap::new();
    for file in &files {
        let path = docs_dir.join(file);
        let text = std::fs::read_to_string(&path).map_err(|e| ExtractError::Io {
            path: path.clone(),
            source: e.to_string(),
        })?;
        let Some(title) = page_title(&text) else {
            continue;
        };
        if let Some((name, section)) = split_title(&title, " Cell (") {
            if !file.contains("-cell-") {
                continue;
            }
            let name = name.replace(' ', "");
            let section_key = section_key(&section);
            let summary = page_summary(&text);
            // Metadata rows read `**Cell name:**`; normalise the key
            // (`Cell name`).
            let pairs: Vec<(String, String)> = kv_pairs(&text)
                .into_iter()
                .map(|(k, v)| (k.trim_end_matches(':').trim().to_string(), v))
                .collect();
            let section_const = first_identifier(&pairs, "Section index");
            let cell_const = first_identifier(&pairs, "Cell index");
            let row_consts: Vec<String> = pairs
                .iter()
                .filter(|(k, _)| key_eq(k, "Row index"))
                .filter_map(|(_, v)| leading_identifier(v))
                .collect();
            let row_repeats = pairs
                .iter()
                .filter(|(k, _)| key_eq(k, "Row index"))
                .any(|(_, v)| v.contains('+'));
            cells.push(CellPage {
                file: file.clone(),
                name,
                section,
                section_key,
                summary,
                section_const,
                row_consts,
                cell_const,
                row_repeats,
                has_value_table: text.contains("**Value**"),
            });
        } else if let Some((name, section)) = split_title(&title, " Row (") {
            // Row pages document row *types* (MoveTo, LineTo, ...);
            // their row-index constants come from the cell pages.
            let _ = (name, section);
        } else if let Some((name, _)) = split_title(&title, " Section") {
            // How-to pages ("Add, Show, Hide, or Delete a Section")
            // have comma-separated imperative names; section names
            // are noun phrases. The MS-VSDX schema pages
            // (`cell-element-*-visio-xml.md`) are named like
            // "Cell element (Geometry Section)" and are not
            // ShapeSheet section pages.
            if name.contains(',') || !file.ends_with("-section.md") {
                continue;
            }
            let key = section_key(&name);
            sections.entry(key).or_insert(SectionPage {
                file: file.clone(),
                name,
            });
        }
    }
    Ok((cells, sections))
}

/// The first `# ` heading of a page, entities decoded.
fn page_title(text: &str) -> Option<String> {
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("# ") {
            return Some(unescape_entities(rest.trim()));
        }
        if !line.trim().is_empty() && !line.starts_with("---") {
            // Keep scanning: titles come after the YAML front matter.
            continue;
        }
    }
    None
}

/// The first paragraph of a page (used only for type inference).
fn page_summary(text: &str) -> String {
    let mut out = Vec::new();
    let mut started = false;
    for line in text.lines() {
        if line.starts_with("# ") {
            started = true;
            continue;
        }
        if !started {
            continue;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !out.is_empty() {
                break;
            }
            continue;
        }
        if trimmed.starts_with('#') || trimmed.starts_with('|') || trimmed.starts_with('>') {
            break;
        }
        out.push(trimmed);
    }
    unescape_entities(&out.join(" "))
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Splits `"<Name> Cell (<Section> Section)"` style titles at
/// `marker`. Returns `(name, section)`; tolerates the pages whose
/// closing paren is missing. For the `" Section"` marker the second
/// half is empty: `"Shape Transform Section"` names the section in
/// its first half.
fn split_title(title: &str, marker: &str) -> Option<(String, String)> {
    let idx = title.find(marker)?;
    let name = title[..idx].trim().to_string();
    let mut rest = title[idx + marker.len()..].trim().to_string();
    if let Some(stripped) = rest.strip_suffix(')') {
        rest = stripped.to_string();
    }
    if (marker.starts_with(" Cell") || marker.starts_with(" Row"))
        && let Some(stripped) = rest.strip_suffix(" Section")
    {
        rest = stripped.to_string();
    }
    let rest = rest.trim().to_string();
    if name.is_empty() || (rest.is_empty() && marker != " Section") {
        return None;
    }
    Some((name, rest))
}

/// Groups names across typography: `Action Tags` == `Action Tag`,
/// `PrintProperties` == `Print Properties`, `User-Defined Cells` ==
/// `User-defined Cells`.
fn section_key(name: &str) -> String {
    let lower = unescape_entities(name).to_ascii_lowercase();
    let joined: String = lower
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '&')
        .collect();
    match joined.strip_suffix('s') {
        Some(stripped) if stripped.len() > 2 => stripped.to_string(),
        _ => joined,
    }
}

/// True when a normalised metadata key matches. The pages write
/// `**Cell name:**`; `super::markdown::kv_pairs` keeps the colon.
fn key_eq(key: &str, want: &str) -> bool {
    key.trim_end_matches(':').trim() == want
}

/// First value of a metadata key, as its leading identifier
/// (`**visSectionObject**` -> `visSectionObject`).
fn first_identifier(pairs: &[(String, String)], key: &str) -> Option<String> {
    pairs
        .iter()
        .find(|(k, _)| key_eq(k, key))
        .and_then(|(_, v)| leading_identifier(v))
}

/// The leading `vis*` identifier of a metadata value.
fn leading_identifier(value: &str) -> Option<String> {
    let trimmed = value.trim();
    let end = trimmed
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(trimmed.len());
    let candidate = &trimmed[..end];
    if candidate.starts_with("vis") && candidate.len() > 3 {
        return Some(candidate.to_string());
    }
    None
}

/// Infers `(type, default_unit)` from a cell's remark text.
///
/// The vocabulary is ADR 0008's type enum (`Number`, `String`,
/// `Bool`, `Color`, `List`) plus unit tokens from `VisUnitCodes`
/// (`Inches`, `Degrees`, `Points`, `Percent`, `Number`) and the
/// ShapeSheet UI's non-numeric units (`String`, `Color`, `NoCast`).
pub fn infer_type_unit(summary: &str, has_value_table: bool) -> (&'static str, &'static str) {
    let s = summary.to_ascii_lowercase();
    if s.contains("rgb") || s.contains("color") || s.contains("colour") {
        return ("Color", "Color");
    }
    if s.contains("formula") && !s.contains("default formula") {
        return ("String", "String");
    }
    if s.contains("whether") {
        return ("Bool", "NoCast");
    }
    if s.contains("degrees") || s.contains("angle") {
        return ("Number", "Degrees");
    }
    if s.contains("points") {
        return ("Number", "Points");
    }
    if s.contains("percent") {
        return ("Number", "Percent");
    }
    if s.contains("inches")
        || s.contains("inch ")
        || s.contains("coordinate")
        || s.contains("origin of its parent")
        || s.contains("width")
        || s.contains("height")
        || s.contains("distance")
    {
        return ("Number", "Inches");
    }
    if s.contains("number") {
        return ("Number", "Number");
    }
    if s.contains("url")
        || s.contains("string")
        || s.contains("address")
        || s.contains("caption")
        || s.contains("name")
        || s.contains("font")
    {
        return ("String", "String");
    }
    if has_value_table {
        return ("List", "NoCast");
    }
    ("Number", "Number")
}

/// Resolves the `VisSectionIndices` constant of a section: the
/// constant its cells reference, else a name match against the
/// enumeration constants.
fn resolve_section_const(name: &str, cells: &[&CellPage], enums: &Enums) -> Option<(String, i64)> {
    let mut votes: BTreeMap<String, usize> = BTreeMap::new();
    for c in cells {
        if let Some(k) = &c.section_const {
            *votes.entry(k.clone()).or_default() += 1;
        }
    }
    if let Some((k, _)) = votes
        .iter()
        .max_by_key(|(k, v)| (**v, std::cmp::Reverse(*k)))
        && let Some(v) = enums.section_indices.get(k.as_str())
    {
        return Some((k.clone(), *v));
    }
    // Fallback for sections with no cell pages (Scratch, Fill
    // Gradient, ...): the section name must be a prefix of the
    // constant's words (`Fill Gradient` / `visSectionFillGradientStops`).
    let want: Vec<String> = name
        .split_whitespace()
        .map(|w| w.to_ascii_lowercase())
        .collect();
    if want.is_empty() {
        return None;
    }
    let mut candidates: Vec<(String, i64)> = enums
        .section_indices
        .iter()
        .filter(|(k, _)| k.starts_with("visSection"))
        .filter_map(|(k, v)| {
            let words = super::commands::camel_words(k.trim_start_matches("visSection"));
            let matches =
                words.len() >= want.len() && words.iter().zip(want.iter()).all(|(a, b)| a == b);
            if matches { Some((k.clone(), *v)) } else { None }
        })
        .collect();
    candidates.sort();
    candidates.into_iter().next()
}

/// Renders the whole `ui-spec/shapesheet.toml`.
fn render_toml(
    cells: &[CellPage],
    sections: &BTreeMap<String, SectionPage>,
    enums: &Enums,
) -> ExtractResult<String> {
    // Group cells by section key.
    let mut by_section: BTreeMap<String, Vec<&CellPage>> = BTreeMap::new();
    for c in cells {
        by_section.entry(c.section_key.clone()).or_default().push(c);
    }

    // Canonical section set: every page-backed section plus every
    // section the cells reference. Sorted by section name (ties by
    // key), per the task's output contract.
    let mut section_names: BTreeMap<String, String> = BTreeMap::new();
    for (key, page) in sections {
        section_names.insert(key.clone(), page.name.clone());
    }
    for (key, group) in &by_section {
        section_names
            .entry(key.clone())
            .or_insert_with(|| group.first().map(|c| c.section.clone()).unwrap_or_default());
    }
    let mut ordered: Vec<(String, String)> = section_names.into_iter().collect();
    ordered.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));

    let mut section_tables: Vec<Toml> = Vec::new();
    // Row constant -> owning section name (first alphabetically).
    let mut row_owner: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    for (key, name) in &ordered {
        let group: Vec<&CellPage> = by_section.get(key).cloned().unwrap_or_default();
        let mut cell_tables: Vec<Toml> = Vec::new();
        let mut unit_votes: BTreeMap<String, usize> = BTreeMap::new();
        let mut row_consts: BTreeSet<String> = BTreeSet::new();
        for c in &group {
            for r in &c.row_consts {
                row_consts.insert(r.clone());
                row_owner.entry(r.clone()).or_default().insert(name.clone());
            }
        }
        let mut sorted_cells: Vec<&&CellPage> = group.iter().collect();
        sorted_cells.sort_by(|a, b| a.name.cmp(&b.name));
        for c in sorted_cells {
            let (ty, unit) = infer_type_unit(&c.summary, c.has_value_table);
            *unit_votes.entry(unit.to_string()).or_default() += 1;
            let mut t = toml::Table::new();
            t.insert("name".into(), Toml::String(c.name.clone()));
            if let Some(constant) = &c.cell_const
                && let Some(v) = enums.cell_indices.get(constant.as_str())
            {
                t.insert("cell_index".into(), Toml::Integer(*v));
            }
            t.insert("type".into(), Toml::String(ty.into()));
            t.insert("default_unit".into(), Toml::String(unit.into()));
            t.insert("description_source".into(), Toml::String(c.file.clone()));
            cell_tables.push(Toml::Table(t));
        }

        let kind = if group.iter().any(|c| c.row_repeats) || group.is_empty() {
            "collection"
        } else {
            "object"
        };
        let default_unit = unit_votes
            .iter()
            .max_by_key(|(u, v)| (**v, std::cmp::Reverse(u.as_str())))
            .map(|(u, _)| u.clone())
            .unwrap_or_else(|| "NoCast".to_string());

        let mut t = toml::Table::new();
        t.insert("name".into(), Toml::String(name.clone()));
        if let Some((_, index)) = resolve_section_const(name, &group, enums) {
            t.insert("section_index".into(), Toml::Integer(index));
        }
        t.insert("kind".into(), Toml::String(kind.into()));
        if let Some(page) = sections.get(key) {
            t.insert("description_source".into(), Toml::String(page.file.clone()));
        }
        t.insert("default_unit".into(), Toml::String(default_unit));
        if !row_consts.is_empty() {
            t.insert(
                "rows".into(),
                Toml::Array(row_consts.iter().map(|r| Toml::String(r.clone())).collect()),
            );
        }
        if !cell_tables.is_empty() {
            t.insert("cells".into(), Toml::Array(cell_tables));
        }
        section_tables.push(Toml::Table(t));
    }

    // Global row table (VisRowIndices).
    let mut row_tables: Vec<Toml> = Vec::new();
    for (name, index) in &enums.row_indices {
        let mut t = toml::Table::new();
        t.insert("name".into(), Toml::String(name.clone()));
        t.insert("row_index".into(), Toml::Integer(*index));
        if let Some(owners) = row_owner.get(name)
            && let Some(owner) = owners.iter().next()
        {
            t.insert("section".into(), Toml::String(owner.clone()));
        }
        row_tables.push(Toml::Table(t));
    }

    // The four enumerations, verbatim (constant -> integer).
    let mut enumerations = toml::Table::new();
    for (table_name, table) in [
        (ENUM_TABLE_NAMES[0], &enums.section_indices),
        (ENUM_TABLE_NAMES[1], &enums.row_indices),
        (ENUM_TABLE_NAMES[2], &enums.cell_indices),
        (ENUM_TABLE_NAMES[3], &enums.unit_codes),
    ] {
        let inner: toml::Table = table
            .iter()
            .map(|(k, v)| (k.clone(), Toml::Integer(*v)))
            .collect();
        enumerations.insert(table_name.into(), Toml::Table(inner));
    }

    let mut root = toml::Table::new();
    root.insert("schema_version".into(), Toml::Integer(1));
    root.insert("product".into(), Toml::String("visio".into()));
    root.insert("source".into(), Toml::String(SOURCE.into()));
    root.insert(
        "source_revision".into(),
        Toml::String(SOURCE_REVISION.into()),
    );
    root.insert("enumerations".into(), Toml::Table(enumerations));
    root.insert("rows".into(), Toml::Array(row_tables));
    root.insert("sections".into(), Toml::Array(section_tables));

    toml::to_string_pretty(&root).map_err(|e| ExtractError::Parse {
        path: PathBuf::from("ui-spec/shapesheet.toml"),
        message: format!("TOML serialize: {e}"),
    })
}

/// Decodes the HTML entities that leak into the markdown titles
/// (`Ruler &amp; Grid Section`).
fn unescape_entities(text: &str) -> String {
    text.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    const ENUM_FIXTURE: &str = "\
# VisSectionIndices enumeration (Visio)

|Constant|Value|Description|
|:-----|:-----|:-----|
| **visSectionObject**|1|Stores general properties.|
| **visSectionScratch**|6|Holds scratch cells.|
";

    const CELL_FIXTURE: &str = "\
---
title: \"PinX Cell (Shape Transform Section)\"
---

# PinX Cell (Shape Transform Section)

Represents the  *x*  -coordinate of the shape's pin in relation to the origin of its parent.

## Remarks

To get a reference to the PinX cell by index:

||Value |
|:-----|:-----|
| **Cell name:**  <br/> | PinX  <br/> |
| **Section index:**  <br/> |**visSectionObject** <br/> |
| **Row index:**  <br/> |**visRowXFormOut** <br/> |
| **Cell index:**  <br/> |**visXFormPinX** <br/> |
";

    const CELL_REPEAT_FIXTURE: &str = "\
# Y Justify Cell (Action Tags Section)

The y-offset of the action tag button.

||Value |
|:-----|:-----|
| **Section index:**  <br/> |**visSectionSmartTag** <br/> |
| **Row index:**  <br/> |**visRowSmartTag** +  *i*  where  *i*  = 0, 1, 2... |
| **Cell index:**  <br/> |**visSmartTagYJustify** <br/> |
";

    const SECTION_FIXTURE: &str = "\
# Shape Transform Section

Contains general positioning information about a shape.
";

    fn enums_from(text: &str) -> Enums {
        Enums {
            section_indices: parse_enum_table(text).unwrap(),
            row_indices: parse_enum_table(
                "|Constant|Value|\n|---|---|\n| **visRowXFormOut**|1|\n| **visRowSmartTag**|0|\n",
            )
            .unwrap(),
            cell_indices: parse_enum_table(
                "|Constant|Value|\n|---|---|\n| **visXFormPinX**|0|\n| **visSmartTagYJustify**|2|\n",
            )
            .unwrap(),
            unit_codes: parse_enum_table("|Constant|Value|\n|---|---|\n| **visInches**|65|\n").unwrap(),
            pages_read: 4,
        }
    }

    #[test]
    fn parses_enum_tables() {
        let t = parse_enum_table(ENUM_FIXTURE).unwrap();
        assert_eq!(t.get("visSectionObject"), Some(&1));
        assert_eq!(t.get("visSectionScratch"), Some(&6));
        assert_eq!(t.len(), 2);
    }

    #[test]
    fn enum_table_skips_non_integer_values() {
        // A few unused enum pages carry hex values (`&H2000`); those
        // rows are skipped, not fatal.
        let text = "|Constant|Value|\n|---|---|\n| **visX**|&H2000|\n| **visY**|7|\n";
        let t = parse_enum_table(text).unwrap();
        assert_eq!(t.get("visY"), Some(&7));
        assert!(!t.contains_key("visX"));
    }

    #[test]
    fn title_parsing_handles_cells_rows_and_sections() {
        assert_eq!(
            split_title("PinX Cell (Shape Transform Section)", " Cell ("),
            Some(("PinX".into(), "Shape Transform".into()))
        );
        // The pages whose closing paren is missing.
        assert_eq!(
            split_title("FontSchemeIndex Cell (Theme Properties Section", " Cell ("),
            Some(("FontSchemeIndex".into(), "Theme Properties".into()))
        );
        assert_eq!(
            split_title("LineTo Row (Geometry Section)", " Row ("),
            Some(("LineTo".into(), "Geometry".into()))
        );
        assert_eq!(
            split_title("Shape Transform Section", " Section"),
            Some(("Shape Transform".into(), "".into()))
        );
    }

    #[test]
    fn section_keys_are_typography_insensitive() {
        assert_eq!(section_key("Action Tags"), section_key("Action Tag"));
        assert_eq!(
            section_key("Print Properties"),
            section_key("PrintProperties")
        );
        assert_eq!(
            section_key("User-Defined Cells"),
            section_key("User-defined Cells")
        );
        assert_ne!(section_key("Fill Format"), section_key("Fill Gradient"));
    }

    #[test]
    fn cell_pages_parse_metadata() {
        let cells = [
            CellPage {
                file: "pinx.md".into(),
                name: "PinX".into(),
                section: "Shape Transform".into(),
                section_key: section_key("Shape Transform"),
                summary: page_summary(CELL_FIXTURE),
                section_const: Some("visSectionObject".into()),
                row_consts: vec!["visRowXFormOut".into()],
                cell_const: Some("visXFormPinX".into()),
                row_repeats: false,
                has_value_table: false,
            },
            CellPage {
                file: "yjustify.md".into(),
                name: "YJustify".into(),
                section: "Action Tags".into(),
                section_key: section_key("Action Tags"),
                summary: page_summary(CELL_REPEAT_FIXTURE),
                section_const: Some("visSectionSmartTag".into()),
                row_consts: vec!["visRowSmartTag".into()],
                cell_const: Some("visSmartTagYJustify".into()),
                row_repeats: true,
                has_value_table: false,
            },
        ];
        assert_eq!(cells[0].name, "PinX");
        assert_eq!(cells[0].cell_const.as_deref(), Some("visXFormPinX"));
        assert!(cells[1].row_repeats);
    }

    #[test]
    fn summary_and_metadata_extraction_from_the_fixture() {
        let pairs = kv_pairs(CELL_FIXTURE);
        assert_eq!(
            first_identifier(&pairs, "Section index").as_deref(),
            Some("visSectionObject")
        );
        assert_eq!(
            first_identifier(&pairs, "Cell index").as_deref(),
            Some("visXFormPinX")
        );
        let summary = page_summary(CELL_FIXTURE);
        assert!(summary.starts_with("Represents"), "{summary}");
        assert!(!summary.contains("Remarks"));
    }

    #[test]
    fn type_unit_patterns() {
        assert_eq!(
            infer_type_unit("Indicates whether the layer is active.", false),
            ("Bool", "NoCast")
        );
        assert_eq!(
            infer_type_unit(
                "Determines the color of the bevel's contour in RGB value.",
                false
            ),
            ("Color", "Color")
        );
        assert_eq!(
            infer_type_unit("Represents the shape's current angle of rotation.", false),
            ("Number", "Degrees")
        );
        assert_eq!(
            infer_type_unit("Determines the width of the bottom bevel in points.", false),
            ("Number", "Points")
        );
        assert_eq!(
            infer_type_unit("Represents the x-coordinate of the shape's pin.", false),
            ("Number", "Inches")
        );
        assert_eq!(
            infer_type_unit("Contains the formula to be executed.", false),
            ("String", "String")
        );
        assert_eq!(
            infer_type_unit("Specifies the tab alignment.", true),
            ("List", "NoCast")
        );
        assert_eq!(infer_type_unit("", false), ("Number", "Number"));
    }

    #[test]
    fn renders_sorted_deterministic_toml() {
        let cells = [
            CellPage {
                file: "yjustify.md".into(),
                name: "YJustify".into(),
                section: "Action Tags".into(),
                section_key: section_key("Action Tags"),
                summary: String::new(),
                section_const: Some("visSectionSmartTag".into()),
                row_consts: vec!["visRowSmartTag".into()],
                cell_const: Some("visSmartTagYJustify".into()),
                row_repeats: true,
                has_value_table: false,
            },
            CellPage {
                file: "pinx.md".into(),
                name: "PinX".into(),
                section: "Shape Transform".into(),
                section_key: section_key("Shape Transform"),
                summary: String::new(),
                section_const: Some("visSectionObject".into()),
                row_consts: vec!["visRowXFormOut".into()],
                cell_const: Some("visXFormPinX".into()),
                row_repeats: false,
                has_value_table: false,
            },
        ];
        let mut sections = BTreeMap::new();
        sections.insert(
            section_key("Shape Transform"),
            SectionPage {
                file: "shape-transform-section.md".into(),
                name: "Shape Transform".into(),
            },
        );
        let enums = enums_from(ENUM_FIXTURE);
        let a = render_toml(&cells, &sections, &enums).unwrap();
        let b = render_toml(&cells, &sections, &enums).unwrap();
        assert_eq!(a, b, "output is deterministic");

        let parsed: toml::Table = toml::from_str(&a).unwrap();
        let secs = parsed.get("sections").and_then(Toml::as_array).unwrap();
        // Sorted by section name: Action Tags before Shape Transform.
        assert_eq!(secs[0].get("name").unwrap().as_str(), Some("Action Tags"));
        assert_eq!(
            secs[1].get("name").unwrap().as_str(),
            Some("Shape Transform")
        );
        // Action Tags repeats per row; Shape Transform is fixed.
        assert_eq!(secs[0].get("kind").unwrap().as_str(), Some("collection"));
        assert_eq!(secs[1].get("kind").unwrap().as_str(), Some("object"));
        // Indices come from the enumeration lookups.
        assert_eq!(secs[1].get("section_index").unwrap().as_integer(), Some(1));
        let cells_of = secs[1].get("cells").and_then(Toml::as_array).unwrap();
        assert_eq!(cells_of[0].get("name").unwrap().as_str(), Some("PinX"));
        assert_eq!(cells_of[0].get("cell_index").unwrap().as_integer(), Some(0));
        // Rows table is the global VisRowIndices table.
        let rows = parsed.get("rows").and_then(Toml::as_array).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0].get("name").unwrap().as_str(),
            Some("visRowSmartTag")
        );
        assert_eq!(
            rows[0].get("section").unwrap().as_str(),
            Some("Action Tags")
        );
        // The four enumerations are embedded.
        let en = parsed.get("enumerations").and_then(Toml::as_table).unwrap();
        assert!(en.contains_key("vis_section_indices"));
        assert!(en.contains_key("vis_row_indices"));
        assert!(en.contains_key("vis_cell_indices"));
        assert!(en.contains_key("vis_unit_codes"));
    }

    #[test]
    fn load_pages_classifies_the_fixture() {
        let tmp = crate::util::test_support::TempDir::new("shapesheet");
        std::fs::write(
            tmp.join("pinx-cell-shape-transform-section.md"),
            CELL_FIXTURE,
        )
        .unwrap();
        std::fs::write(
            tmp.join("y-justify-cell-action-tags-section.md"),
            CELL_REPEAT_FIXTURE,
        )
        .unwrap();
        std::fs::write(tmp.join("shape-transform-section.md"), SECTION_FIXTURE).unwrap();
        std::fs::write(
            tmp.join("lineto-row-geometry-section.md"),
            "# LineTo Row (Geometry Section)\n\nA row.\n",
        )
        .unwrap();
        std::fs::write(
            tmp.join("about-cell-references.md"),
            "# About Cell References\n\nProse.\n",
        )
        .unwrap();
        let (cells, sections) = load_pages(tmp.path()).unwrap();
        assert_eq!(
            cells.len(),
            2,
            "{:?}",
            cells.iter().map(|c| &c.file).collect::<Vec<_>>()
        );
        assert_eq!(sections.len(), 1);
        assert!(sections.contains_key(&section_key("Shape Transform")));
    }

    #[test]
    fn missing_docs_dir_is_a_missing_error() {
        let tmp = crate::util::test_support::TempDir::new("shapesheet2");
        let err = run(tmp.path(), tmp.path()).unwrap_err();
        assert!(matches!(err, ExtractError::Missing { .. }), "{err}");
    }

    /// Real-data checks: the P1-01 reference is present and the
    /// extractor is byte-stable. Skipped without `reference/`.
    #[test]
    fn real_reference_is_stable_and_sorted() {
        let root = crate::util::root();
        let ref_dir = root.join("reference");
        let docs = ref_dir
            .join("ms-vsdx")
            .join("office-developer-client-docs")
            .join("docs")
            .join("visio");
        if !docs.is_dir() {
            eprintln!("reference/ not present; skipping");
            return;
        }
        let out = crate::util::test_support::TempDir::new("shapesheet3");
        let written = run(&ref_dir, out.path()).unwrap();
        let first = std::fs::read_to_string(&written[0]).unwrap();
        run(&ref_dir, out.path()).unwrap();
        let second = std::fs::read_to_string(&written[0]).unwrap();
        assert_eq!(first, second, "re-running is a no-op");

        let parsed: toml::Table = toml::from_str(&first).unwrap();
        let secs = parsed.get("sections").and_then(Toml::as_array).unwrap();
        let mut names: Vec<&str> = secs
            .iter()
            .filter_map(|s| s.get("name").and_then(Toml::as_str))
            .collect();
        let sorted = names.clone();
        names.sort();
        assert_eq!(names, sorted, "sections are sorted by name");
        for s in secs {
            let cells = s.get("cells").and_then(Toml::as_array);
            if let Some(cells) = cells {
                let mut cell_names: Vec<&str> = cells
                    .iter()
                    .filter_map(|c| c.get("name").and_then(Toml::as_str))
                    .collect();
                let sorted = cell_names.clone();
                cell_names.sort();
                assert_eq!(cell_names, sorted, "cells sorted in {:?}", s.get("name"));
            }
        }
    }

    /// The other 108 enum pages are read but not committed.
    #[test]
    fn real_reference_reads_all_enum_pages() {
        let root = crate::util::root();
        let enums_dir = root.join("reference").join("learn").join("enums");
        if !enums_dir.is_dir() {
            eprintln!("reference/ not present; skipping");
            return;
        }
        let enums = load_enums(&enums_dir).expect("enums load");
        assert_eq!(enums.pages_read, 112, "enum pages read");
        assert!(!enums.section_indices.is_empty());
        assert!(!enums.row_indices.is_empty());
        assert!(!enums.cell_indices.is_empty());
        assert!(!enums.unit_codes.is_empty());
    }
}
