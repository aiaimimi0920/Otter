//! 真实工作线程到 Canvas 的擦除提交链，不读取用户图片或启动系统对话框。
use super::*;
use crate::{
    editor::Tool,
    erase_job::Job,
    viewport::{Point, Viewport},
};
fn canvas() -> Canvas {
    let mut canvas = Canvas::new(Some(crate::desktop::image::Image {
        width: 32,
        height: 32,
        pixels: vec![255; 32 * 32 * 4].into(),
    }))
    .unwrap();
    canvas.viewport = Viewport {
        origin: Point::default(),
        scale: 1.0,
    };
    canvas.select_tool(Tool::Eraser);
    canvas
}
fn finish(canvas: &mut Canvas) -> crate::erase_job::Completed {
    let mut job = Job::default();
    let request = canvas.erase_pending.take().unwrap();
    canvas.erase_display = Some(request.clone());
    let plan = canvas.editor.prepare_raster_edit().unwrap().unwrap();
    job.start_edit(request, plan).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        if let Some(result) = job.poll() {
            return result;
        }
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}
#[test]
fn cropped_pointer_captures_annotation_frame_and_commits_single_history_entry() {
    let mut canvas = canvas();
    let mut layers = canvas.editor.layers().unwrap().clone();
    layers.annotations = Some([64, 64, 64, 128].repeat(32 * 32).into());
    canvas.editor = crate::editor::Editor::with_layers(layers.clone()).unwrap();
    let original_blur =
        crate::blur_image::BlurImage::with_strength(32, 32, vec![128; 32 * 32 * 4].into(), 2.4)
            .unwrap();
    canvas
        .editor
        .begin_blur(Point { x: 20.0, y: 12.0 }, original_blur.clone())
        .unwrap();
    assert!(canvas.editor.commit());
    canvas
        .editor
        .set_style(canvas.editor.style().with_width(4.0).unwrap());
    let rect = crate::crop_geometry::Rect {
        x: 16.0,
        y: 4.0,
        width: 8.0,
        height: 16.0,
    };
    assert!(canvas.editor.crop_to(rect).unwrap());
    let crop = canvas.editor.crop_view();
    let revision = canvas.editor.revision();
    let point = Point { x: 20.0, y: 12.0 };
    assert!(canvas.pointer_down(point));
    assert_eq!(
        canvas.erase_gesture.as_ref().unwrap().annotation_frame,
        Some(rect)
    );
    assert!(canvas.pointer_up(point));
    let completed = finish(&mut canvas);
    assert_eq!(canvas.editor.revision(), revision);
    assert!(accept(&mut canvas, completed).unwrap());
    assert_eq!(canvas.editor.revision(), revision + 1);
    assert_eq!(canvas.editor.crop_view(), crop);
    let result = canvas.editor.layers().unwrap();
    assert_eq!(result.base[(12 * 32 + 20) * 4 + 3], 0);
    assert_eq!(
        result.annotations.as_ref().unwrap()[(16 * 32 + 16) * 4 + 3],
        0
    );
    let composed = crate::raster_source::compose_in_frame(result, Some(rect), || false).unwrap();
    let expected = crate::blur::region_precise(
        &composed,
        (32, 32),
        crate::blur::Region {
            x: 0,
            y: 0,
            width: 32,
            height: 32,
        },
        2.4,
        || false,
    )
    .unwrap();
    assert_eq!(
        &canvas.editor.elements()[0].blur.as_ref().unwrap().pixels()[..],
        &expected[..]
    );
    assert!(canvas.editor.undo());
    assert!(canvas.editor.layers().unwrap().same_storage(&layers));
    assert!(canvas.editor.elements()[0]
        .blur
        .as_ref()
        .unwrap()
        .pixels()
        .same_storage(original_blur.pixels()));
    assert_eq!(canvas.editor.crop_view(), crop);
    assert!(canvas.editor.redo());
    assert_eq!(
        canvas
            .editor
            .layers()
            .unwrap()
            .annotations
            .as_ref()
            .unwrap()[(16 * 32 + 16) * 4 + 3],
        0
    );
}

