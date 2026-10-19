//! `insert.*` commands (P5-06).
//!
//! Insert commands: new shapes from the drawing tools and from the Shapes
//! window (ADR 0034, ADR 0046).
//!
//! Argument names are the command contract (task P5-06):
//!
//! - `insert.shape` — `stencil` and `master` name a catalog master, `x`
//!   and `y` are the dropped `PinX`/`PinY` in inches, optional `width`
//!   and `height` override the master size, and optional `glue_to`
//!   (target Visio id) plus `glue_point` (target connection row `IX`)
//!   add one `Connect` record (ADR 0046);
//! - `insert.drawing.rectangle`, `insert.drawing.ellipse`,
//!   `insert.drawing.line`, `insert.drawing.text_box` — `x`, `y`,
//!   `width`, `height` in inches. The rectangle, ellipse and text box
//!   use `x`/`y` as the lower-left corner; the line uses them as its
//!   start and `width`/`height` as the signed delta.
//!
//! Every command creates exactly one shape and returns exactly one
//! [`CompositeEdit`](crate::CompositeEdit) step ([`StructuralEdit::insertion`],
//! one undo step, ADR 0040). The new shape becomes the selection.
//!
//! The catalog master is materialised into the document on first use
//! (the same master data the Shapes-window preview builds); later drops
//! reuse it. The materialisation itself is catalog infrastructure, not
//! part of the shape's undo step: undoing a first drop removes the shape
//! (and its glue), leaving an unreferenced master behind.

use ferrogram_geom::{Angle, Point, Size, Transform, Unit};
use ferrogram_model::mutate::VIS_CONNECTION_POINT;
use ferrogram_model::stencils::{self, MasterDef};
use ferrogram_model::{
    Cell, CellName, CellReader, CellValue, Connect, Document, MasterId, Preserved, Row, Section,
    SectionName, Shape, ShapeId, SheetRef,
};

use crate::app::{AppCtx, SessionId};
use crate::error::CommandError;
use crate::{Invocation, Outcome};

use super::edit::{StructuralEdit, document_mut};
use super::{CommandTable, edit_outcome, has_pages, model_error, need_finite, no_document};

/// The implemented `insert.*` commands (task P5-06).
pub(crate) const TABLE: CommandTable = &[
    ("insert.shape", "Insert Shape", has_pages, run_insert_shape),
    (
        "insert.drawing.rectangle",
        "Rectangle",
        has_pages,
        run_rectangle,
    ),
    ("insert.drawing.ellipse", "Ellipse", has_pages, run_ellipse),
    ("insert.drawing.line", "Line", has_pages, run_line),
    (
        "insert.drawing.text_box",
        "Text Box",
        has_pages,
        run_text_box,
    ),
];

/// The kind of drawing shape one `insert.drawing.*` command creates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Drawing {
    /// A filled rectangle.
    Rectangle,
    /// A filled ellipse.
    Ellipse,
    /// A line from `(x, y)` by `(width, height)`.
    Line,
    /// An unfilled text box.
    TextBox,
}

/// `insert.shape`: one master instance at a point (ADR 0046).
pub(super) fn run_insert_shape(
    ctx: &mut AppCtx,
    inv: &Invocation,
) -> Result<Outcome, CommandError> {
    let stencil = need_text(&inv.args, "stencil", "the stencil name")?;
    let master = need_text(&inv.args, "master", "the master name")?;
    let x = need_finite(&inv.args, "x", "drop x in inches")?;
    let y = need_finite(&inv.args, "y", "drop y in inches")?;
    let size = optional_size(&inv.args)?;
    let glue_to = optional_visio_id(&inv.args, "glue_to")?;
    let glue_point = optional_row(&inv.args, "glue_point")?;
    if glue_point.is_some() && glue_to.is_none() {
        return Err(CommandError::invalid_args(
            "glue_point needs a glue_to target",
        ));
    }
    let definition = catalog_master(&stencil, &master)?;
    let (session, page) = active_page(ctx)?;
    let label = format!("Insert {}", definition.name);

    let shape = {
        let doc = document_mut(ctx, session)?;
        let master_id = ensure_catalog_master(doc, &definition)?;
        let shape = doc
            .instantiate_master(master_id, page, Point::new(x, y), size)
            .map_err(model_error)?;
        if let Some(size) = size
            && (size.width != definition.width || size.height != definition.height)
        {
            doc.scale_geometry(shape, Size::new(definition.width, definition.height), size)
                .map_err(model_error)?;
        }
        if let Some(target) = glue_to
            && let Err(error) = add_drop_glue(doc, page, shape, target, glue_point)
        {
            // The drop is one command: a glue failure must not leave the
            // shape behind.
            let _ = doc.detach_shape(shape);
            return Err(error);
        }
        shape
    };

    let edit = {
        let doc = document_mut(ctx, session)?;
        StructuralEdit::insertion(doc, session, label.clone(), shape)?
    };
    if glue_to.is_some() {
        // Glue-on-drop: "any glued connector endpoints on the page
        // re-route" (ADR 0046).
        if let Err(error) = super::connector::reroute_after_edit(ctx, page) {
            ctx.record_warning(format!("connectors were not re-routed: {error}"));
        }
    }
    select_new(ctx, session, page, shape)?;
    ctx.touch_session(session);
    Ok(edit_outcome(label, Vec::new(), Box::new(edit)))
}

