//! 外部源不占 undo，但所有旧 raster 历史必须跟随新基线；不牺牲本地层换同步成功。
use super::*;
use crate::erase_layers::{Layers, Mode};
fn document() -> Editor {
    Editor::with_layers(Layers {
        size: (8, 4),
        base: vec![255; 128].into(),
        annotations: Some(vec![128; 128].into()),
        annotation_size: None,
    })
    .unwrap()
}
#[test]
fn projection_sync_source_rebases_current_and_redo_raster_preserving_vectors_crop_and_layers() {
    let mut editor = document();
    editor.begin(Tool::Brush, Point { x: 1.0, y: 1.0 }).unwrap();
    assert!(editor.commit());
    let source = editor.layers().unwrap().clone();
    let annotations = source
        .erase(
            Mode::RasterAnnotations,
            &[Point { x: 1.0, y: 1.0 }],
            4.0,
            || false,
        )
        .unwrap();
    assert!(editor
        .replace_layers(editor.revision(), &source, annotations.clone())
        .unwrap());
    assert!(editor
        .crop_to(crate::crop_geometry::Rect {
            x: 1.0,
            y: 0.0,
            width: 4.0,
            height: 3.0
        })
        .unwrap());
    let crop = editor.crop_view();
    assert!(editor.undo()); // redo 中同样有原底图。
    let base: crate::pixels::Pixels = vec![32; 128].into();
    assert!(editor.replace_source(base.clone()));
    assert_eq!(editor.elements().len(), 1);
    assert!(editor
        .layers()
        .unwrap()
        .annotations
        .as_ref()
        .unwrap()
        .same_storage(annotations.annotations.as_ref().unwrap()));
    assert!(editor.redo());
    assert_eq!(editor.crop_view(), crop);
    for _ in 0..3 {
        assert!(editor.undo());
        assert!(editor.layers().unwrap().base.same_storage(&base));
    }
    for _ in 0..3 {
        assert!(editor.redo());
        assert!(editor.layers().unwrap().base.same_storage(&base));
    }
    assert!(!editor.redo());
}
#[test]
fn projection_sync_source_refuses_destructive_history_without_dropping_content_or_undo() {
    let mut editor = document();
    let old = editor.layers().unwrap().clone();
    let erased = old
        .erase(Mode::Content, &[Point { x: 2.0, y: 2.0 }], 4.0, || false)
        .unwrap();
    assert!(editor.replace_layers(0, &old, erased.clone()).unwrap());
    assert!(!editor.replace_source(vec![32; 128].into()));
    assert!(editor.layers().unwrap().same_storage(&erased));
    assert!(editor.undo());
    assert!(!editor.source_replaceable()); // redo 仍含破坏性层。
    assert!(editor.redo());
}
#[test]
fn projection_sync_source_rejects_draft_and_wrong_size_atomically() {
    let mut editor = document();
    let old = editor.layers().unwrap().clone();
    assert!(!editor.replace_source(vec![0; 4].into()));
    editor.begin(Tool::Brush, Point { x: 1.0, y: 1.0 }).unwrap();
    assert!(!editor.replace_source(vec![0; 128].into()));
    assert!(editor.layers().unwrap().same_storage(&old));
    assert!(editor.draft().is_some());
}
