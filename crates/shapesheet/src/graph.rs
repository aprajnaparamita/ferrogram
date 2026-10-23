//! The dependency graph and recalc order (ADR 0055).
//!
//! One node is one cell ([`CellAddress`]); a formula's references are its
//! dependencies. The graph answers "what must be recomputed after this
//! change" ([`DepGraph::evaluation_order`]) and is where a reference
//! cycle is detected. The recalc pass itself is [`crate::recalc`].
//!
//! [`DepGraph::build`] parses every formula of a document once;
//! [`DepGraph::update`] re-links only the nodes an edit touched. A node's
//! dependencies are the cells its formula references (resolved per
//! ADR 0054) plus, for an inherited formula, the rung that defines it.
//!
//! Addresses must name cells the way the model spells them: a row target
//! carries both the position and the `IX` when the row has one
//! ([`normalize_address`]). The order of [`DepGraph::evaluation_order`]
//! is deterministic: dependencies first, ties broken by address order.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use ferrogram_model::{
    Cell, CellLookup, CellName, Document, MasterId, Resolved, Row, RowTarget, Section, SectionName,
    Sheet, SheetRef, ValueSource,
};

use crate::Error;
use crate::formula::{CellRef, Formula};
use crate::names;
use crate::recalc::FormulaEngine;

/// The formula the corpus writes on a cell whose value is inherited
/// (`F='Inh'`): it defines nothing (ADR 0013), so the rung that does
/// defines the cell's formula.
pub(crate) const INHERIT_FORMULA: &str = "Inh";

/// The address of one cell: the sheet, the section and row it lives in
/// and its name (ADR 0055).
///
/// A row is named the way the model names it ([`RowTarget`]): by `IX` or
/// by position, because a formula can address a row either way. The type
/// is `Clone`, not `Copy` ([`CellName`] holds a `Cow`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellAddress {
    /// The sheet the cell is on.
    pub sheet: SheetRef,
    /// The section, or `None` for a sheet's direct cells.
    pub section: Option<SectionName>,
    /// The row in a repeated section, or `None`.
    pub row: Option<RowTarget>,
    /// The cell name.
    pub cell: CellName,
}

impl CellAddress {
    /// The cell `cell` on `sheet`, outside any section.
    pub const fn new(sheet: SheetRef, cell: CellName) -> Self {
        Self {
            sheet,
            section: None,
            row: None,
            cell,
        }
    }

    /// The same cell in `section`, in `row`.
    pub const fn in_row(
        sheet: SheetRef,
        section: SectionName,
        row: RowTarget,
        cell: CellName,
    ) -> Self {
        Self {
            sheet,
            section: Some(section),
            row: Some(row),
            cell,
        }
    }
}

/// The deterministic address order (ADR 0055's stable tie-break): page
/// sheets before shape sheets, by index; then the section (absent
/// first), the row and the cell name.
impl PartialOrd for CellAddress {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// The deterministic address order (see [`PartialOrd`]).
impl Ord for CellAddress {
    fn cmp(&self, other: &Self) -> Ordering {
        sheet_key(&self.sheet)
            .cmp(&sheet_key(&other.sheet))
            .then_with(|| {
                self.section
                    .as_ref()
                    .map(|name| name.as_str())
                    .cmp(&other.section.as_ref().map(|name| name.as_str()))
            })
            .then_with(|| {
                self.row
                    .map(|row| (row.position, row.ix))
                    .cmp(&other.row.map(|row| (row.position, row.ix)))
            })
            .then_with(|| self.cell.as_str().cmp(other.cell.as_str()))
    }
}

/// The sort key of a sheet: page sheets first, then shape sheets.
const fn sheet_key(sheet: &SheetRef) -> (u8, u32) {
    match sheet {
        SheetRef::Page(id) => (0, id.index),
        SheetRef::Shape(id) => (1, id.index),
    }
}

/// The dependency graph of a document's formulas (ADR 0055).
#[derive(Debug, Clone, PartialEq)]
pub struct DepGraph {
    /// For every cell with a formula: the cells its formula reads and,
    /// when the formula is inherited, the rung that defines it.
    deps: BTreeMap<CellAddress, Vec<CellAddress>>,
    /// The reverse edges: every cell that reads a cell, sorted.
    rdeps: BTreeMap<CellAddress, Vec<CellAddress>>,
}

impl Default for DepGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl DepGraph {
    /// An empty graph; [`DepGraph::build`] fills one in from a document.
    pub fn new() -> Self {
        Self {
            deps: BTreeMap::new(),
            rdeps: BTreeMap::new(),
        }
    }

    /// Builds the graph for `document`.
    ///
    /// Every sheet is walked once and every formula is parsed once
    /// (O(cells with formulas)). A formula that fails to parse
    /// contributes no edges: the document is never failed by one bad
    /// formula. The result is always `Ok`; the `Result` keeps the
    /// interface P6-01 signed.
    pub fn build(document: &Document) -> Result<DepGraph, Error> {
        Self::build_with(document, &mut crate::recalc::DocumentEngine)
    }

    /// Builds the graph for `document`, extracting references with
    /// `engine` (ADR 0055).
    ///
    /// This is the seam the tests drive with a small engine; the
    /// production engine is `DocumentEngine`.
    pub fn build_with(
        document: &Document,
        engine: &mut dyn FormulaEngine,
    ) -> Result<DepGraph, Error> {
        let mut graph = Self::new();
        for (sheet, data) in sheet_refs(document) {
            for (address, cell) in formula_cells(sheet, data) {
                graph.link_node(document, &address, cell, engine);
            }
        }
        Ok(graph)
    }

    /// Re-links only the nodes in `cells`, whose formulas or existence an
    /// edit changed (ADR 0055: "the pass rebuilds only those nodes'
    /// edges"). A cell that no longer has a formula loses its edges.
    pub fn update(
        &mut self,
        document: &Document,
        cells: &[CellAddress],
        engine: &mut dyn FormulaEngine,
    ) {
        for address in cells {
            let address = normalize_address(document, address);
            self.unlink(&address);
            if let Some(cell) = local_cell(document, &address)
                && cell.formula.is_some()
            {
                self.link_node(document, &address, cell, engine);
            }
        }
    }

