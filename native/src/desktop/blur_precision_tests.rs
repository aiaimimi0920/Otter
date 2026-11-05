//! 分数强度从选择、连续调节、工作线程到历史和菜单模型的端到端回归。
use super::*;

#[test]
fn fractional_new_tool_preparation_survives_worker_and_rejects_same_rounded_cache() {
    let mut canvas = Canvas::new(Some(crate::desktop::image::Image {
        width: 8,
        height: 8,
        pixels: vec![255; 256].into(),
    }))
    .unwrap();
    canvas.select_tool(Tool::Blur);
    assert!(canvas.tool_profiles.set_blur_strength(2.4));
    let request = Request {
        stamp: canvas.editor.document_stamp(),
        target: None,
        annotation_frame: None,
        base_projection: None,
        source: canvas.editor.layers().unwrap().clone(),
        size: (8, 8),
        area: Region {
            x: 0,
            y: 0,
            width: 8,
            height: 8,
        },
        sigma: canvas.tool_profiles.blur_strength(),
        revision: canvas.editor.revision(),
    };
    let mut job = crate::blur_job::Job::default();
    job.request(request).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let completed = loop {
        if let Some(completed) = job.poll() {
            break completed;
        }
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(1));
    };
    assert!(accept(&mut canvas, completed).unwrap());
    assert_eq!(canvas.blur_ready.as_ref().unwrap().1.strength(), 2.4);
    let point = crate::viewport::Point {
        x: canvas.viewport.origin.x + 4.0 * canvas.viewport.scale,
        y: canvas.viewport.origin.y + 4.0 * canvas.viewport.scale,
    };
    assert!(canvas.pointer_down(point));
    canvas.pointer_up(point);
    assert_eq!(
        canvas.editor.elements()[0]
            .blur
            .as_ref()
            .unwrap()
            .strength(),
        2.4
    );
    assert!(canvas.tool_profiles.set_blur_strength(2.49));
    assert!(!canvas.pointer_down(point));
    assert_eq!(canvas.editor.elements().len(), 1);
    canvas.select_tool(Tool::Brush);
    canvas.select_tool(Tool::Blur);
    assert_eq!(canvas.tool_profiles.blur_strength(), 2.49);
    for invalid in [f32::NAN, f32::INFINITY, 1.99, 64.01] {
        assert!(!canvas.tool_profiles.set_blur_strength(invalid));
        assert_eq!(canvas.tool_profiles.blur_strength(), 2.49);
    }
}

#[test]
fn absolute_fraction_survives_worker_commit_and_undo_redo() {
    let mut pixels = vec![255; 8 * 8 * 4];
    for (index, pixel) in pixels.chunks_exact_mut(4).enumerate() {
        pixel[0] = (index * 3) as u8;
    }
    let mut canvas = Canvas::new(Some(crate::desktop::image::Image {
        width: 8,
        height: 8,
        pixels: pixels.into(),
    }))
    .unwrap();
    let original = BlurImage::with_strength(8, 8, vec![255; 256].into(), 64.0).unwrap();
    canvas
        .editor
        .begin_blur(crate::viewport::Point { x: 4.0, y: 4.0 }, original.clone())
        .unwrap();
    assert!(canvas.editor.commit());
    canvas.select_tool(Tool::Select);
    canvas.selected = Some((0, canvas.editor.revision()));
    let sigma = 1.234567f32;
    let request = super::super::blur_selection::absolute(&canvas, None, sigma).unwrap();
    let expected = crate::blur::region_precise(
        &request.source.base,
        request.size,
        request.area,
        sigma,
        || false,
    )
    .unwrap();
    let mut job = crate::blur_job::Job::default();
    job.request(request).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let completed = loop {
        if let Some(completed) = job.poll() {
            break completed;
        }
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(1));
    };
    assert!(accept(&mut canvas, completed).unwrap());
    let result = canvas.editor.elements()[0].blur.as_ref().unwrap();
    assert_eq!(result.strength().to_bits(), sigma.to_bits());
    assert_eq!(&result.pixels()[..], &expected[..]);
    assert!(canvas.editor.undo());
    assert!(canvas.editor.elements()[0]
        .blur
        .as_ref()
        .unwrap()
        .pixels()
        .same_storage(original.pixels()));
    assert!(canvas.editor.redo());
    assert_eq!(
        canvas.editor.elements()[0]
            .blur
            .as_ref()
            .unwrap()
            .strength()
            .to_bits(),
        sigma.to_bits()
    );
}

#[test]
fn fractional_selection_adjustment_keeps_precision_through_worker_history_and_menu() {
    let mut canvas = Canvas::new(Some(crate::desktop::image::Image {
        width: 16,
        height: 16,
        pixels: vec![255; 16 * 16 * 4].into(),
    }))
    .unwrap();
    let original = BlurImage::with_strength(16, 16, vec![255; 16 * 16 * 4].into(), 4.4).unwrap();
    canvas
        .editor
        .begin_blur(crate::viewport::Point { x: 8.0, y: 8.0 }, original.clone())
        .unwrap();
    assert!(canvas.editor.commit());
    canvas.select_tool(Tool::Select);
    canvas.selected = Some((0, canvas.editor.revision()));
    assert_eq!(selected_strength(&canvas), Some(4.4));
    let first = selection_request(&canvas, None, 1.0).unwrap();
    assert!((first.sigma - 5.4).abs() < 0.00001);
    let latest = selection_request(&canvas, Some(&first), 1.0).unwrap();
    assert!((latest.sigma - 6.4).abs() < 0.00001);
    assert!(selection_request(&canvas, None, f32::NAN).is_none());
    assert!(selection_request(&canvas, None, f32::INFINITY).is_none());
    assert_eq!(selection_request(&canvas, None, -100.0).unwrap().sigma, 1.0);
    assert_eq!(selection_request(&canvas, None, 100.0).unwrap().sigma, 64.0);
    let expected = crate::blur::region_precise(
        &latest.source.base,
        latest.size,
        latest.area,
        latest.sigma,
        || false,
    )
    .unwrap();
    let mut job = crate::blur_job::Job::default();
    job.request(first).unwrap();
    job.request(latest).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let result = loop {
        if let Some(result) = job.poll() {
            break result;
        }
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(1));
    };
    assert!(accept(&mut canvas, result).unwrap());
    let current = canvas.editor.elements()[0].blur.as_ref().unwrap();
    assert!((current.strength() - 6.4).abs() < 0.00001);
    assert_eq!(&current.pixels()[..], &expected[..]);
    assert_eq!(selected_strength(&canvas), Some(current.strength()));
    assert_eq!(canvas.tool_profiles.blur_strength(), 8.0);
    assert!(canvas.editor.undo());
    assert!(canvas.editor.elements()[0]
        .blur
        .as_ref()
        .unwrap()
        .pixels()
        .same_storage(original.pixels()));
    assert_eq!(
        canvas.editor.elements()[0]
            .blur
            .as_ref()
            .unwrap()
            .strength(),
        4.4
    );
    assert!(canvas.editor.redo());
    assert!(
        (canvas.editor.elements()[0]
            .blur
            .as_ref()
            .unwrap()
            .strength()
            - 6.4)
            .abs()
            < 0.00001
    );
}
