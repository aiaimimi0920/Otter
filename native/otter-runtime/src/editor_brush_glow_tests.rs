//! Ordinary brush glow sampling provenance lives only until commit; history/sessions reuse existing Highlighter.
use super::*;
fn p(x: f32, y: f32) -> Point {
    Point { x, y }
}
#[test]
fn glow_keeps_brush_sampling_and_smooths_once_without_changing_independent_highlighter() {
    let mut editor = Editor::default();
    editor.set_style(editor.style().with_opacity(0.5).unwrap());
    assert!(editor.begin_brush_highlighter(p(0., 0.)).unwrap());
    assert_eq!(editor.draft().unwrap().tool, Tool::Highlighter);
    assert!(!editor.extend(p(0.5, 0.)).unwrap());
    editor.extend(p(10., 10.)).unwrap();
    editor.extend(p(20., 0.)).unwrap();
    assert_eq!(editor.draft().unwrap().points[1], p(10., 10.));
    assert!(editor.commit_smoothed(100.));
    let stroke = &editor.elements()[0];
    assert_eq!(stroke.tool, Tool::Highlighter);
    assert_eq!(stroke.opacity, 0.5);
    assert_eq!(stroke.points[1], p(10., 1.25));
    let points = stroke.points.clone();
    assert!(editor.undo());
    assert!(!editor.undo());
    assert!(editor.redo());
    assert!(!editor.redo());
    assert_eq!(editor.elements()[0].points, points);
    editor.begin(Tool::Highlighter, p(0., 0.)).unwrap();
    editor.extend(p(10., 10.)).unwrap();
    editor.extend(p(20., 0.)).unwrap();
    assert!(editor.commit_smoothed(100.));
    assert_eq!(editor.elements()[1].points[1], p(10., 10.));
}
#[test]
fn glow_uses_the_brush_point_budget_and_straight_constraint() {
    let mut editor = Editor::default();
    editor.begin_brush_highlighter(p(0., 0.)).unwrap();
    for x in 1..=2048 {
        editor.extend(p(x as f32, 0.)).unwrap();
    }
    let draft = editor.draft().unwrap();
    assert_eq!(draft.points.len(), 1025);
    assert_eq!(draft.points[1], p(2., 0.));
    assert_eq!(draft.points.last(), Some(&p(2048., 0.)));
    editor.extend_constrained(p(3000., 10.), true).unwrap();
    assert_eq!(editor.draft().unwrap().points, [p(0., 0.), p(3000., 10.)]);
    editor.extend(p(3010., 20.)).unwrap();
    assert_eq!(editor.draft().unwrap().points.len(), 3);
}
#[test]
fn rejected_begin_does_not_relabel_existing_draft_and_cancel_clears_sampling_origin() {
    let mut editor = Editor::default();
    editor.begin(Tool::Line, p(0., 0.)).unwrap();
    assert!(!editor.begin_brush_highlighter(p(1., 1.)).unwrap());
    assert_eq!(editor.draft().unwrap().tool, Tool::Line);
    editor.cancel();
    assert!(editor.begin_brush_highlighter(p(0., 0.)).unwrap());
    assert!(!editor.export_snapshot().draft_brush_highlighter);
    assert!(editor.cancel());
    editor.begin(Tool::Highlighter, p(0., 0.)).unwrap();
    editor.extend(p(10., 10.)).unwrap();
    editor.extend(p(20., 0.)).unwrap();
    editor.commit_smoothed(100.);
    assert_eq!(editor.elements()[0].points[1], p(10., 10.));
    assert!(!editor.draft_brush_highlighter);
}