    /// The cells `cell`'s formula reads (and the rung an inherited
    /// formula comes from). An unknown cell reads nothing.
    pub fn dependencies(&self, cell: &CellAddress) -> Result<Vec<CellAddress>, Error> {
        Ok(self.deps.get(cell).cloned().unwrap_or_default())
    }

    /// The cells whose formulas read `cell`. An unknown cell has no
    /// dependents.
    pub fn dependents(&self, cell: &CellAddress) -> Result<Vec<CellAddress>, Error> {
        Ok(self.rdeps.get(cell).cloned().unwrap_or_default())
    }

    /// The order to evaluate `dirty` and their transitive dependents in
    /// (dependencies first, ADR 0055).
    ///
    /// Cells that lie on a dependency cycle are **not** in the order:
    /// they keep their cached values and the caller reports them
    /// (`DepGraph::order_and_cycles`). Ties break on the address order
    /// of [`CellAddress`], so the same input always gives the same
    /// order.
    pub fn evaluation_order(&self, dirty: &[CellAddress]) -> Result<Vec<CellAddress>, Error> {
        Ok(self.order_and_cycles(dirty).0)
    }

    /// The evaluation order and the cycles the walk cut, in the order
    /// the walk found them (each cycle lists its cells in walk order).
    pub(crate) fn order_and_cycles(
        &self,
        dirty: &[CellAddress],
    ) -> (Vec<CellAddress>, Vec<Vec<CellAddress>>) {
        let reachable = self.reachable(dirty);
        let cycles = self.find_cycles(&reachable);
        let mut cut: BTreeSet<CellAddress> = BTreeSet::new();
        for cycle in &cycles {
            for cell in cycle {
                cut.insert(cell.clone());
            }
        }
        let nodes: BTreeSet<CellAddress> = reachable.difference(&cut).cloned().collect();
        (self.topological_order(&nodes), cycles)
    }

    /// The cells and their transitive dependents (`dirty` included).
    pub(crate) fn transitive_dependents(&self, dirty: &[CellAddress]) -> BTreeSet<CellAddress> {
        let mut seen: BTreeSet<CellAddress> = BTreeSet::new();
        let mut queue: Vec<CellAddress> = Vec::new();
        for cell in dirty {
            if seen.insert(cell.clone()) {
                queue.push(cell.clone());
            }
        }
        while let Some(next) = queue.pop() {
            if let Some(dependents) = self.rdeps.get(&next) {
                for dependent in dependents {
                    if seen.insert(dependent.clone()) {
                        queue.push(dependent.clone());
                    }
                }
            }
        }
        seen
    }

    /// The cells and their transitive dependents: what one pass covers.
    fn reachable(&self, dirty: &[CellAddress]) -> BTreeSet<CellAddress> {
        self.transitive_dependents(dirty)
    }

    /// Links one formula-bearing cell: its formula's references, and the
    /// defining rung when the formula is inherited.
    fn link_node(
        &mut self,
        document: &Document,
        address: &CellAddress,
        cell: &Cell,
        engine: &mut dyn FormulaEngine,
    ) {
        let inherited = cell.formula.as_deref() == Some(INHERIT_FORMULA);
        let effective = effective(document, address).ok().flatten();
        let text = if inherited {
            effective.as_ref().map(|found| found.formula.clone())
        } else {
            cell.formula.clone()
        };
        if let Some(text) = text
            && let Ok(references) = engine.references(&Formula::new(text))
        {
            for reference in &references {
                if let Some(dependency) = resolve_ref_address(document, address.sheet, reference) {
                    self.add_edge(address.clone(), dependency);
                }
            }
        }
        if inherited
            && let Some(source) = effective.map(|found| found.source)
            && let Some(rung) = rung_address(document, address, source)
        {
            self.add_edge(address.clone(), rung);
        }
    }

    /// Records one edge `from` depends on `to`, keeping both sides sorted
    /// and duplicate-free.
    fn add_edge(&mut self, from: CellAddress, to: CellAddress) {
        sorted_insert(self.deps.entry(from.clone()).or_default(), to.clone());
        sorted_insert(self.rdeps.entry(to).or_default(), from);
    }

    /// Drops one node's outgoing edges and their reverse edges.
    fn unlink(&mut self, node: &CellAddress) {
        if let Some(old) = self.deps.remove(node) {
            for dependency in old {
                if let Some(back) = self.rdeps.get_mut(&dependency) {
                    back.retain(|entry| entry != node);
                }
            }
        }
    }

    /// The cells on a dependency cycle inside `reachable`, one cycle per
    /// back edge the depth-first walk finds (deterministic: the walk
    /// visits nodes and dependencies in address order).
    fn find_cycles(&self, reachable: &BTreeSet<CellAddress>) -> Vec<Vec<CellAddress>> {
        // 0 = unseen, 1 = on the walk stack, 2 = done.
        let mut color: BTreeMap<CellAddress, u8> = BTreeMap::new();
        let mut cycles = Vec::new();
        for start in reachable {
            if color.contains_key(start) {
                continue;
            }
            let mut stack: Vec<(CellAddress, Vec<CellAddress>, usize)> = Vec::new();
            color.insert(start.clone(), 1);
            stack.push((start.clone(), self.local_deps(start, reachable), 0));
            while let Some((node, deps, next)) = stack.pop() {
                if let Some(dependency) = deps.get(next).cloned() {
                    stack.push((node, deps, next.saturating_add(1)));
                    match color.get(&dependency).copied().unwrap_or(0) {
                        0 => {
                            color.insert(dependency.clone(), 1);
                            let more = self.local_deps(&dependency, reachable);
                            stack.push((dependency, more, 0));
                        }
                        1 => {
                            // A back edge closes a cycle: the stack from
                            // the dependency up to the current node.
                            let mut cycle = Vec::new();
                            let mut walking = false;
                            for (member, _, _) in &stack {
                                if *member == dependency {
                                    walking = true;
                                }
                                if walking {
                                    cycle.push(member.clone());
                                }
                            }
                            cycles.push(cycle);
                        }
                        _ => {}
                    }
                } else {
                    color.insert(node, 2);
                }
            }
        }
        cycles
    }

