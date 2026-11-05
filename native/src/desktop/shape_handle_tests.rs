//! 四角编辑使用真实 Canvas 输入，旋转后仍固定对角，预览不写文档。
use super::super::image::Image;
use super::*;
fn p(x: f32, y: f32) -> Point {
    Point { x, y }
}
fn world(stroke: &Stroke, handle: usize) -> Point {
    rotation::rotate(
        stroke.shape_handles().unwrap()[handle],
        rotation::center(&stroke.points),
        stroke.rotation,
    )
}
#[test]
fn all_shape_corners_preview_cancel_commit_and_undo_at_different_rotations() {
    for tool in [
        Tool::Rectangle,
        Tool::RoundedRectangle,
        Tool::Ellipse,
        Tool::Triangle,
        Tool::Polygon,
    ] {
        for angle in [0.0, 45.0, 90.0] {
            for handle in 0..4 {
                let mut canvas = Canvas::new(Some(Image {
                    width: 200,
                    height: 200,
                    pixels: vec![255; 200 * 200 * 4].into(),
                }))
                .unwrap();
                canvas.viewport.origin = Point::default();
                canvas.editor.begin(tool, p(40.0, 40.0)).unwrap();
                canvas.editor.extend(p(120.0, 120.0)).unwrap();
                canvas.editor.commit();
                canvas.editor.rotate(0, angle).unwrap();
                canvas.select_tool(Tool::Select);
                canvas.selected = Some((0, canvas.editor.revision()));
                let before = canvas.editor.elements()[0].clone();
                let start = world(&before, handle);
                let target = p(start.x + 7.0, start.y + 9.0);
                canvas.pointer_down(start);
                assert!(canvas.path_drag.is_some());
                canvas.pointer_move(target);
                assert_eq!(canvas.editor.elements()[0].points, before.points);
                let preview = &canvas.path_drag.as_ref().unwrap().preview;
                let fixed = world(preview, 3 - handle);
                let old_fixed = world(&before, 3 - handle);
                assert!((fixed.x - old_fixed.x).hypot(fixed.y - old_fixed.y) < 0.001);
                canvas.cancel_gesture();
                assert_eq!(canvas.editor.elements()[0].points, before.points);
                canvas.pointer_down(start);
                canvas.pointer_up(target);
                let moved = world(&canvas.editor.elements()[0], handle);
                assert!((moved.x - target.x).hypot(moved.y - target.y) < 0.001);
                assert!(canvas.editor.undo());
                assert_eq!(canvas.editor.elements()[0].points, before.points);
                assert!(canvas.editor.redo());
                canvas.selected = Some((0, canvas.editor.revision()));
                let point = world(&canvas.editor.elements()[0], handle);
                let revision = canvas.editor.revision();
                canvas.pointer_down(point);
                canvas.pointer_up(point);
                assert_eq!(canvas.editor.revision(), revision);
            }
        }
    }
}
