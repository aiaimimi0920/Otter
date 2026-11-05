use super::*;
fn raw(elements: Value) -> Box<RawValue> {
    RawValue::from_string(serde_json::json!({"elements":elements,"serialCounter":7}).to_string())
        .unwrap()
}
fn shape(id: &str, kind: &str, z: u32) -> Value {
    serde_json::json!({"id":id,"type":kind,"zIndex":z,"x":-10,"y":-20,"w":30,"h":40,
        "style":{"width":3,"color":"#ef4444","fill":"transparent","opacity":0.5,"dashPattern":"dash-1"}})
}
#[test]
fn rgb_and_opaque_alpha_colors_restore_without_losing_style_opacity() {
    let mut item = shape("rgb", "rect", 1);
    item["style"]["color"] = Value::String("rgb(17, 34, 51)".into());
    item["style"]["fill"] = Value::String("#aabbccff".into());
    let input = raw(serde_json::json!([item]));
    let before = input.get().to_owned();
    let mut restored = restore(Some(&input)).unwrap();
    assert_eq!(restored.editor.elements()[0].color, Color(17, 34, 51));
    assert_eq!(
        restored.editor.elements()[0].fill,
        Some(Color(170, 187, 204))
    );
    assert_eq!(restored.editor.elements()[0].opacity, 0.5);
    assert!(!restored.editor.undo());
    assert_eq!(input.get(), before);
    let mut unsupported = shape("invalid", "ellipse", 2);
    unsupported["style"]["color"] = Value::String("url(color)".into());
    let input = raw(serde_json::json!([shape("first", "rect", 1), unsupported]));
    assert!(matches!(restore(Some(&input)), Err(Error::Style)));
}
#[test]
fn shapes_keep_order_identity_style_and_are_not_an_undoable_import() {
    let input = raw(serde_json::json!([
        shape("last", "ellipse", 9),
        shape("first", "rect", 1)
    ]));
    let mut restored = restore(Some(&input)).unwrap();
    assert_eq!(restored.ids, ["first", "last"]);
    assert_eq!(restored.serial_counter, 7);
    assert_eq!(
        restored.editor.elements()[0].points[0],
        Point { x: -10.0, y: -20.0 }
    );
    assert_eq!(restored.editor.elements()[0].dash, DashPattern::Dash);
    assert_eq!(restored.editor.elements()[0].opacity, 0.5);
    assert!(!restored.editor.undo());
    assert!(restored
        .editor
        .translate(0, Point { x: 5.0, y: 6.0 })
        .unwrap());
    assert!(restored.editor.undo());
    assert_eq!(restored.editor.elements()[0].points[0].x, -10.0);
}
#[test]
fn stored_close_points_are_not_decimated_by_interactive_sampling() {
    let input = raw(serde_json::json!([{"id":"path","type":"brush","zIndex":0,
        "points":[{"x":1,"y":1},{"x":1.01,"y":1.01},{"x":1.02,"y":1.02}],
        "style":{"width":1,"color":"#fff"}}]));
    let restored = restore(Some(&input)).unwrap();
    assert_eq!(restored.editor.elements()[0].points.len(), 3);
    assert_eq!(restored.editor.elements()[0].color, Color(255, 255, 255));
}
#[test]
fn unsupported_or_duplicate_annotation_rejects_whole_result() {
    let input = raw(serde_json::json!([
        shape("first", "rect", 1),
        shape("other", "future-effect", 2)
    ]));
    assert!(matches!(restore(Some(&input)), Err(Error::Unsupported)));
    let input = raw(serde_json::json!([
        shape("same", "rect", 1),
        shape("same", "ellipse", 2)
    ]));
    assert!(matches!(restore(Some(&input)), Err(Error::Identity)));
}
#[test]
fn bezier_keeps_control_points_and_tapered_brush_keeps_pressure_weights() {
    let curve = serde_json::json!({"id":"curve","type":"bezier","zIndex":0,"showArrowHead":true,
        "points":[{"x":0,"y":0},{"x":10,"y":20},{"x":20,"y":20},{"x":30,"y":0}],
        "style":{"width":3,"color":"#123456"}});
    let brush = serde_json::json!({"id":"brush","type":"brush","zIndex":1,
        "points":[{"x":1,"y":2},{"x":1.01,"y":2.01},{"x":5,"y":6}],
        "style":{"width":16,"color":"#fff","brushTip":"tapered","brushWeights":[0.4,0.7,1.0]}});
    let input = raw(serde_json::json!([curve, brush]));
    let mut restored = restore(Some(&input)).unwrap();
    let elements = restored.editor.elements();
    assert_eq!(elements[0].tool, Tool::Bezier);
    assert_eq!(elements[0].points.len(), 4);
    assert_eq!(elements[0].points[1], Point { x: 10.0, y: 20.0 });
    assert!(elements[0].arrow);
    assert_eq!(elements[1].tool, Tool::Paintbrush);
    assert_eq!(elements[1].points.len(), 3);
    assert_eq!(elements[1].brush_weights, [0.4, 0.7, 1.0]);
    let samples = crate::paintbrush_samples::Samples::new(
        &elements[1].points,
        elements[1].width,
        Some(&elements[1].brush_weights),
    )
    .unwrap();
    assert!(samples.count() > 1);
    assert!(!restored.editor.undo());
}
#[test]
fn malformed_bezier_is_not_reinterpreted_as_a_polyline() {
    let input = raw(serde_json::json!([{"id":"bad","type":"bezier","zIndex":0,
        "points":[{"x":0,"y":0},{"x":1,"y":1},{"x":2,"y":2}],"style":{"width":3,"color":"#fff"}}]));
    assert!(matches!(restore(Some(&input)), Err(Error::Geometry)));
}
#[test]
fn text_baseline_and_serial_circle_restore_without_guessing_text_width() {
    let input = raw(serde_json::json!([
        {"id":"text","type":"text","zIndex":0,"x":100,"y":200,"text":"你好 Otter","fontFamily":"Segoe UI","fontSize":18,
        "style":{"color":"#123456","width":1}},
        {"id":"serial","type":"serial","zIndex":1,"x":10,"y":30,"text":"7",
        "style":{"color":"#fff","fill":"#000","width":2,"cornerRadius":999}}
    ]));
    let restored = restore_with_text(Some(&input), |text| {
        assert_eq!(text.content(), "你好 Otter");
        Ok(TextMetrics {
            size: Point { x: 108.0, y: 28.0 },
            baseline: 19.0,
        })
    })
    .unwrap();
    let text = &restored.editor.elements()[0];
    assert_eq!(text.tool, Tool::Text);
    assert_eq!(text.points[0], Point { x: 96.0, y: 181.0 });
    assert_eq!(text.points[1], Point { x: 204.0, y: 209.0 });
    let serial = &restored.editor.elements()[1];
    assert_eq!(serial.tool, Tool::Serial);
    assert_eq!(serial.text.as_ref().unwrap().font_size(), 1149.0);
    assert!(serial.text.as_ref().unwrap().bold());
    assert_eq!(serial.points[1].x - serial.points[0].x, 1998.0);
    assert!(matches!(
        restore_with_text(Some(&input), |_| Ok(TextMetrics {
            size: Point {
                x: f32::NAN,
                y: 10.0
            },
            baseline: 1.0
        })),
        Err(Error::Geometry)
    ));
}
#[test]
fn brush_mosaic_restores_its_mask_width_and_rounded_grid_strength() {
    let input = raw(
        serde_json::json!([{"id":"mosaic","type":"mosaic","zIndex":0,
        "x":0,"y":0,"w":50,"h":50,"points":[{"x":10,"y":10},{"x":30,"y":30}],
        "brushWidth":24,"strength":7.6,"style":{"width":1,"color":"#fff","opacity":0.75}}]),
    );
    let mut restored = restore(Some(&input)).unwrap();
    let stroke = &restored.editor.elements()[0];
    assert_eq!(stroke.tool, Tool::Mosaic);
    assert_eq!(stroke.width, 24.0);
    assert_eq!(stroke.mosaic_cell, 8.0);
    assert_eq!(stroke.opacity, 0.75);
    assert_eq!(
        stroke.points,
        [Point { x: 10.0, y: 10.0 }, Point { x: 30.0, y: 30.0 }]
    );
    assert!(stroke.blur.is_none());
    assert!(!restored.editor.undo());
    let rectangle = raw(serde_json::json!([{"id":"rect","type":"mosaic","zIndex":0,
        "x":0,"y":0,"w":50,"h":50,"style":{"width":1,"color":"#fff"}}]));
    assert!(restore(Some(&rectangle)).is_err());
}
#[test]
fn blur_restore_requires_matching_prepared_resource_and_shares_it_between_paths() {
    let effect = |id: &str| {
        serde_json::json!({"id":id,"type":"blur","zIndex":0,
        "points":[{"x":2,"y":2},{"x":6,"y":6}],"brushWidth":12,"strength":4.4,
        "style":{"width":1,"color":"#fff"}})
    };
    let input = raw(serde_json::json!([effect("one"), effect("two")]));
    let source: crate::pixels::Pixels = vec![255; 8 * 8 * 4].into();
    let pixels = crate::blur::region_precise(
        &source,
        (8, 8),
        crate::blur::Region {
            x: 0,
            y: 0,
            width: 8,
            height: 8,
        },
        4.4,
        || false,
    )
    .unwrap();
    let prepared = crate::blur_image::BlurImage::with_strength(8, 8, pixels, 4.4).unwrap();
    let restored = restore_with_resources(
        Some(&input),
        |_| panic!("no text"),
        |strength| {
            assert_eq!(strength, 4.4);
            Ok(prepared.clone())
        },
    )
    .unwrap();
    for stroke in restored.editor.elements() {
        assert_eq!(stroke.tool, Tool::Blur);
        assert_eq!(stroke.width, 12.0);
        assert_eq!(stroke.blur.as_ref().unwrap().strength(), 4.4);
        assert!(stroke
            .blur
            .as_ref()
            .unwrap()
            .pixels()
            .same_storage(prepared.pixels()));
    }
    assert!(matches!(restore(Some(&input)), Err(Error::Unsupported)));
    let wrong = crate::blur_image::BlurImage::with_strength(8, 8, source, 8.0).unwrap();
    assert!(matches!(
        restore_with_resources(Some(&input), |_| panic!("no text"), |_| Ok(wrong.clone())),
        Err(Error::Style)
    ));
}
