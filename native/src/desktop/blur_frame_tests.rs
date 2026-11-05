//! 独立标注画框的后台模糊与缓存失效；不创建 HWND 或读取用户文件。
use super::*;
use crate::{crop_geometry::Rect, erase_layers::Layers, viewport::Point};

fn canvas() -> Canvas {
    let mut canvas = Canvas::new(Some(crate::desktop::image::Image {
        width: 16,
        height: 16,
        pixels: vec![0; 16 * 16 * 4].into(),
    }))
    .unwrap();
    canvas.editor = crate::editor::Editor::with_layers(Layers {
        annotation_size: None,
        size: (16, 16),
        base: vec![0; 16 * 16 * 4].into(),
        annotations: Some(
            (0..256)
                .flat_map(|i| {
                    if i % 16 < 8 {
                        [0, 0, 255, 255]
                    } else {
                        [0, 255, 0, 255]
                    }
                })
                .collect::<Vec<_>>()
                .into(),
        ),
    })
    .unwrap();
    canvas.sync_raster_source();
    canvas.select_tool(Tool::Blur);
    canvas
}
fn request(canvas: &Canvas) -> Request {
    Request {
        stamp: canvas.editor.document_stamp(),
        target: None,
        annotation_frame: canvas.editor.crop_view().map(|view| view.source.rect),
        base_projection: canvas.editor.raster_base_projection(),
        source: canvas.editor.layers().unwrap().clone(),
        size: (16, 16),
        area: Region {
            x: 0,
            y: 0,
            width: 16,
            height: 16,
        },
        sigma: canvas.tool_profiles.blur_strength(),
        revision: canvas.editor.revision(),
    }
}