    /// The dependencies of `node` that stay inside `reachable`, sorted.
    fn local_deps(
        &self,
        node: &CellAddress,
        reachable: &BTreeSet<CellAddress>,
    ) -> Vec<CellAddress> {
        self.deps
            .get(node)
            .map(|deps| {
                deps.iter()
                    .filter(|dependency| reachable.contains(*dependency))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// A topological order of `nodes` (dependencies first), ties broken
    /// by address. `nodes` must hold no cycle.
    fn topological_order(&self, nodes: &BTreeSet<CellAddress>) -> Vec<CellAddress> {
        let mut waiting: BTreeMap<CellAddress, usize> = BTreeMap::new();
        for node in nodes {
            let count = self.local_deps(node, nodes).len();
            waiting.insert(node.clone(), count);
        }
        let mut ready: BTreeSet<CellAddress> = waiting
            .iter()
            .filter(|(_, count)| **count == 0)
            .map(|(node, _)| node.clone())
            .collect();
        let mut order = Vec::with_capacity(nodes.len());
        while let Some(next) = ready.pop_first() {
            order.push(next.clone());
            if let Some(dependents) = self.rdeps.get(&next) {
                for dependent in dependents {
                    if !nodes.contains(dependent) {
                        continue;
                    }
                    if let Some(count) = waiting.get_mut(dependent) {
                        *count = count.saturating_sub(1);
                        if *count == 0 {
                            ready.insert(dependent.clone());
                        }
                    }
                }
            }
        }
        // Defensive: anything left could only be a cut cycle; keep the
        // order total and deterministic.
        for (node, count) in waiting {
            if count > 0 && !order.contains(&node) {
                order.push(node);
            }
        }
        order
    }
}

/// Inserts `value` into a sorted, duplicate-free list.
fn sorted_insert(list: &mut Vec<CellAddress>, value: CellAddress) {
    match list.binary_search(&value) {
        Ok(_) => {}
        Err(at) => list.insert(at, value),
    }
}

/// What defines a cell: the formula that will be evaluated and the state
/// of the cell's own entry (ADR 0054).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EffectiveCell {
    /// The formula that defines the cell: its own, or the inherited
    /// rung's as written.
    pub formula: String,
    /// True when the cell's own entry holds the formula. False for an
    /// inherited one (`Inh` or absent locally).
    pub own: bool,
    /// The cell's own entry's formula text (`"Inh"` included), when the
    /// entry exists.
    pub local_formula: Option<String>,
    /// The cell's own entry's cached value, when the entry exists.
    pub local_value: Option<ferrogram_model::CellValue>,
    /// Which rung defines the formula (ADR 0013).
    pub source: ValueSource,
}

/// The formula that defines `address`, resolved through the rungs of
/// ADR 0013 (local, master, style chain, section default, hard default).
///
/// `Ok(None)` when no rung has a formula: a literal or an absent cell has
/// nothing to evaluate. `Err` when the address names no sheet.
pub(crate) fn effective(
    document: &Document,
    address: &CellAddress,
) -> Result<Option<EffectiveCell>, ferrogram_model::Error> {
    let local = local_cell(document, address);
    let defined = match (address.sheet, &address.section, &address.row) {
        (SheetRef::Shape(id), None, None) => defined(document.resolve(id, &address.cell)?),
        (SheetRef::Page(id), None, None) => defined(document.resolve_page_cell(id, &address.cell)?),
        (sheet, Some(section), Some(row)) => {
            let position = row_position(document, sheet, section, *row)
                .ok_or(ferrogram_model::Error::BadCell("no such row"))?;
            defined(document.resolve_row_cell(sheet, section.as_str(), position, &address.cell)?)
        }
        (sheet, Some(section), None) => section_rung(document, sheet, section, &address.cell),
        (_, None, Some(_)) => None,
    };
    let Some((formula, source)) = defined else {
        return Ok(None);
    };
    Ok(Some(EffectiveCell {
        formula,
        own: source == ValueSource::Local,
        local_formula: local.and_then(|cell| cell.formula.clone()),
        local_value: local.and_then(|cell| cell.value.clone()),
        source,
    }))
}

/// The formula a resolution found and the rung that defines it, when one
/// does (a default or an `Inh` cell defines nothing, ADR 0054).
fn defined(resolved: Resolved<'_>) -> Option<(String, ValueSource)> {
    let formula = resolved.formula.map(str::to_owned)?;
    Some((formula, resolved.source))
}

/// The rung that defines a **section-level** cell (a section, no row:
/// the `NoFill` family of `Geometry`, ADR 0019) and its formula: the
/// section's own cell first, then the corresponding cell of the master
/// chain, with [`rung_address`]'s rule (the master's first top-level
/// shape). A cell whose formula is `Inh` or absent defines nothing
/// (ADR 0054), and there is no style rung for a repeated-section cell
/// (ADR 0019 decision 5).
fn section_rung(
    document: &Document,
    sheet: SheetRef,
    section: &SectionName,
    cell: &CellName,
) -> Option<(String, ValueSource)> {
    let own = sheet_of(document, sheet)
        .and_then(|data| find_section(data, section))
        .and_then(|found| find_cell(&found.cells, cell))
        .and_then(defining_formula);
    if let Some(formula) = own {
        return Some((formula.to_owned(), ValueSource::Local));
    }
    let mut master_id = match sheet {
        SheetRef::Shape(id) => document.shape(id)?.master,
        SheetRef::Page(_) => None,
    };
    let mut visited: Vec<MasterId> = Vec::new();
    while let Some(master) = master_id {
        // A cyclic master chain is cut (ADR 0013), never looped on.
        if visited.contains(&master) {
            break;
        }
        visited.push(master);
        let rung = document.shape(*document.master(master)?.shapes.first()?)?;
        // A rung that has no such section or cell defines nothing; the
        // chain continues at that rung's own master.
        let defined = find_section(&rung.sheet, section)
            .and_then(|found| find_cell(&found.cells, cell))
            .and_then(defining_formula);
        if let Some(formula) = defined {
            return Some((formula.to_owned(), ValueSource::Master(master)));
        }
        master_id = rung.master;
    }
    None
}

/// The formula that defines a cell: a present formula that is not the
/// `Inh` marker (ADR 0054: `Inh` means "the next rung's formula").
fn defining_formula(cell: &Cell) -> Option<&str> {
    match cell.formula.as_deref() {
        Some(formula) if formula != INHERIT_FORMULA => Some(formula),
        _ => None,
    }
}

/// The cell's own entry at `address`, before any inheritance lookup.
pub(crate) fn local_cell<'d>(document: &'d Document, address: &CellAddress) -> Option<&'d Cell> {
    let sheet = sheet_of(document, address.sheet)?;
    match (&address.section, &address.row) {
        (None, None) => find_cell(&sheet.cells, &address.cell),
        (Some(section), None) => find_cell(&find_section(sheet, section)?.cells, &address.cell),
        (Some(section), Some(row)) => {
            let section = find_section(sheet, section)?;
            find_cell(&find_row(section, *row)?.cells, &address.cell)
        }
        (None, Some(_)) => None,
    }
}

/// The row target of `address` filled in both ways the model matches
/// rows: by `IX` and by position. The cell and section names take the
/// model's spelling when a case-only difference matches.
pub(crate) fn normalize_address(document: &Document, address: &CellAddress) -> CellAddress {
    let mut out = address.clone();
    if let Some(cell) = local_cell(document, address) {
        out.cell = cell.name.clone();
    }
    if let (Some(section), Some(row)) = (&address.section, address.row)
        && (row.position.is_none() || row.ix.is_none())
        && let Some(sheet) = sheet_of(document, address.sheet)
        && let Some(found) = find_section(sheet, section)
        && let Some((position, target)) = find_row_with_position(found, row)
    {
        out.row = Some(RowTarget {
            position: Some(position),
            ix: target,
        });
    }
    out
}

/// The cell `reference` names on `sheet`, with the model's spellings
/// (ADR 0054's reference forms).
///
/// A **read** may come from a master rung behind the target sheet (the
/// resolver's rungs of ADR 0013), so the search walks the target sheet
/// and then its rungs, preferring the rung whose cell defines the value.
/// Best effort: a reference whose form needs the full resolver (a
/// master's named sub-shape, a page path) resolves to no address and so
/// adds no graph edge. `None` is not an error.
pub(crate) fn resolve_ref_address(
    document: &Document,
    sheet: SheetRef,
    reference: &CellRef,
) -> Option<CellAddress> {
    resolve_address(document, sheet, reference, true)
}

/// The cell a `SETF` write lands on (ADR 0056): the target sheet's own
/// cell, never a master rung's — a write is the local override.
pub(crate) fn resolve_write_address(
    document: &Document,
    sheet: SheetRef,
    reference: &CellRef,
) -> Option<CellAddress> {
    resolve_address(document, sheet, reference, false)
}

/// The addressing behind [`resolve_ref_address`] and
/// [`resolve_write_address`]: `walk_rungs` extends the search to the
/// master rungs behind the target sheet (reads inherit, writes do not).
/// The name's meaning is the one rule of [`crate::names`] on both paths.
fn resolve_address(
    document: &Document,
    sheet: SheetRef,
    reference: &CellRef,
    walk_rungs: bool,
) -> Option<CellAddress> {
    let target = match reference.sheets.as_slice() {
        [] => sheet,
        [one] => sheet_part(document, sheet, one)?,
        _ => return None,
    };
    let (section, written) = crate::resolve::split_names(reference);
    let name = written.first()?;
    let explicit = written.get(1).map(String::as_str);
    let mut fallback = None;
    for (candidate, data) in candidate_sheets(document, target, walk_rungs) {
        let found = match section {
            Some(section) => section_address(data, candidate, section, name, explicit),
            None => bare_address(data, candidate, name),
        };
        let Some(address) = found else {
            continue;
        };
        // The rung whose cell defines the value is the one a read takes
        // (ADR 0054); a rung that merely carries the cell is the
        // fallback.
        if local_cell(document, &address).is_some_and(crate::resolve::defines) {
            return Some(address);
        }
        fallback = fallback.or(Some(address));
    }
    fallback.or_else(|| {
        // A bare name no sheet holds still names a cell (the node of a
        // table default), on the target sheet.
        section
            .is_none()
            .then(|| CellAddress::new(target, names::spelled_cell(name)))
    })
}

/// The sheets a reference may name, in rung order (ADR 0013): the target
/// sheet first, then the master rungs behind it when `walk_rungs`.
fn candidate_sheets<'d>(
    document: &'d Document,
    target: SheetRef,
    walk_rungs: bool,
) -> Vec<(SheetRef, &'d Sheet)> {
    let mut out = Vec::new();
    if let Some(data) = sheet_of(document, target) {
        out.push((target, data));
    }
    if walk_rungs && let SheetRef::Shape(shape) = target {
        for (_, rung) in crate::resolve::master_rungs(document, shape) {
            if let Some(data) = sheet_of(document, SheetRef::Shape(rung)) {
                out.push((SheetRef::Shape(rung), data));
            }
        }
    }
    out
}