/// `insert.drawing.rectangle`: one rectangle.
pub(super) fn run_rectangle(ctx: &mut AppCtx, inv: &Invocation) -> Result<Outcome, CommandError> {
    insert_drawing(ctx, inv, Drawing::Rectangle)
}

/// `insert.drawing.ellipse`: one ellipse.
pub(super) fn run_ellipse(ctx: &mut AppCtx, inv: &Invocation) -> Result<Outcome, CommandError> {
    insert_drawing(ctx, inv, Drawing::Ellipse)
}

/// `insert.drawing.line`: one line from a start point and a delta.
pub(super) fn run_line(ctx: &mut AppCtx, inv: &Invocation) -> Result<Outcome, CommandError> {
    insert_drawing(ctx, inv, Drawing::Line)
}

/// `insert.drawing.text_box`: one text box (the text tool opens it).
pub(super) fn run_text_box(ctx: &mut AppCtx, inv: &Invocation) -> Result<Outcome, CommandError> {
    insert_drawing(ctx, inv, Drawing::TextBox)
}

/// Creates one drawing shape and its insertion undo step.
fn insert_drawing(
    ctx: &mut AppCtx,
    inv: &Invocation,
    drawing: Drawing,
) -> Result<Outcome, CommandError> {
    let x = need_finite(&inv.args, "x", "x in inches")?;
    let y = need_finite(&inv.args, "y", "y in inches")?;
    let (width, height) = match drawing {
        Drawing::Line => (
            need_finite(&inv.args, "width", "line delta x in inches")?,
            need_finite(&inv.args, "height", "line delta y in inches")?,
        ),
        _ => (
            need_finite(&inv.args, "width", "width in inches")?,
            need_finite(&inv.args, "height", "height in inches")?,
        ),
    };
    if drawing == Drawing::Line && width == 0.0 && height == 0.0 {
        return Err(CommandError::invalid_args("the line has zero length"));
    }
    if drawing != Drawing::Line && (width < 0.0 || height < 0.0) {
        return Err(CommandError::invalid_args(
            "width and height must not be negative",
        ));
    }
    let (session, page) = active_page(ctx)?;
    let (label, shape) = {
        let doc = document_mut(ctx, session)?;
        let shape = drawing_shape(drawing, x, y, width, height);
        let shape = doc.insert_shape(page, shape, None).map_err(model_error)?;
        (drawing.label().to_owned(), shape)
    };
    let edit = {
        let doc = document_mut(ctx, session)?;
        StructuralEdit::insertion(doc, session, label.clone(), shape)?
    };
    select_new(ctx, session, page, shape)?;
    ctx.touch_session(session);
    Ok(edit_outcome(label, Vec::new(), Box::new(edit)))
}

impl Drawing {
    /// The undo label and status-bar message.
    fn label(self) -> &'static str {
        match self {
            Drawing::Rectangle => "Draw Rectangle",
            Drawing::Ellipse => "Draw Ellipse",
            Drawing::Line => "Draw Line",
            Drawing::TextBox => "Insert Text Box",
        }
    }
}

