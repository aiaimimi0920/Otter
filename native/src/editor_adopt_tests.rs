//! 接管事务与原有历史共存；不触及窗口、系统剪贴板或磁盘。
use super::*;

fn image(value: u8) -> Editor {
    Editor::with_layers(crate::erase_layers::Layers {
        annotation_size: None,
        size: (2, 2),
        base: vec![value; 16].into(),
        annotations: None,
    })
    .unwrap()
}
fn mark(editor: &mut Editor, x: f32) {
    editor.begin(Tool::Brush, Point { x, y: 1.0 }).unwrap();
    assert!(editor.commit());
}

#[test]
fn adoption_preserves_old_history_and_source_snapshot_through_undo_redo() {
    let mut target = image(10);
    mark(&mut target, 0.0);
    let original = target.layers().unwrap().clone();
    let mut source = image(20);
    mark(&mut source, 1.0);
    source
        .crop_to(crate::crop_geometry::Rect {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        })
        .unwrap();
    let incoming = source.layers().unwrap().clone();
    let stamp = target.document_stamp();
    assert!(target.adopt_snapshot(target.revision(), &source).unwrap());
    assert!(!stamp.matches(&target));
    assert!(target.layers().unwrap().same_storage(&incoming));
    assert_eq!(target.elements()[0].points[0].x, 1.0);
    assert!(target.crop_view().is_some());
    mark(&mut target, 0.5);
    assert!(target.undo());
    assert!(target.undo());
    assert!(target.layers().unwrap().same_storage(&original));
    assert!(target.crop_view().is_none());
    assert_eq!(target.elements()[0].points[0].x, 0.0);
    assert!(target.undo());
    assert!(target.elements().is_empty());
    for _ in 0..3 {
        assert!(target.redo());
    }
    assert!(target.layers().unwrap().same_storage(&incoming));
    assert_eq!(target.elements().len(), 2);
    assert_eq!(source.elements().len(), 1);
    assert!(source.crop_view().is_some());
}

#[test]
fn empty_target_restores_no_raster_and_branch_replacement_releases_pixels() {
    let mut target = Editor::default();
    let source = image(30);
    let weak = source.layers().unwrap().base.downgrade();
    assert!(target.adopt_snapshot(0, &source).unwrap());
    drop(source);
    assert!(target.undo());
    assert!(target.layers().is_none());
    assert!(weak.upgrade().is_some());
    mark(&mut target, 1.0);
    assert!(!target.redo());
    assert!(weak.upgrade().is_none());
}

#[test]
fn repeated_adoption_respects_history_eviction_and_point_accounting() {
    let mut target = image(10);
    mark(&mut target, 0.0);
    let oldest = target.layers().unwrap().base.downgrade();
    let mut source = image(20);
    mark(&mut source, 1.0);
    let points = source.elements()[0].points.len();
    for _ in 0..60 {
        assert!(target.adopt_snapshot(target.revision(), &source).unwrap());
    }
    assert!(oldest.upgrade().is_none());
    assert_eq!(target.retained_points, 51 * points);
    let mut undo_count = 0;
    while target.undo() {
        undo_count += 1;
    }
    assert_eq!(undo_count, 50);
    mark(&mut target, 0.5);
    assert_eq!(target.retained_points, 2 * points);
    assert!(!target.redo());
}

#[test]
fn stale_revision_drafts_and_budget_fail_without_mutating_history() {
    let mut target = image(10);
    let mut source = image(20);
    assert!(!target.adopt_snapshot(1, &source).unwrap());
    source.begin(Tool::Brush, Point { x: 1.0, y: 1.0 }).unwrap();
    assert!(!target.adopt_snapshot(0, &source).unwrap());
    source.cancel();
    target.begin(Tool::Brush, Point { x: 1.0, y: 1.0 }).unwrap();
    assert!(!target.adopt_snapshot(0, &source).unwrap());
    target.cancel();
    mark(&mut source, 1.0);
    target.retained_points = MAX_TOTAL_POINTS;
    assert_eq!(
        target.adopt_snapshot(0, &source),
        Err(EditError::BudgetExceeded)
    );
    assert_eq!(target.revision(), 0);
    assert!(!target.undo());
    assert!(target.elements().is_empty());
}