/// The address of the cell a section-form name names on one sheet
/// (`Char.Size`, `Prop.Cost`, `Geometry1.X2`): the section's row shape
/// decides the name's meaning (the one rule of [`crate::names`]), and a
/// section-level cell is addressed as one.
fn section_address(
    data: &Sheet,
    sheet: SheetRef,
    section: &str,
    name: &str,
    explicit: Option<&str>,
) -> Option<CellAddress> {
    let (base, instance) = names::split_section_name(section);
    let canon = names::canonical_section(base.as_str());
    let found = names::match_section(data, canon.as_str(), instance)?;
    let kind = names::row_kind(canon.as_str(), Some(found), name);
    let slot = names::section_slot(data, canon.as_str(), instance, &kind, explicit)?;
    slot_address(data, sheet, slot)
}

/// The address of the cell `name` names on one sheet without a section:
/// a direct cell first, then a section cell, then a row cell (the
/// model's own name search order).
fn bare_address(data: &Sheet, sheet: SheetRef, name: &str) -> Option<CellAddress> {
    let (_, slot) = names::find_entry(data, name)?;
    slot_address(data, sheet, slot)
}

/// The address of the cell at `slot`, with the model's spellings (names
/// match case-insensitively, ADR 0054).
fn slot_address(data: &Sheet, sheet: SheetRef, slot: names::Slot) -> Option<CellAddress> {
    match slot {
        names::Slot::Direct(index) => data
            .cells
            .get(index)
            .map(|cell| CellAddress::new(sheet, cell.name.clone())),
        names::Slot::SectionLevel(section, cell) => {
            let section = data.sections.get(section)?;
            let cell = section.cells.get(cell)?;
            Some(CellAddress {
                sheet,
                section: Some(section.name.clone()),
                row: None,
                cell: cell.name.clone(),
            })
        }
        names::Slot::Row(section, position, cell) => {
            let section = data.sections.get(section)?;
            let row = section.rows.get(position)?;
            let cell = row.cells.get(cell)?;
            Some(CellAddress::in_row(
                sheet,
                section.name.clone(),
                RowTarget {
                    position: Some(position),
                    ix: row.ix,
                },
                cell.name.clone(),
            ))
        }
    }
}