/// One drawing shape with its transform, style and geometry cells.
fn drawing_shape(drawing: Drawing, x: f64, y: f64, width: f64, height: f64) -> Shape {
    let (name, name_u) = match drawing {
        Drawing::Rectangle => ("Rectangle", "Rectangle"),
        Drawing::Ellipse => ("Ellipse", "Ellipse"),
        Drawing::Line => ("Line", "Line"),
        Drawing::TextBox => ("Text Box", "TextBox"),
    };
    let mut shape = Shape::new(ShapeId { index: 0 }, 0);
    shape.name = name.to_owned();
    shape.name_u = name_u.to_owned();
    let (pin_x, pin_y, w, h, loc_x, loc_y) = match drawing {
        Drawing::Line => (x, y, width.abs(), height.abs(), 0.0, 0.0),
        _ => (
            x + width * 0.5,
            y + height * 0.5,
            width,
            height,
            width * 0.5,
            height * 0.5,
        ),
    };
    shape.sheet.cells = vec![
        number_cell("Width", w, Unit::Inches),
        number_cell("Height", h, Unit::Inches),
        number_cell("PinX", pin_x, Unit::Inches),
        number_cell("PinY", pin_y, Unit::Inches),
        number_cell("LocPinX", loc_x, Unit::Inches),
        number_cell("LocPinY", loc_y, Unit::Inches),
        number_cell("Angle", 0.0, Unit::Radians),
        bool_cell("FlipX", false),
        bool_cell("FlipY", false),
        number_cell("LineWeight", 0.01, Unit::Inches),
        number_cell("LinePattern", 1.0, Unit::Unitsless),
        number_cell("LineColor", 0.0, Unit::Unitsless),
        number_cell(
            "FillPattern",
            if drawing == Drawing::Line || drawing == Drawing::TextBox {
                0.0
            } else {
                1.0
            },
            Unit::Unitsless,
        ),
        number_cell("FillForegnd", 1.0, Unit::Unitsless),
        number_cell("FillBkgnd", 1.0, Unit::Unitsless),
    ];
    shape.sheet.sections.push(match drawing {
        Drawing::Rectangle | Drawing::TextBox => rectangle_geometry(width, height),
        Drawing::Ellipse => ellipse_geometry(width, height),
        Drawing::Line => line_geometry(width, height),
    });
    shape
}

/// The closed rectangle geometry of a `width` x `height` box.
fn rectangle_geometry(width: f64, height: f64) -> Section {
    let mut section = Section::new(SectionName::from_static("Geometry"));
    for (ix, (kind, x, y)) in [
        ("MoveTo", 0.0, 0.0),
        ("LineTo", width, 0.0),
        ("LineTo", width, height),
        ("LineTo", 0.0, height),
        ("LineTo", 0.0, 0.0),
    ]
    .into_iter()
    .enumerate()
    {
        section.rows.push(point_row(kind, ix, x, y));
    }
    section
}

/// The conjugate-diameter `Ellipse` row of a `width` x `height` box.
fn ellipse_geometry(width: f64, height: f64) -> Section {
    let mut section = Section::new(SectionName::from_static("Geometry"));
    let mut row = Row::new();
    row.row_type = Some("Ellipse".to_owned());
    row.ix = Some(0);
    row.cells = vec![
        number_cell("X", width * 0.5, Unit::Inches),
        number_cell("Y", height * 0.5, Unit::Inches),
        number_cell("A", width, Unit::Inches),
        number_cell("B", height * 0.5, Unit::Inches),
        number_cell("C", width * 0.5, Unit::Inches),
        number_cell("D", height, Unit::Inches),
    ];
    section.rows.push(row);
    section
}

/// The two points of a line from `(0, 0)` to `(dx, dy)`.
fn line_geometry(dx: f64, dy: f64) -> Section {
    let mut section = Section::new(SectionName::from_static("Geometry"));
    section.rows.push(point_row("MoveTo", 0, 0.0, 0.0));
    section.rows.push(point_row("LineTo", 1, dx, dy));
    section
}

