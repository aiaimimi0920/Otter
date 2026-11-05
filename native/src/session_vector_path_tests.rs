//! 多点/单点路径恢复、变换及canonical往返；固定坐标oracle独立于新箭头几何。
use super::*;
fn raw(kind: &str, points: Value) -> Box<RawValue> {
    serde_json::value::to_raw_value(&serde_json::json!({"serialCounter":7,"elements":[{
        "id":"path-stable","type":kind,"zIndex":3,"points":points,
        "style":{"width":4,"color":"#ff0000"}}]}))
    .unwrap()
}
fn bent() -> Value {
    serde_json::json!([{"x":8.0,"y":8.0},{"x":8.0,"y":48.0},{"x":56.0,"y":48.0},{"x":56.0,"y":48.0}])
}
#[test]
fn restored_line_arrow_and_polyline_keep_every_sample_and_stable_identity_through_save() {
    for kind in ["line", "arrow", "polyline"] {
        let input = raw(kind, bent());
        let mut editor = restore(Some(&input)).unwrap().editor;
        assert_eq!(editor.elements()[0].points.len(), 4);
        assert_eq!(editor.elements()[0].points[1], Point { x: 8.0, y: 48.0 });
        assert!(!editor.undo());
        for _ in 0..3 {
            let encoded =
                crate::session_vector_write::encode(&editor, Some(&input), |_| unreachable!())
                    .unwrap();
            let raw = RawValue::from_string(String::from_utf8(encoded).unwrap()).unwrap();
            let encoded: Value = serde_json::from_str(raw.get()).unwrap();
            assert_eq!(encoded["elements"][0]["points"], bent());
            assert_eq!(encoded["elements"][0]["id"], "path-stable");
            editor = restore(Some(&raw)).unwrap().editor;
        }
    }
}
#[test]
fn imported_bent_paths_edit_resize_undo_and_serialize_without_collapsing_to_chord() {
    for kind in ["line", "arrow"] {
        let input = raw(kind, bent());
        let mut editor = restore(Some(&input)).unwrap().editor;
        editor.translate(0, Point { x: 2.0, y: 3.0 }).unwrap();
        assert_eq!(editor.elements()[0].points[1], Point { x: 10.0, y: 51.0 });
        editor
            .resize_many_axes_around(
                &[0],
                Point::default(),
                Point { x: 2.0, y: 1.0 },
                |_| unreachable!(),
            )
            .unwrap();
        assert_eq!(editor.elements()[0].points[1], Point { x: 20.0, y: 51.0 });
        assert_eq!(editor.elements()[0].width, 6.0);
        assert!(editor.undo());
        assert!(editor.undo());
        assert_eq!(editor.elements()[0].points[1], Point { x: 8.0, y: 48.0 });
        assert!(editor.redo());
        assert!(editor.redo());
        let encoded =
            crate::session_vector_write::encode(&editor, Some(&input), |_| unreachable!()).unwrap();
        let encoded: Value = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(
            encoded["elements"][0]["points"][1],
            serde_json::json!({"x":20.0,"y":51.0})
        );
        assert_eq!(
            encoded["elements"][0]["points"].as_array().unwrap().len(),
            4
        );
        assert_eq!(encoded["elements"][0]["id"], "path-stable");
    }
}
#[test]
fn single_point_paths_roundtrip_but_empty_invalid_and_oversized_paths_still_fail() {
    for kind in ["line", "arrow", "polyline"] {
        let one = raw(kind, serde_json::json!([{"x":3,"y":4}]));
        let editor = restore(Some(&one)).unwrap().editor;
        let bytes =
            crate::session_vector_write::encode(&editor, Some(&one), |_| unreachable!()).unwrap();
        let raw = RawValue::from_string(String::from_utf8(bytes).unwrap()).unwrap();
        assert_eq!(
            restore(Some(&raw)).unwrap().editor.elements()[0].points,
            [Point { x: 3.0, y: 4.0 }]
        );
        assert!(restore(Some(&self::raw(kind, serde_json::json!([])))).is_err());
        assert!(restore(Some(&self::raw(
            kind,
            serde_json::json!([{"x":null,"y":4}])
        )))
        .is_err());
    }
    let input = raw(
        "line",
        vec![serde_json::json!({"x":0,"y":0}); 65_537].into(),
    );
    assert!(matches!(restore(Some(&input)), Err(Error::Budget)));
}
