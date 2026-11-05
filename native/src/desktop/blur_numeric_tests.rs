//! 输入快照不接受跨文档、跨工具、跨选择或待处理强度变化。
use super::*;
fn fixture() -> Canvas {
    let mut canvas = Canvas::new(Some(crate::desktop::image::Image {
        width: 8,
        height: 8,
        pixels: vec![255; 256].into(),
    }))
    .unwrap();
    let image =
        crate::blur_image::BlurImage::with_strength(8, 8, vec![255; 256].into(), 64.0).unwrap();
    canvas
        .editor
        .begin_blur(crate::viewport::Point { x: 4.0, y: 4.0 }, image)
        .unwrap();
    assert!(canvas.editor.commit());
    canvas.select_tool(crate::editor::Tool::Select);
    canvas.selected = Some((0, canvas.editor.revision()));
    canvas
}
#[test]
fn absolute_fraction_is_exact_and_invalid_or_unchanged_input_is_rejected() {
    let canvas = fixture();
    let requested = 1.234567f32;
    let request = Pending::capture(&canvas, None)
        .unwrap()
        .request(&canvas, None, &requested.to_string())
        .unwrap();
    assert_eq!(request.sigma.to_bits(), requested.to_bits());
    assert_eq!(request.target, Some(0));
    for invalid in ["64", "0", "64.1", "NaN", "inf", "-1", ""] {
        assert!(Pending::capture(&canvas, None)
            .unwrap()
            .request(&canvas, None, invalid)
            .is_none());
    }
    assert_eq!(
        super::super::blur_selection::absolute(&canvas, None, 1.0)
            .unwrap()
            .sigma,
        1.0
    );
}
#[test]
fn stale_document_tool_selection_and_pending_strength_do_not_generate_requests() {
    let mut canvas = fixture();
    let pending = Pending::capture(&canvas, None).unwrap();
    canvas.editor = canvas.editor.export_snapshot();
    assert!(pending.request(&canvas, None, "5").is_none());
    canvas.selected = Some((0, canvas.editor.revision()));
    let pending = Pending::capture(&canvas, None).unwrap();
    canvas.select_tool(crate::editor::Tool::Brush);
    assert!(pending.request(&canvas, None, "5").is_none());
    canvas.select_tool(crate::editor::Tool::Select);
    canvas.selected = Some((0, canvas.editor.revision()));
    let pending = Pending::capture(&canvas, None).unwrap();
    canvas.selected = None;
    assert!(pending.request(&canvas, None, "5").is_none());
    canvas.selected = Some((0, canvas.editor.revision()));
    let previous = super::super::blur_selection::absolute(&canvas, None, 4.4).unwrap();
    let pending = Pending::capture(&canvas, Some(&previous)).unwrap();
    assert_eq!(pending.strength, 4.4);
    let newer = super::super::blur_selection::absolute(&canvas, Some(&previous), 5.4).unwrap();
    assert!(pending.request(&canvas, Some(&newer), "7").is_none());
}
