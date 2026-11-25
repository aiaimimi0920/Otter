//! 文字批次的单步历史、失败原子性和历史点预算，不依赖系统剪贴板。
use super::*;

fn row(label: &str) -> (Point, Point, TextAnnotation) {
    (
        Point { x: 10.0, y: 20.0 },
        Point { x: 80.0, y: 24.0 },
        TextAnnotation::new(label, "微软雅黑", 16.0, false).unwrap(),
    )
}

#[test]
fn batch_is_one_revision_and_one_undo_without_replacing_existing_text() {
    let mut editor = Editor::default();
    let (start, size, text) = row("原有");
    editor.insert_text(start, size, text).unwrap();
    let original = editor.elements()[0].annotation_id().to_owned();
    let revision = editor.revision();
    assert_eq!(
        editor.insert_text_batch(&[row("第一行"), row("第二行")]),
        Ok(1..3)
    );
    assert_eq!(editor.revision(), revision.wrapping_add(1));
    let ids: Vec<_> = editor
        .elements()
        .iter()
        .map(|s| s.annotation_id().to_owned())
        .collect();
    assert_ne!(ids[1], ids[2]);
    assert!(editor.undo());
    assert_eq!(editor.elements().len(), 1);
    assert_eq!(editor.elements()[0].annotation_id(), original);
    assert!(editor.redo());
    assert_eq!(
        editor
            .elements()
            .iter()
            .map(|s| s.annotation_id().to_owned())
            .collect::<Vec<_>>(),
        ids
    );
    assert_eq!(
        editor.elements()[2].text.as_ref().unwrap().content(),
        "第二行"
    );
}

#[test]
fn invalid_late_row_preserves_document_revision_and_redo() {
    let mut editor = Editor::default();
    editor.insert_text_batch(&[row("待重做")]).unwrap();
    assert!(editor.undo());
    let stamp = editor.document_stamp();
    let mut invalid = row("非法末行");
    invalid.1.x = f32::NAN;
    assert_eq!(
        editor.insert_text_batch(&[row("不应提交"), invalid]),
        Err(EditError::InvalidPoint)
    );
    assert!(stamp.matches(&editor));
    assert!(editor.elements().is_empty());
    assert_eq!(editor.retained_points, 2);
    assert!(editor.redo());
    assert_eq!(
        editor.elements()[0].text.as_ref().unwrap().content(),
        "待重做"
    );
}

#[test]
fn batch_and_document_budgets_reject_without_partial_edits() {
    let mut editor = Editor::default();
    let rows = vec![row("行"); 256];
    assert_eq!(
        editor.insert_text_batch(&vec![row("超限"); 257]),
        Err(EditError::BudgetExceeded)
    );
    for _ in 0..MAX_ELEMENTS / 256 {
        editor.insert_text_batch(&rows).unwrap();
    }
    let stamp = editor.document_stamp();
    assert_eq!(
        editor.insert_text_batch(&[row("溢出")]),
        Err(EditError::BudgetExceeded)
    );
    assert!(stamp.matches(&editor));
    assert_eq!(editor.elements().len(), MAX_ELEMENTS);
    let mut points = Editor {
        retained_points: MAX_TOTAL_POINTS - 1,
        ..Editor::default()
    };
    assert_eq!(
        points.insert_text_batch(&[row("点预算")]),
        Err(EditError::BudgetExceeded)
    );
    assert!(points.elements().is_empty());
}

#[test]
fn empty_or_active_draft_does_not_consume_history() {
    let mut editor = Editor::default();
    let stamp = editor.document_stamp();
    assert_eq!(editor.insert_text_batch(&[]), Ok(0..0));
    assert!(stamp.matches(&editor));
    editor.begin(Tool::Brush, Point { x: 1.0, y: 1.0 }).unwrap();
    let revision = editor.revision();
    assert_eq!(editor.insert_text_batch(&[row("忙碌")]), Ok(0..0));
    assert_eq!(editor.revision(), revision);
    assert!(editor.draft.is_some());
}

#[test]
fn pruning_and_redo_replacement_account_for_batch_points() {
    let mut editor = Editor::default();
    for _ in 0..60 {
        editor.insert_text_batch(&[row("一"), row("二")]).unwrap();
    }
    assert_eq!(editor.retained_points, 240);
    for _ in 0..50 {
        assert!(editor.undo());
    }
    assert!(!editor.undo());
    assert_eq!(editor.elements().len(), 20);
    assert_eq!(editor.retained_points, 240);
    editor
        .insert_text_batch(&[row("替代一"), row("替代二")])
        .unwrap();
    assert!(!editor.redo());
    assert_eq!(editor.retained_points, 44);
    assert_eq!(editor.elements().len(), 22);
}
