//! 标注事务的身份、点预算、历史与栅格所有权回归；不创建窗口或读取用户文件。
use super::*;
use crate::crop_geometry::{Crop, Rect, Size};

fn image(value: u8) -> Editor {
    Editor::with_layers(crate::erase_layers::Layers {
        size: (8, 4),
        base: vec![value; 128].into(),
        annotations: None,
        annotation_size: None,
    })
    .unwrap()
}
fn mark(editor: &mut Editor, x: f32) {
    editor.begin(Tool::Brush, Point { x, y: 1.0 }).unwrap();
    assert!(editor.commit());
}
fn frame() -> Crop {
    Crop {
        rect: Rect {
            x: 1.25,
            y: 0.5,
            width: 2.5,
            height: 1.5,
        },
        source: Size {
            width: 7.5,
            height: 3.75,
        },
    }
}

#[test]
fn annotation_and_source_rebase_are_one_transaction_and_undo_never_resurrects_old_pixels() {
    let mut target = image(10);
    mark(&mut target, 1.0);
    let mut source = image(20);
    mark(&mut source, 2.0);
    let new = vec![30; 128].into();
    assert!(target
        .prepare_annotation_replacement(&source)
        .unwrap()
        .apply_rebased(&mut target, Some(new))
        .unwrap());
    assert_eq!(target.elements[0].points[0].x, 2.0);
    assert_eq!(&*target.layers().unwrap().base, &[30; 128]);
    assert!(target.undo());
    assert_eq!(target.elements[0].points[0].x, 1.0);
    assert_eq!(&*target.layers().unwrap().base, &[30; 128]);
    assert!(target.undo());
    assert!(target.elements.is_empty());
    assert_eq!(&*target.layers().unwrap().base, &[30; 128]);
}
#[test]
fn bad_rebase_size_or_stale_target_never_publishes_annotations_or_pixels() {
    let mut target = image(10);
    mark(&mut target, 1.0);
    let mut source = image(20);
    mark(&mut source, 2.0);
    let stamp = target.document_stamp();
    assert!(!target
        .prepare_annotation_replacement(&source)
        .unwrap()
        .apply_rebased(&mut target, Some(vec![30; 4].into()))
        .unwrap());
    assert!(stamp.matches(&target));
    assert_eq!(target.elements[0].points[0].x, 1.0);
    assert_eq!(&*target.layers().unwrap().base, &[10; 128]);
    let prepared = target.prepare_annotation_replacement(&source).unwrap();
    target.translate(0, Point { x: 1.0, y: 0.0 }).unwrap();
    assert!(!prepared
        .apply_rebased(&mut target, Some(vec![30; 128].into()))
        .unwrap());
    assert_eq!(&*target.layers().unwrap().base, &[10; 128]);
}

#[test]
fn replacement_undo_redo_preserve_raster_fractional_crop_style_and_prior_history() {
    let mut target = image(10).with_restored_crop(frame()).unwrap();
    mark(&mut target, 1.0);
    let old_id = target.elements[0].annotation_id().to_owned();
    let layers = target.layers().unwrap().clone();
    let crop = target.crop_view();
    let mut source = image(20);
    mark(&mut source, 2.0);
    source.serial_count = 9;
    let id = source.elements[0].annotation_id().to_owned();
    target.set_style(target.style().with_width(8.0).unwrap());
    let style = target.style();
    let stamp = target.document_stamp();
    assert!(target
        .prepare_annotation_replacement(&source)
        .unwrap()
        .apply(&mut target)
        .unwrap());
    assert!(stamp.same_document(&target));
    assert!(!stamp.matches(&target));
    assert_eq!(target.elements[0].annotation_id(), id);
    assert_eq!(target.serial_count, 9);
    assert_eq!(target.style(), style);
    assert!(target.layers().unwrap().same_storage(&layers));
    assert_eq!(target.crop_view(), crop);
    assert_eq!(target.visible_frame(), Some(frame().rect));
    assert!(target.undo());
    assert_eq!(target.elements[0].annotation_id(), old_id);
    assert_eq!(target.serial_count, 0);
    assert!(target.layers().unwrap().same_storage(&layers));
    assert_eq!(target.crop_view(), crop);
    assert!(target.undo());
    assert!(target.elements.is_empty());
    assert!(target.redo());
    assert!(target.redo());
    assert_eq!(target.elements[0].annotation_id(), id);
    assert_eq!(target.serial_count, 9);
    assert_eq!(target.style(), style);
    assert!(target.layers().unwrap().same_storage(&layers));
    assert_eq!(target.crop_view(), crop);
}