/// The cell the rung `source` defines the formula from (edge 2 of
/// ADR 0055): the master's shape for a master rung.
///
/// A style rung has no address ([`SheetRef`] names shapes and pages
/// only), so a style edit does not dirty instances through the graph.
/// A master with several top-level shapes approximates the rung with its
/// first shape.
pub(crate) fn rung_address(
    document: &Document,
    address: &CellAddress,
    source: ValueSource,
) -> Option<CellAddress> {
    let ValueSource::Master(master) = source else {
        return None;
    };
    let shape = document.master(master)?.shapes.first()?;
    let mut out = address.clone();
    out.sheet = SheetRef::Shape(*shape);
    Some(out)
}

/// The sheet part of a cross-sheet reference: `ThePage`, `Sheet.8`
/// (a shape's Visio `ID`) or a shape's name (`Star`, `Executive.2`).
fn sheet_part(document: &Document, sheet: SheetRef, part: &str) -> Option<SheetRef> {
    if part.eq_ignore_ascii_case("ThePage") {
        return match sheet {
            SheetRef::Page(id) => Some(SheetRef::Page(id)),
            SheetRef::Shape(id) => document.page_of(id).map(SheetRef::Page),
        };
    }
    let (base, visio_id) = split_visio_id(part);
    let page = match sheet {
        SheetRef::Page(id) => Some(id),
        SheetRef::Shape(id) => document.page_of(id),
    }?;
    if base.eq_ignore_ascii_case("Sheet") {
        return document
            .shape_by_visio_id(page, visio_id?)
            .map(SheetRef::Shape);
    }
    if base.contains('[') || base.contains(']') {
        return None;
    }
    find_shape_by_name(document, page, base, visio_id).map(SheetRef::Shape)
}

/// Finds a shape of `page` by `NameU`/`Name` (case-insensitive) and,
/// when given, by Visio `ID`.
fn find_shape_by_name(
    document: &Document,
    page: ferrogram_model::PageId,
    name: &str,
    visio_id: Option<i32>,
) -> Option<ferrogram_model::ShapeId> {
    let page = document.page(page)?;
    let mut queue: Vec<ferrogram_model::ShapeId> = page.shapes.clone();
    let mut seen: BTreeSet<u32> = BTreeSet::new();
    while let Some(id) = queue.pop() {
        if !seen.insert(id.index) {
            continue;
        }
        let Some(shape) = document.shape(id) else {
            continue;
        };
        if visio_id.is_none_or(|wanted| wanted == shape.visio_id)
            && (shape.name_u.eq_ignore_ascii_case(name) || shape.name.eq_ignore_ascii_case(name))
        {
            return Some(id);
        }
        queue.extend(shape.children.iter().copied());
    }
    None
}

/// The row at `row` in `section` with its position and `IX`.
fn find_row_with_position(section: &Section, row: RowTarget) -> Option<(usize, Option<i32>)> {
    if let Some(ix) = row.ix
        && let Some(position) = section.rows.iter().position(|found| found.ix == Some(ix))
    {
        return Some((position, Some(ix)));
    }
    let position = row.position?;
    let found = section.rows.get(position)?;
    Some((position, found.ix))
}

/// The row at `row` in `section`, matched the way the model matches
/// rows: by `IX` first, then by position.
fn find_row(section: &Section, row: RowTarget) -> Option<&Row> {
    find_row_with_position(section, row).and_then(|(position, _)| section.rows.get(position))
}

/// The position of `row` in `section`, or `None`.
fn row_position(
    document: &Document,
    sheet: SheetRef,
    section: &SectionName,
    row: RowTarget,
) -> Option<usize> {
    let data = sheet_of(document, sheet)?;
    find_row_with_position(find_section(data, section)?, row).map(|(position, _)| position)
}

/// The first section of `sheet` with this name (case-insensitive when an
/// exact match is missing).
fn find_section<'a>(sheet: &'a Sheet, name: &SectionName) -> Option<&'a Section> {
    let exact = sheet.sections.iter().find(|section| section.name == *name);
    exact.or_else(|| {
        sheet
            .sections
            .iter()
            .find(|section| section.name.as_str().eq_ignore_ascii_case(name.as_str()))
    })
}