#[test]
fn legacy_worker_and_cache_follow_base_mapping_even_when_annotation_frame_is_unchanged() {
    let mut canvas = canvas();
    let size = crate::crop_geometry::Size {
        width: 16.0,
        height: 16.0,
    };
    canvas.editor = crate::editor::Editor::with_layers(Layers {
        size: (16, 16),
        base: (0..256)
            .flat_map(|i| {
                let v = if (3..=8).contains(&(i % 16)) { 64 } else { 192 };
                [v, v, v, 255]
            })
            .collect::<Vec<_>>()
            .into(),
        annotations: None,
        annotation_size: None,
    })
    .unwrap()
    .with_restored_size(size)
    .unwrap()
    .with_restored_legacy_crop(
        Rect {
            x: 4.0,
            y: 0.0,
            width: 4.0,
            height: 16.0,
        },
        size,
    )
    .unwrap();
    canvas.sync_raster_source();
    let mut invalid = request(&canvas);
    invalid.base_projection = None;
    assert!(!accept(&mut canvas, compute(invalid)).unwrap());
    let valid = request(&canvas);
    let sigma = valid.sigma as f64;
    let completed = compute(valid);
    assert!(accept(&mut canvas, completed).unwrap());
    // 既有Gaussian合同是图像外透明，不是边缘clamp。对常色矩形独立积分离散核。
    let radius = (sigma * 3.0).ceil() as i32;
    let weight = |d: i32| (-(d as f64).powi(2) / (2.0 * sigma * sigma)).exp();
    let normalization: f64 = (-radius..=radius).map(weight).sum();
    let mass = |p: i32| {
        (0..16)
            .map(|q: i32| q - p)
            .filter(|d| d.abs() <= radius)
            .map(weight)
            .sum::<f64>()
            / normalization
    };
    let expected: Vec<u8> = (0..16)
        .flat_map(|y| {
            (0..16).flat_map(move |x| {
                let coverage = mass(x) * mass(y);
                [64.0, 64.0, 64.0, 255.0].map(|v| (v * coverage).round() as u8)
            })
        })
        .collect();
    assert_eq!(
        &canvas.blur_ready.as_ref().unwrap().1.pixels()[..],
        &expected
    );
    let before = canvas.editor.annotation_frame();
    assert!(canvas.editor.reset_crop());
    assert_eq!(before, canvas.editor.annotation_frame());
    assert!(!canvas.pointer_down(Point { x: 8.0, y: 8.0 }));
    canvas.sync_raster_source();
    assert!(canvas.blur_ready.is_none());
}
fn compute(request: Request) -> Completed {
    let mut job = crate::blur_job::Job::default();
    job.request(request).unwrap();
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
fn replacement_document_rejects_prepared_cache_with_same_pixels_and_revision() {
    let mut canvas = canvas();
    let completed = compute(request(&canvas));
    canvas.editor = canvas.editor.export_snapshot();
    assert!(!accept(&mut canvas, completed).unwrap());
    assert!(canvas.blur_ready.is_none());
}

#[test]
fn replacement_document_does_not_inherit_pending_strength_or_result() {
    let mut canvas = canvas();
    let original = BlurImage::with_strength(16, 16, vec![255; 16 * 16 * 4].into(), 4.4).unwrap();
    canvas
        .editor
        .begin_blur(crate::viewport::Point { x: 8.0, y: 8.0 }, original.clone())
        .unwrap();
    assert!(canvas.editor.commit());
    canvas.select_tool(Tool::Select);
    canvas.selected = Some((0, canvas.editor.revision()));
    let previous = selection_request(&canvas, None, 1.0).unwrap();
    let completed = compute(previous.clone());
    canvas.editor = canvas.editor.export_snapshot();
    let next = selection_request(&canvas, Some(&previous), 1.0).unwrap();
    assert!((next.sigma - 5.4).abs() < 0.00001);
    assert!(!accept(&mut canvas, completed).unwrap());
    assert!(canvas.editor.elements()[0]
        .blur
        .as_ref()
        .unwrap()
        .pixels()
        .same_storage(original.pixels()));
}
#[test]
fn cropped_worker_prepares_visible_overlay_and_rejects_old_cache_after_reset() {
    let mut canvas = canvas();
    let rect = Rect {
        x: 8.0,
        y: 4.0,
        width: 8.0,
        height: 4.0,
    };
    canvas.editor.crop_to(rect).unwrap();
    let work = request(&canvas);
    let source =
        crate::raster_source::compose_in_frame(&work.source, Some(rect), || false).unwrap();
    let expected =
        crate::blur::region_precise(&source, work.size, work.area, work.sigma, || false).unwrap();
    assert!(accept(&mut canvas, compute(work)).unwrap());
    assert_eq!(
        &canvas.blur_ready.as_ref().unwrap().1.pixels()[..],
        &expected[..]
    );
    let layers = canvas.editor.layers().unwrap().clone();
    assert!(canvas.editor.reset_crop());
    assert!(canvas.editor.layers().unwrap().same_storage(&layers));
    // 尚未 sync 时指针入口也必须拒绝旧画框缓存。
    assert!(!canvas.pointer_down(Point { x: 10.0, y: 6.0 }));
    canvas.sync_raster_source();
    assert!(canvas.blur_ready.is_none());
}
#[test]
fn frame_mismatch_is_rejected_even_with_matching_revision_and_source() {
    let mut canvas = canvas();
    let mut work = request(&canvas);
    work.annotation_frame = Some(Rect {
        x: 2.0,
        y: 2.0,
        width: 8.0,
        height: 8.0,
    });
    assert!(!accept(&mut canvas, compute(work)).unwrap());
    assert!(canvas.blur_ready.is_none());
}
#[test]
fn strength_request_captures_frame_and_keeps_fractional_value() {
    let mut canvas = canvas();
    canvas
        .editor
        .begin_blur(
            Point { x: 10.0, y: 6.0 },
            BlurImage::with_strength(16, 16, vec![0; 1024].into(), 2.4).unwrap(),
        )
        .unwrap();
    canvas.editor.commit();
    let rect = Rect {
        x: 8.0,
        y: 4.0,
        width: 8.0,
        height: 4.0,
    };
    canvas.editor.crop_to(rect).unwrap();
    canvas.select_tool(Tool::Select);
    canvas.selected = Some((0, canvas.editor.revision()));
    let work = selection_request(&canvas, None, 1.0).unwrap();
    assert_eq!(work.annotation_frame, Some(rect));
    assert_eq!(work.sigma, 3.4);
    assert!(accept(&mut canvas, compute(work)).unwrap());
    assert_eq!(
        canvas.editor.elements()[0]
            .blur
            .as_ref()
            .unwrap()
            .strength(),
        3.4
    );
    assert!(canvas.editor.undo());
    assert_eq!(
        canvas.editor.elements()[0]
            .blur
            .as_ref()
            .unwrap()
            .strength(),
        2.4
    );
}
