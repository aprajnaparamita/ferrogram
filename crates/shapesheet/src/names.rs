//! What a written cell name means (ADR 0054).
//!
//! A reference keeps its parts as written (ADR 0051); this module is the
//! one place that turns them into meaning: the row index a trailing
//! number or `[n]` carries, the section a written section name names, and
//! the **one column-versus-named-row rule** (`Char.Size` is a column of
//! the `Character` section's row shape, `Prop.Cost` is a named row's
//! default cell). Both the resolver ([`crate::resolve`]) and the
//! dependency graph ([`crate::graph`]) ask this module, so the two can
//! never disagree (the P6-12 review's F2).
//!
//! Names match case-insensitively; the file's own spelling is what later
//! gets written back.

use ferrogram_model::{Cell, CellName, Row, Section, Sheet};

/// The default cell of a named row (`Prop.Cost` reads `Cost`'s `Value`).
pub(crate) const DEFAULT_ROW_CELL: &str = "Value";

/// Whether two written names are the same name (ADR 0054: names match
/// case-insensitively).
pub(crate) fn eq_ci(left: &str, right: &str) -> bool {
    left.to_lowercase() == right.to_lowercase()
}

/// The parts of one dotted name after the section (`A5`, `Font[3]`,
/// `Cost`): the leading name and the row index its trailing digits or
/// `[n]` carry (ADR 0054).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NameParts {
    /// The name without the row index.
    pub base: String,
    /// The row index (default 0).
    pub index: usize,
    /// The name as written.
    pub full: String,
}

/// Splits a written name into its row-shape base and row index.
pub(crate) fn split_row_name(part: &str) -> NameParts {
    let full = part.to_owned();
    if part.ends_with(']')
        && let Some(open) = part.rfind('[')
        && let Some(inside) = part.get(open + 1..part.len().saturating_sub(1))
        && let Ok(index) = inside.parse::<usize>()
        && open > 0
    {
        return NameParts {
            base: part.get(..open).unwrap_or("").to_owned(),
            index,
            full,
        };
    }
    let digits = part
        .chars()
        .rev()
        .take_while(|character| character.is_ascii_digit())
        .count();
    if digits > 0 && digits < part.len() {
        let (base, tail) = part.split_at(part.len() - digits);
        if let Ok(index) = tail.parse::<usize>() {
            return NameParts {
                base: base.to_owned(),
                index,
                full,
            };
        }
    }
    NameParts {
        base: full.clone(),
        index: 0,
        full,
    }
}

/// Splits a written section name into its base and its instance index
/// (`Geometry1` is the first `Geometry` section; a name with no number
/// is the first, [`section_instance_position`] turns both into the same
/// slot).
pub(crate) fn split_section_name(part: &str) -> (String, usize) {
    let digits = part
        .chars()
        .rev()
        .take_while(|character| character.is_ascii_digit())
        .count();
    if digits > 0 && digits < part.len() {
        let (base, tail) = part.split_at(part.len() - digits);
        if let Ok(index) = tail.parse::<usize>() {
            return (base.to_owned(), index);
        }
    }
    (part.to_owned(), 0)
}

/// The 0-based slot of the section a written instance number names:
/// `Geometry` and `Geometry1` both name the first `Geometry` section and
/// `Geometry2` the second (ADR 0054).
pub(crate) const fn section_instance_position(instance: usize) -> usize {
    instance.saturating_sub(1)
}

/// The section a written section name names: the file spelling and the
/// ShapeSheet-table spelling match, and the short formula spellings
/// (`Char`, `Prop`) map to their sections.
pub(crate) fn canonical_section(name: &str) -> String {
    let lowered = name.to_lowercase();
    let mapped: &str = match lowered.as_str() {
        "char" | "character" => "Character",
        "prop" | "property" | "shape data" => "Shape Data",
        "user" | "user-defined cells" => "User-defined Cells",
        other => other,
    };
    if mapped == lowered {
        // A file spelling with its own alias (`Property` -> `Shape Data`).
        return ferrogram_model::aliases::table_name(name).to_owned();
    }
    mapped.to_owned()
}

