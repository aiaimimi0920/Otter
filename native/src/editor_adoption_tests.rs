//! 暂存接管结算不接触窗口；覆盖取消、首笔合并、身份替换和预算。
use super::*;
fn image(value: u8) -> Editor {
    Editor::with_layers(crate::erase_layers::Layers {
        annotation_size: None,
        size: (1, 1),
        base: vec![value; 4].into(),
        annotations: None,
    })
    .unwrap()
}
fn mark(editor: &mut Editor) {
    editor.begin(Tool::Brush, Point { x: 0.0, y: 0.0 }).unwrap();
    assert!(editor.commit());
}
#[test]
fn cancelled_adoption_preserves_identity_and_original_redo_branch() {
    let mut editor = image(10);
    mark(&mut editor);
    assert!(editor.undo());
    let original = editor.document_stamp();
    let source = image(20);
    let mut pending = editor.begin_adoption(&source).unwrap().unwrap();
    editor.begin(Tool::Brush, Point::default()).unwrap();
    assert_eq!(
        pending.finish(&mut editor).unwrap(),
        AdoptionOutcome::Pending
    );
    editor.cancel();
    assert_eq!(
        pending.finish(&mut editor).unwrap(),
        AdoptionOutcome::Cancelled
    );
    assert!(original.matches(&editor));
    assert_eq!(&*editor.layers().unwrap().base, &[10; 4]);
    assert!(editor.redo());
    assert_eq!(editor.elements().len(), 1);
}
#[test]
fn first_edit_and_adoption_form_one_undo_step_without_mutating_source() {
    let mut editor = image(10);
    mark(&mut editor);
    let source = image(20);
    let mut pending = editor.begin_adoption(&source).unwrap().unwrap();
    mark(&mut editor);
    assert_eq!(
        pending.finish(&mut editor).unwrap(),
        AdoptionOutcome::Committed
    );
    assert_eq!(&*editor.layers().unwrap().base, &[20; 4]);
    assert!(editor.undo());
    assert_eq!(&*editor.layers().unwrap().base, &[10; 4]);
    assert_eq!(editor.elements().len(), 1);
    assert!(editor.undo());
    assert!(editor.elements().is_empty());
    assert!(editor.redo() && editor.redo());
    assert_eq!(&*editor.layers().unwrap().base, &[20; 4]);
    assert!(source.elements().is_empty());
}
#[test]
fn replacement_identity_is_never_overwritten_by_late_settlement() {
    let mut editor = image(10);
    let weak = editor.layers().unwrap().base.downgrade();
    let mut pending = editor.begin_adoption(&image(20)).unwrap().unwrap();
    editor = image(30);
    assert_eq!(
        pending.finish(&mut editor).unwrap(),
        AdoptionOutcome::Superseded
    );
    assert_eq!(&*editor.layers().unwrap().base, &[30; 4]);
    assert!(weak.upgrade().is_none());
}
#[test]
fn temporary_edits_include_original_history_in_point_budget() {
    let mut editor = image(10);
    editor.retained_points = MAX_TOTAL_POINTS;
    let mut pending = editor.begin_adoption(&image(20)).unwrap().unwrap();
    assert_eq!(
        editor.begin(Tool::Brush, Point::default()),
        Err(EditError::BudgetExceeded)
    );
    assert_eq!(
        pending.finish(&mut editor).unwrap(),
        AdoptionOutcome::Cancelled
    );
    assert_eq!(editor.retained_points, MAX_TOTAL_POINTS);
}
