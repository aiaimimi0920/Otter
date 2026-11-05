//! 独立wire常量核全部类型/边界和原值往返，不用适配器输出构造预期。
use super::*;
use crate::{
    editor::Tool,
    projection_edit::{view::Size, Basis},
    viewport::Point,
};
fn metrics(text: &TextAnnotation) -> Result<TextMetrics, session_vectors::Error> {
    Ok(TextMetrics {
        size: Point {
            x: text.font_size() * 2.0 + 8.0,
            y: text.font_size() + 8.0,
        },
        baseline: text.font_size() * 0.75 + 4.0,
    })
}

#[test]
fn mapped_prepare_keeps_canonical_offset_text_anchor_and_original_basis_values() {
    let source = view(
        vec![
            shape("shape", "rect"),
            json!({"id":"text","type":"text","zIndex":1,"x":32,"y":40,"text":"A","fontSize":20,"style":{"color":"#f00","width":1}}),
        ],
        (120.0, 100.0),
    );
    let mapping =
        crate::session_vector_write::Mapping::from_components(0.5, Point { x: 10.0, y: 20.0 })
            .unwrap();
    let mut editor = Editor::default();
    let bridge = prepare_mapped(&source, &editor, mapping, metrics)
        .unwrap()
        .apply(&mut editor)
        .unwrap()
        .unwrap();
    assert_eq!(editor.elements()[0].points[0], Point { x: 0.0, y: 0.0 });
    assert_eq!(editor.elements()[0].points[1], Point { x: 64.0, y: 48.0 });
    assert_eq!(editor.elements()[1].points[0], Point { x: 36.0, y: 2.0 });
    assert_eq!(bridge.current(&editor, metrics).unwrap(), source);
    editor.translate(0, Point { x: 2.0, y: 4.0 }).unwrap();
    let current = bridge.current(&editor, metrics).unwrap();
    assert_eq!(current.basis_objects()["shape"]["x"], json!(11.0));
    assert_eq!(current.basis_objects()["shape"]["y"], json!(22.0));
    assert!(editor.undo());
    assert_eq!(bridge.current(&editor, metrics).unwrap(), source);
}
fn shape(id: &str, kind: &str) -> Value {
    json!({"id":id,"type":kind,"zIndex":0.125,"x":10,"y":20,"w":32,"h":24,
        "style":{"width":8192,"color":"rgba(255,0,0,0.25)","fill":"#0000ff80","secondaryFill":"transparent","cornerRadius":8192}})
}
fn view(objects: Vec<Value>, size: (f64, f64)) -> View {
    View::from_basis(
        &Basis::new(&"a".repeat(64), 120, 100).unwrap(),
        Size::new(size.0, size.1).unwrap(),
        objects
            .into_iter()
            .map(|v| (v["id"].as_str().unwrap().to_owned(), v))
            .collect(),
    )
    .unwrap()
}
fn install(source: &View) -> (Editor, Bridge) {
    let mut editor = Editor::default();
    let bridge = prepare(source, &editor, metrics)
        .unwrap()
        .apply(&mut editor)
        .unwrap()
        .unwrap();
    (editor, bridge)
}
#[test]
fn unobserved_checkpoint_keeps_existing_editor_objects_as_local_additions() {
    let local = view(vec![shape("local", "rect")], (120.0, 100.0));
    let (editor, _) = install(&local);
    let stamp = editor.document_stamp();
    let empty = view(vec![], (120.0, 100.0));
    let mapping =
        crate::session_vector_write::Mapping::from_components(1.0, Point::default()).unwrap();
    let bridge = Bridge::from_checkpoint(empty.clone(), &editor, mapping, metrics).unwrap();
    let current = bridge.current(&editor, metrics).unwrap();
    assert_eq!(current.basis_objects().len(), 1);
    assert_eq!(current.basis_objects()["local"]["x"].as_f64(), Some(10.0));
    assert_eq!(empty.changes_to(&current).unwrap().len(), 1);
    assert!(stamp.matches(&editor));
    assert_eq!(editor.elements()[0].annotation_id(), "local");
}
#[test]
fn all_wire_types_and_wide_styles_roundtrip_without_default_fields_or_z_rewrite() {
    let mut objects = Vec::new();
    for kind in ["rect", "round-rect", "ellipse", "triangle", "polygon"] {
        let mut object = shape(kind, kind);
        object["x"] = json!(10.123456789123);
        object["rotation"] = json!(-35999.123456789);
        object["sides"] = json!(64);
        if kind == "rect" {
            object["w"] = json!(0);
            object["h"] = json!(0);
            object["style"]["width"] = json!(0);
        }
        objects.push(object);
    }
    for kind in ["line", "polyline", "arrow", "brush", "highlighter"] {
        objects.push(json!({"id":kind,"type":kind,"zIndex":-0.25,"points":[{"x":1.123456789123,"y":2},{"x":16,"y":24},{"x":32,"y":24}],"style":{"width":0,"color":"transparent"}}));
    }
    for (i, text) in ["", " \t\r\n", "A\0B\r\nC", "你好🦦"].iter().enumerate() {
        objects.push(json!({"id":format!("text-{i}"),"type":"text","zIndex":0.125,"x":32,"y":48,"text":text,"fontSize":8192,"fontFamily":"中".repeat(128),"rotation":36000,"style":{"width":0,"color":"#123456"}}));
    }
    objects.push(json!({"id":"serial","type":"serial","zIndex":0.125,"x":64,"y":80,"text":"7","fontFamily":"","style":{"width":0,"cornerRadius":14.25,"color":"#ff0000"}}));
    let source = view(objects, (120.0, 100.0));
    let (editor, bridge) = install(&source);
    assert_eq!(editor.elements().len(), 15);
    assert_eq!(bridge.current(&editor, metrics).unwrap(), source);
    assert!(source
        .changes_to(&bridge.current(&editor, metrics).unwrap())
        .unwrap()
        .is_empty());
    for stroke in editor.elements() {
        let _ = stroke.style();
    }
}
#[test]
fn nonuniform_view_move_keeps_unedited_wire_numbers_colors_ids_and_undo() {
    let mut object = shape("stable", "polygon");
    object["style"]["width"] = json!(256.123456789);
    object["sides"] = json!(64);
    object["rotation"] = json!(36000);
    let source = view(vec![object], (240.0, 100.0));
    let (mut editor, bridge) = install(&source);
    assert_eq!(editor.elements()[0].points[0], Point { x: 20.0, y: 20.0 });
    assert!(editor.translate(0, Point { x: 20.0, y: 4.0 }).unwrap());
    let current = bridge.current(&editor, metrics).unwrap();
    let mut expected = source.basis_objects()["stable"].clone();
    expected["x"] = json!(20.0);
    expected["y"] = json!(24.0);
    assert_eq!(current.basis_objects()["stable"], expected);
    assert_eq!(source.changes_to(&current).unwrap().len(), 1);
    assert!(editor.undo());
    assert_eq!(bridge.current(&editor, metrics).unwrap(), source);
    assert!(editor.redo());
    assert_eq!(bridge.current(&editor, metrics).unwrap(), current);
}
#[test]
fn text_and_serial_anchors_and_fractional_resize_use_existing_editor_history() {
    let source = view(
        vec![
            json!({"id":"a","type":"text","zIndex":1.25,"x":32,"y":40,"text":"A\r\nB","fontSize":20,"style":{"color":"#f00","width":1}}),
            json!({"id":"b","type":"serial","zIndex":1.25,"x":50,"y":60,"text":"2","fontSize":20,"style":{"color":"#f00","width":1,"cornerRadius":14.25}}),
        ],
        (120.0, 100.0),
    );
    let (mut editor, bridge) = install(&source);
    assert_eq!(editor.elements()[0].points[0], Point { x: 28.0, y: 21.0 });
    assert_eq!(
        editor.elements()[1].points,
        [Point { x: 50.0, y: 35.75 }, Point { x: 78.5, y: 64.25 }]
    );
    assert!(editor.resize(1, 1.5).unwrap());
    let changed = bridge.current(&editor, metrics).unwrap();
    assert_eq!(
        changed.basis_objects()["b"]["style"]["cornerRadius"],
        21.375
    );
    assert_eq!(changed.basis_objects()["b"]["fontSize"], 30.0);
    assert_eq!(changed.basis_objects()["a"], source.basis_objects()["a"]);
    assert!(editor.undo());
    assert_eq!(bridge.current(&editor, metrics).unwrap(), source);
}
#[test]
fn deletion_preserves_other_fractional_layers_and_duplicate_has_new_identity() {
    let source = view(
        vec![shape("a", "rect"), shape("b", "rect"), shape("c", "rect")],
        (120.0, 100.0),
    );
    let (mut editor, bridge) = install(&source);
    assert!(editor.remove(0));
    let changed = bridge.current(&editor, metrics).unwrap();
    assert_eq!(changed.basis_objects()["b"], source.basis_objects()["b"]);
    assert_eq!(changed.basis_objects()["c"], source.basis_objects()["c"]);
    assert_eq!(source.changes_to(&changed).unwrap()[0].value, Value::Null);
    assert!(editor.undo());
    editor.duplicate(2).unwrap();
    let current = bridge.current(&editor, metrics).unwrap();
    assert_eq!(current.basis_objects().len(), 4);
    for id in ["a", "b", "c"] {
        assert_eq!(current.basis_objects()[id], source.basis_objects()[id]);
    }
    assert_ne!(editor.elements()[3].annotation_id(), "c");
}
#[test]
fn stale_prepare_wrong_document_draft_and_unsupported_tools_never_partially_publish() {
    let source = view(vec![shape("a", "rect")], (120.0, 100.0));
    let mut target = Editor::default();
    let prepared = prepare(&source, &target, metrics).unwrap();
    target.begin(Tool::Brush, Point::default()).unwrap();
    target.commit();
    assert!(prepared.apply(&mut target).unwrap().is_none());
    assert_eq!(target.elements()[0].tool, Tool::Brush);
    let (mut target, bridge) = install(&source);
    assert_eq!(
        bridge.current(&Editor::default(), metrics),
        Err(Error::Identity)
    );
    target.begin(Tool::Brush, Point::default()).unwrap();
    assert_eq!(bridge.current(&target, metrics), Err(Error::Identity));
    target.cancel();
    target.begin(Tool::Mosaic, Point::default()).unwrap();
    target.commit();
    assert!(bridge.current(&target, metrics).is_err());
    assert_eq!(bridge.source(), &source);
}
#[test]
fn failed_metrics_and_native_resource_bounds_leave_base_crop_and_history_unchanged() {
    let source = view(
        vec![
            json!({"id":"a","type":"text","zIndex":0,"x":0,"y":0,"text":"A","style":{"color":"#000","width":0}}),
        ],
        (120.0, 100.0),
    );
    let target = Editor::default();
    let stamp = target.document_stamp();
    assert!(prepare(&source, &target, |_| Err(
        session_vectors::Error::Unsupported
    ))
    .is_err());
    assert!(stamp.matches(&target));
    let huge = source.resized(Size::new(1_000_000.0, 1_000_000.0).unwrap());
    assert!(prepare(&huge, &target, |_| Ok(TextMetrics {
        size: Point {
            x: 2_000_000.0,
            y: 8.0
        },
        baseline: 4.0
    }))
    .is_err());
    assert!(stamp.matches(&target));
}

