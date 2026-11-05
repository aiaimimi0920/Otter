//! 真D2D/WIC、DirectWrite及命中；像素oracle为固定色块，不由新适配器生成。
use super::*;
use serde_json::{json, Value};
fn restore(object: Value) -> Editor {
    let raw =
        serde_json::value::to_raw_value(&json!({"elements":[object],"serialCounter":0})).unwrap();
    crate::session_vectors::restore_with_text(Some(&raw), measure)
        .unwrap()
        .editor
}
fn measure(
    text: &crate::text_annotation::TextAnnotation,
) -> std::result::Result<crate::session_vectors::TextMetrics, crate::session_vectors::Error> {
    unsafe {
        Ok(crate::session_vectors::TextMetrics {
            size: super::super::annotation_text::measure(text)
                .map_err(|_| crate::session_vectors::Error::Geometry)?,
            baseline: super::super::annotation_text::measure_flip_metrics(text)
                .map_err(|_| crate::session_vectors::Error::Geometry)?
                .baseline_from_top,
        })
    }
}
fn object(kind: &str) -> Value {
    json!({"id":"domain","type":kind,"zIndex":1.25,"x":16,"y":16,"w":32,"h":32,"style":{"width":0,"color":"#ff0000","fill":"#0000ff","cornerRadius":8192},"sides":64})
}
fn render(editor: &Editor) -> Image {
    tests::decode_png(
        encode(
            &Image {
                width: 64,
                height: 64,
                pixels: vec![0; 64 * 64 * 4].into(),
            },
            editor,
        )
        .unwrap(),
    )
}
fn pixel(image: &Image, x: usize, y: usize) -> &[u8] {
    &image.pixels[(y * 64 + x) * 4..(y * 64 + x + 1) * 4]
}
#[test]
fn zero_width_is_not_a_hairline_and_64_sided_round_polygon_has_safe_geometry() {
    let _com = Apartment::new().unwrap();
    for kind in ["rect", "round-rect", "ellipse", "triangle", "polygon"] {
        let editor = restore(object(kind));
        let image = render(&editor);
        assert_eq!(pixel(&image, 32, 32), [255, 0, 0, 255], "{kind}");
        assert_eq!(
            pixel(&image, 15, 32),
            [0; 4],
            "{kind} must not draw width=0 hairline"
        );
        unsafe {
            let factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).unwrap();
            assert_eq!(
                super::super::selection_hit::pick(
                    &factory,
                    &editor,
                    Point { x: 32.0, y: 32.0 },
                    0.1
                )
                .unwrap(),
                Some(0)
            );
            assert_eq!(
                super::super::selection_hit::pick(
                    &factory,
                    &editor,
                    Point { x: 15.0, y: 32.0 },
                    0.1
                )
                .unwrap(),
                None
            );
        }
    }
}
#[test]
fn zero_shape_and_fully_transparent_paints_neither_render_nor_capture_input() {
    let _com = Apartment::new().unwrap();
    for kind in ["rect", "round-rect", "ellipse", "triangle", "polygon"] {
        for zero in [true, false] {
            let mut raw = object(kind);
            if zero {
                raw["w"] = json!(0);
            } else {
                raw["style"]["fill"] = json!("transparent");
                raw["style"]["width"] = json!(8192);
                raw["style"]["color"] = json!("transparent");
            }
            let editor = restore(raw);
            assert!(
                render(&editor).pixels.iter().all(|b| *b == 0),
                "{kind} zero={zero}"
            );
            unsafe {
                let factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).unwrap();
                assert_eq!(
                    super::super::selection_hit::pick(
                        &factory,
                        &editor,
                        Point { x: 16.0, y: 32.0 },
                        5.0
                    )
                    .unwrap(),
                    None
                );
            }
        }
    }
}
#[test]
fn maximum_width_and_rotation_render_periodically_and_save_without_ui_clipping() {
    let _com = Apartment::new().unwrap();
    let mut raw = object("rect");
    raw["style"]["width"] = json!(8192);
    raw["rotation"] = json!(36000);
    let editor = restore(raw.clone());
    let before = render(&editor);
    for (x, y) in [(0, 0), (32, 32), (63, 63)] {
        assert_eq!(pixel(&before, x, y), [0, 0, 255, 255]);
    }
    raw["rotation"] = json!(0);
    assert_eq!(before.pixels, render(&restore(raw)).pixels);
    let bytes = crate::session_vector_write::encode(&editor, None, |_| unreachable!()).unwrap();
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["elements"][0]["style"]["width"], 8192.0);
    assert_eq!(value["elements"][0]["rotation"], 36000.0);
    assert_eq!(value["elements"][0]["zIndex"], 1.25);
}
#[test]
fn actual_directwrite_accepts_restored_text_domain_and_bridge_keeps_exact_originals() {
    use crate::projection_edit::{
        self,
        view::{Size, View},
    };
    let _com = Apartment::new().unwrap();
    for (text, family, size) in [
        ("", "", 8192),
        (" \r\n", "Segoe UI", 8192),
        ("A\0B\r\nC", "Segoe UI", 18),
        ("A", "中", 8192),
    ] {
        let family = if family == "中" {
            "中".repeat(128)
        } else {
            family.into()
        };
        let raw = json!({"id":"text","type":"text","zIndex":-1.125,"x":24,"y":32,"text":text,"fontFamily":family,"fontSize":size,"style":{"width":0,"color":"#ff0000"}});
        let source = View::from_basis(
            &projection_edit::Basis::new(&"a".repeat(64), 64, 64).unwrap(),
            Size::new(64.0, 64.0).unwrap(),
            projection_edit::Objects::from([("text".into(), raw)]),
        )
        .unwrap();
        let mut editor = Editor::default();
        let bridge = projection_edit::editor::prepare(&source, &editor, measure)
            .unwrap()
            .apply(&mut editor)
            .unwrap()
            .unwrap();
        assert_eq!(bridge.current(&editor, measure).unwrap(), source);
        let _ = render(&editor);
        let bytes = crate::session_vector_write::encode(&editor, None, |text| {
            Ok(measure(text).unwrap().baseline)
        })
        .unwrap();
        let saved: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(saved["elements"][0]["text"], text);
        assert_eq!(saved["elements"][0]["fontFamily"], family);
        assert_eq!(saved["elements"][0]["fontSize"], size as f64);
    }
}
#[test]
fn fractional_serial_radius_repeated_real_png_save_restore_has_identical_pixels() {
    let _com = Apartment::new().unwrap();
    let mut editor = restore(
        json!({"id":"serial","type":"serial","zIndex":-0.125,"x":16,"y":41,"text":"7","fontSize":18,"style":{"color":"#ff0000","fill":"#0000ff","width":0,"cornerRadius":14.25}}),
    );
    let expected = render(&editor);
    assert_eq!(pixel(&expected, 22, 25), [255, 0, 0, 255]);
    for _ in 0..4 {
        let bytes = crate::session_vector_write::encode(&editor, None, |_| unreachable!()).unwrap();
        let raw =
            serde_json::value::RawValue::from_string(String::from_utf8(bytes).unwrap()).unwrap();
        editor = crate::session_vectors::restore(Some(&raw)).unwrap().editor;
        assert_eq!(render(&editor).pixels, expected.pixels);
    }
}
