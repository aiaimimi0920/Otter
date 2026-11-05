use super::*;

fn restore(bytes: Vec<u8>) -> crate::session_vectors::Restored {
    let raw = RawValue::from_string(String::from_utf8(bytes).unwrap()).unwrap();
    crate::session_vectors::restore(Some(&raw)).unwrap()
}
fn drawing(tool: Tool) -> Editor {
    let mut editor = Editor::default();
    editor.begin(tool, Point { x: 10.0, y: 20.0 }).unwrap();
    editor.extend(Point { x: 50.0, y: 60.0 }).unwrap();
    assert!(editor.commit());
    editor
}
#[test]
fn shapes_and_paths_round_trip_style_geometry_and_stable_id() {
    for tool in [
        Tool::Rectangle,
        Tool::RoundedRectangle,
        Tool::Ellipse,
        Tool::Triangle,
        Tool::Polygon,
        Tool::Line,
        Tool::Arrow,
        Tool::Brush,
        Tool::Highlighter,
    ] {
        let editor = drawing(tool);
        let bytes = encode(&editor, None, |_| Err(Error::Unsupported)).unwrap();
        let restored = restore(bytes);
        let a = &editor.elements()[0];
        let b = &restored.editor.elements()[0];
        assert_eq!(a.annotation_id(), b.annotation_id());
        assert_eq!(a.tool, b.tool);
        assert_eq!(a.points, b.points);
        assert_eq!(a.color, b.color);
        assert_eq!(a.opacity, b.opacity);
        assert_eq!(a.width, b.width);
    }
}
#[test]
fn unknown_fields_follow_identity_not_shifted_array_index() {
    let mut editor = drawing(Tool::Rectangle);
    editor.duplicate(0).unwrap();
    let bytes = encode(&editor, None, |_| Err(Error::Unsupported)).unwrap();
    let mut value: Value = serde_json::from_slice(&bytes).unwrap();
    value["extension"] = json!({"keep":true});
    value["elements"][0]["extension"] = json!("removed");
    value["elements"][1]["extension"] = json!("remaining");
    value["elements"][1]["style"]["future"] = json!([1, 2, 3]);
    let original = RawValue::from_string(value.to_string()).unwrap();
    editor.remove(0);
    let bytes = encode(&editor, Some(&original), |_| Err(Error::Unsupported)).unwrap();
    let saved: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(saved["extension"], value["extension"]);
    assert_eq!(saved["elements"].as_array().unwrap().len(), 1);
    assert_eq!(saved["elements"][0]["extension"], "remaining");
    assert_eq!(saved["elements"][0]["style"]["future"], json!([1, 2, 3]));
    assert_eq!(
        saved["elements"][0]["id"],
        editor.elements()[0].annotation_id()
    );
    assert_eq!(saved["elements"][0]["zIndex"].as_f64(), Some(1.0));
    assert_eq!(original.get(), value.to_string());
}
#[test]
fn line_rotation_is_baked_into_points_not_an_ignored_main_field() {
    let mut editor = drawing(Tool::Line);
    editor.rotate(0, 90.0).unwrap();
    let source = &editor.elements()[0];
    let expected: Vec<_> = source
        .points
        .iter()
        .map(|point| {
            crate::rotation::rotate(
                *point,
                crate::rotation::center(&source.points),
                source.rotation,
            )
        })
        .collect();
    let restored = restore(encode(&editor, None, |_| Err(Error::Unsupported)).unwrap());
    assert_eq!(restored.editor.elements()[0].points, expected);
    assert_eq!(restored.editor.elements()[0].rotation, 0.0);
}
#[test]
fn text_and_serial_use_inverse_anchor_mapping_and_preserve_counter() {
    let mut editor = Editor::default();
    let text = crate::text_annotation::TextAnnotation::new("中文", "Arial", 18.0, false).unwrap();
    editor
        .insert_text(Point { x: 10.0, y: 20.0 }, Point { x: 30.0, y: 24.0 }, text)
        .unwrap();
    editor
        .insert_serial(Point { x: 90.0, y: 100.0 }, 14.0, "Arial")
        .unwrap();
    let bytes = encode(&editor, None, |_| Ok(17.0)).unwrap();
    let raw = RawValue::from_string(String::from_utf8(bytes).unwrap()).unwrap();
    let restored = crate::session_vectors::restore_with_text(Some(&raw), |_| {
        Ok(crate::session_vectors::TextMetrics {
            size: Point { x: 30.0, y: 24.0 },
            baseline: 17.0,
        })
    })
    .unwrap();
    assert_eq!(restored.serial_counter, editor.serial_count());
    for (a, b) in editor.elements().iter().zip(restored.editor.elements()) {
        assert_eq!(a.points, b.points);
        assert_eq!(a.annotation_id(), b.annotation_id());
        assert_eq!(
            a.text.as_ref().unwrap().content(),
            b.text.as_ref().unwrap().content()
        );
    }
}
#[test]
fn long_single_line_roundtrips_without_splitting_or_truncating_content() {
    let content = "x".repeat(crate::text_annotation::MAX_TEXT_UNITS);
    let mut editor = Editor::default();
    let size = Point {
        x: 600000.0,
        y: 24.0,
    };
    editor
        .insert_text(
            Point::default(),
            size,
            crate::text_annotation::TextAnnotation::new(&content, "Arial", 16.0, false).unwrap(),
        )
        .unwrap();
    let bytes = encode(&editor, None, |_| Ok(17.0)).unwrap();
    let raw = RawValue::from_string(String::from_utf8(bytes).unwrap()).unwrap();
    let restored = crate::session_vectors::restore_with_text(Some(&raw), |_| {
        Ok(crate::session_vectors::TextMetrics {
            size,
            baseline: 17.0,
        })
    })
    .unwrap();
    assert_eq!(restored.editor.elements().len(), 1);
    assert_eq!(
        restored.editor.elements()[0]
            .text
            .as_ref()
            .unwrap()
            .content(),
        content
    );
}
#[test]
fn unsupported_bold_text_and_duplicate_original_ids_fail_without_mutation() {
    let mut editor = Editor::default();
    editor
        .insert_text(
            Point::default(),
            Point { x: 30.0, y: 20.0 },
            crate::text_annotation::TextAnnotation::new("bold", "Arial", 18.0, true).unwrap(),
        )
        .unwrap();
    assert_eq!(encode(&editor, None, |_| Ok(12.0)), Err(Error::Unsupported));
    let old =
        RawValue::from_string(r#"{"elements":[{"id":"same"},{"id":"same"}]}"#.into()).unwrap();
    assert_eq!(
        encode(&Editor::default(), Some(&old), |_| Ok(0.0)),
        Err(Error::Identity)
    );
    assert_eq!(editor.elements().len(), 1);
}
#[test]
fn bounded_sink_never_appends_bytes_past_limit() {
    use std::io::Write;
    let mut output = Bounded(vec![0; LIMIT - 1]);
    assert!(output.write_all(&[1, 2]).is_err());
    assert_eq!(output.0.len(), LIMIT - 1);
    output.write_all(&[1]).unwrap();
    assert!(output.write_all(&[2]).is_err());
    assert_eq!(output.0.len(), LIMIT);
}

#[test]
fn maximum_layer_and_deletion_keep_original_values_until_append_overflows() {
    let raw = RawValue::from_string(json!({"serialCounter":0,"elements":[
        {"id":"low","type":"rect","zIndex":12.25,"x":0,"y":0,"w":10,"h":10,"style":{"width":1,"color":"#000000"}},
        {"id":"top","type":"rect","zIndex":1000000,"x":0,"y":0,"w":10,"h":10,"style":{"width":1,"color":"#000000"}}
    ]}).to_string()).unwrap();
    let mut editor = crate::session_vectors::restore(Some(&raw)).unwrap().editor;
    for old in [None, Some(&*raw)] {
        let value: Value =
            serde_json::from_slice(&encode(&editor, old, |_| unreachable!()).unwrap()).unwrap();
        assert_eq!(value["elements"][0]["zIndex"], 12.25);
        assert_eq!(value["elements"][1]["zIndex"].as_f64(), Some(1_000_000.0));
    }
    assert!(editor.remove(0));
    let value: Value =
        serde_json::from_slice(&encode(&editor, Some(&raw), |_| unreachable!()).unwrap()).unwrap();
    assert_eq!(value["elements"][0]["zIndex"].as_f64(), Some(1_000_000.0));
    editor.duplicate(0).unwrap();
    let value: Value =
        serde_json::from_slice(&encode(&editor, Some(&raw), |_| unreachable!()).unwrap()).unwrap();
    assert_eq!(value["elements"][0]["zIndex"], 0);
    assert_eq!(value["elements"][1]["zIndex"], 1);
    assert!(editor.undo());
    let value: Value =
        serde_json::from_slice(&encode(&editor, Some(&raw), |_| unreachable!()).unwrap()).unwrap();
    assert_eq!(value["elements"][0]["zIndex"].as_f64(), Some(1_000_000.0));
}

#[test]
fn old_array_budget_precedes_identity_merge_and_layer_sorting() {
    let old = RawValue::from_string(
        json!({"elements":vec![json!({"id":"duplicate","zIndex":0});4097]}).to_string(),
    )
    .unwrap();
    let editor = drawing(Tool::Rectangle);
    let stamp = editor.document_stamp();
    assert_eq!(
        encode(&editor, Some(&old), |_| unreachable!()),
        Err(Error::Budget)
    );
    assert!(stamp.matches(&editor));
}

#[test]
fn effects_round_trip_path_bounds_strength_and_resource_reference() {
    let mut editor = Editor::default();
    editor
        .append_restored(Tool::Mosaic, vec![Point { x: 2.0, y: 3.0 }], 25.0, vec![])
        .unwrap();
    let image =
        crate::blur_image::BlurImage::with_strength(1, 1, vec![255; 4].into(), 4.4).unwrap();
    editor
        .append_restored_blur(vec![Point { x: 8.0, y: 9.0 }], 0.0, image.clone())
        .unwrap();
    let bytes = encode(&editor, None, |_| Err(Error::Unsupported)).unwrap();
    let raw = RawValue::from_string(String::from_utf8(bytes).unwrap()).unwrap();
    let value: Value = serde_json::from_str(raw.get()).unwrap();
    assert!(value["elements"][0]["w"].as_f64().unwrap() > 0.0);
    let restored = crate::session_vectors::restore_with_resources(
        Some(&raw),
        |_| unreachable!(),
        |strength| {
            assert_eq!(strength, 4.4);
            Ok(image.clone())
        },
    )
    .unwrap();
    for (a, b) in editor.elements().iter().zip(restored.editor.elements()) {
        assert_eq!(a.points, b.points);
        assert_eq!(a.rotation, b.rotation);
        assert_eq!(a.annotation_id(), b.annotation_id());
        assert_eq!(a.tool, b.tool);
    }
    assert_eq!(
        restored.editor.elements()[1]
            .blur
            .as_ref()
            .unwrap()
            .strength(),
        4.4
    );
}

#[test]
fn tapered_samples_and_bezier_arrow_survive_round_trip() {
    let mut editor = Editor::default();
    editor
        .append_restored(
            Tool::Paintbrush,
            vec![Point { x: 1.0, y: 2.0 }, Point { x: 3.0, y: 4.0 }],
            0.0,
            vec![0.25, 0.75],
        )
        .unwrap();
    editor.set_style(crate::stroke_style::StrokeStyle::default().with_arrow(true));
    editor
        .append_restored(
            Tool::Bezier,
            vec![
                Point { x: 1.0, y: 2.0 },
                Point { x: 3.0, y: 4.0 },
                Point { x: 5.0, y: 6.0 },
                Point { x: 7.0, y: 8.0 },
            ],
            0.0,
            vec![],
        )
        .unwrap();
    let restored = restore(encode(&editor, None, |_| Err(Error::Unsupported)).unwrap());
    assert_eq!(
        restored.editor.elements()[0].brush_weights,
        vec![0.25, 0.75]
    );
    assert_eq!(restored.editor.elements()[0].tool, Tool::Paintbrush);
    assert!(restored.editor.elements()[1].arrow);
    assert_eq!(
        restored.editor.elements()[1].points,
        editor.elements()[1].points
    );
}
