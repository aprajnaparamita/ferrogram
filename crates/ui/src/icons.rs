//! Placeholder icons for ribbon and menu controls (ADR 0030).
//!
//! Phase 4 draws simple marks of our own (a filled rounded square
//! with the first letter of the control id); no icon set is committed
//! and no Office icon is ever used (AGENTS.md section 4). The
//! screenshot pass may replace the marks with an open-licensed set
//! whose licence goes in `ATTRIBUTION.md` in the same commit.
//!
//! Marks are `SizeVariant`-independent for now: the same mark is drawn
//! at every requested size (ADR 0030).
//!
//! Owned by task P4-04.

use egui::{Align2, Atom, AtomPaintArgs, FontId, Rect, Response, Sense, Ui, Vec2};

use crate::UiError;

/// Placeholder size used when the requested size is not usable.
const DEFAULT_SIZE: f32 = 12.0;

/// Smallest placeholder size, in points.
const MIN_SIZE: f32 = 4.0;

/// Largest placeholder size, in points.
const MAX_SIZE: f32 = 64.0;

/// Draws the placeholder icon for a control id at `size` points.
///
/// The icon is a simple mark of our own (ADR 0030): a filled rounded
/// square with the first letter of the control id. `size` is clamped
/// to `4.0..=64.0` points; a non-finite size falls back to 12.0. The
/// widget only reserves the square; it is not interactive.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// let response = ferrogram_ui::icons::placeholder(ui, "FileSave", 16.0).expect("a rect");
/// assert_eq!(response.rect.size(), egui::Vec2::splat(16.0));
/// # });
/// ```
pub fn placeholder(ui: &mut Ui, id: &str, size: f32) -> Result<Response, UiError> {
    let size = clamp_size(size);
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    paint_mark(ui, rect, id);
    Ok(response)
}

/// The placeholder icon for `id` as a paint atom, for embedding in a
/// button or other atom-based widget (ADR 0030).
///
/// Same mark as [`placeholder`], painted into the cell the widget
/// gives it. The id is copied so the atom is `'static`.
pub fn icon(id: &str, size: f32) -> Atom<'static> {
    let size = clamp_size(size);
    let id = id.to_owned();
    Atom::paint(Vec2::splat(size), move |ui: &Ui, args: AtomPaintArgs| {
        paint_mark(ui, args.rect, &id);
    })
}

/// A small filled downward triangle, for dropdown arrows (ADR 0030).
///
/// Painted rather than typed: egui's bundled fonts have no triangle
/// glyph. The mark uses the widget's text colour.
pub fn dropdown_arrow(size: f32) -> Atom<'static> {
    let size = clamp_size(size);
    Atom::paint(Vec2::splat(size), |ui: &Ui, args: AtomPaintArgs| {
        let rect = args.rect;
        let half = rect.width().min(rect.height()) * 0.25;
        let center = rect.center();
        ui.painter().add(egui::Shape::convex_polygon(
            vec![
                egui::pos2(center.x - half, center.y - half * 0.6),
                egui::pos2(center.x + half, center.y - half * 0.6),
                egui::pos2(center.x, center.y + half * 0.8),
            ],
            args.fallback_text_color,
            egui::Stroke::NONE,
        ));
    })
}

/// Sanitizes a requested icon size: finite and inside
/// `MIN_SIZE..=MAX_SIZE`, else [`DEFAULT_SIZE`].
fn clamp_size(size: f32) -> f32 {
    if size.is_finite() {
        size.clamp(MIN_SIZE, MAX_SIZE)
    } else {
        DEFAULT_SIZE
    }
}

/// Paints the placeholder mark: a filled rounded square in the
/// faint background colour with the first letter of `id` on top.
fn paint_mark(ui: &Ui, rect: Rect, id: &str) {
    let visuals = ui.visuals();
    let painter = ui.painter();
    let side = rect.width().min(rect.height());
    let corner = ((side * 0.2).round().clamp(0.0, 255.0)) as u8;
    painter.rect_filled(
        rect,
        egui::CornerRadius::same(corner),
        visuals.faint_bg_color,
    );
    let letter = first_letter(id);
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        letter,
        FontId::proportional(side * 0.7),
        visuals.text_color(),
    );
}

/// The letter drawn on the mark: the first character of the id,
/// uppercased; `?` for an empty id.
fn first_letter(id: &str) -> String {
    match id.chars().next() {
        Some(first) => first.to_uppercase().collect(),
        None => "?".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::with_ui;

    #[test]
    fn placeholder_reserves_the_requested_square() {
        with_ui(|ui| {
            let response = placeholder(ui, "FileSave", 16.0).expect("a rect");
            assert_eq!(response.rect.size(), Vec2::splat(16.0));
            assert!(!response.clicked());
        });
    }

    #[test]
    fn placeholder_clamps_bad_sizes() {
        with_ui(|ui| {
            for (asked, want) in [
                (0.0, MIN_SIZE),
                (-3.0, MIN_SIZE),
                (2.5, MIN_SIZE),
                (1000.0, MAX_SIZE),
                (f32::NAN, DEFAULT_SIZE),
                (f32::INFINITY, DEFAULT_SIZE),
                (f32::NEG_INFINITY, DEFAULT_SIZE),
            ] {
                let response = placeholder(ui, "X", asked).expect("a rect");
                assert_eq!(response.rect.size(), Vec2::splat(want), "asked {asked}");
            }
        });
    }

    #[test]
    fn placeholder_handles_an_empty_id() {
        with_ui(|ui| {
            let response = placeholder(ui, "", 8.0).expect("a rect");
            assert_eq!(response.rect.size(), Vec2::splat(8.0));
        });
    }

    #[test]
    fn the_mark_letter_comes_from_the_id() {
        assert_eq!(first_letter("fileSave"), "F");
        assert_eq!(first_letter("2Cut"), "2");
        assert_eq!(first_letter(""), "?");
        // A non-ASCII id still yields one letter, not a panic.
        assert_eq!(first_letter("édition"), "É");
    }

    #[test]
    fn clamped_sizes_are_finite_and_bounded() {
        for size in [f32::NAN, -1.0, 0.0, 3.0, 12.0, 70.0, f32::INFINITY] {
            let clamped = clamp_size(size);
            assert!(clamped.is_finite());
            assert!((MIN_SIZE..=MAX_SIZE).contains(&clamped));
        }
    }

    #[test]
    fn icon_paints_without_panicking() {
        // The atom paints inside a button; here it is laid out bare.
        with_ui(|ui| {
            let atom = icon("FileSave", 12.0);
            ui.add(egui::Button::new((atom, "Save")));
        });
    }

    #[test]
    fn the_dropdown_arrow_paints_a_triangle() {
        with_ui(|ui| {
            ui.add(egui::Button::new(dropdown_arrow(12.0)));
            // A bad size clamps like every other mark.
            let atom = dropdown_arrow(f32::NAN);
            ui.add(egui::Button::new(atom));
        });
    }
}
