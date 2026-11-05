//! 真实 D2D 命中与 Canvas 选择版本测试，不以包围框代替空心形状内部判断。
use super::super::{canvas::Canvas, image::Image};
use super::*;
use crate::stroke_style::{Color, StrokeStyle};

fn add(editor: &mut Editor, tool: Tool, a: Point, b: Point) {
    editor.begin(tool, a).unwrap();
    editor.extend(b).unwrap();
    assert!(editor.commit());
}
fn p(x: f32, y: f32) -> Point {
    Point { x, y }
}

#[test]
fn paintbrush_hit_follows_tapered_envelope_not_full_width_centerline() {
    unsafe {
        let factory: ID2D1Factory =
            D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).unwrap();
        let mut editor = Editor::default();
        editor.set_style(StrokeStyle::new(8.0, Color(255, 0, 0)).unwrap());
        add(&mut editor, Tool::Paintbrush, p(4.0, 32.0), p(60.0, 32.0));
        assert_eq!(
            pick(&factory, &editor, p(32.0, 34.0), 0.1).unwrap(),
            Some(0)
        );
        assert_eq!(pick(&factory, &editor, p(59.0, 35.0), 0.1).unwrap(), None);
    }
}

#[test]
fn optional_line_arrow_selects_head_only_when_enabled() {
    unsafe {
        let factory: ID2D1Factory =
            D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).unwrap();
        let mut editor = Editor::default();
        editor.set_style(
            StrokeStyle::new(4.0, Color(255, 0, 0))
                .unwrap()
                .with_arrow(true),
        );
        add(&mut editor, Tool::Line, p(4.0, 32.0), p(60.0, 32.0));
        assert_eq!(
            pick(&factory, &editor, p(40.0, 26.0), 0.1).unwrap(),
            Some(0)
        );
        editor.restyle(0, editor.style().with_arrow(false));
        assert_eq!(pick(&factory, &editor, p(40.0, 26.0), 0.1).unwrap(), None);
    }
}

#[test]
fn curved_arrow_head_participates_in_native_selection() {
    unsafe {
        let factory: ID2D1Factory =
            D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).unwrap();
        let mut editor = Editor::default();
        editor.set_style(editor.style().with_arrow(true));
        add(&mut editor, Tool::Bezier, p(8.0, 16.0), p(56.0, 16.0));
        let head = crate::bezier_arrow::build(&editor.elements()[0].points, 3.0)
            .unwrap()
            .head;
        let point = p(
            head.iter().map(|p| p.x).sum::<f32>() / 3.0,
            head.iter().map(|p| p.y).sum::<f32>() / 3.0,
        );
        assert_eq!(pick(&factory, &editor, point, 0.1).unwrap(), Some(0));
    }
}

#[test]
fn bezier_hit_uses_curve_not_control_polygon_or_chord() {
    unsafe {
        let factory: ID2D1Factory =
            D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).unwrap();
        let mut editor = Editor::default();
        add(&mut editor, Tool::Bezier, p(10.0, 10.0), p(70.0, 10.0));
        assert_eq!(
            pick(&factory, &editor, p(40.0, 21.25), 0.1).unwrap(),
            Some(0)
        );
        assert_eq!(pick(&factory, &editor, p(40.0, 10.0), 0.1).unwrap(), None);
        assert_eq!(pick(&factory, &editor, p(30.0, 25.0), 0.1).unwrap(), None);
    }
}

#[test]
fn native_geometry_distinguishes_hollow_fill_and_topmost_order() {
    unsafe {
        let factory: ID2D1Factory =
            D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).unwrap();
        for tool in [
            Tool::Rectangle,
            Tool::RoundedRectangle,
            Tool::Ellipse,
            Tool::Triangle,
            Tool::Polygon,
        ] {
            let mut editor = Editor::default();
            add(&mut editor, tool, p(10.0, 10.0), p(90.0, 90.0));
            assert_eq!(
                pick(&factory, &editor, p(50.0, 50.0), 1.0).unwrap(),
                None,
                "{tool:?}"
            );
            editor.set_style(editor.style().with_fill(Some(Color(255, 0, 0))));
            add(&mut editor, tool, p(10.0, 10.0), p(90.0, 90.0));
            assert_eq!(
                pick(&factory, &editor, p(50.0, 50.0), 1.0).unwrap(),
                Some(1),
                "{tool:?}"
            );
        }
    }
}

#[test]
fn arrow_head_and_brush_dab_are_selectable_but_transparent_strokes_are_not() {
    unsafe {
        let factory: ID2D1Factory =
            D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).unwrap();
        let mut editor = Editor::default();
        add(&mut editor, Tool::Arrow, p(4.0, 32.0), p(60.0, 32.0));
        assert_eq!(
            pick(&factory, &editor, p(40.0, 26.0), 0.1).unwrap(),
            Some(0)
        );
        add(&mut editor, Tool::Brush, p(20.0, 20.0), p(20.0, 20.0));
        assert_eq!(
            pick(&factory, &editor, p(20.0, 20.0), 0.1).unwrap(),
            Some(1)
        );
        editor.restyle(1, StrokeStyle::default().with_opacity(0.0).unwrap());
        assert_eq!(pick(&factory, &editor, p(20.0, 20.0), 0.1).unwrap(), None);
    }
}

