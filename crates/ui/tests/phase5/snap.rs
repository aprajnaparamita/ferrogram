//! Snapping and drag-overlay tests through the real canvas (task P5-05,
//! ADR 0043): grid snapping, Alt disabling it, and the snap guides on the
//! overlay.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use ferrogram_render::OverlayMark;

use crate::{CanvasRig, app_with_one_shape, screen, shape_number};

#[test]
fn a_move_snaps_to_the_grid() {
    let (app, id) = app_with_one_shape();
    let mut rig = CanvasRig::new(app);
    rig.click(screen(3.0, 3.0));
    // 0.13 in lands nearest the 1/8 in grid line at 3.125.
    rig.drag(screen(3.0, 3.0), screen(3.13, 3.0));
    let pin = shape_number(&rig.app.borrow(), id, "PinX");
    assert!((pin - 3.125).abs() < 1.0e-9, "{pin}");
}

#[test]
fn the_drag_overlay_draws_a_snap_guide() {
    let (app, _id) = app_with_one_shape();
    let mut rig = CanvasRig::new(app);
    rig.click(screen(3.0, 3.0));
    rig.down(screen(3.0, 3.0));
    rig.move_to(screen(3.13, 3.0));
    {
        let overlay = rig.overlay.borrow();
        assert!(
            overlay
                .marks
                .iter()
                .any(|mark| matches!(mark, OverlayMark::SnapGuide { .. })),
            "{:?}",
            overlay.marks
        );
        assert!(
            overlay
                .marks
                .iter()
                .any(|mark| matches!(mark, OverlayMark::SelectionOutline { .. })),
            "the preview outline draws"
        );
    }
    rig.up(screen(3.13, 3.0));
}

#[test]
fn alt_keeps_the_unsnapped_delta() {
    let (app, id) = app_with_one_shape();
    let mut rig = CanvasRig::new(app);
    rig.click(screen(3.0, 3.0));
    rig.set_alt(true);
    rig.drag(screen(3.0, 3.0), screen(3.13, 3.0));
    rig.set_alt(false);
    let pin = shape_number(&rig.app.borrow(), id, "PinX");
    assert!((pin - 3.13).abs() < 1.0e-5, "{pin}");
}

#[test]
fn the_pointer_and_snapper_share_the_page_scene() {
    // A second frame with no change reuses the pointer's scene cache and
    // keeps the selection overlay; a document change rebuilds it.
    let (app, id) = app_with_one_shape();
    let mut rig = CanvasRig::new(app);
    rig.click(screen(3.0, 3.0));
    rig.run();
    assert!(
        rig.overlay
            .borrow()
            .marks
            .iter()
            .any(|mark| matches!(mark, OverlayMark::Handles { .. })),
        "the selection handles draw while idle"
    );
    {
        let mut app = rig.app.borrow_mut();
        let session = app.active_document_mut().expect("session");
        if let Some(shape) = session.document.shape_mut(id) {
            shape
                .sheet
                .cells
                .retain(|cell| cell.name.as_str() != "Width");
        }
        session.changes = session.changes.wrapping_add(1);
    }
    rig.run();
    // The shape still has a scene node; the cache rebuilt without a
    // panic and the overlay is still a selection overlay.
    assert!(
        rig.overlay
            .borrow()
            .marks
            .iter()
            .any(|mark| matches!(mark, OverlayMark::SelectionOutline { .. })),
    );
}
