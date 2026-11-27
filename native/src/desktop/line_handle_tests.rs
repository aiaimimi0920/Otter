//! Line/arrow endpoints reuse path handles; rotation, previews, and undo all use real Canvas.
use super::super::image::Image;
use super::*;
fn p(x: f32, y: f32) -> Point {
    Point { x, y }
}

#[test]
fn polyline_creation_and_middle_vertex_edit_use_native_input() {
    let mut canvas = Canvas::new(Some(Image {
        width: 100,
        height: 100,
        pixels: vec![255; 100 * 100 * 4].into(),
    }))
    .unwrap();
    canvas.viewport.origin = Point::default();
    canvas.select_tool(Tool::Polyline);
    canvas.pointer_down(p(10.0, 10.0));
    canvas.pointer_move_modified(p(40.0, 20.0), true, true);
    canvas.pointer_move(p(60.0, 50.0));
    canvas.pointer_up(p(70.0, 60.0));
    assert_eq!(
        canvas.editor.elements()[0].points,
        vec![p(10.0, 10.0), p(40.0, 20.0), p(60.0, 50.0), p(70.0, 60.0)]
    );
    canvas.select_tool(Tool::Select);
    canvas.selected = Some((0, canvas.editor.revision()));
    canvas.pointer_down(p(40.0, 20.0));
    assert!(canvas.path_drag.is_some());
    canvas.pointer_move(p(45.0, 25.0));
    assert_eq!(canvas.editor.elements()[0].points[1], p(40.0, 20.0));
    canvas.pointer_up(p(45.0, 25.0));
    assert_eq!(canvas.editor.elements()[0].points[1], p(45.0, 25.0));
    canvas.editor.undo();
    assert_eq!(canvas.editor.elements()[0].points[1], p(40.0, 20.0));
}
fn world(stroke: &Stroke, index: usize) -> Point {
    rotation::rotate(
        stroke.points[index],
        rotation::center(&stroke.points),
        stroke.rotation,
    )
}

#[test]
fn close_serial_tip_drag_keeps_circle_fixed() {
    let mut c = Canvas::new(Some(Image {
        width: 100,
        height: 100,
        pixels: vec![255; 40000].into(),
    }))
    .unwrap();
    c.viewport.origin = Point::default();
    c.viewport.scale = 1.;
    c.editor
        .begin_serial(p(40., 40.), 10., "Arial", None)
        .unwrap();
    c.editor.extend_constrained(p(42., 40.), true).unwrap();
    assert!(c.editor.commit());
    let circle = c.editor.elements()[0].points.clone();
    c.select_tool(Tool::Select);
    c.selected = Some((0, c.editor.revision()));
    c.pointer_down(p(42., 40.));
    c.pointer_move(p(60., 42.));
    c.pointer_up(p(60., 42.));
    assert_eq!(c.editor.elements()[0].points, circle);
    assert_eq!(
        c.editor.elements()[0].serial_handles().unwrap()[1],
        p(60., 42.)
    );
    c.editor.undo();
    assert_eq!(
        c.editor.elements()[0].serial_handles().unwrap()[1],
        p(42., 40.)
    );
}

#[test]
fn line_and_arrow_endpoints_preview_and_commit_without_moving_other_end() {
    for tool in [Tool::Line, Tool::Arrow] {
        for angle in [0.0, 45.0, 90.0] {
            for handle in 0..2 {
                let mut canvas = Canvas::new(Some(Image {
                    width: 200,
                    height: 200,
                    pixels: vec![255; 200 * 200 * 4].into(),
                }))
                .unwrap();
                canvas.viewport.origin = Point::default();
                canvas.editor.begin(tool, p(20.0, 20.0)).unwrap();
                canvas.editor.extend(p(80.0, 40.0)).unwrap();
                canvas.editor.commit();
                canvas.editor.rotate(0, angle).unwrap();
                canvas.select_tool(Tool::Select);
                canvas.selected = Some((0, canvas.editor.revision()));
                let before = canvas.editor.elements()[0].clone();
                let start = world(&before, handle);
                let target = p(start.x + 15.0, start.y + 7.0);
                canvas.pointer_down(start);
                assert!(canvas.path_drag.is_some());
                canvas.pointer_move(target);
                assert_eq!(canvas.editor.elements()[0].points, before.points);
                let preview = &canvas.path_drag.as_ref().unwrap().preview;
                let other = world(preview, 1 - handle);
                let old_other = world(&before, 1 - handle);
                assert!((other.x - old_other.x).hypot(other.y - old_other.y) < 0.001);
                canvas.pointer_up(target);
                let end = world(&canvas.editor.elements()[0], handle);
                assert!((end.x - target.x).hypot(end.y - target.y) < 0.001);
                canvas.editor.undo();
                assert_eq!(canvas.editor.elements()[0].points, before.points);
                canvas.editor.redo();
                canvas.selected = Some((0, canvas.editor.revision()));
                let revision = canvas.editor.revision();
                let start = world(&canvas.editor.elements()[0], handle);
                canvas.pointer_down(start);
                canvas.pointer_up(start);
                assert_eq!(canvas.editor.revision(), revision, "click without movement");
            }
        }
    }
}