/// One `MoveTo`/`LineTo` row.
fn point_row(kind: &str, ix: usize, x: f64, y: f64) -> Row {
    let mut row = Row::new();
    row.row_type = Some(kind.to_owned());
    row.ix = Some(i32::try_from(ix).unwrap_or(i32::MAX));
    row.cells = vec![
        number_cell("X", x, Unit::Inches),
        number_cell("Y", y, Unit::Inches),
    ];
    row
}

/// One numeric cell with a literal formula and its cached value (ADR 0038).
fn number_cell(name: &'static str, value: f64, unit: Unit) -> Cell {
    Cell {
        name: CellName::from_static(name),
        formula: Some(if value == 0.0 {
            "0".to_owned()
        } else {
            value.to_string()
        }),
        value: Some(CellValue::Number(value)),
        unit,
        preserved: Preserved::new(),
    }
}

/// One boolean cell with its cached value.
fn bool_cell(name: &'static str, value: bool) -> Cell {
    Cell {
        name: CellName::from_static(name),
        formula: Some(if value { "1" } else { "0" }.to_owned()),
        value: Some(CellValue::Bool(value)),
        unit: Unit::Unitsless,
        preserved: Preserved::new(),
    }
}

/// The catalog [`MasterDef`] named by a stencil display name and a master
/// `NameU` (or display name).
fn catalog_master(stencil: &str, master: &str) -> Result<MasterDef, CommandError> {
    stencils::catalog()
        .into_iter()
        .find(|entry| entry.name == stencil)
        .ok_or_else(|| CommandError::invalid_args(format!("unknown stencil {stencil:?}")))
        .and_then(|entry| {
            entry
                .masters
                .into_iter()
                .find(|def| def.name_u == master || def.name == master)
                .ok_or_else(|| {
                    CommandError::invalid_args(format!(
                        "unknown master {master:?} in stencil {stencil:?}"
                    ))
                })
        })
}

/// The master's id in `doc`, materialising the catalog master on first use.
fn ensure_catalog_master(
    doc: &mut Document,
    definition: &MasterDef,
) -> Result<MasterId, CommandError> {
    if let Some(id) = doc.master_by_name_u(definition.name_u) {
        return Ok(id);
    }
    // The stencil catalog owns the master's cells, geometry, Shape Data
    // and connection rows. Build one master document and transfer the
    // master's shape subtree into this document's arena: the model's
    // private `ensure_master` is not part of its public mutation API.
    let source = stencils::stencil_document();
    let source_id = source
        .master_by_name_u(definition.name_u)
        .ok_or_else(|| CommandError::failed("the stencil catalog has no master data"))?;
    let source_master = source
        .master(source_id)
        .ok_or_else(|| CommandError::failed("the stencil master is missing"))?;
    let mut master = source_master.clone();
    let mut shapes = Vec::with_capacity(master.shapes.len());
    for id in &master.shapes {
        let shape = source
            .shape(*id)
            .cloned()
            .ok_or_else(|| CommandError::failed("a stencil master shape is missing"))?;
        shapes.push(doc.add_shape(shape));
    }
    master.shapes = shapes;
    doc.add_master(master).map_err(model_error)
}

