//! 与主干指数缩放、组中心/各自中心及单次历史的纯模型对照。
use super::*;

fn canvas() -> Canvas {
    let mut c = Canvas::new(Some(crate::image_decode::Image {
        width: 100,
        height: 100,
        pixels: vec![255; 40000].into(),
    }))
    .unwrap();
    c.pinned = true;
    for x in [10.0, 50.0] {
        c.editor
            .begin(Tool::Rectangle, Point { x, y: 10.0 })
            .unwrap();
        c.editor
            .extend(Point {
                x: x + 20.0,
                y: 30.0,
            })
            .unwrap();
        assert!(c.editor.commit());
    }
    c.select_tool(Tool::Select);
    let revision = c.editor.revision();
    c.selected = Some((0, revision));
    c.multi_selection = Some((vec![0, 1], revision));
    c
}
fn close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.002, "{actual} != {expected}");
}
fn apply(c: &mut Canvas, delta: i16, own: bool) {
    assert!(prepare(c, delta, own).unwrap());
    let plan = c.scale_pending.take().unwrap();
    assert!(c.editor.commit_resize(plan).unwrap());
    let revision = c.editor.revision();
    c.selected = Some((0, revision));
    c.multi_selection = Some((vec![0, 1], revision));
}

#[test]
fn group_and_own_centers_match_main_without_overwriting_toolbar_preferences() {
    for own in [false, true] {
        let mut c = canvas();
        c.transform_own = !own;
        c.scale_axis = crate::transform_gesture::Axis::X;
        let before = c.editor.elements().to_vec();
        let revision = c.editor.revision();
        let stamp = c.editor.document_stamp();
        assert!(prepare(&mut c, 120, own).unwrap());
        assert!(stamp.matches(&c.editor));
        assert!(c
            .editor
            .commit_resize(c.scale_pending.take().unwrap())
            .unwrap());
        assert_eq!(c.editor.revision(), revision + 1);
        let factor = 0.12f32.exp();
        for (i, stroke) in c.editor.elements().iter().enumerate() {
            let original_center = 20.0 + i as f32 * 40.0;
            let center = if own {
                original_center
            } else {
                40.0 + (original_center - 40.0) * factor
            };
            close(stroke.points[0].x, center - 10.0 * factor);
            close(stroke.points[0].y, 20.0 - 10.0 * factor);
            close(stroke.width, before[i].width * factor);
        }
        assert_eq!(c.transform_own, !own);
        assert_eq!(c.scale_axis, crate::transform_gesture::Axis::X);
        assert!(c.selection_enabled);
        assert!(c.editor.undo());
        assert_eq!(c.editor.elements()[0].points, before[0].points);
        assert_eq!(c.editor.elements()[1].style(), before[1].style());
        assert!(c.editor.redo());
    }
}

#[test]
fn fractional_wheel_accumulates_and_inverse_restores_geometry_without_step_rounding() {
    let mut fine = canvas();
    let mut coarse = canvas();
    for _ in 0..10 {
        apply(&mut fine, 1, false);
    }
    apply(&mut coarse, 10, false);
    for (fine, coarse) in fine.editor.elements().iter().zip(coarse.editor.elements()) {
        for (fine, coarse) in fine.points.iter().zip(&coarse.points) {
            close(fine.x, coarse.x);
            close(fine.y, coarse.y);
        }
    }
    apply(&mut coarse, -10, false);
    close(coarse.editor.elements()[0].points[0].x, 10.0);
    close(coarse.editor.elements()[1].points[1].y, 30.0);
    for (delta, factor) in [(i16::MAX, 1.5), (i16::MIN, 0.5)] {
        let mut c = canvas();
        apply(&mut c, delta, false);
        close(
            c.editor.elements()[0].points[1].x - c.editor.elements()[0].points[0].x,
            20.0 * factor,
        );
    }
}

