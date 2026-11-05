//! D2D/WIC实际PNG与命中共同核固定L形oracle；不从新几何生成预期像素。
use super::*;
use crate::{
    editor::Tool,
    stroke_style::{Color, StrokeStyle},
};
use serde_json::{json, value::RawValue};

fn source() -> Image {
    Image {
        width: 64,
        height: 64,
        pixels: vec![0; 64 * 64 * 4].into(),
    }
}
fn restored(kind: &str, points: serde_json::Value) -> Editor {
    let raw = serde_json::value::to_raw_value(&json!({"serialCounter":0,"elements":[{
        "id":"bent","type":kind,"zIndex":0,"points":points,
        "style":{"width":4,"color":"#ff0000"}}]}))
    .unwrap();
    crate::session_vectors::restore(Some(&raw)).unwrap().editor
}
fn bent(kind: &str) -> Editor {
    restored(
        kind,
        json!([{"x":8,"y":8},{"x":8,"y":48},{"x":56,"y":48},{"x":56,"y":48}]),
    )
}
fn pixel(image: &Image, x: usize, y: usize, expected: [u8; 4]) {
    assert_eq!(
        &image.pixels[(y * 64 + x) * 4..(y * 64 + x + 1) * 4],
        &expected,
        "({x},{y})"
    );
}
fn render(editor: &Editor) -> Image {
    tests::decode_png(encode(&source(), editor).unwrap())
}
unsafe fn pick(factory: &ID2D1Factory, editor: &Editor, x: f32, y: f32) -> Option<usize> {
    super::super::selection_hit::pick(factory, editor, Point { x, y }, 0.1).unwrap()
}

#[test]
fn bent_line_renders_and_picks_both_legs_not_the_first_last_chord() {
    let _com = Apartment::new().unwrap();
    let editor = bent("line");
    let image = render(&editor);
    pixel(&image, 8, 28, [0, 0, 255, 255]);
    pixel(&image, 28, 48, [0, 0, 255, 255]);
    pixel(&image, 32, 28, [0; 4]);
    unsafe {
        let factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).unwrap();
        assert_eq!(pick(&factory, &editor, 8.0, 28.0), Some(0));
        assert_eq!(pick(&factory, &editor, 28.0, 48.0), Some(0));
        assert_eq!(pick(&factory, &editor, 32.0, 28.0), None);
    }
}
#[test]
fn bent_arrow_uses_last_effective_segment_for_pixels_hit_bounds_and_save_restore() {
    let _com = Apartment::new().unwrap();
    for kind in ["arrow", "line"] {
        let mut editor = bent(kind);
        if kind == "line" {
            assert!(editor.restyle(0, editor.elements()[0].style().with_arrow(true)));
        }
        let expected = render(&editor);
        pixel(&expected, 8, 28, [0, 0, 255, 255]);
        pixel(&expected, 20, 48, [0, 0, 255, 255]);
        pixel(&expected, 36, 54, [0, 0, 255, 255]);
        pixel(&expected, 32, 28, [0; 4]);
        let bounds = crate::group_bounds::bounds(&editor.elements()[0]).unwrap();
        assert_eq!(bounds.low, Point { x: 6.0, y: 6.0 });
        assert_eq!(bounds.high, Point { x: 58.0, y: 60.0 });
        unsafe {
            let factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).unwrap();
            assert_eq!(pick(&factory, &editor, 8.0, 28.0), Some(0));
            assert_eq!(pick(&factory, &editor, 36.0, 54.0), Some(0));
            assert_eq!(pick(&factory, &editor, 32.0, 28.0), None);
        }
        let bytes = crate::session_vector_write::encode(&editor, None, |_| unreachable!()).unwrap();
        let raw = RawValue::from_string(String::from_utf8(bytes).unwrap()).unwrap();
        let reopened = crate::session_vectors::restore(Some(&raw)).unwrap().editor;
        assert_eq!(render(&reopened).pixels, expected.pixels);
    }
}
#[test]
fn single_point_flat_paths_remain_empty_but_freehand_polyline_keeps_existing_dab() {
    let _com = Apartment::new().unwrap();
    unsafe {
        let factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).unwrap();
        for kind in ["line", "arrow"] {
            for points in [
                json!([{"x":24,"y":24}]),
                json!([{"x":24,"y":24},{"x":24,"y":24}]),
            ] {
                let editor = restored(kind, points);
                assert!(render(&editor).pixels.iter().all(|p| *p == 0), "{kind}");
                assert_eq!(pick(&factory, &editor, 24.0, 24.0), None, "{kind}");
            }
        }
        let editor = restored("polyline", json!([{"x":24,"y":24}]));
        pixel(&render(&editor), 24, 24, [0, 0, 255, 255]);
        assert_eq!(pick(&factory, &editor, 24.0, 24.0), Some(0));
    }
}
#[test]
fn bent_path_replacement_move_undo_and_png_preserve_target_base_and_identity() {
    let _com = Apartment::new().unwrap();
    let mut target = Editor::with_layers(crate::erase_layers::Layers {
        size: (64, 64),
        base: vec![0; 64 * 64 * 4].into(),
        annotation_size: None,
        annotations: None,
    })
    .unwrap();
    target.set_style(StrokeStyle::new(2.0, Color(0, 255, 0)).unwrap());
    target
        .begin(Tool::Brush, Point { x: 40.0, y: 8.0 })
        .unwrap();
    target.commit();
    let base = target.layers().unwrap().clone();
    let source = bent("arrow");
    assert!(target
        .prepare_annotation_replacement(&source)
        .unwrap()
        .apply(&mut target)
        .unwrap());
    let expected = render(&target);
    assert!(target.translate(0, Point { x: 2.0, y: 0.0 }).unwrap());
    assert!(target.undo());
    assert_eq!(render(&target).pixels, expected.pixels);
    assert_eq!(target.elements()[0].annotation_id(), "bent");
    assert!(target.undo());
    assert_eq!(target.elements()[0].tool, Tool::Brush);
    assert!(target.redo());
    assert_eq!(render(&target).pixels, expected.pixels);
    assert!(target.layers().unwrap().same_storage(&base));
}

#[test]
fn bent_arrow_group_opacity_is_not_applied_twice_at_the_head_base_or_bend() {
    let _com = Apartment::new().unwrap();
    let mut editor = bent("arrow");
    let style = editor.elements()[0].style().with_opacity(0.5).unwrap();
    assert!(editor.restyle(0, style));
    let output = render(&editor);
    let max = output.pixels.chunks_exact(4).map(|p| p[3]).max().unwrap();
    assert!(max.abs_diff(128) <= 1, "max={max}");
    for (x, y) in [(8, 28), (20, 48), (36, 54)] {
        let alpha = output.pixels[(y * 64 + x) * 4 + 3];
        assert!(alpha.abs_diff(128) <= 1, "({x},{y}) alpha={alpha}");
    }
}