/// Adds one glue record from the new shape's nearest connection point to
/// `target`'s connection point (ADR 0046).
fn add_drop_glue(
    doc: &mut Document,
    page: ferrogram_model::PageId,
    shape: ShapeId,
    target_visio_id: i32,
    glue_point: Option<usize>,
) -> Result<(), CommandError> {
    let target = doc
        .shape_by_visio_id(page, target_visio_id)
        .ok_or_else(|| {
            CommandError::invalid_args(format!(
                "unknown glue target {target_visio_id} on the active page"
            ))
        })?;
    let target_points = doc.connection_points_indexed(target);
    if target_points.is_empty() {
        return Err(CommandError::failed(
            "the glue target has no connection points",
        ));
    }
    let source_points = doc.connection_points_indexed(shape);
    if source_points.is_empty() {
        return Err(CommandError::failed(
            "the dropped master has no connection points to glue",
        ));
    }
    let target_transform = shape_transform(doc, target);
    let source_transform = shape_transform(doc, shape);
    let target_ix = match glue_point {
        Some(ix)
            if target_points
                .iter()
                .any(|(row, _)| *row == i32::try_from(ix).unwrap_or(-1)) =>
        {
            ix
        }
        Some(ix) => {
            return Err(CommandError::invalid_args(format!(
                "the glue target has no connection point {ix}"
            )));
        }
        None => nearest_target_index(
            &target_points,
            target_transform,
            &source_points,
            source_transform,
        )
        .ok_or_else(|| CommandError::failed("no finite connection point to glue"))?,
    };
    let target_page = target_transform.apply(
        target_points
            .iter()
            .find(|(row, _)| *row == i32::try_from(target_ix).unwrap_or(-1))
            .map(|(_, point)| *point)
            .ok_or_else(|| CommandError::failed("the glue target point is missing"))?,
    );
    let source_ix = nearest_source_index(&source_points, source_transform, target_page)
        .ok_or_else(|| CommandError::failed("no finite connection point to glue"))?;
    let source = doc
        .shape(shape)
        .ok_or_else(|| CommandError::failed("the new shape is missing"))?
        .visio_id;
    let target_id = doc
        .shape(target)
        .ok_or_else(|| CommandError::failed("the glue target is missing"))?
        .visio_id;
    let mut connect = Connect::new();
    connect.from_sheet = source;
    connect.from_cell = format!("Connections.X{}", source_ix.saturating_add(1));
    connect.from_part =
        VIS_CONNECTION_POINT.saturating_add(i32::try_from(source_ix).unwrap_or(i32::MAX));
    connect.to_sheet = target_id;
    connect.to_cell = format!("Connections.X{}", target_ix.saturating_add(1));
    connect.to_part =
        VIS_CONNECTION_POINT.saturating_add(i32::try_from(target_ix).unwrap_or(i32::MAX));
    connect.preserved = Preserved::new();
    doc.add_connect(page, connect).map_err(model_error)
}

/// The target connection point nearest to any source connection point
/// (both in page inches).
fn nearest_target_index(
    target_points: &[(i32, Point)],
    target_transform: Transform,
    source_points: &[(i32, Point)],
    source_transform: Transform,
) -> Option<usize> {
    let mut best: Option<(usize, f64)> = None;
    for (t_ix, t_point) in target_points {
        let t_page = target_transform.apply(*t_point);
        if !t_page.x.is_finite() || !t_page.y.is_finite() {
            continue;
        }
        for (_, s_point) in source_points {
            let s_page = source_transform.apply(*s_point);
            if !s_page.x.is_finite() || !s_page.y.is_finite() {
                continue;
            }
            let delta = s_page.sub(t_page);
            let distance = delta.x.hypot(delta.y);
            let t_ix = usize::try_from(*t_ix).unwrap_or(usize::MAX);
            if distance.is_finite() && best.is_none_or(|(_, best)| distance < best) {
                best = Some((t_ix, distance));
            }
        }
    }
    best.map(|(ix, _)| ix)
}

/// The source connection point nearest to `target_page`.
fn nearest_source_index(
    source_points: &[(i32, Point)],
    source_transform: Transform,
    target_page: Point,
) -> Option<usize> {
    let mut best: Option<(usize, f64)> = None;
    for (ix, point) in source_points {
        let page = source_transform.apply(*point);
        if !page.x.is_finite() || !page.y.is_finite() {
            continue;
        }
        let delta = page.sub(target_page);
        let distance = delta.x.hypot(delta.y);
        let ix = usize::try_from(*ix).unwrap_or(usize::MAX);
        if distance.is_finite() && best.is_none_or(|(_, best)| distance < best) {
            best = Some((ix, distance));
        }
    }
    best.map(|(ix, _)| ix)
}

