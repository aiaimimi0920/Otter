use super::*;

fn p(x: f32, y: f32) -> Point {
    Point { x, y }
}

#[test]
fn polyline_retains_drag_vertices_and_allows_closed_path_with_exact_history() {
    let mut editor = Editor::default();
    editor.begin(Tool::Polyline, p(10.0, 10.0)).unwrap();
    assert!(!editor.commit());
    editor.begin(Tool::Polyline, p(10.0, 10.0)).unwrap();
    for point in [p(30.0, 10.0), p(30.0, 30.0), p(10.0, 10.0)] {
        editor.extend_constrained(point, true).unwrap();
    }
    assert!(editor.commit());
    let before = editor.elements()[0].points.clone();
    assert_eq!(before.len(), 4);
    assert_eq!(editor.retained_points, 4);
    editor.move_line_endpoint(0, 1, p(40.0, 15.0)).unwrap();
    assert_eq!(editor.retained_points, 8);
    assert_eq!(editor.elements()[0].points[1], p(40.0, 15.0));
    editor.undo();
    assert_eq!(editor.elements()[0].points, before);
    editor.undo();
    assert!(editor.elements().is_empty());
    editor.redo();
    assert_eq!(editor.elements()[0].points, before);
}

#[test]
fn bezier_creation_replaces_controls_and_history_retains_exact_curve() {
    let mut editor = Editor::default();
    editor.begin(Tool::Bezier, p(10.0, 10.0)).unwrap();
    editor.extend(p(30.0, 30.0)).unwrap();
    editor.extend(p(70.0, 10.0)).unwrap();
    let expected = crate::bezier_geometry::create(p(10.0, 10.0), p(70.0, 10.0));
    assert_eq!(editor.draft().unwrap().points.as_slice(), expected);
    assert!(editor.commit());
    assert_eq!(editor.retained_points, 4);
    assert!(editor.undo());
    assert!(editor.redo());
    assert_eq!(editor.elements()[0].points.as_slice(), expected);
    editor.begin(Tool::Bezier, p(0.0, 0.0)).unwrap();
    editor.retained_points = MAX_TOTAL_POINTS - 3;
    assert_eq!(editor.extend(p(10.0, 0.0)), Err(EditError::BudgetExceeded));
    assert_eq!(editor.draft().unwrap().points.len(), 1);
}

#[test]
fn straight_freehand_replaces_draft_and_resumes_with_one_history_entry() {
    for tool in [Tool::Brush, Tool::Highlighter] {
        let mut editor = Editor::default();
        editor.begin(tool, p(0.0, 0.0)).unwrap();
        editor.extend(p(5.0, 7.0)).unwrap();
        editor.extend(p(10.0, 10.0)).unwrap();
        assert!(editor.extend_constrained(p(10.0, 10.0), true).unwrap());
        assert_eq!(
            editor.draft().unwrap().points,
            vec![p(0.0, 0.0), p(10.0, 10.0)]
        );
        assert!(!editor.extend_constrained(p(10.0, 10.0), true).unwrap());
        editor.extend(p(12.0, 15.0)).unwrap();
        assert_eq!(editor.draft().unwrap().points.len(), 3);
        assert!(editor.commit());
        assert_eq!(editor.retained_points, 3);
        assert!(editor.undo());
        assert!(!editor.undo());
        assert!(editor.redo());
        assert_eq!(editor.elements()[0].points.len(), 3);
    }
}

#[test]
fn constrained_draft_validation_and_budget_remain_transactional() {
    let mut editor = Editor::default();
    editor.begin(Tool::Brush, p(0.0, 0.0)).unwrap();
    editor.retained_points = MAX_TOTAL_POINTS - 1;
    assert_eq!(
        editor.extend_constrained(p(1.0, 1.0), true),
        Err(EditError::BudgetExceeded)
    );
    assert_eq!(editor.draft().unwrap().points.len(), 1);
    editor.retained_points = 0;
    editor.extend(p(1.0, 2.0)).unwrap();
    editor.extend(p(2.0, 3.0)).unwrap();
    assert_eq!(
        editor.extend_constrained(p(f32::NAN, 1.0), true),
        Err(EditError::InvalidPoint)
    );
    assert_eq!(editor.draft().unwrap().points.len(), 3);
}
fn dab(editor: &mut Editor, x: f32) {
    assert!(editor.begin(Tool::Brush, p(x, 1.0)).unwrap());
    assert!(editor.commit());
}

#[test]
fn export_snapshot_is_independent_of_later_edits() {
    let mut editor = Editor::default();
    dab(&mut editor, 1.0);
    let snapshot = editor.export_snapshot();
    editor.undo();
    dab(&mut editor, 9.0);
    assert_eq!(snapshot.elements()[0].points, vec![p(1.0, 1.0)]);
    assert_eq!(editor.elements()[0].points, vec![p(9.0, 1.0)]);
}

