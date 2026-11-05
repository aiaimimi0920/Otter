//! 跨 Unit 自然源必须匹配真实来源文档；不依赖 PNG 渲染实现生成期望结果。
use super::*;
use crate::desktop::canvas::{minified_source::Source, Canvas};
use crate::{editor::Tool, viewport::Point};

fn image(width: u32, value: u8) -> Image {
    Image {
        width,
        height: 2,
        pixels: vec![value; width as usize * 8].into(),
    }
}
fn source() -> (UnitId, Canvas, Image) {
    let mut canvas = Canvas::new(Some(image(2, 80))).unwrap();
    let natural = image(8, 160);
    canvas.minified_source = Some(Source::new(natural.clone(), &canvas.editor));
    (UnitId::parse("source").unwrap(), canvas, natural)
}

#[test]
fn local_frame_recovers_only_matching_source_natural_pixels() {
    let (unit, canvas, natural) = source();
    let frame = Frame::capture(
        unit.clone(),
        canvas.image.clone().unwrap(),
        Some(&canvas.editor),
    );
    let selected = frame.natural_source(&unit, &canvas).unwrap();
    assert_eq!((selected.width, selected.height), (8, 2));
    assert!(selected.pixels.same_storage(&natural.pixels));
    assert!(frame
        .natural_source(&UnitId::parse("other").unwrap(), &canvas)
        .is_none());
    let (_, different, _) = source();
    assert!(frame.natural_source(&unit, &different).is_none());
}

#[test]
fn source_revision_change_rejects_old_frame_even_when_retained_still_matches() {
    let (unit, mut canvas, _) = source();
    let frame = Frame::capture(
        unit.clone(),
        canvas.image.clone().unwrap(),
        Some(&canvas.editor),
    );
    canvas
        .editor
        .begin(Tool::Brush, Point { x: 1.0, y: 1.0 })
        .unwrap();
    canvas.editor.commit();
    assert!(canvas
        .minified_source
        .as_ref()
        .unwrap()
        .matches(&canvas.editor));
    assert!(frame.natural_source(&unit, &canvas).is_none());
    let refreshed = Frame::capture(
        unit.clone(),
        canvas.image.clone().unwrap(),
        Some(&canvas.editor),
    );
    assert!(refreshed.natural_source(&unit, &canvas).is_some());
    canvas.replace_image(image(2, 120)).unwrap();
    assert!(refreshed.natural_source(&unit, &canvas).is_none());
}

#[test]
fn formal_and_overlay_frames_never_borrow_local_retained_pixels() {
    let (unit, canvas, _) = source();
    let formal = Frame::capture(unit.clone(), image(2, 200), None);
    assert!(!formal.has_local_source());
    assert!(formal.natural_source(&unit, &canvas).is_none());
    let overlay =
        Frame::capture_overlay(unit.clone(), unit.clone(), image(2, 200), &canvas.editor).unwrap();
    assert!(!overlay.has_local_source());
    assert!(overlay.natural_source(&unit, &canvas).is_none());
}