/// The cell with this name (case-insensitive when an exact match is
/// missing).
fn find_cell<'a>(cells: &'a [Cell], name: &CellName) -> Option<&'a Cell> {
    let exact = cells.iter().find(|cell| cell.name == *name);
    exact.or_else(|| {
        cells
            .iter()
            .find(|cell| cell.name.as_str().eq_ignore_ascii_case(name.as_str()))
    })
}

/// The sheet a [`SheetRef`] names.
fn sheet_of(document: &Document, sheet: SheetRef) -> Option<&Sheet> {
    match sheet {
        SheetRef::Shape(id) => document.shape(id).map(|shape| &shape.sheet),
        SheetRef::Page(id) => document.page(id).map(|page| &page.sheet),
    }
}

/// Every sheet of the document with its address: page sheets first, then
/// every shape's sheet (master shapes included).
fn sheet_refs(document: &Document) -> Vec<(SheetRef, &Sheet)> {
    let mut out = Vec::new();
    for page in &document.pages {
        out.push((SheetRef::Page(page.id), &page.sheet));
    }
    for shape in &document.shapes {
        out.push((SheetRef::Shape(shape.id), &shape.sheet));
    }
    out
}

/// Every cell of `data` that carries a formula, with its address.
fn formula_cells(sheet: SheetRef, data: &Sheet) -> Vec<(CellAddress, &Cell)> {
    let mut out = Vec::new();
    for cell in &data.cells {
        if cell.formula.is_some() {
            out.push((CellAddress::new(sheet, cell.name.clone()), cell));
        }
    }
    for section in &data.sections {
        let section_name = section.name.clone();
        for cell in &section.cells {
            if cell.formula.is_some() {
                let mut address = CellAddress::new(sheet, cell.name.clone());
                address.section = Some(section_name.clone());
                out.push((address, cell));
            }
        }
        for (position, row) in section.rows.iter().enumerate() {
            let target = RowTarget {
                position: Some(position),
                ix: row.ix,
            };
            for cell in &row.cells {
                if cell.formula.is_some() {
                    out.push((
                        CellAddress::in_row(sheet, section_name.clone(), target, cell.name.clone()),
                        cell,
                    ));
                }
            }
        }
    }
    out
}