#[test]
fn pointer_stroke_commits_once_and_undo_restores_original_shared_image() {
    let mut canvas = canvas();
    let original = canvas.editor.layers().unwrap().clone();
    assert_eq!(canvas.editor.style().width(), 20.0);
    assert!(canvas.pointer_down(Point { x: 8.0, y: 16.0 }));
    assert!(canvas.pointer_move(Point { x: 24.0, y: 16.0 }));
    assert_eq!(canvas.editor.revision(), 0);
    assert!(canvas.editor.elements().is_empty());
    assert!(canvas.pointer_up(Point { x: 24.0, y: 16.0 }));
    let completed = finish(&mut canvas);
    assert!(
        canvas.erase_display.is_some(),
        "keep preview until committed pixels are available"
    );
    assert!(accept(&mut canvas, completed).unwrap());
    assert!(canvas.erase_display.is_none());
    assert_eq!(canvas.editor.revision(), 1);
    assert_eq!(
        canvas.image.as_ref().unwrap().pixels[(16 * 32 + 16) * 4 + 3],
        0
    );
    assert!(canvas.editor.undo());
    canvas.sync_raster_source();
    assert!(canvas
        .image
        .as_ref()
        .unwrap()
        .pixels
        .same_storage(&original.base));
    assert!(!canvas.editor.undo());
}
#[test]
fn cancellation_empty_annotation_mode_and_stale_worker_do_not_commit() {
    let mut canvas = canvas();
    let point = Point { x: 16.0, y: 16.0 };
    canvas.erase_annotations_only = true;
    assert!(!canvas.pointer_down(point));
    canvas.erase_annotations_only = false;
    assert!(canvas.pointer_down(point));
    assert!(canvas.cancel_gesture());
    assert!(canvas.erase_gesture.is_none());
    assert!(canvas.erase_pending.is_none());
    assert_eq!(canvas.editor.revision(), 0);
    assert!(canvas.pointer_down(point));
    assert!(canvas.pointer_up(point));
    let completed = finish(&mut canvas);
    canvas.editor.begin(Tool::Brush, point).unwrap();
    assert!(canvas.editor.commit());
    assert!(!accept(&mut canvas, completed).unwrap());
    assert_eq!(
        canvas.image.as_ref().unwrap().pixels[(16 * 32 + 16) * 4 + 3],
        255
    );
}

#[test]
fn erase_and_fractional_blur_refresh_share_one_commit_and_preview_handoff() {
    for annotations_only in [false, true] {
        let mut canvas = canvas();
        let original = canvas.editor.layers().unwrap().clone();
        let mut layers = original.clone();
        layers.annotations = Some([255, 0, 0, 255].repeat(32 * 32).into());
        canvas
            .editor
            .replace_layers(0, &original, layers.clone())
            .unwrap();
        let old = crate::blur_image::BlurImage::with_strength(
            32,
            32,
            [255, 0, 0, 255].repeat(32 * 32).into(),
            2.4,
        )
        .unwrap();
        canvas
            .editor
            .begin_blur(Point { x: 16.0, y: 16.0 }, old.clone())
            .unwrap();
        canvas.editor.commit();
        let points = canvas.editor.elements()[0].points.clone();
        let revision = canvas.editor.revision();
        canvas.erase_annotations_only = annotations_only;
        let point = Point { x: 16.0, y: 16.0 };
        assert!(canvas.pointer_down(point));
        assert!(canvas.pointer_up(point));
        let completed = finish(&mut canvas);
        assert!(canvas.erase_display.is_some());
        assert_eq!(canvas.editor.revision(), revision);
        assert!(canvas.editor.elements()[0]
            .blur
            .as_ref()
            .unwrap()
            .pixels()
            .same_storage(old.pixels()));
        assert!(accept(&mut canvas, completed).unwrap());
        assert!(canvas.erase_display.is_none());
        assert_eq!(canvas.editor.revision(), revision + 1);
        assert_eq!(canvas.editor.elements()[0].points, points);
        let source =
            crate::raster_source::compose(canvas.editor.layers().unwrap(), || false).unwrap();
        let expected = crate::blur::region_precise(
            &source,
            (32, 32),
            crate::blur::Region {
                x: 0,
                y: 0,
                width: 32,
                height: 32,
            },
            2.4,
            || false,
        )
        .unwrap();
        let updated = canvas.editor.elements()[0].blur.as_ref().unwrap();
        assert_eq!(updated.strength(), 2.4);
        assert_eq!(&updated.pixels()[..], &expected[..]);
        assert!(canvas.editor.undo());
        assert!(canvas.editor.layers().unwrap().same_storage(&layers));
        assert!(canvas.editor.elements()[0]
            .blur
            .as_ref()
            .unwrap()
            .pixels()
            .same_storage(old.pixels()));
        assert!(canvas.editor.redo());
        assert_eq!(
            &canvas.editor.elements()[0].blur.as_ref().unwrap().pixels()[..],
            &expected[..]
        );
    }
}
