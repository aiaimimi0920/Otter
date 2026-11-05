use super::*;
fn space() -> crate::session_placement::Space {
    crate::session_placement::Space {
        origin: (-1920, -200),
        scale: 1.5,
    }
}
fn mapping() -> Mapping {
    Mapping::from_viewport(
        crate::viewport::Viewport {
            scale: 2.0,
            origin: Point { x: -3.0, y: 6.0 },
        },
        144,
        space(),
    )
    .unwrap()
}
#[test]
fn mapping_preserves_physical_position_across_window_and_session_dpi() {
    for dpi in [96, 120, 144, 192] {
        for scale in [0.75, 1.0, 1.5, 2.0] {
            let view = crate::viewport::Viewport {
                scale: 2.25,
                origin: Point { x: -13.5, y: 9.25 },
            };
            let space = crate::session_placement::Space { scale, ..space() };
            let map = Mapping::from_viewport(view, dpi, space).unwrap();
            let input = Point { x: 12.25, y: -3.5 };
            let output = map.point(input).unwrap();
            let factor = dpi as f64 / 96.0;
            assert!(
                (f64::from(output.x) * scale
                    - f64::from(input.x * view.scale + view.origin.x) * factor)
                    .abs()
                    < 0.001
            );
            assert!(
                (f64::from(output.y) * scale
                    - f64::from(input.y * view.scale + view.origin.y) * factor)
                    .abs()
                    < 0.001
            );
        }
    }
}

#[test]
fn annotation_reference_frame_removes_crop_origin_but_rejects_anisotropic_style() {
    let frame = crate::crop_geometry::Rect {
        x: 10.0,
        y: 5.0,
        width: 100.0,
        height: 50.0,
    };
    let target = crate::crop_geometry::Size {
        width: 200.0,
        height: 100.0,
    };
    let mapping = Mapping::from_annotation_frame(Some(frame), target).unwrap();
    assert_eq!(
        mapping.point(Point { x: 12.0, y: 8.0 }).unwrap(),
        Point { x: 4.0, y: 6.0 }
    );
    assert!(matches!(
        Mapping::from_annotation_frame(
            Some(frame),
            crate::crop_geometry::Size {
                height: 90.0,
                ..target
            }
        ),
        Err(Error::Unsupported)
    ));
    assert!(matches!(
        Mapping::from_annotation_frame(
            Some(frame),
            crate::crop_geometry::Size {
                width: f64::NAN,
                ..target
            }
        ),
        Err(Error::Geometry)
    ));
}
#[test]
fn mapped_shapes_preserve_identity_and_scale_geometry_and_style_once() {
    let mut editor = Editor::default();
    editor
        .begin(Tool::Rectangle, Point { x: 10.0, y: 20.0 })
        .unwrap();
    editor.extend(Point { x: 50.0, y: 60.0 }).unwrap();
    assert!(editor.commit());
    let before = editor.document_stamp();
    let bytes = encode_mapped(&editor, None, mapping(), |_| Err(Error::Unsupported)).unwrap();
    let raw = RawValue::from_string(String::from_utf8(bytes).unwrap()).unwrap();
    let restored = crate::session_vectors::restore(Some(&raw)).unwrap();
    let saved = &restored.editor.elements()[0];
    assert_eq!(
        saved.points,
        vec![Point { x: 17.0, y: 46.0 }, Point { x: 97.0, y: 126.0 }]
    );
    assert_eq!(saved.width, editor.elements()[0].width * 2.0);
    assert_eq!(saved.annotation_id(), editor.elements()[0].annotation_id());
    assert!(before.matches(&editor));
}
#[test]
fn text_remeasures_scaled_font_and_keeps_fixed_main_anchor_inset() {
    let mut editor = Editor::default();
    let text = crate::text_annotation::TextAnnotation::new("text", "Arial", 18.0, false).unwrap();
    editor
        .insert_text(Point { x: 10.0, y: 20.0 }, Point { x: 30.0, y: 24.0 }, text)
        .unwrap();
    let bytes =
        encode_mapped(&editor, None, mapping(), |text| Ok(text.font_size() * 0.75)).unwrap();
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["elements"][0]["x"], 21.0);
    assert_eq!(value["elements"][0]["fontSize"], 36.0);
    let raw = RawValue::from_string(String::from_utf8(bytes).unwrap()).unwrap();
    let restored = crate::session_vectors::restore_with_text(Some(&raw), |text| {
        Ok(crate::session_vectors::TextMetrics {
            size: Point { x: 60.0, y: 48.0 },
            baseline: text.font_size() * 0.75,
        })
    })
    .unwrap();
    assert_eq!(
        restored.editor.elements()[0].points,
        vec![Point { x: 17.0, y: 46.0 }, Point { x: 77.0, y: 94.0 }]
    );
}
#[test]
fn invalid_context_is_rejected_but_restored_width_is_not_limited_by_ui() {
    let view = crate::viewport::Viewport::default();
    assert!(Mapping::from_viewport(view, 0, space()).is_err());
    for scale in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(Mapping::from_viewport(
            view,
            96,
            crate::session_placement::Space { scale, ..space() }
        )
        .is_err());
    }
    let map = Mapping::from_viewport(
        crate::viewport::Viewport {
            scale: 1000.0,
            ..view
        },
        96,
        space(),
    )
    .unwrap();
    let mut editor = Editor::default();
    editor.set_style(editor.style().with_width(3.0).unwrap());
    editor.begin(Tool::Brush, Point::default()).unwrap();
    editor.commit();
    let bytes = encode_mapped(&editor, None, map, |_| Err(Error::Unsupported)).unwrap();
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    // 3px * 1000视口 * (96/96 DPI) / 1.5会话空间 = 2000；不调用Mapping构造预期。
    assert_eq!(value["elements"][0]["style"]["width"], 2000.0);
}

