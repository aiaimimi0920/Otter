//! 控制柄使用真实 Canvas 输入与 D2D 工厂；不依赖用户窗口或图片。
use super::super::image::Image;
use super::*;

fn p(x: f32, y: f32) -> Point {
    Point { x, y }
}
fn canvas(angle: f32) -> Canvas {
    let mut canvas = Canvas::new(Some(Image {
        width: 200,
        height: 200,
        pixels: vec![255; 200 * 200 * 4].into(),
    }))
    .unwrap();
    canvas.editor.begin(Tool::Bezier, p(20.0, 20.0)).unwrap();
    canvas.editor.extend(p(80.0, 20.0)).unwrap();
    canvas.editor.commit();
    canvas.editor.rotate(0, angle).unwrap();
    canvas.select_tool(Tool::Select);
    canvas.selected = Some((0, canvas.editor.revision()));
    canvas
}
fn world(stroke: &Stroke, index: usize) -> Point {
    rotation::rotate(
        stroke.points[index],
        rotation::center(&stroke.points),
        stroke.rotation,
    )
}
fn screen(canvas: &Canvas, point: Point) -> Point {
    p(
        canvas.viewport.origin.x + point.x * canvas.viewport.scale,
        canvas.viewport.origin.y + point.y * canvas.viewport.scale,
    )
}
fn close(a: Point, b: Point) {
    assert!((a.x - b.x).hypot(a.y - b.y) < 0.001, "{a:?} != {b:?}");
}

#[test]
fn extension_handles_preview_both_ends_without_shifting_rotated_original() {
    for angle in [0.0, 45.0, 90.0] {
        for side in 0..2 {
            let mut canvas = canvas(angle);
            let source = canvas.editor.elements()[0].clone();
            let local = crate::bezier_geometry::extension_handles(&source.points).unwrap()[side];
            let visible = rotation::rotate(local, rotation::center(&source.points), angle);
            let start = screen(&canvas, visible);
            assert!(canvas.pointer_down(start));
            assert!(canvas.path_drag.is_some());
            let target = p(visible.x + 12.0, visible.y + 20.0);
            canvas.pointer_move(screen(&canvas, target));
            let preview = &canvas.path_drag.as_ref().unwrap().preview;
            assert_eq!(preview.points.len(), 7);
            let offset = if side == 0 { 3 } else { 0 };
            for i in 0..4 {
                close(world(preview, i + offset), world(&source, i));
            }
            close(world(preview, if side == 0 { 0 } else { 6 }), target);
            assert_eq!(canvas.editor.elements()[0].points, source.points);
            canvas.pointer_up(screen(&canvas, target));
            assert_eq!(canvas.editor.elements()[0].points.len(), 7);
            canvas.editor.undo();
            assert_eq!(canvas.editor.elements()[0].points, source.points);
            canvas.editor.redo();
            assert_eq!(canvas.editor.elements()[0].points.len(), 7);
        }
    }
}

#[test]
fn extension_click_without_drag_and_cancel_do_not_add_segment() {
    let mut canvas = canvas(0.0);
    let source = canvas.editor.elements()[0].clone();
    let handle = crate::bezier_geometry::extension_handles(&source.points).unwrap()[1];
    let start = screen(&canvas, handle);
    let revision = canvas.editor.revision();
    canvas.pointer_down(start);
    canvas.pointer_up(start);
    assert_eq!(canvas.editor.revision(), revision);
    canvas.pointer_down(start);
    canvas.pointer_move(p(start.x + 10.0, start.y + 10.0));
    canvas.cancel_gesture();
    assert_eq!(canvas.editor.elements()[0].points, source.points);
    assert_eq!(canvas.editor.revision(), revision);
}

#[test]
fn rotated_handle_preview_keeps_other_points_fixed_and_commits_once() {
    for angle in [0.0, 45.0, 90.0] {
        let mut canvas = canvas(angle);
        canvas.viewport.scale = 2.0;
        let source = canvas.editor.elements()[0].clone();
        let start = world(&source, 1);
        let target = p(start.x + 12.0, start.y + 8.0);
        let revision = canvas.editor.revision();
        assert!(canvas.pointer_down(screen(&canvas, start)));
        assert!(canvas.path_drag.is_some());
        canvas.pointer_move(screen(&canvas, target));
        let preview = &canvas.path_drag.as_ref().unwrap().preview;
        close(world(preview, 1), target);
        for i in [0, 2, 3] {
            close(world(preview, i), world(&source, i));
        }
        assert_eq!(canvas.editor.revision(), revision);
        assert_eq!(
            canvas.editor.export_snapshot().elements()[0].points,
            source.points
        );
        canvas.pointer_up(screen(&canvas, target));
        assert!(canvas.path_drag.is_none());
        close(world(&canvas.editor.elements()[0], 1), target);
        assert!(canvas.selected_index().is_some());
        canvas.editor.undo();
        assert_eq!(canvas.editor.elements()[0].points, source.points);
        canvas.editor.redo();
        close(world(&canvas.editor.elements()[0], 1), target);
    }
}

#[test]
fn cancellation_and_stale_revision_discard_handle_preview() {
    let mut canvas = canvas(0.0);
    let source = canvas.editor.elements()[0].clone();
    let start = screen(&canvas, world(&source, 1));
    canvas.pointer_down(start);
    canvas.pointer_move(p(start.x + 10.0, start.y + 20.0));
    assert!(canvas.cancel_gesture());
    assert_eq!(canvas.editor.elements()[0].points, source.points);
    canvas.pointer_down(start);
    canvas.editor.undo();
    canvas.pointer_up(p(start.x + 10.0, start.y + 20.0));
    assert!(canvas.path_drag.is_none());
    assert!(canvas.editor.elements().is_empty());
    assert!(canvas.editor.redo());
    assert_eq!(canvas.editor.elements()[0].points, source.points);
}
