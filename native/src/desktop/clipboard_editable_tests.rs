use super::*;
use crate::{editor::Tool, viewport::Point};
#[path = "clipboard_capture_tests.rs"]
mod capture;

fn content() -> Content {
    let image = Image {
        width: 8,
        height: 8,
        pixels: vec![255; 256].into(),
    };
    let mut editor = Editor::with_layers(crate::erase_layers::Layers {
        size: (8, 8),
        base: image.pixels.clone(),
        annotations: Some(vec![0; 256].into()),
        annotation_size: None,
    })
    .unwrap();
    editor.begin(Tool::Brush, Point { x: 3.0, y: 4.0 }).unwrap();
    editor.commit();
    Content {
        image,
        editor,
        opacity: 0.5,
        capture_source: None,
        origin: None,
    }
}

#[test]
fn each_paste_has_fresh_document_annotations_and_history_but_shared_pixels() {
    let content = content();
    let source = content.editor.document_stamp();
    let first = content.prepare(|| false).unwrap();
    let first = first.editable.unwrap();
    let mut second = content.prepare(|| false).unwrap().editable.unwrap();
    assert!(!source.same_document(&first.editor));
    assert!(!first.editor.document_stamp().same_document(&second.editor));
    assert_ne!(
        first.editor.elements()[0].annotation_id(),
        second.editor.elements()[0].annotation_id()
    );
    assert_ne!(
        first.editor.elements()[0].annotation_id(),
        content.editor.elements()[0].annotation_id()
    );
    assert_eq!(
        first.editor.elements()[0].points,
        content.editor.elements()[0].points
    );
    assert!(first
        .editor
        .layers()
        .unwrap()
        .base
        .same_storage(&content.image.pixels));
    assert!(!second.editor.undo());
    second
        .editor
        .begin(Tool::Brush, Point { x: 1.0, y: 1.0 })
        .unwrap();
    second.editor.commit();
    assert_eq!(second.editor.elements().len(), 2);
    assert_eq!(first.editor.elements().len(), 1);
    assert!(source.matches(&content.editor));
}

#[test]
fn display_fallback_keeps_fractional_overlay_coordinates_and_existing_raster() {
    let mut content = content();
    content.editor = Editor::with_layers(content.editor.layers().unwrap().clone())
        .unwrap()
        .with_restored_size(crate::crop_geometry::Size {
            width: 7.5,
            height: 7.25,
        })
        .unwrap();
    content
        .editor
        .begin(Tool::Brush, Point { x: 3.0, y: 4.0 })
        .unwrap();
    content.editor.commit();
    let frame = content.editor.annotation_frame();
    let base = content.editor.layers().unwrap().base.clone();
    // Existing raster edits win over the display image fallback.
    content.image.pixels = vec![0; 256].into();
    let prepared = content.prepare(|| false).unwrap();
    assert!(prepared.image.pixels.same_storage(&base));
    assert_eq!(prepared.editable.unwrap().editor.annotation_frame(), frame);
    content.editor = content.editor.export_overlay_snapshot();
    let prepared = content.prepare(|| false).unwrap();
    assert!(prepared.image.pixels.same_storage(&content.image.pixels));
    let editor = prepared.editable.unwrap().editor;
    assert_eq!(editor.annotation_frame(), frame);
    assert_eq!(
        editor.elements()[0].points,
        content.editor.elements()[0].points
    );
    assert!(!editor.document_stamp().same_document(&content.editor));
    assert!(content.editor.layers().is_none());
    // An invalid fallback must never be installed into a previously vector-only editor.
    content.image.width = 9;
    assert!(content.prepare(|| false).is_err());
}
#[test]
fn cache_is_single_slot_and_zero_or_changed_sequence_discards_content() {
    let mut cache = Cache::default();
    let first = Arc::new(content());
    let weak = Arc::downgrade(&first);
    cache.published(1, Some(first));
    assert!(cache.matching(1).is_some());
    cache.published(2, Some(Arc::new(content())));
    assert!(weak.upgrade().is_none());
    assert!(cache.matching(1).is_none());
    assert!(cache.matching(2).is_none());
    cache.published(0, Some(Arc::new(content())));
    assert!(cache.matching(0).is_none());
    cache.published(3, Some(Arc::new(content())));
    cache.published(4, None);
    assert!(cache.matching(4).is_none());
}

#[test]
fn invalid_opacity_or_cancelled_region_is_not_delivered() {
    let mut content = content();
    assert!(content.prepare(|| true).is_err());
    for opacity in [f32::NAN, f32::INFINITY, -0.1, 1.1] {
        content.opacity = opacity;
        assert!(content.prepare(|| false).is_err());
    }
}