/// Where a section-form name points (ADR 0054): a column of the section's
/// row shape at a row index, or a named row.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum RowKind {
    /// A column of the row shape at a row index (`X2`, `Font[3]`, `Size`).
    Column { column: String, index: usize },
    /// A named row (`Cost`).
    Named(String),
}

impl RowKind {
    /// The cell name this kind reads: the column, or a named row's default
    /// cell ([`DEFAULT_ROW_CELL`]).
    pub(crate) fn cell_name(&self) -> &str {
        match self {
            RowKind::Column { column, .. } => column.as_str(),
            RowKind::Named(_) => DEFAULT_ROW_CELL,
        }
    }
}

/// What `name` means against the section named `canon` (ADR 0054): the
/// one column-versus-named-row rule.
///
/// A name matches a column of the section's row shape first (the
/// ShapeSheet table, or a cell the file itself writes in the section's
/// level cells or rows) and a named row of a named-row section second.
/// `section` is the section of the sheet being read, when it has one; a
/// sheet without the section still knows the table's columns.
pub(crate) fn row_kind(canon: &str, section: Option<&Section>, name: &str) -> RowKind {
    let parts = split_row_name(name);
    if column_known(canon, section, parts.base.as_str()) {
        RowKind::Column {
            column: parts.base,
            index: parts.index,
        }
    } else {
        RowKind::Named(parts.full)
    }
}

/// Whether the name is a column of the section's row shape: the ShapeSheet
/// table, or a cell the file itself writes in those rows (the table has
/// gaps; ADR 0060's deferred items).
pub(crate) fn column_known(canon: &str, section: Option<&Section>, name: &str) -> bool {
    table_column(canon, name)
        || section.is_some_and(|section| {
            section
                .cells
                .iter()
                .any(|cell| eq_ci(cell.name.as_str(), name))
                || section
                    .rows
                    .iter()
                    .any(|row| row.cells.iter().any(|cell| eq_ci(cell.name.as_str(), name)))
        })
}

/// Whether the ShapeSheet table lists `name` as a cell of `canon`.
pub(crate) fn table_column(canon: &str, name: &str) -> bool {
    column_kind(Some(canon), name).is_some()
}

/// The cell type the ShapeSheet table records for `name`, when it knows
/// the cell.
pub(crate) fn column_kind(section: Option<&str>, name: &str) -> Option<&'static str> {
    let mut fallback = None;
    for def in ferrogram_model::generated::CELLS {
        if !eq_ci(def.name, name) {
            continue;
        }
        match section {
            Some(section) if eq_ci(def.section, section) => return Some(def.kind),
            Some(_) => {}
            None if fallback.is_none() => fallback = Some(def.kind),
            None => {}
        }
    }
    fallback
}

/// The spelling the ShapeSheet table uses for a cell name, when it knows
/// one (the model's table lookup matches spelling exactly).
pub(crate) fn table_spelling(name: &str) -> Option<String> {
    ferrogram_model::generated::CELLS
        .iter()
        .find(|def| eq_ci(def.name, name))
        .map(|def| def.name.to_owned())
}

/// The sections of `sheet` the written section name names, in file order.
pub(crate) fn sections_named<'a>(sheet: &'a Sheet, canon: &str) -> impl Iterator<Item = &'a Section> {
    sheet
        .sections
        .iter()
        .filter(move |section| eq_ci(canonical_section(section.name.as_str()).as_str(), canon))
}

/// The section instance a section-form name names (`Geometry1` is the
/// first `Geometry` section).
pub(crate) fn match_section<'a>(
    sheet: &'a Sheet,
    canon: &str,
    instance: usize,
) -> Option<&'a Section> {
    sections_named(sheet, canon).nth(section_instance_position(instance))
}

