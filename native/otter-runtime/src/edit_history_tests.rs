//! 删除和历史淘汰保持所有权与顺序，不依赖生产文档整份快照。
use super::*;

fn add(editor: &mut Editor, x: f32) {
    editor.begin(Tool::Brush, Point { x, y: 1.0 }).unwrap();
    assert!(editor.commit());
}
fn xs(editor: &Editor) -> Vec<f32> {
    editor.elements().iter().map(|s| s.points[0].x).collect()
}

#[test]
fn middle_delete_restores_exact_order_and_point_allocation() {
    let mut editor = Editor::default();
    for x in [1.0, 2.0, 3.0] {
        add(&mut editor, x);
    }
    let points = editor.elements()[1].points.as_ptr();
    let snapshot = editor.export_snapshot();
    assert!(editor.remove(1));
    assert_eq!(xs(&editor), vec![1.0, 3.0]);
    assert_eq!(editor.retained_points, 3);
    assert!(editor.undo());
    assert_eq!(xs(&editor), vec![1.0, 2.0, 3.0]);
    assert_eq!(editor.elements()[1].points.as_ptr(), points);
    assert!(editor.redo());
    assert_eq!(xs(&snapshot), vec![1.0, 2.0, 3.0]);
    assert_eq!(xs(&editor), vec![1.0, 3.0]);
}

#[test]
fn branch_discards_only_redo_owned_points() {
    let mut editor = Editor::default();
    add(&mut editor, 1.0);
    add(&mut editor, 2.0);
    editor.undo();
    assert!(editor.remove(0));
    assert_eq!(editor.retained_points, 1);
    assert!(!editor.redo());
    editor.undo();
    assert_eq!(xs(&editor), vec![1.0]);
    editor.redo();
    add(&mut editor, 3.0);
    assert_eq!(editor.retained_points, 2);
    editor.undo();
    editor.undo();
    assert_eq!(xs(&editor), vec![1.0]);
}

#[test]
fn deleting_many_elements_evicts_old_owned_points_but_keeps_fifty_undo_steps() {
    let mut editor = Editor::default();
    for i in 0..70 {
        add(&mut editor, i as f32);
    }
    for _ in 0..70 {
        assert!(editor.remove_last());
    }
    assert!(editor.elements().is_empty());
    assert_eq!(editor.retained_points, 50);
    for _ in 0..50 {
        assert!(editor.undo());
    }
    assert!(!editor.undo());
    assert_eq!(xs(&editor), (0..50).map(|i| i as f32).collect::<Vec<_>>());
    for _ in 0..50 {
        assert!(editor.redo());
    }
    assert_eq!(editor.retained_points, 50);
}

#[test]
fn invalid_delete_and_draft_do_not_change_document_revision() {
    let mut editor = Editor::default();
    add(&mut editor, 1.0);
    let revision = editor.revision();
    assert!(!editor.remove(1));
    editor.begin(Tool::Brush, Point { x: 2.0, y: 2.0 }).unwrap();
    assert!(!editor.remove_last());
    assert_eq!(editor.revision(), revision);
    assert_eq!(xs(&editor), vec![1.0]);
}

#[test]
fn restyle_roundtrips_without_copying_points_or_mutating_export_snapshot() {
    let mut editor = Editor::default();
    add(&mut editor, 1.0);
    let original = editor.elements()[0].style();
    let pointer = editor.elements()[0].points.as_ptr();
    let snapshot = editor.export_snapshot();
    let changed = original
        .with_width(30.0)
        .unwrap()
        .with_color(Color(1, 2, 3))
        .with_opacity(0.25)
        .unwrap()
        .with_fill(Some(Color(4, 5, 6)))
        .with_corner_radius(40.0)
        .unwrap()
        .with_sides(8)
        .with_dash(crate::stroke_style::DashPattern::DashDot);
    assert!(editor.restyle(0, changed));
    assert_eq!(editor.retained_points, 1);
    assert_eq!(editor.elements()[0].style(), changed);
    assert_eq!(editor.elements()[0].points.as_ptr(), pointer);
    assert_eq!(snapshot.elements()[0].style(), original);
    editor.undo();
    assert_eq!(editor.elements()[0].style(), original);
    editor.redo();
    assert_eq!(editor.elements()[0].style(), changed);
    assert_eq!(editor.elements()[0].points.as_ptr(), pointer);
}

#[test]
fn style_delete_and_branch_history_keep_correct_target() {
    let mut editor = Editor::default();
    add(&mut editor, 1.0);
    add(&mut editor, 2.0);
    let changed = editor.style().with_color(Color(0, 0, 255));
    editor.restyle(0, changed);
    editor.remove(0);
    editor.undo();
    assert_eq!(editor.elements()[0].style(), changed);
    editor.undo();
    assert_ne!(editor.elements()[0].style(), changed);
    assert!(
        !editor.restyle(0, editor.elements()[0].style()),
        "no-op preserves redo"
    );
    assert!(editor.redo());
    assert!(editor.redo());
    assert_eq!(xs(&editor), vec![2.0]);
    editor.undo();
    editor.restyle(1, changed);
    assert!(!editor.redo());
    assert_eq!(editor.retained_points, 2);
}

#[test]
fn move_undo_restores_original_float_bits_and_accounts_for_owned_points() {
    let mut editor = Editor::default();
    add(&mut editor, 0.1);
    let original = editor.elements()[0].points.clone();
    let pointer = editor.elements()[0].points.as_ptr();
    assert!(editor.translate(0, Point { x: 0.2, y: -0.7 }).unwrap());
    assert_eq!(editor.retained_points, 2);
    let moved = editor.elements()[0].points.clone();
    for _ in 0..20 {
        editor.undo();
        assert_eq!(editor.elements()[0].points, original);
        assert_eq!(editor.elements()[0].points.as_ptr(), pointer);
        editor.redo();
        assert_eq!(editor.elements()[0].points, moved);
    }
    editor.undo();
    add(&mut editor, 2.0);
    assert_eq!(editor.retained_points, 2);
    assert!(!editor.redo());
}

#[test]
fn invalid_or_empty_move_does_not_mutate_document() {
    let mut editor = Editor::default();
    add(&mut editor, 1.0);
    let revision = editor.revision();
    assert!(!editor.translate(0, Point::default()).unwrap());
    assert!(editor
        .translate(
            0,
            Point {
                x: 1_000_000.0,
                y: 0.0
            }
        )
        .is_err());
    assert_eq!(editor.revision(), revision);
    assert_eq!(xs(&editor), vec![1.0]);
    assert_eq!(editor.retained_points, 1);
}

#[test]
fn rotation_history_is_constant_size_and_preserves_geometry() {
    let mut editor = Editor::default();
    add(&mut editor, 1.0);
    let pointer = editor.elements()[0].points.as_ptr();
    assert!(editor.rotate(0, -15.0).unwrap());
    assert_eq!(editor.elements()[0].rotation, 345.0);
    assert_eq!(editor.retained_points, 1);
    assert_eq!(editor.elements()[0].points.as_ptr(), pointer);
    editor.undo();
    assert_eq!(editor.elements()[0].rotation, 0.0);
    editor.redo();
    assert_eq!(editor.elements()[0].rotation, 345.0);
    let revision = editor.revision();
    assert!(!editor.rotate(0, 360.0).unwrap());
    assert!(editor.rotate(0, f32::NAN).is_err());
    assert_eq!(editor.revision(), revision);
}
