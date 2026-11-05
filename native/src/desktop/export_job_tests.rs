//! 保存框前后状态核验，不打开对话框、不创建用户文件。
use super::*;
use crate::{
    desktop::{canvas::Canvas, image::Image},
    editor::{Editor, Tool},
    viewport::Point,
};

fn canvas() -> Canvas {
    Canvas::new(Some(Image {
        width: 2,
        height: 2,
        pixels: vec![255; 16].into(),
    }))
    .unwrap()
}

#[test]
fn normal_opacity_change_invalidates_output_but_mini_presentation_does_not() {
    let mut canvas = canvas();
    let stamp = crate::desktop::display_snapshot::Stamp::capture(&canvas, None);
    canvas.window_opacity = 0.25;
    canvas.minified_opacity = 0.25;
    assert!(snapshot_if_current(&canvas, None, &stamp).is_some());
    canvas.normal_opacity = 0.5;
    assert!(snapshot_if_current(&canvas, None, &stamp).is_none());
}

#[test]
fn changed_display_during_save_dialog_rejects_old_destination() {
    let canvas = Canvas::new(None).unwrap();
    let image = Image {
        width: 1,
        height: 1,
        pixels: vec![1, 2, 3, 255].into(),
    };
    let id = crate::desktop::document_state::new_unit_id().unwrap();
    let frame = crate::desktop::display_frame::Frame::capture(id.clone(), image.clone(), None);
    let stamp = crate::desktop::display_snapshot::Stamp::capture(&canvas, Some(&frame));
    let (snapshot, _) = snapshot_if_current(&canvas, Some(&frame), &stamp).unwrap();
    assert!(snapshot.pixels.same_storage(&image.pixels));
    let replacement = crate::desktop::display_frame::Frame::capture(id, image, None);
    assert!(snapshot_if_current(&canvas, Some(&replacement), &stamp).is_none());
    assert!(snapshot_if_current(&canvas, None, &stamp).is_none());
    let local = crate::desktop::display_snapshot::Stamp::capture(&canvas, None);
    assert!(snapshot_if_current(&canvas, Some(&frame), &local).is_none());
}

#[test]
fn dialog_gate_blocks_reentry_and_releases_after_cancel_or_error() {
    let busy = std::cell::Cell::new(false);
    let guard = Dialog::begin(&busy).unwrap();
    assert!(busy.get());
    assert!(Dialog::begin(&busy).is_none());
    assert!(busy.get());
    drop(guard);
    assert!(!busy.get());
    let failed = || -> Result<(), ()> {
        let _guard = Dialog::begin(&busy).unwrap();
        Err(())
    };
    assert!(failed().is_err());
    assert!(!busy.get());
    assert!(Dialog::begin(&busy).is_some());
    assert!(!busy.get());
}

#[test]
fn replacement_or_edit_during_dialog_cannot_export_under_old_path() {
    for replacement in [false, true] {
        let mut canvas = canvas();
        let stamp = crate::desktop::display_snapshot::Stamp::capture(&canvas, None);
        if replacement {
            canvas.editor = canvas.editor.export_snapshot();
        } else {
            canvas
                .editor
                .begin(Tool::Brush, Point { x: 1.0, y: 1.0 })
                .unwrap();
            assert!(canvas.editor.commit());
        }
        assert!(snapshot_if_current(&canvas, None, &stamp).is_none());
    }
}

#[test]
fn stable_document_exports_independent_snapshot_but_active_draft_is_rejected() {
    let mut canvas = canvas();
    canvas
        .editor
        .begin(Tool::Brush, Point { x: 0.0, y: 0.0 })
        .unwrap();
    assert!(canvas.editor.commit());
    let stamp = crate::desktop::display_snapshot::Stamp::capture(&canvas, None);
    let (image, snapshot) = snapshot_if_current(&canvas, None, &stamp).unwrap();
    assert!(image
        .pixels
        .same_storage(&canvas.image.as_ref().unwrap().pixels));
    assert!(!canvas.editor.document_stamp().matches(&snapshot));
    canvas
        .editor
        .begin(Tool::Brush, Point { x: 1.0, y: 1.0 })
        .unwrap();
    assert!(snapshot_if_current(&canvas, None, &stamp).is_none());
    canvas.editor = Editor::default();
    assert_eq!(snapshot.elements().len(), 1);
}