#[test]
fn maximum_layer_is_preserved_until_new_objects_require_bounded_renumbering() {
    for top in [1_000_000.0, 999_999.5] {
        let mut object = shape("old", "rect");
        object["zIndex"] = json!(top);
        let source = view(vec![object], (120.0, 100.0));
        let (mut editor, bridge) = install(&source);
        assert_eq!(bridge.current(&editor, metrics).unwrap(), source);
        // 同时保护普通canonical写入：不能因next已越界而改写未变对象。
        let saved: Value = serde_json::from_slice(
            &crate::session_vector_write::encode(&editor, None, |_| unreachable!()).unwrap(),
        )
        .unwrap();
        assert_eq!(saved["elements"][0]["zIndex"], top);
        editor.duplicate(0).unwrap();
        let changed = bridge.current(&editor, metrics).unwrap();
        assert_eq!(changed.basis_objects()["old"]["zIndex"], 0);
        let added = editor.elements()[1].annotation_id();
        assert_eq!(changed.basis_objects()[added]["zIndex"], 1);
        assert!(editor.undo());
        assert_eq!(bridge.current(&editor, metrics).unwrap(), source);
        assert!(editor.redo());
        assert_eq!(bridge.current(&editor, metrics).unwrap(), changed);
        assert!(editor.remove(1));
        assert_eq!(bridge.current(&editor, metrics).unwrap(), source);
    }
}