/// The row index a [`RowKind`] names in one section (ADR 0054): a row
/// index is the row carrying that `IX` (the corpus writes `X1` for the
/// row with `IX` 1, and `graph::find_row_with_position` links the same
/// row), then the row at that position; a named row is the row with
/// that `N`.
pub(crate) fn row_index_of(section: &Section, kind: &RowKind) -> Option<usize> {
    match kind {
        RowKind::Column { index, .. } => {
            let ix = i32::try_from(*index).unwrap_or(i32::MAX);
            if let Some(found) = section.rows.iter().position(|row| row.ix == Some(ix)) {
                return Some(found);
            }
            section.rows.get(*index).map(|_| *index)
        }
        RowKind::Named(name) => section
            .rows
            .iter()
            .position(|row| row.name.as_deref().is_some_and(|row| eq_ci(row, name))),
    }
}

/// The row a [`RowKind`] names in one section.
pub(crate) fn match_row<'a>(section: &'a Section, kind: &RowKind) -> Option<&'a Row> {
    let index = row_index_of(section, kind)?;
    section.rows.get(index)
}

/// Whether the section carries the row the kind names.
pub(crate) fn row_known(section: &Section, kind: &RowKind) -> bool {
    match_row(section, kind).is_some()
}

/// The cell a section-form name names on one section: the section's own
/// cells first (ADR 0013 amendment), then the row's cells.
pub(crate) fn section_cell<'a>(
    section: &'a Section,
    kind: &RowKind,
    explicit: Option<&str>,
) -> Option<&'a Cell> {
    let name = explicit.unwrap_or_else(|| kind.cell_name());
    if let Some(found) = section
        .cells
        .iter()
        .find(|cell| eq_ci(cell.name.as_str(), name))
    {
        return Some(found);
    }
    let row = match_row(section, kind)?;
    row.cells
        .iter()
        .find(|cell| eq_ci(cell.name.as_str(), name))
}

/// Where one cell sits on one sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Slot {
    /// A direct cell of the sheet, at this index.
    Direct(usize),
    /// A section-level cell: section index, cell index.
    SectionLevel(usize, usize),
    /// A row cell: section index, row index, cell index.
    Row(usize, usize, usize),
}

/// The cell of `sheet` named `name` and where it sits: the sheet's own
/// cells first, then each section's level cells and row cells, in file
/// order (the lookup's local rung, matched case-insensitively).
pub(crate) fn find_entry<'a>(sheet: &'a Sheet, name: &str) -> Option<(&'a Cell, Slot)> {
    if let Some(index) = sheet
        .cells
        .iter()
        .position(|cell| eq_ci(cell.name.as_str(), name))
    {
        return sheet
            .cells
            .get(index)
            .map(|cell| (cell, Slot::Direct(index)));
    }
    for (section_index, section) in sheet.sections.iter().enumerate() {
        if let Some(cell_index) = section
            .cells
            .iter()
            .position(|cell| eq_ci(cell.name.as_str(), name))
        {
            return section
                .cells
                .get(cell_index)
                .map(|cell| (cell, Slot::SectionLevel(section_index, cell_index)));
        }
        for (row_index, row) in section.rows.iter().enumerate() {
            if let Some(cell_index) = row
                .cells
                .iter()
                .position(|cell| eq_ci(cell.name.as_str(), name))
            {
                return row
                    .cells
                    .get(cell_index)
                    .map(|cell| (cell, Slot::Row(section_index, row_index, cell_index)));
            }
        }
    }
    None
}

/// The slot a section-form name writes to: the section and row of the
/// sheet, and the cell [`section_cell`] would find there.
pub(crate) fn section_slot(
    sheet: &Sheet,
    canon: &str,
    instance: usize,
    kind: &RowKind,
    explicit: Option<&str>,
) -> Option<Slot> {
    let name = explicit.unwrap_or_else(|| kind.cell_name());
    let (section_index, section) = sections_named(sheet, canon)
        .enumerate()
        .nth(section_instance_position(instance))?;
    if let Some(cell) = section
        .cells
        .iter()
        .position(|cell| eq_ci(cell.name.as_str(), name))
    {
        return Some(Slot::SectionLevel(section_index, cell));
    }
    let row_index = row_index_of(section, kind)?;
    let row = section.rows.get(row_index)?;
    let cell = row
        .cells
        .iter()
        .position(|cell| eq_ci(cell.name.as_str(), name))?;
    Some(Slot::Row(section_index, row_index, cell))
}