#[test]
fn empty_stale_non_select_and_active_gestures_never_prepare_or_cancel_other_input() {
    for case in 0..9 {
        let mut c = canvas();
        match case {
            0 => c.pinned = false,
            1 => {
                c.select_tool(Tool::Pan);
            }
            2 => {
                c.selected = None;
                c.multi_selection = None;
            }
            3 => {
                c.selected = Some((0, 0));
                c.multi_selection = None;
            }
            4 => {
                c.selection_drag = Some((Point::default(), Point::default()));
            }
            5 => {
                assert!(c.begin_rotation_at(Point { x: 50.0, y: 40.0 }));
            }
            6 => {
                assert!(c.begin_scale_at(Point { x: 50.0, y: 40.0 }));
            }
            7 => c.crop_mode = true,
            _ => {
                c.editor.begin(Tool::Line, Point::default()).unwrap();
            }
        };
        let stamp = c.editor.document_stamp();
        assert!(!prepare(&mut c, 120, false).unwrap());
        assert!(c.scale_pending.is_none());
        assert!(stamp.matches(&c.editor));
        assert_eq!(c.editor.draft().is_some(), case == 8);
    }
    let mut c = canvas();
    assert!(!prepare(&mut c, 0, false).unwrap());
    assert!(prepare(&mut c, 120, false).unwrap());
    assert!(!prepare(&mut c, 120, true).unwrap());
}

#[test]
fn text_and_serial_keep_layout_and_labels_in_the_same_resize_transaction() {
    let mut c = canvas();
    c.editor
        .insert_text(
            Point { x: 10.0, y: 50.0 },
            Point { x: 40.0, y: 20.0 },
            crate::text_annotation::TextAnnotation::new("测试", "微软雅黑", 16.0, false).unwrap(),
        )
        .unwrap();
    c.editor
        .insert_serial(Point { x: 75.0, y: 75.0 }, 10.0, "微软雅黑")
        .unwrap();
    let revision = c.editor.revision();
    c.selected = Some((2, revision));
    c.multi_selection = Some((vec![2, 3], revision));
    let before = c.editor.elements().to_vec();
    assert!(prepare(&mut c, 120, true).unwrap());
    assert!(c
        .editor
        .commit_resize(c.scale_pending.take().unwrap())
        .unwrap());
    for i in [2, 3] {
        close(
            c.editor.elements()[i].text.as_ref().unwrap().font_size(),
            before[i].text.as_ref().unwrap().font_size() * 0.12f32.exp(),
        );
        assert_eq!(
            c.editor.elements()[i].text.as_ref().unwrap().content(),
            before[i].text.as_ref().unwrap().content()
        );
    }
    assert!(c.editor.undo());
    assert_eq!(c.editor.elements()[2].points, before[2].points);
    assert_eq!(c.editor.elements()[3].text, before[3].text);
}

#[test]
fn bounded_session_preserves_original_delta_center_and_fifo_when_full() {
    let c = canvas();
    let mut session = Session::default();
    session.begin(&c);
    for i in 0..LIMIT {
        session
            .push(Intent {
                delta: i as i16 - 2048,
                own: i % 2 == 0,
            })
            .unwrap();
    }
    assert!(matches!(
        session.push(Intent {
            delta: 120,
            own: false
        }),
        Err(EditError::BudgetExceeded)
    ));
    let batch = session.0.as_mut().unwrap();
    assert_eq!(batch.pending.len(), LIMIT);
    for i in 0..LIMIT {
        assert_eq!(
            batch.pending.pop_front(),
            Some(Intent {
                delta: i as i16 - 2048,
                own: i % 2 == 0
            })
        );
    }
    assert!(batch.pending.is_empty());
}

#[test]
fn session_requires_the_same_document_selection_and_select_mode() {
    for case in 0..5 {
        let mut c = canvas();
        let mut session = Session::default();
        session.begin(&c);
        assert!(session.matches(&c));
        match case {
            0 => c.multi_selection = None,
            1 => {
                c.select_tool(Tool::Pan);
            }
            2 => {
                c.select_tool(Tool::Rectangle);
            }
            3 => {
                assert!(c.editor.undo());
            }
            _ => c.editor = c.editor.export_snapshot(),
        }
        assert!(!session.matches(&c), "case {case}");
    }
}