fn replace_ids(editor: &mut Editor, ids: &[&str]) {
    let mut replacement = Editor::default();
    for _ in ids {
        replacement
            .append_restored(
                Tool::Rectangle,
                vec![Point { x: 10.0, y: 20.0 }, Point { x: 42.0, y: 44.0 }],
                0.0,
                vec![],
            )
            .unwrap();
    }
    let replacement = replacement
        .with_restored_annotation_ids(&ids.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>())
        .unwrap();
    assert!(editor
        .prepare_annotation_replacement(&replacement)
        .unwrap()
        .apply(editor)
        .unwrap());
}

#[test]
fn new_layers_follow_actual_editor_order_not_lexicographic_identity() {
    let source = view(vec![shape("old", "rect")], (120.0, 100.0));
    let (mut editor, bridge) = install(&source);
    replace_ids(&mut editor, &["old", "z-added", "a-added"]);
    let changed = bridge.current(&editor, metrics).unwrap();
    for (id, expected) in [("old", 0.125), ("z-added", 1.125), ("a-added", 2.125)] {
        assert_eq!(changed.basis_objects()[id]["zIndex"], expected);
    }
    assert!(editor.undo());
    assert_eq!(bridge.current(&editor, metrics).unwrap(), source);
    assert!(editor.redo());
    assert_eq!(bridge.current(&editor, metrics).unwrap(), changed);
}

#[test]
fn real_reordering_renumbers_all_current_objects_in_editor_order() {
    let source = view(vec![shape("a", "rect"), shape("b", "rect")], (120.0, 100.0));
    let (mut editor, bridge) = install(&source);
    replace_ids(&mut editor, &["b", "new", "a"]);
    let changed = bridge.current(&editor, metrics).unwrap();
    for (id, expected) in [("b", 0), ("new", 1), ("a", 2)] {
        assert_eq!(changed.basis_objects()[id]["zIndex"], expected);
    }
    assert!(editor.undo());
    assert_eq!(bridge.current(&editor, metrics).unwrap(), source);
}