#[test]
fn selection_targets_clicked_annotation_and_invalidates_after_history_change() {
    let mut canvas = Canvas::new(Some(Image {
        width: 100,
        height: 100,
        pixels: vec![255; 40000].into(),
    }))
    .unwrap();
    add(
        &mut canvas.editor,
        Tool::Brush,
        p(20.0, 20.0),
        p(20.0, 20.0),
    );
    add(
        &mut canvas.editor,
        Tool::Brush,
        p(70.0, 70.0),
        p(70.0, 70.0),
    );
    canvas.select_tool(Tool::Select);
    canvas.viewport.scale = 2.0;
    canvas.pointer_down(p(60.0, 60.0));
    canvas.pointer_up(p(60.0, 60.0));
    assert_eq!(canvas.selected_index(), Some(0));
    canvas
        .editor
        .set_style(canvas.editor.style().with_color(Color(0, 0, 255)));
    assert!(canvas.restyle_target());
    assert_eq!(canvas.selected_index(), Some(0));
    assert_eq!(canvas.editor.elements()[0].color, Color(0, 0, 255));
    canvas.editor.undo();
    assert_eq!(canvas.selected_index(), None);
    assert!(
        !canvas.delete_target(),
        "stale index must not delete an unrelated element"
    );
    canvas.pointer_down(p(60.0, 60.0));
    canvas.pointer_up(p(60.0, 60.0));
    assert!(canvas.delete_target());
    assert_eq!(canvas.editor.elements()[0].points[0], p(70.0, 70.0));
}

#[test]
fn drag_previews_without_document_mutation_and_release_commits_once() {
    let mut canvas = Canvas::new(Some(Image {
        width: 100,
        height: 100,
        pixels: vec![255; 40000].into(),
    }))
    .unwrap();
    add(
        &mut canvas.editor,
        Tool::Brush,
        p(20.0, 20.0),
        p(20.0, 20.0),
    );
    canvas.select_tool(Tool::Select);
    canvas.viewport.scale = 2.0;
    let revision = canvas.editor.revision();
    canvas.pointer_down(p(60.0, 60.0));
    canvas.pointer_move(p(80.0, 70.0));
    assert_eq!(canvas.editor.elements()[0].points[0], p(20.0, 20.0));
    assert_eq!(canvas.editor.revision(), revision);
    assert_eq!(
        canvas.editor.export_snapshot().elements()[0].points[0],
        p(20.0, 20.0)
    );
    canvas.pointer_up(p(100.0, 80.0));
    assert_eq!(canvas.editor.elements()[0].points[0], p(40.0, 30.0));
    canvas.editor.undo();
    assert_eq!(canvas.editor.elements()[0].points[0], p(20.0, 20.0));
    canvas.pointer_down(p(60.0, 60.0));
    canvas.pointer_move(p(100.0, 100.0));
    assert!(canvas.cancel_gesture());
    canvas.pointer_up(p(120.0, 120.0));
    assert_eq!(canvas.editor.elements()[0].points[0], p(20.0, 20.0));
    assert!(canvas.editor.redo(), "cancelled preview preserves redo");
}

#[test]
fn rotated_geometry_uses_inverse_pointer_transform() {
    unsafe {
        let factory: ID2D1Factory =
            D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).unwrap();
        let mut editor = Editor::default();
        editor.set_style(editor.style().with_fill(Some(Color(1, 2, 3))));
        add(&mut editor, Tool::Rectangle, p(12.0, 24.0), p(52.0, 40.0));
        editor.rotate(0, 90.0).unwrap();
        assert_eq!(
            pick(&factory, &editor, p(32.0, 16.0), 0.1).unwrap(),
            Some(0)
        );
        assert_eq!(pick(&factory, &editor, p(16.0, 32.0), 0.1).unwrap(), None);
    }
}

#[test]
fn duplicate_selects_new_topmost_annotation_and_requires_valid_selection() {
    let mut canvas = Canvas::new(Some(Image {
        width: 100,
        height: 100,
        pixels: vec![255; 40000].into(),
    }))
    .unwrap();
    add(
        &mut canvas.editor,
        Tool::Brush,
        p(20.0, 20.0),
        p(20.0, 20.0),
    );
    canvas.select_tool(Tool::Select);
    assert!(!canvas.duplicate_selected());
    canvas.pointer_down(p(40.0, 40.0));
    canvas.pointer_up(p(40.0, 40.0));
    assert!(canvas.duplicate_selected());
    assert_eq!(canvas.selected_index(), Some(1));
    assert_eq!(canvas.editor.elements()[1].points[0], p(32.0, 32.0));
    canvas.editor.undo();
    assert!(!canvas.duplicate_selected());
    assert_eq!(canvas.editor.elements().len(), 1);
}

#[test]
fn selection_order_matches_shared_highlighter_render_layer() {
    unsafe {
        let factory: ID2D1Factory =
            D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).unwrap();
        let mut editor = Editor::default();
        add(&mut editor, Tool::Brush, p(20.0, 20.0), p(20.0, 20.0));
        add(&mut editor, Tool::Highlighter, p(20.0, 20.0), p(20.0, 20.0));
        assert_eq!(
            pick(&factory, &editor, p(20.0, 20.0), 1.0).unwrap(),
            Some(0)
        );
        editor.remove(0);
        assert_eq!(
            pick(&factory, &editor, p(20.0, 20.0), 1.0).unwrap(),
            Some(0)
        );
        assert_eq!(editor.elements()[0].tool, Tool::Highlighter);
    }
}