#[test]
fn style_is_captured_at_gesture_start_without_mutating_history() {
    let mut editor = Editor::default();
    let red = StrokeStyle::new(12.0, Color(255, 0, 0)).unwrap();
    let blue = StrokeStyle::new(2.0, Color(0, 0, 255)).unwrap();
    editor.set_style(red);
    editor.begin(Tool::Brush, p(10.0, 10.0)).unwrap();
    editor.set_style(blue);
    assert_eq!(editor.draft().unwrap().color, red.color());
    editor.commit();
    let revision = editor.revision();
    editor.set_style(blue);
    assert_eq!(editor.revision(), revision);
    assert_eq!(editor.elements()[0].width, 12.0);
    assert_eq!(editor.export_snapshot().elements()[0].color, red.color());
    editor.undo();
    editor.set_style(red);
    assert!(editor.redo(), "tool settings do not invalidate redo");
    assert_eq!(editor.elements()[0].color, red.color());
}

#[test]
fn replacement_warning_includes_redo_branch_and_draft() {
    let mut editor = Editor::default();
    assert!(!editor.has_edit_state());
    editor.begin(Tool::Brush, p(1.0, 1.0)).unwrap();
    assert!(editor.has_edit_state());
    editor.cancel();
    assert!(!editor.has_edit_state());
    dab(&mut editor, 1.0);
    editor.undo();
    assert!(editor.elements().is_empty());
    assert!(editor.has_edit_state());
}

#[test]
fn gesture_is_one_transaction_and_cancel_preserves_redo() {
    let mut editor = Editor::default();
    dab(&mut editor, 1.0);
    editor.undo();
    editor.begin(Tool::Brush, p(2.0, 2.0)).unwrap();
    for x in 3..100 {
        editor.extend(p(x as f32, 2.0)).unwrap();
    }
    assert!(editor.elements().is_empty());
    assert!(!editor.redo());
    assert!(editor.cancel());
    assert!(editor.redo());
    assert_eq!(editor.elements()[0].points, vec![p(1.0, 1.0)]);
}

#[test]
fn commit_invalidates_redo_without_cloning_existing_elements() {
    let mut editor = Editor::default();
    dab(&mut editor, 1.0);
    let pointer = editor.elements()[0].points.as_ptr();
    dab(&mut editor, 2.0);
    editor.undo();
    dab(&mut editor, 3.0);
    assert!(!editor.redo());
    assert_eq!(editor.retained_points, 2);
    assert_eq!(editor.elements()[0].points.as_ptr(), pointer);
    assert_eq!(editor.elements()[1].points, vec![p(3.0, 1.0)]);
}

#[test]
fn history_keeps_document_but_limits_undo_steps() {
    let mut editor = Editor::default();
    for x in 0..70 {
        dab(&mut editor, x as f32);
    }
    for _ in 0..50 {
        assert!(editor.undo());
    }
    assert!(!editor.undo());
    assert_eq!(editor.elements().len(), 20);
    for _ in 0..50 {
        assert!(editor.redo());
    }
    assert_eq!(editor.elements().len(), 70);
}

#[test]
fn shapes_replace_endpoint_and_degenerate_shapes_do_not_commit() {
    let mut editor = Editor::default();
    editor.begin(Tool::Rectangle, p(2.0, 3.0)).unwrap();
    for x in 4..100 {
        editor.extend(p(x as f32, 3.0)).unwrap();
    }
    assert_eq!(editor.draft().unwrap().points.len(), 2);
    assert!(!editor.commit());
    editor.begin(Tool::Line, p(2.0, 3.0)).unwrap();
    editor.extend(p(2.0, 8.0)).unwrap();
    assert!(editor.commit());
}

#[test]
fn budget_and_invalid_input_do_not_mutate_committed_content() {
    let mut editor = Editor::default();
    dab(&mut editor, 1.0);
    assert_eq!(
        editor.begin(Tool::Brush, p(f32::NAN, 0.0)),
        Err(EditError::InvalidPoint)
    );
    editor.begin(Tool::Polyline, p(0.0, 0.0)).unwrap();
    for x in 1..MAX_POINTS_PER_STROKE {
        editor.extend(p(x as f32, 0.0)).unwrap();
    }
    assert_eq!(
        editor.extend(p(70_000.0, 0.0)),
        Err(EditError::BudgetExceeded)
    );
    editor.cancel();
    assert_eq!(editor.retained_points, 1);
    assert_eq!(editor.elements().len(), 1);
}
