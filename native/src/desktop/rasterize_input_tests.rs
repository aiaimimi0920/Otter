//! 实际 WIC 工作线程→Canvas→仅标注擦除→混合历史恢复的回归。
use super::*;
use crate::{editor::Tool, erase_layers::Mode, viewport::Point};
fn canvas() -> Canvas {
    let mut canvas = Canvas::new(Some(crate::desktop::image::Image {
        width: 32,
        height: 32,
        pixels: vec![255; 32 * 32 * 4].into(),
    }))
    .unwrap();
    for x in [8.0, 24.0] {
        canvas
            .editor
            .begin(Tool::Brush, Point { x, y: 16.0 })
            .unwrap();
        assert!(canvas.editor.commit());
    }
    canvas
}
fn finish(job: &mut Job) -> Result<RasterRefreshPlan, String> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !job.worker.as_ref().unwrap().is_finished() {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    job.worker.take().unwrap().join().unwrap()
}
#[test]
fn selected_rasterization_then_annotation_erase_undo_in_order() {
    let mut canvas = canvas();
    assert!(prepare(&canvas, true).is_none());
    canvas.selected = Some((1, canvas.editor.revision()));
    let (request, snapshot) = prepare(&canvas, true).unwrap();
    assert_eq!(request.indices, vec![1]);
    assert_eq!(snapshot.elements().len(), 1);
    assert!(snapshot.layers().is_none());
    let original = canvas.editor.layers().unwrap().clone();
    let mut job = spawn(request, snapshot).unwrap();
    let result = finish(&mut job);
    assert!(accept(&mut canvas, &job.request, result).unwrap());
    assert_eq!(canvas.editor.elements().len(), 1);
    assert!(canvas.selected.is_none());
    let rasterized = canvas.editor.layers().unwrap().clone();
    let erased = rasterized
        .erase(
            Mode::RasterAnnotations,
            &[Point { x: 24.0, y: 16.0 }],
            12.0,
            || false,
        )
        .unwrap();
    assert!(erased.base.same_storage(&original.base));
    assert_eq!(
        erased.annotations.as_ref().unwrap()[(16 * 32 + 24) * 4 + 3],
        0
    );
    assert!(canvas
        .editor
        .replace_layers(canvas.editor.revision(), &rasterized, erased)
        .unwrap());
    assert!(canvas.editor.undo());
    assert!(canvas.editor.layers().unwrap().same_storage(&rasterized));
    assert!(canvas.editor.undo());
    assert!(canvas.editor.layers().unwrap().same_storage(&original));
    assert_eq!(canvas.editor.elements().len(), 2);
}
#[test]
fn all_scope_stale_result_and_cancel_do_not_publish_partial_edits() {
    let mut canvas = canvas();
    let (request, snapshot) = prepare(&canvas, false).unwrap();
    assert_eq!(request.indices, vec![0, 1]);
    let mut job = spawn(request, snapshot).unwrap();
    let result = finish(&mut job);
    assert!(canvas.editor.remove(0));
    let revision = canvas.editor.revision();
    assert!(!accept(&mut canvas, &job.request, result).unwrap());
    assert_eq!(canvas.editor.revision(), revision);
    assert!(canvas.editor.layers().unwrap().annotations.is_none());
    let (request, snapshot) = prepare(&canvas, false).unwrap();
    let mut job = spawn(request, snapshot).unwrap();
    job.cancel.store(true, Ordering::Release);
    let _ = finish(&mut job);
    assert!(job.cancel.load(Ordering::Acquire));
    assert_eq!(canvas.editor.revision(), revision);
}

#[test]
fn multi_selection_rasterizes_the_whole_set_in_one_history_entry() {
    let mut canvas = canvas();
    canvas.viewport.origin = Point::default();
    canvas.select_tool(Tool::Select);
    for x in [24.0, 8.0] {
        assert!(canvas.toggle_selection_at(Point { x, y: 16.0 }));
    }
    assert!(canvas.selected_index().is_none());
    let (request, snapshot) = prepare(&canvas, true).unwrap();
    assert_eq!(request.indices, vec![0, 1]);
    let revision = canvas.editor.revision();
    let mut job = spawn(request, snapshot).unwrap();
    let result = finish(&mut job);
    assert!(accept(&mut canvas, &job.request, result).unwrap());
    assert_eq!(canvas.editor.revision(), revision + 1);
    assert!(canvas.editor.elements().is_empty());
    assert!(canvas.selected_indices().is_empty());
    assert!(canvas.editor.undo());
    assert_eq!(canvas.editor.elements().len(), 2);
    assert!(canvas.editor.layers().unwrap().annotations.is_none());
}

#[test]
fn partial_rasterization_refreshes_remaining_blurs_and_restores_interleaved_indices() {
    let mut canvas = canvas();
    let old =
        crate::blur_image::BlurImage::with_strength(32, 32, vec![255; 32 * 32 * 4].into(), 2.4)
            .unwrap();
    for tool in [Tool::Blur, Tool::Brush, Tool::Blur] {
        let point = Point { x: 16.0, y: 16.0 };
        if tool == Tool::Blur {
            canvas.editor.begin_blur(point, old.clone()).unwrap();
        } else {
            canvas.editor.begin(tool, point).unwrap();
        }
        assert!(canvas.editor.commit());
    }
    let original = canvas.editor.layers().unwrap().clone();
    let pointers: Vec<_> = canvas
        .editor
        .elements()
        .iter()
        .map(|s| s.points.as_ptr())
        .collect();
    let revision = canvas.editor.revision();
    canvas.selected = Some((0, revision));
    canvas.multi_selection = Some((vec![0, 3], revision));
    let (request, snapshot) = prepare(&canvas, true).unwrap();
    assert_eq!(request.indices, vec![0, 3]);
    let mut job = spawn(request, snapshot).unwrap();
    let result = finish(&mut job);
    assert_eq!(canvas.editor.revision(), revision);
    assert_eq!(canvas.editor.elements().len(), 5);
    assert!(accept(&mut canvas, &job.request, result).unwrap());
    assert_eq!(canvas.editor.revision(), revision + 1);
    assert_eq!(canvas.editor.elements().len(), 3);
    let source = crate::raster_source::compose(canvas.editor.layers().unwrap(), || false).unwrap();
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
    for _ in 0..3 {
        let first = canvas.editor.elements()[1].blur.as_ref().unwrap();
        let second = canvas.editor.elements()[2].blur.as_ref().unwrap();
        assert_eq!(&first.pixels()[..], &expected[..]);
        assert!(first.pixels().same_storage(second.pixels()));
        assert!(canvas.editor.undo());
        assert!(canvas.editor.layers().unwrap().same_storage(&original));
        assert_eq!(
            canvas
                .editor
                .elements()
                .iter()
                .map(|s| s.points.as_ptr())
                .collect::<Vec<_>>(),
            pointers
        );
        for index in [2, 4] {
            assert!(canvas.editor.elements()[index]
                .blur
                .as_ref()
                .unwrap()
                .pixels()
                .same_storage(old.pixels()));
        }
        assert!(canvas.editor.redo());
    }
    // 放弃重做分支时，已移出的矢量与效果候选均按历史预算释放。
    assert!(canvas.editor.undo());
    canvas
        .editor
        .begin(Tool::Brush, Point { x: 1.0, y: 1.0 })
        .unwrap();
    assert!(canvas.editor.commit());
    assert!(!canvas.editor.redo());
    assert_eq!(canvas.editor.elements().len(), 6);
}
