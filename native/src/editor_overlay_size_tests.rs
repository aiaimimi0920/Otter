use super::*;
fn source() -> Editor {
    Editor::with_layers(crate::erase_layers::Layers {
        size: (8, 8),
        annotation_size: None,
        annotations: None,
        base: vec![255; 256].into(),
    })
    .unwrap()
}

#[test]
fn anisotropic_overlay_mapping_preserves_original_history_and_crop() {
    let mut editor = source();
    editor.begin(Tool::Brush, Point { x: 2.0, y: 3.0 }).unwrap();
    assert!(editor.commit());
    let width = editor.elements()[0].width;
    editor
        .crop_to(crate::crop_geometry::Rect {
            x: 1.0,
            y: 2.0,
            width: 4.0,
            height: 4.0,
        })
        .unwrap();
    let stamp = editor.document_stamp();
    let mapped = editor
        .export_overlay_for_size((16, 24), |_| unreachable!())
        .unwrap();
    assert!(mapped.layers().is_none());
    assert_eq!(mapped.elements()[0].points[0], Point { x: 4.0, y: 9.0 });
    assert_eq!(mapped.elements()[0].width, width * 2.5);
    let rect = mapped.crop_view().unwrap().source.rect;
    assert_eq!(
        (rect.x, rect.y, rect.width, rect.height),
        (2.0, 6.0, 8.0, 12.0)
    );
    assert!(stamp.matches(&editor));
    assert_eq!(editor.elements()[0].points[0], Point { x: 2.0, y: 3.0 });
    assert!(editor.undo());
    assert!(editor.crop_view().is_none());
}

#[test]
fn text_uses_scaled_font_metrics_and_failed_measurement_leaves_source_unchanged() {
    let mut editor = source();
    editor
        .insert_text(
            Point { x: 1.0, y: 1.0 },
            Point { x: 4.0, y: 2.0 },
            crate::text_annotation::TextAnnotation::new("测试", "微软雅黑", 16.0, false).unwrap(),
        )
        .unwrap();
    let stamp = editor.document_stamp();
    let mapped = editor
        .export_overlay_for_size((16, 16), |text| {
            assert_eq!(text.font_size(), 32.0);
            Ok(Point { x: 8.0, y: 4.0 })
        })
        .unwrap();
    assert_eq!(
        mapped.elements()[0].text.as_ref().unwrap().font_size(),
        32.0
    );
    assert!(editor
        .export_overlay_for_size((16, 16), |_| Err(EditError::InvalidPoint))
        .is_err());
    assert!(editor
        .export_overlay_for_size((0, 16), |_| unreachable!())
        .is_err());
    assert!(stamp.matches(&editor));
}