/// The cell name of `name` in the ShapeSheet table's spelling, when the
/// table knows it (the model's own lookup matches spelling exactly).
pub(crate) fn spelled_cell(name: &str) -> CellName {
    CellName::new(table_spelling(name).unwrap_or_else(|| name.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferrogram_model::{CellName, Preserved};

    fn cell(name: &str) -> Cell {
        Cell {
            name: CellName::new(name),
            formula: None,
            value: None,
            unit: ferrogram_geom::Unit::Unitsless,
            preserved: Preserved::new(),
        }
    }

    fn section_with(name: &str, rows: Vec<Vec<&str>>) -> Section {
        let mut section = Section::new(ferrogram_model::SectionName::new(name.to_owned()));
        for (index, cells) in rows.into_iter().enumerate() {
            let mut row = Row::new();
            row.ix = Some(i32::try_from(index).unwrap_or(i32::MAX));
            row.cells = cells.into_iter().map(cell).collect();
            section.rows.push(row);
        }
        section
    }

    /// The one rule: a name matches a column first and a named row
    /// second, against the section the sheet holds (P6-12 review F2).
    #[test]
    fn one_rule_decides_column_versus_named_row() {
        let section = section_with("Shape Data", vec![vec!["Value", "Label"]]);
        // `Size` is a table column of `Character`, whatever the sheet holds.
        assert_eq!(
            row_kind("Character", None, "Size"),
            RowKind::Column {
                column: String::from("Size"),
                index: 0
            }
        );
        assert_eq!(
            row_kind("Character", None, "Size2"),
            RowKind::Column {
                column: String::from("Size"),
                index: 2
            }
        );
        // A named row is not a column.
        assert_eq!(
            row_kind("Shape Data", Some(&section), "Cost"),
            RowKind::Named(String::from("Cost"))
        );
        // A cell the file writes in the rows is a column even when the
        // table does not list it.
        assert_eq!(
            row_kind("Scratch", Some(&section), "Label[1]"),
            RowKind::Column {
                column: String::from("Label"),
                index: 1
            }
        );
    }

    /// The row index comes from `IX` first and position second; a named
    /// row matches its `N`.
    #[test]
    fn rows_match_by_ix_then_position_and_by_name() {
        let mut section = section_with("Geometry", vec![vec!["X"], vec!["X"]]);
        section.rows[0].ix = Some(5);
        assert_eq!(
            row_index_of(&section, &RowKind::Column {
                column: String::from("X"),
                index: 5
            }),
            Some(0)
        );
        assert_eq!(
            row_index_of(&section, &RowKind::Column {
                column: String::from("X"),
                index: 1
            }),
            Some(1)
        );
        section.rows[1].name = Some(String::from("Cost"));
        assert_eq!(
            row_index_of(&section, &RowKind::Named(String::from("cost"))),
            Some(1)
        );
        assert_eq!(
            row_index_of(&section, &RowKind::Named(String::from("Gone"))),
            None
        );
    }

    /// A section name folds its instance number and its aliases.
    #[test]
    fn section_names_fold_the_instance_and_the_aliases() {
        assert_eq!(split_section_name("Geometry1"), (String::from("Geometry"), 1));
        assert_eq!(split_section_name("Geometry"), (String::from("Geometry"), 0));
        assert_eq!(section_instance_position(1), 0);
        assert_eq!(canonical_section("char"), "Character");
        assert_eq!(canonical_section("Char"), "Character");
        assert_eq!(canonical_section("Geometry"), "Geometry");
    }

    /// A name splits its trailing row index, in both spellings.
    #[test]
    fn names_split_the_row_index() {
        assert_eq!(split_row_name("A5").base, "A");
        assert_eq!(split_row_name("A5").index, 5);
        assert_eq!(split_row_name("Font[3]").base, "Font");
        assert_eq!(split_row_name("Font[3]").index, 3);
        assert_eq!(split_row_name("Cost").base, "Cost");
        assert_eq!(split_row_name("Cost").index, 0);
    }
}