//! 稳定身份不是数组下标：删除/复制/历史/快照均走真实 Editor 操作。
use super::*;

fn restored() -> Editor {
    let mut editor = Editor::default();
    for x in [1.0, 2.0] {
        editor
            .append_restored(Tool::Brush, vec![Point { x, y: 0.0 }], 0.0, vec![])
            .unwrap();
    }
    editor
        .with_restored_annotation_ids(&["old-a".into(), "old-b".into()])
        .unwrap()
}

fn ids(editor: &Editor) -> Vec<String> {
    editor
        .elements()
        .iter()
        .map(|s| s.annotation_id().to_owned())
        .collect()
}

#[test]
fn deletion_and_history_keep_identity_attached_to_the_object() {
    let mut editor = restored();
    let snapshot = editor.export_snapshot();
    assert!(editor.remove(0));
    assert_eq!(ids(&editor), ["old-b"]);
    assert!(editor.undo());
    assert_eq!(ids(&editor), ["old-a", "old-b"]);
    assert!(editor.redo());
    assert_eq!(ids(&editor), ["old-b"]);
    assert_eq!(ids(&snapshot), ["old-a", "old-b"]);
}

#[test]
fn duplicate_gets_new_identity_but_preview_edits_and_redo_preserve_it() {
    let mut editor = restored();
    let index = editor.duplicate(0).unwrap().unwrap();
    let copy = editor.elements()[index].annotation_id().to_owned();
    assert_ne!(copy, "old-a");
    assert_ne!(copy, "old-b");
    assert_eq!(copy.len(), 36);
    editor.translate(index, Point { x: 3.0, y: 2.0 }).unwrap();
    assert_eq!(editor.elements()[index].annotation_id(), copy);
    assert!(editor.undo());
    assert!(editor.undo());
    assert_eq!(ids(&editor), ["old-a", "old-b"]);
    assert!(editor.redo());
    assert_eq!(editor.elements()[index].annotation_id(), copy);
    assert_eq!(ids(&editor.export_snapshot()), ids(&editor));
}

#[test]
fn new_strokes_text_and_serial_have_distinct_ids() {
    let mut editor = Editor::default();
    editor.begin(Tool::Brush, Point::default()).unwrap();
    assert!(editor.commit());
    let text = crate::text_annotation::TextAnnotation::new("a", "Arial", 12.0, false).unwrap();
    editor
        .insert_text(Point::default(), Point { x: 20.0, y: 20.0 }, text)
        .unwrap();
    editor
        .insert_serial(Point { x: 30.0, y: 30.0 }, 12.0, "Arial")
        .unwrap();
    let ids = ids(&editor);
    assert_eq!(ids.len(), 3);
    assert_eq!(
        ids.iter().collect::<std::collections::HashSet<_>>().len(),
        3
    );
    assert!(ids.iter().all(|id| id.len() == 36));
}

#[test]
fn restored_identity_rejects_bad_count_duplicates_and_active_editor() {
    for ids in [
        vec!["a".into()],
        vec!["a".into(), "a".into()],
        vec!["".into(), "b".into()],
        vec!["a\n".into(), "b".into()],
        vec!["x".repeat(257), "b".into()],
    ] {
        assert!(restored().with_restored_annotation_ids(&ids).is_err());
    }
    let mut editor = restored();
    editor.begin(Tool::Brush, Point::default()).unwrap();
    assert!(editor
        .with_restored_annotation_ids(&["a".into(), "b".into()])
        .is_err());
    let mut editor = restored();
    editor.translate(0, Point { x: 1.0, y: 0.0 }).unwrap();
    assert!(editor
        .with_restored_annotation_ids(&["a".into(), "b".into()])
        .is_err());
}

#[test]
fn session_import_installs_z_sorted_ids_in_the_actual_editor() {
    let raw = serde_json::value::RawValue::from_string(r##"{"serialCounter":0,"elements":[{"id":"upper","zIndex":3,"type":"brush","points":[{"x":1,"y":2}],"style":{"width":2,"color":"#000000"}},{"id":"lower","zIndex":1,"type":"brush","points":[{"x":3,"y":4}],"style":{"width":2,"color":"#000000"}}]}"##.into()).unwrap();
    let restored = crate::session_vectors::restore(Some(&raw)).unwrap();
    assert_eq!(ids(&restored.editor), ["lower", "upper"]);
    assert_eq!(ids(&restored.editor), restored.ids);
}