/// The name and Visio `ID` of a sheet part (`Executive.2` is `Executive`
/// with `ID` 2).
fn split_visio_id(part: &str) -> (&str, Option<i32>) {
    if let Some(dot) = part.rfind('.')
        && let Some(head) = part.get(..dot)
        && let Some(id) = part
            .get(dot.saturating_add(1)..)
            .and_then(|digits| digits.parse::<i32>().ok())
    {
        return (head, Some(id));
    }
    (part, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recalc::testing::TestEngine;
    use ferrogram_geom::Unit;
    use ferrogram_model::{Cell, CellValue, Master, Page, PageId, Preserved, Shape, ShapeId};

    fn cell(name: &str, formula: Option<&str>, value: Option<CellValue>) -> Cell {
        Cell {
            name: CellName::new(name),
            formula: formula.map(str::to_owned),
            value,
            unit: Unit::Unitsless,
            preserved: Preserved::new(),
        }
    }

    fn shape_with(visio_id: i32, cells: Vec<Cell>) -> Shape {
        let mut shape = Shape::new(ShapeId { index: 0 }, visio_id);
        shape.sheet.cells = cells;
        shape
    }

    fn one_shape_document(cells: Vec<Cell>) -> (Document, CellAddress) {
        let mut doc = Document::new();
        let shape = doc.add_shape(shape_with(1, cells));
        doc.pages.push(Page::new(PageId { index: 0 }, 1));
        if let Some(page) = doc.pages.first_mut() {
            page.shapes.push(shape);
        }
        let address = CellAddress::new(SheetRef::Shape(shape), CellName::from_static("Alpha"));
        (doc, address)
    }

    fn address() -> CellAddress {
        CellAddress::new(
            SheetRef::Page(PageId { index: 0 }),
            CellName::from_static("PinX"),
        )
    }

    #[test]
    fn an_address_holds_its_parts() {
        let a = address();
        assert_eq!(a.cell.as_str(), "PinX");
        assert_eq!(a.section, None);
        assert_eq!(a.row, None);
    }

    #[test]
    fn an_address_can_name_a_row_cell() {
        let a = CellAddress::in_row(
            SheetRef::Page(PageId { index: 0 }),
            SectionName::from_static("Geometry"),
            RowTarget::at(1),
            CellName::from_static("X"),
        );
        assert_eq!(a.section, Some(SectionName::from_static("Geometry")));
        assert_eq!(a.row, Some(RowTarget::at(1)));
        assert_eq!(a.cell.as_str(), "X");
    }

    #[test]
    fn addresses_order_by_sheet_section_row_and_name() {
        let page = CellAddress::new(
            SheetRef::Page(PageId { index: 0 }),
            CellName::from_static("Alpha"),
        );
        let shape = CellAddress::new(
            SheetRef::Shape(ShapeId { index: 0 }),
            CellName::from_static("Alpha"),
        );
        let later = CellAddress::new(
            SheetRef::Page(PageId { index: 1 }),
            CellName::from_static("Alpha"),
        );
        assert!(page < later);
        assert!(page < shape);
        assert_eq!(page.partial_cmp(&shape), Some(Ordering::Less));
        assert_eq!(page.cmp(&page), Ordering::Equal);
    }

    #[test]
    fn an_empty_graph_answers_empty_lists() {
        let g = DepGraph::new();
        assert_eq!(DepGraph::default(), g);
        assert_eq!(g.dependencies(&address()), Ok(Vec::new()));
        assert_eq!(g.dependents(&address()), Ok(Vec::new()));
        assert_eq!(g.evaluation_order(&[address()]), Ok(vec![address()]));
    }

    #[test]
    fn build_records_the_formula_dependencies() {
        let (doc, alpha) = one_shape_document(vec![
            cell("Alpha", Some("Beta+1"), Some(CellValue::Number(0.0))),
            cell("Beta", Some("0"), Some(CellValue::Number(0.0))),
        ]);
        let mut engine = TestEngine::new();
        let g = DepGraph::build_with(&doc, &mut engine).expect("build");
        let beta = CellAddress::new(alpha.sheet, CellName::from_static("Beta"));
        assert_eq!(g.dependencies(&alpha), Ok(vec![beta.clone()]));
        assert_eq!(g.dependents(&beta), Ok(vec![alpha.clone()]));
        assert_eq!(g.dependents(&alpha), Ok(Vec::new()));
    }

    #[test]
    fn a_literal_cell_is_no_node() {
        let (doc, alpha) =
            one_shape_document(vec![cell("Alpha", None, Some(CellValue::Number(3.0)))]);
        let mut engine = TestEngine::new();
        let g = DepGraph::build_with(&doc, &mut engine).expect("build");
        assert_eq!(g.dependencies(&alpha), Ok(Vec::new()));
    }

    #[test]
    fn an_inherited_formula_depends_on_its_rung_and_reads_its_references() {
        let mut doc = Document::new();
        let master_shape = doc.add_shape(shape_with(
            1,
            vec![cell("Alpha", Some("Beta*2"), Some(CellValue::Number(4.0)))],
        ));
        doc.masters
            .push(Master::new(ferrogram_model::MasterId { index: 0 }, 1));
        if let Some(master) = doc.masters.first_mut() {
            master.shapes.push(master_shape);
        }
        let instance = doc.add_shape({
            let mut shape = shape_with(
                2,
                vec![cell("Alpha", Some("Inh"), Some(CellValue::Number(3.0)))],
            );
            shape.master = Some(ferrogram_model::MasterId { index: 0 });
            shape
        });
        let instance_address =
            CellAddress::new(SheetRef::Shape(instance), CellName::from_static("Alpha"));
        let mut engine = TestEngine::new();
        let g = DepGraph::build_with(&doc, &mut engine).expect("build");
        let rung = CellAddress::new(
            SheetRef::Shape(master_shape),
            CellName::from_static("Alpha"),
        );
        let beta = CellAddress::new(SheetRef::Shape(instance), CellName::from_static("Beta"));
        let deps = g.dependencies(&instance_address).expect("deps");
        assert!(deps.contains(&rung), "edge to the defining rung: {deps:?}");
        assert!(
            deps.contains(&beta),
            "edge to the referenced cell: {deps:?}"
        );
    }

    #[test]
    fn update_replaces_the_edges_of_a_changed_formula() {
        let (mut doc, alpha) = one_shape_document(vec![
            cell("Alpha", Some("Beta+1"), Some(CellValue::Number(0.0))),
            cell("Beta", Some("0"), Some(CellValue::Number(0.0))),
            cell("Gamma", Some("0"), Some(CellValue::Number(0.0))),
        ]);
        let mut engine = TestEngine::new();
        let mut g = DepGraph::build_with(&doc, &mut engine).expect("build");
        let gamma = CellAddress::new(alpha.sheet, CellName::from_static("Gamma"));
        if let Some(found) = doc
            .shape_mut(ShapeId { index: 0 })
            .and_then(|shape| shape.sheet.cells.first_mut())
        {
            found.formula = Some(String::from("Gamma+1"));
        }
        g.update(&doc, std::slice::from_ref(&alpha), &mut engine);
        assert_eq!(g.dependencies(&alpha), Ok(vec![gamma]));
    }

    #[test]
    fn update_drops_the_edges_of_a_cell_that_lost_its_formula() {
        let (mut doc, alpha) = one_shape_document(vec![
            cell("Alpha", Some("Beta+1"), Some(CellValue::Number(0.0))),
            cell("Beta", Some("0"), Some(CellValue::Number(0.0))),
        ]);
        let mut engine = TestEngine::new();
        let mut g = DepGraph::build_with(&doc, &mut engine).expect("build");
        if let Some(found) = doc
            .shape_mut(ShapeId { index: 0 })
            .and_then(|shape| shape.sheet.cells.first_mut())
        {
            found.formula = None;
        }
        g.update(&doc, std::slice::from_ref(&alpha), &mut engine);
        assert_eq!(g.dependencies(&alpha), Ok(Vec::new()));
    }

    #[test]
    fn evaluation_order_puts_dependencies_first() {
        let (doc, alpha) = one_shape_document(vec![
            cell("Alpha", Some("Beta+1"), Some(CellValue::Number(0.0))),
            cell("Beta", Some("Gamma+1"), Some(CellValue::Number(0.0))),
            cell("Gamma", Some("0"), Some(CellValue::Number(0.0))),
        ]);
        let mut engine = TestEngine::new();
        let g = DepGraph::build_with(&doc, &mut engine).expect("build");
        let beta = CellAddress::new(alpha.sheet, CellName::from_static("Beta"));
        let gamma = CellAddress::new(alpha.sheet, CellName::from_static("Gamma"));
        // Dirtying the source recomputes its dependents, dependencies
        // first: Gamma, then Beta, then Alpha.
        let order = g
            .evaluation_order(std::slice::from_ref(&gamma))
            .expect("order");
        assert_eq!(order, vec![gamma, beta, alpha]);
    }

    #[test]
    fn a_diamond_orders_the_shared_dependency_once() {
        let (doc, alpha) = one_shape_document(vec![
            cell("Alpha", Some("0"), Some(CellValue::Number(0.0))),
            cell("Beta", Some("Alpha+1"), Some(CellValue::Number(0.0))),
            cell("Gamma", Some("Alpha+2"), Some(CellValue::Number(0.0))),
            cell("Delta", Some("Beta+Gamma"), Some(CellValue::Number(0.0))),
        ]);
        let mut engine = TestEngine::new();
        let g = DepGraph::build_with(&doc, &mut engine).expect("build");
        let beta = CellAddress::new(alpha.sheet, CellName::from_static("Beta"));
        let gamma = CellAddress::new(alpha.sheet, CellName::from_static("Gamma"));
        let delta = CellAddress::new(alpha.sheet, CellName::from_static("Delta"));
        let order = g
            .evaluation_order(std::slice::from_ref(&alpha))
            .expect("order");
        assert_eq!(order, vec![alpha.clone(), beta, gamma, delta]);
        // The shared dependency appears once, before both paths.
        let once = order.iter().filter(|entry| **entry == alpha).count();
        assert_eq!(once, 1);
    }

    #[test]
    fn a_cycle_is_cut_out_of_the_order_and_named() {
        let (doc, alpha) = one_shape_document(vec![
            cell("Alpha", Some("Beta+1"), Some(CellValue::Number(1.0))),
            cell("Beta", Some("Alpha+1"), Some(CellValue::Number(2.0))),
        ]);
        let mut engine = TestEngine::new();
        let g = DepGraph::build_with(&doc, &mut engine).expect("build");
        let (order, cycles) = g.order_and_cycles(std::slice::from_ref(&alpha));
        assert!(
            order.is_empty(),
            "cycle cells keep cached values: {order:?}"
        );
        assert_eq!(cycles.len(), 1);
        let cycle = cycles.first().cloned().unwrap_or_default();
        assert_eq!(cycle.len(), 2);
        assert!(cycle.contains(&alpha));
    }

    #[test]
    fn a_self_reference_is_a_cycle_of_one() {
        let (doc, alpha) = one_shape_document(vec![cell(
            "Alpha",
            Some("Alpha+1"),
            Some(CellValue::Number(1.0)),
        )]);
        let mut engine = TestEngine::new();
        let g = DepGraph::build_with(&doc, &mut engine).expect("build");
        let (order, cycles) = g.order_and_cycles(std::slice::from_ref(&alpha));
        assert!(order.is_empty());
        assert_eq!(cycles, vec![vec![alpha]]);
    }

    #[test]
    fn evaluation_order_is_deterministic() {
        let (doc, alpha) = one_shape_document(vec![
            cell("Alpha", Some("0"), Some(CellValue::Number(0.0))),
            cell("Beta", Some("Alpha+1"), Some(CellValue::Number(0.0))),
            cell("Gamma", Some("Alpha+2"), Some(CellValue::Number(0.0))),
            cell("Delta", Some("Alpha+3"), Some(CellValue::Number(0.0))),
        ]);
        let mut engine = TestEngine::new();
        let g = DepGraph::build_with(&doc, &mut engine).expect("build");
        let first = g
            .evaluation_order(std::slice::from_ref(&alpha))
            .expect("order");
        let second = g
            .evaluation_order(std::slice::from_ref(&alpha))
            .expect("order");
        assert_eq!(first, second);
    }

    #[test]
    fn reference_addresses_resolve_the_written_forms() {
        let (doc, alpha) = one_shape_document(vec![
            cell("Alpha", Some("0"), Some(CellValue::Number(0.0))),
            cell("Beta", Some("0"), Some(CellValue::Number(0.0))),
        ]);
        let sheet = alpha.sheet;
        let bare = resolve_ref_address(&doc, sheet, &CellRef::new("Beta"));
        assert_eq!(
            bare,
            Some(CellAddress::new(sheet, CellName::from_static("Beta")))
        );
        let unknown_sheet = resolve_ref_address(
            &doc,
            sheet,
            &CellRef {
                sheets: vec![String::from("Masters[Gear]")],
                section: None,
                names: vec![String::from("Shaft")],
            },
        );
        assert_eq!(unknown_sheet, None);
        let missing_shape = resolve_ref_address(
            &doc,
            sheet,
            &CellRef {
                sheets: vec![String::from("Sheet.9")],
                section: None,
                names: vec![String::from("Width")],
            },
        );
        assert_eq!(missing_shape, None);
        let the_page = resolve_ref_address(
            &doc,
            sheet,
            &CellRef {
                sheets: vec![String::from("ThePage")],
                section: None,
                names: vec![String::from("Width")],
            },
        );
        assert_eq!(
            the_page,
            Some(CellAddress::new(
                SheetRef::Page(PageId { index: 0 }),
                CellName::from_static("Width")
            ))
        );
    }

    #[test]
    fn row_reference_forms_resolve_to_row_cells() {
        let mut doc = Document::new();
        let mut shape = shape_with(1, vec![]);
        shape.sheet.sections.push({
            let mut section = Section::new(SectionName::from_static("Geometry"));
            section.rows.push({
                let mut row = Row::new();
                row.ix = Some(0);
                row.cells
                    .push(cell("X", None, Some(CellValue::Number(0.0))));
                row.cells
                    .push(cell("Y", None, Some(CellValue::Number(0.0))));
                row
            });
            section.rows.push({
                let mut row = Row::new();
                row.ix = Some(1);
                row.cells
                    .push(cell("X", None, Some(CellValue::Number(1.0))));
                row
            });
            section
        });
        let shape = doc.add_shape(shape);
        let sheet = SheetRef::Shape(shape);
        let x2 = resolve_ref_address(
            &doc,
            sheet,
            &CellRef {
                sheets: Vec::new(),
                section: Some(String::from("Geometry1")),
                names: vec![String::from("X1")],
            },
        );
        assert_eq!(
            x2,
            Some(CellAddress::in_row(
                sheet,
                SectionName::from_static("Geometry"),
                RowTarget {
                    position: Some(1),
                    ix: Some(1)
                },
                CellName::from_static("X")
            ))
        );
    }

    #[test]
    fn the_production_build_never_fails_the_document() {
        let (doc, _) = one_shape_document(vec![cell(
            "Alpha",
            Some("Beta+1"),
            Some(CellValue::Number(0.0)),
        )]);
        // With the real engine this is `Ok` once the parser lands
        // (P6-02); while it is a stub, no formula yields edges. Either
        // way the build succeeds and the document is untouched.
        assert!(DepGraph::build(&doc).is_ok());
    }
}