/// The shape-local to page transform of ADR 0020, read through the cached
/// cells (the same rule as the renderer).
fn shape_transform(doc: &Document, shape: ShapeId) -> Transform {
    let mut reader = CellReader::new(doc);
    let sheet = SheetRef::Shape(shape);
    let pin_x = reader.number(sheet, &CellName::new("PinX")).unwrap_or(0.0);
    let pin_y = reader.number(sheet, &CellName::new("PinY")).unwrap_or(0.0);
    let loc_x = reader
        .number(sheet, &CellName::new("LocPinX"))
        .unwrap_or(0.0);
    let loc_y = reader
        .number(sheet, &CellName::new("LocPinY"))
        .unwrap_or(0.0);
    let angle = reader
        .angle(sheet, &CellName::new("Angle"))
        .map(|angle| angle.radians())
        .unwrap_or(0.0);
    let flip_x = reader
        .boolean(sheet, &CellName::new("FlipX"))
        .unwrap_or(false);
    let flip_y = reader
        .boolean(sheet, &CellName::new("FlipY"))
        .unwrap_or(false);
    Transform::translate(Point::new(-loc_x, -loc_y))
        .then(Transform::scale(
            if flip_x { -1.0 } else { 1.0 },
            if flip_y { -1.0 } else { 1.0 },
        ))
        .then(Transform::rotate(Angle::from_radians(angle)))
        .then(Transform::translate(Point::new(pin_x, pin_y)))
}

/// The active session and page, or a typed failure.
fn active_page(ctx: &AppCtx) -> Result<(SessionId, ferrogram_model::PageId), CommandError> {
    let session = ctx.active_document().ok_or_else(no_document)?;
    let page = session
        .active_page
        .ok_or_else(|| CommandError::failed("no active page"))?;
    Ok((session.id, page))
}

/// Makes the newly inserted shape the selection (the command, not the
/// tool, owns the selection; ADR 0041).
fn select_new(
    ctx: &mut AppCtx,
    _session: SessionId,
    page: ferrogram_model::PageId,
    shape: ShapeId,
) -> Result<(), CommandError> {
    ctx.selection.shapes = vec![shape];
    ctx.selection.anchor = Some(shape);
    ctx.selection.page = Some(page);
    ctx.selection.entered = None;
    Ok(())
}

/// One required text argument.
fn need_text(args: &crate::Args, name: &str, prompt: &str) -> Result<String, CommandError> {
    match args.get(name) {
        None => Err(CommandError::NeedsInput(crate::NeededInput::Text {
            prompt: prompt.to_owned(),
            default: None,
        })),
        Some(crate::ArgValue::Text(text)) if !text.is_empty() => Ok(text.clone()),
        Some(crate::ArgValue::Text(_)) => Err(CommandError::invalid_args(format!(
            "{name} must not be empty"
        ))),
        Some(_) => Err(CommandError::invalid_args(format!("{name} must be text"))),
    }
}

/// The optional `width`/`height` pair of `insert.shape`.
fn optional_size(args: &crate::Args) -> Result<Option<Size>, CommandError> {
    let width = optional_number(args, "width", "width in inches")?;
    let height = optional_number(args, "height", "height in inches")?;
    match (width, height) {
        (None, None) => Ok(None),
        (Some(width), Some(height)) => {
            if width < 0.0 || height < 0.0 {
                return Err(CommandError::invalid_args(
                    "width and height must not be negative",
                ));
            }
            Ok(Some(Size::new(width, height)))
        }
        _ => Err(CommandError::invalid_args(
            "width and height must be given together",
        )),
    }
}

/// One optional finite number argument.
fn optional_number(
    args: &crate::Args,
    name: &str,
    prompt: &str,
) -> Result<Option<f64>, CommandError> {
    match args.get(name) {
        None => Ok(None),
        Some(_) => need_finite(args, name, prompt).map(Some),
    }
}

/// An optional `ArgValue::Integer` as a Visio id.
fn optional_visio_id(args: &crate::Args, name: &str) -> Result<Option<i32>, CommandError> {
    match args.get(name) {
        None => Ok(None),
        Some(crate::ArgValue::Integer(value)) => i32::try_from(*value)
            .map(Some)
            .map_err(|_| CommandError::invalid_args(format!("{name} is out of range"))),
        Some(_) => Err(CommandError::invalid_args(format!(
            "{name} must be an integer Visio id"
        ))),
    }
}

/// An optional non-negative connection row index.
fn optional_row(args: &crate::Args, name: &str) -> Result<Option<usize>, CommandError> {
    match args.get(name) {
        None => Ok(None),
        Some(crate::ArgValue::Integer(value)) if *value >= 0 => usize::try_from(*value)
            .map(Some)
            .map_err(|_| CommandError::invalid_args(format!("{name} is out of range"))),
        Some(_) => Err(CommandError::invalid_args(format!(
            "{name} must be a non-negative row index"
        ))),
    }
}