#[test]
fn prepared_source_is_frozen_and_does_not_retain_source_pixels_or_history() {
    let mut target = image(10);
    let mut source = image(20);
    mark(&mut source, 2.0);
    let id = source.elements[0].annotation_id().to_owned();
    let weak = source.layers().unwrap().base.downgrade();
    let prepared = target.prepare_annotation_replacement(&source).unwrap();
    source.translate(0, Point { x: 3.0, y: 0.0 }).unwrap();
    source.serial_count = 42;
    drop(source);
    assert!(
        weak.upgrade().is_none(),
        "candidate must not pin unrelated source raster"
    );
    assert!(prepared.apply(&mut target).unwrap());
    assert_eq!(target.elements[0].annotation_id(), id);
    assert_eq!(target.elements[0].points[0].x, 2.0);
    assert_eq!(target.serial_count, 0);
    assert!(target.undo());
    assert!(!target.undo(), "source history must not be imported");
}

#[test]
fn foreign_same_revision_stale_revision_and_active_draft_reject_without_partial_changes() {
    let mut source = Editor::default();
    mark(&mut source, 2.0);
    let target = image(10);
    let mut foreign = image(30);
    let prepared = target.prepare_annotation_replacement(&source).unwrap();
    assert_eq!(target.revision(), foreign.revision());
    assert!(!prepared.apply(&mut foreign).unwrap());
    assert!(!foreign.undo());
    let mut target = target;
    let prepared = target.prepare_annotation_replacement(&source).unwrap();
    mark(&mut target, 1.0);
    assert!(!prepared.apply(&mut target).unwrap());
    assert_eq!(target.elements[0].points[0].x, 1.0);
    let prepared = target.prepare_annotation_replacement(&source).unwrap();
    target.begin(Tool::Brush, Point::default()).unwrap();
    assert!(!prepared.apply(&mut target).unwrap());
    assert!(target.draft().is_some());
    assert!(target.cancel());
    assert_eq!(target.elements.len(), 1);
}

#[test]
fn invalid_source_and_budget_failure_keep_target_identity_history_and_pixels() {
    let mut target = image(10);
    let mut source = Editor::default();
    source.begin(Tool::Brush, Point::default()).unwrap();
    assert!(matches!(
        target.prepare_annotation_replacement(&source),
        Err(EditError::InvalidPoint)
    ));
    source.commit();
    let stamp = target.document_stamp();
    target.retained_points = MAX_TOTAL_POINTS;
    assert!(matches!(
        target.prepare_annotation_replacement(&source),
        Err(EditError::BudgetExceeded)
    ));
    assert!(stamp.matches(&target));
    assert!(target.elements.is_empty());
    assert!(!target.undo());
    assert_eq!(target.layers().unwrap().base.as_ref(), &[10; 128]);
}

#[test]
fn history_eviction_and_redo_branch_account_for_all_replacement_points() {
    let mut target = Editor::default();
    let mut source = Editor::default();
    mark(&mut source, 2.0);
    mark(&mut target, 1.0);
    for _ in 0..60 {
        assert!(target
            .prepare_annotation_replacement(&source)
            .unwrap()
            .apply(&mut target)
            .unwrap());
    }
    assert_eq!(target.retained_points, 51);
    let mut count = 0;
    while target.undo() {
        count += 1;
    }
    assert_eq!(count, 50);
    mark(&mut target, 3.0);
    assert_eq!(target.retained_points, 2);
    assert!(!target.redo());
    let empty = Editor::default();
    assert!(target
        .prepare_annotation_replacement(&empty)
        .unwrap()
        .apply(&mut target)
        .unwrap());
    assert!(target.elements.is_empty());
    assert!(target.undo());
    assert_eq!(target.elements.len(), 2);
    assert!(target.redo());
    assert!(target.elements.is_empty());
}
