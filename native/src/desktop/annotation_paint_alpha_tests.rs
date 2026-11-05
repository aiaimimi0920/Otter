//! 真D2D/WIC合成与PNG解码，独立source-over常量oracle，不用新渲染器生成预期。
use super::*;
use crate::stroke_style::PaintAlpha;
use serde_json::{json, value::RawValue};

fn image() -> Image {
    Image {
        width: 64,
        height: 64,
        pixels: vec![0; 64 * 64 * 4].into(),
    }
}
fn restored(kind: &str) -> Editor {
    let mut object = json!({"id":"paint","type":kind,"zIndex":0,"x":16,"y":16,"w":32,"h":32,
        "style":{"width":8,"color":"rgba(255,0,0,0.25)","fill":"rgba(0,0,255,0.5)",
            "opacity":0.5,"cornerRadius":4}});
    if kind == "serial" {
        object.as_object_mut().unwrap().remove("w");
        object.as_object_mut().unwrap().remove("h");
        object["y"] = json!(41);
        object["text"] = json!("I");
        object["fontSize"] = json!(18);
        object["style"]["cornerRadius"] = json!(16);
    }
    let raw: Box<RawValue> =
        serde_json::value::to_raw_value(&json!({"serialCounter":1,"elements":[object]})).unwrap();
    crate::session_vectors::restore(Some(&raw)).unwrap().editor
}
fn render(editor: &Editor) -> Image {
    super::super::tests::decode_png(encode(&image(), editor).unwrap())
}
fn check(image: &Image, x: usize, y: usize, expected: [u8; 4]) {
    let actual = &image.pixels[(y * 64 + x) * 4..(y * 64 + x + 1) * 4];
    for (a, e) in actual.iter().zip(expected) {
        assert!(
            a.abs_diff(e) <= 1,
            "({x},{y}) actual={actual:?} expected={expected:?}"
        );
    }
}
#[test]
fn fill_outline_and_group_opacity_combine_in_order_including_overlap() {
    let _com = Apartment::new().unwrap();
    for kind in ["rect", "round-rect", "ellipse"] {
        let image = render(&restored(kind));
        check(&image, 32, 32, [64, 0, 0, 64]); // fill .5 * group .5
        check(&image, 14, 32, [0, 0, 32, 32]); // stroke .25 * group .5
        check(&image, 18, 32, [48, 0, 32, 80]); // (.5 blue under .25 red), then group .5
        check(&image, 4, 4, [0; 4]);
    }
}
#[test]
fn polygon_triangle_and_serial_use_independent_fill_and_outline_paints() {
    let _com = Apartment::new().unwrap();
    for kind in ["polygon", "triangle"] {
        check(&render(&restored(kind)), 32, 32, [64, 0, 0, 64]);
    }
    let serial = render(&restored("serial"));
    check(&serial, 24, 32, [64, 0, 0, 64]);
    check(&serial, 14, 32, [0, 0, 32, 32]);
    check(&serial, 18, 32, [48, 0, 32, 80]);
}
#[test]
fn zero_stroke_alpha_keeps_fill_and_save_restore_repeats_exact_pixels() {
    let _com = Apartment::new().unwrap();
    let mut editor = restored("rect");
    let style = editor.elements()[0]
        .style()
        .with_paint_alpha(PaintAlpha::new(0.0, 0.5).unwrap());
    assert!(editor.restyle(0, style));
    let original = render(&editor);
    check(&original, 14, 32, [0; 4]);
    check(&original, 18, 32, [64, 0, 0, 64]);
    for _ in 0..4 {
        let encoded =
            crate::session_vector_write::encode(&editor, None, |_| unreachable!()).unwrap();
        let raw = RawValue::from_string(String::from_utf8(encoded).unwrap()).unwrap();
        editor = crate::session_vectors::restore(Some(&raw)).unwrap().editor;
        assert_eq!(render(&editor).pixels, original.pixels);
    }
}
#[test]
fn path_arrow_and_paintbrush_alpha_is_applied_once_not_at_self_crossings() {
    let _com = Apartment::new().unwrap();
    for tool in [
        Tool::Brush,
        Tool::Highlighter,
        Tool::Paintbrush,
        Tool::Polyline,
        Tool::Line,
        Tool::Arrow,
    ] {
        let mut editor = Editor::default();
        editor.set_style(
            StrokeStyle::new(8.0, Color(255, 0, 0))
                .unwrap()
                .with_opacity(0.5)
                .unwrap()
                .with_paint_alpha(PaintAlpha::new(0.5, 1.0).unwrap()),
        );
        editor.begin(tool, Point { x: 8.0, y: 8.0 }).unwrap();
        for (x, y) in [(56.0, 56.0), (8.0, 56.0), (56.0, 8.0)] {
            editor.extend(Point { x, y }).unwrap();
        }
        assert!(editor.commit());
        let output = render(&editor);
        let expected = if tool == Tool::Highlighter { 22 } else { 64 };
        let max = output.pixels.chunks_exact(4).map(|p| p[3]).max().unwrap();
        assert!(max.abs_diff(expected) <= 1, "{tool:?} max={max}");
        if !matches!(tool, Tool::Line | Tool::Arrow) {
            check(&output, 32, 32, [0, 0, expected, expected]);
        }
    }
}
#[test]
fn text_color_alpha_is_independent_of_group_opacity() {
    let _com = Apartment::new().unwrap();
    let mut editor = Editor::default();
    editor.set_style(
        StrokeStyle::default()
            .with_color(Color(255, 0, 0))
            .with_opacity(0.5)
            .unwrap()
            .with_paint_alpha(PaintAlpha::new(0.25, 1.0).unwrap()),
    );
    editor
        .insert_text(
            Point { x: 8.0, y: 8.0 },
            Point { x: 48.0, y: 48.0 },
            crate::text_annotation::TextAnnotation::new("M", "Segoe UI", 36.0, false).unwrap(),
        )
        .unwrap();
    let output = render(&editor);
    let max = output.pixels.chunks_exact(4).map(|p| p[3]).max().unwrap();
    assert!(max.abs_diff(32) <= 1, "text alpha={max}");
    assert!(output
        .pixels
        .chunks_exact(4)
        .all(|p| p[0] == 0 && p[1] == 0 && p[2] <= 33 && p[3] <= 33));
}
