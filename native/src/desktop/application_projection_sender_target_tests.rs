//! Independent ownership of resolved pixels and target edits: real menu/TLS/checkpoint flow, without foreground or real-profile dependencies.
use super::*;
use crate::{
    editor::{Editor, Tool},
    erase_layers::Layers,
    viewport::Point,
};

pub(super) unsafe fn install(app: &mut Application, beautify: bool) -> Image {
    let unit = app.documents[0].state.unit_id.clone();
    let base = Image {
        width: 64,
        height: 32,
        pixels: [255, 0, 0, 255].repeat(64 * 32).into(),
    };
    let upstream = {
        let mut editor = Editor::with_layers(Layers {
            size: (64, 32),
            base: base.pixels.clone(),
            annotations: None,
            annotation_size: None,
        })
        .unwrap();
        editor.set_style(
            crate::stroke_style::StrokeStyle::new(64.0, crate::stroke_style::Color(0, 255, 0))
                .unwrap(),
        );
        editor
            .begin(Tool::Brush, Point { x: 32.0, y: 16.0 })
            .unwrap();
        assert!(editor.commit());
        editor
            .crop_to(crate::crop_geometry::Rect {
                x: 0.0,
                y: 0.0,
                width: 8.0,
                height: 8.0,
            })
            .unwrap();
        editor
    };
    // A resolved local source must be an actually live document matching DocumentStamp, not a fabricated unit.
    let upstream_index = app.documents.len();
    app.add_state(State::new(Some(base.clone()), true).unwrap(), false)
        .unwrap();
    let upstream_unit = app.documents[upstream_index].state.unit_id.clone();
    app.documents[upstream_index]
        .state
        .canvas
        .borrow_mut()
        .editor = upstream;
    settle(app);
    {
        let mut canvas = app.documents[0].state.canvas.borrow_mut();
        canvas
            .replace_image(Image {
                width: 32,
                height: 16,
                pixels: [0, 255, 0, 255].repeat(32 * 16).into(),
            })
            .unwrap();
        let before = canvas.editor.layers().unwrap().clone();
        let mut after = before.clone();
        let mut raster = vec![0; 32 * 16 * 4];
        for y in 6..10 {
            for x in 12..16 {
                raster[(y * 32 + x) * 4..(y * 32 + x) * 4 + 4].copy_from_slice(&[0, 255, 255, 255]);
            }
        }
        after.annotations = Some(raster.into());
        let revision = canvas.editor.revision();
        canvas
            .editor
            .replace_layers(revision, &before, after)
            .unwrap();
        canvas
            .editor
            .crop_to(crate::crop_geometry::Rect {
                x: 0.0,
                y: 0.0,
                width: 24.0,
                height: 12.0,
            })
            .unwrap();
    }
    // A new Unit may not yet be in Record.original; derive fixture baselines only from the current committed complete canonical.
    let canonical = app
        .workspace_checkpoint
        .as_ref()
        .unwrap()
        .sender_source()
        .unwrap()
        .clone();
    let mut value: serde_json::Value = serde_json::from_slice(&canonical).unwrap();
    let target = value["stickers"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|v| v["id"] == unit.as_str())
        .unwrap();
    target["imageEditState"] = serde_json::json!({
        "borderWidth": if beautify { 2 } else { 0 }, "borderColor":"#ff0000",
        "beautify":{"enabled":beautify,"padding":3,"backgroundId":"white","cornerRadius":0,"shadow":false}
    });
    app.documents[0].state.canvas.borrow_mut().frame_style =
        crate::pin_frame_style::Style::read(&target["imageEditState"]).unwrap();
    app.restored_session
        .as_mut()
        .unwrap()
        .edit_original(serde_json::to_vec(&value).unwrap().into());
    settle(app);
    {
        let mut canvas = app.documents[0].state.canvas.borrow_mut();
        assert!(canvas.editor.reset_crop());
        canvas
            .editor
            .crop_to(crate::crop_geometry::Rect {
                x: 8.0,
                y: 4.0,
                width: 16.0,
                height: 8.0,
            })
            .unwrap();
    }
    *app.documents[0].state.resolved_display.borrow_mut() =
        Some(crate::desktop::display_frame::Frame::capture(
            upstream_unit,
            base,
            Some(&app.documents[upstream_index].state.canvas.borrow().editor),
        ));
    // Independently build a full-image oracle without calling the selection, edit-composition, or appearance wrappers under test.
    let padding = if beautify { 3 } else { 0 };
    let (width, height) = (32 + padding * 2, 16 + padding * 2);
    let mut pixels = [255; 4].repeat((width * height) as usize);
    for y in 0..16 {
        for x in 0..32 {
            // Mainline lays the complete raster layer over the visible frame; do not reuse base-image/vector source-frame cropping.
            let color = if (12..16).contains(&x) && (6..10).contains(&y) {
                [0, 255, 255, 255]
            } else if beautify && (!(4..28).contains(&x) || !(4..12).contains(&y)) {
                [0, 0, 255, 255]
            } else {
                [255, 0, 0, 255]
            };
            let at = (((y + padding) * width + x + padding) * 4) as usize;
            pixels[at..at + 4].copy_from_slice(&color);
        }
    }
    Image {
        width,
        height,
        pixels: pixels.into(),
    }
}
#[test]
fn native_sender_resolved_uses_target_crop_and_raster_not_upstream_editor() {
    tls::run_source(0, source::Case::TargetResolved);
}
#[test]
fn native_sender_resolved_target_border_and_beautify_are_signed_and_checkpointed() {
    tls::run_source(0, source::Case::TargetBeautified);
}