#[test]
fn mapped_serial_preserves_anchor_counter_and_rotation() {
    let mut editor = Editor::default();
    editor
        .insert_serial(Point { x: 90.0, y: 100.0 }, 14.0, "Arial")
        .unwrap();
    editor.rotate(0, 30.0).unwrap();
    let bytes = encode_mapped(&editor, None, mapping(), |_| unreachable!()).unwrap();
    let raw = RawValue::from_string(String::from_utf8(bytes).unwrap()).unwrap();
    let restored = crate::session_vectors::restore(Some(&raw)).unwrap();
    let source = &editor.elements()[0];
    let saved = &restored.editor.elements()[0];
    assert_eq!(
        saved.points,
        source
            .points
            .iter()
            .map(|p| mapping().point(*p).unwrap())
            .collect::<Vec<_>>()
    );
    assert_eq!(saved.rotation, source.rotation);
    assert_eq!(saved.annotation_id(), source.annotation_id());
    assert_eq!(restored.serial_counter, editor.serial_count());
}

#[test]
fn mapped_effects_scale_bounds_and_strength_but_not_unknown_extensions() {
    let mut editor = Editor::default();
    editor
        .append_restored(Tool::Mosaic, vec![Point { x: 2.0, y: 3.0 }], 25.0, vec![])
        .unwrap();
    let image =
        crate::blur_image::BlurImage::with_strength(1, 1, vec![255; 4].into(), 4.4).unwrap();
    editor
        .append_restored_blur(vec![Point { x: 8.0, y: 9.0 }], 0.0, image)
        .unwrap();
    let original = encode(&editor, None, |_| unreachable!()).unwrap();
    let mut original: Value = serde_json::from_slice(&original).unwrap();
    original["elements"][0]["extension"] = json!({"x":123,"width":456});
    let raw = RawValue::from_string(original.to_string()).unwrap();
    let bytes = encode_mapped(&editor, Some(&raw), mapping(), |_| unreachable!()).unwrap();
    let saved: Value = serde_json::from_slice(&bytes).unwrap();
    for index in 0..2 {
        for key in ["w", "h", "brushWidth", "strength"] {
            let actual = saved["elements"][index][key].as_f64().unwrap();
            let expected = original["elements"][index][key].as_f64().unwrap() * 2.0;
            assert!((actual - expected).abs() < 0.001, "{key}");
        }
        assert_eq!(
            saved["elements"][index]["rotation"],
            original["elements"][index]["rotation"]
        );
    }
    assert_eq!(
        saved["elements"][0]["extension"],
        original["elements"][0]["extension"]
    );
    assert_eq!(raw.get(), original.to_string());
}

#[test]
fn fractional_serial_radius_roundtrips_without_changing_editor() {
    let mut editor = Editor::default();
    editor
        .insert_serial(Point { x: 90.0, y: 100.0 }, 14.0, "Arial")
        .unwrap();
    let before = editor.document_stamp();
    let map = Mapping {
        scale: 1.01,
        offset: Point::default(),
    };
    let bytes = encode_mapped(&editor, None, map, |_| unreachable!()).unwrap();
    let raw = RawValue::from_string(String::from_utf8(bytes).unwrap()).unwrap();
    let reopened = crate::session_vectors::restore(Some(&raw)).unwrap().editor;
    let points = &reopened.elements()[0].points;
    assert!(((points[1].x - points[0].x) / 2.0 - 14.14).abs() < 0.0001);
    assert!(before.matches(&editor));
}
