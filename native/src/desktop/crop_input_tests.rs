//! 裁剪后的输入仍存储源坐标；不同缩放/平移下拒绝框外起点并夹紧拖动终点。
use super::*;
use crate::crop_geometry::Rect;

#[test]
fn reset_fractional_document_rejects_buffer_padding_and_clamps_in_logical_space() {
    let mut canvas = Canvas::new(Some(crate::desktop::image::Image {
        width: 8,
        height: 4,
        pixels: vec![255; 128].into(),
    }))
    .unwrap();
    canvas.editor = std::mem::take(&mut canvas.editor)
        .with_restored_crop(crate::crop_geometry::Crop {
            rect: Rect {
                x: 1.0,
                y: 0.5,
                width: 2.25,
                height: 1.25,
            },
            source: crate::crop_geometry::Size {
                width: 7.25,
                height: 3.25,
            },
        })
        .unwrap();
    assert!(canvas.editor.reset_crop());
    for scale in [0.5, 1.0, 1.5, 2.0] {
        canvas.viewport.scale = scale;
        canvas.viewport.origin = Point { x: -10.0, y: 25.0 };
        let screen = |x, y| Point {
            x: -10.0 + x * scale,
            y: 25.0 + y * scale,
        };
        assert!(canvas.image_point(screen(7.5, 2.0), false).is_none());
        assert!(canvas.image_point(screen(2.0, 3.5), false).is_none());
        assert_eq!(
            canvas.image_point(screen(7.5, 3.5), true),
            Some(Point { x: 7.25, y: 3.25 })
        );
        assert_eq!(
            canvas.image_point(screen(7.25, 3.25), false),
            Some(Point { x: 7.25, y: 3.25 })
        );
    }
}

#[test]
fn crop_pointer_commit_uses_main_four_unit_minimum_on_both_axes() {
    for (width, height, expected) in [(3.9, 8.0, false), (8.0, 3.9, false), (4.0, 4.0, true)] {
        let mut canvas = Canvas::new(Some(crate::desktop::image::Image {
            width: 32,
            height: 32,
            pixels: vec![255; 32 * 32 * 4].into(),
        }))
        .unwrap();
        canvas.viewport.origin = Point::default();
        canvas.select_tool(Tool::Select);
        canvas.crop_mode = true;
        canvas.pointer_down(Point { x: 2.0, y: 2.0 });
        canvas.pointer_up(Point {
            x: 2.0 + width,
            y: 2.0 + height,
        });
        assert_eq!(canvas.editor.crop_view().is_some(), expected);
        assert_eq!(canvas.editor.revision(), u64::from(expected));
    }
}

#[test]
fn crop_drag_reversed_repeat_cancel_and_stale_document() {
    let mut canvas = Canvas::new(Some(crate::desktop::image::Image {
        width: 64,
        height: 32,
        pixels: vec![255; 64 * 32 * 4].into(),
    }))
    .unwrap();
    canvas.viewport.origin = Point::default();
    canvas.select_tool(Tool::Select);
    canvas.crop_mode = true;
    let revision = canvas.editor.revision();
    assert!(canvas.pointer_down(Point { x: 50.0, y: 25.0 }));
    assert!(canvas.pointer_move(Point { x: 10.0, y: 5.0 }));
    assert!(canvas.editor.crop_view().is_none());
    assert_eq!(canvas.editor.revision(), revision);
    assert!(canvas.pointer_up(Point { x: 10.0, y: 5.0 }));
    assert_eq!(
        canvas.editor.crop_view().unwrap().source.rect,
        Rect {
            x: 10.0,
            y: 5.0,
            width: 40.0,
            height: 20.0
        }
    );
    assert!(canvas.crop_drag.is_none());
    assert!(canvas.selection_drag.is_none());
    let first = canvas.editor.crop_view();
    assert!(canvas.pointer_down(Point { x: 15.0, y: 10.0 }));
    assert!(canvas.pointer_up(Point { x: 35.0, y: 20.0 }));
    assert_eq!(
        canvas.editor.crop_view().unwrap().source.rect,
        Rect {
            x: 15.0,
            y: 10.0,
            width: 20.0,
            height: 10.0
        }
    );
    assert!(canvas.editor.undo());
    assert_eq!(canvas.editor.crop_view(), first);
    assert!(canvas.pointer_down(Point { x: 15.0, y: 10.0 }));
    assert!(canvas.cancel_gesture());
    assert!(canvas.crop_drag.is_none());
    assert_eq!(canvas.editor.crop_view(), first);
    assert!(canvas.editor.redo());
    let point = Point { x: 20.0, y: 15.0 };
    assert!(canvas.pointer_down(point));
    let revision = canvas.editor.revision();
    assert!(canvas.pointer_up(point)); // 零面积不提交。
    assert_eq!(canvas.editor.revision(), revision);
    assert!(canvas.pointer_down(point));
    canvas.editor.undo();
    canvas.pointer_move(Point { x: 30.0, y: 20.0 });
    assert!(canvas.crop_drag.is_none());
    assert_eq!(canvas.editor.crop_view(), first);
    canvas.select_tool(Tool::Brush);
    assert!(!canvas.crop_mode);
}

#[test]
fn cropped_pointer_mapping_preserves_source_coordinates_and_reset_restores_access() {
    for scale in [0.5, 1.0, 2.0] {
        let mut canvas = Canvas::new(Some(crate::desktop::image::Image {
            width: 64,
            height: 32,
            pixels: vec![255; 64 * 32 * 4].into(),
        }))
        .unwrap();
        canvas
            .editor
            .crop_to(Rect {
                x: 10.0,
                y: 5.0,
                width: 20.0,
                height: 15.0,
            })
            .unwrap();
        canvas.viewport.scale = scale;
        canvas.viewport.origin = Point {
            x: 40.0 - 10.0 * scale,
            y: 30.0 - 5.0 * scale,
        };
        let screen = |x: f32, y: f32| Point {
            x: 40.0 + (x - 10.0) * scale,
            y: 30.0 + (y - 5.0) * scale,
        };
        assert_eq!(
            canvas.image_point(screen(15.0, 10.0), false),
            Some(Point { x: 15.0, y: 10.0 })
        );
        assert!(canvas.image_point(screen(9.0, 10.0), false).is_none());
        assert!(canvas.image_point(screen(20.0, 21.0), false).is_none());
        assert_eq!(
            canvas.image_point(screen(40.0, -2.0), true),
            Some(Point { x: 30.0, y: 5.0 })
        );
        assert!(canvas
            .image_point(
                Point {
                    x: f32::NAN,
                    y: 0.0
                },
                true
            )
            .is_none());
        canvas.select_tool(Tool::Brush);
        assert!(!canvas.pointer_down(screen(9.0, 10.0)));
        assert!(canvas.pointer_down(screen(15.0, 10.0)));
        assert!(canvas.pointer_up(screen(20.0, 15.0)));
        assert_eq!(
            canvas.editor.elements()[0].points[0],
            Point { x: 15.0, y: 10.0 }
        );
        assert!(canvas.editor.reset_crop());
        assert_eq!(
            canvas.image_point(screen(9.0, 10.0), false),
            Some(Point { x: 9.0, y: 10.0 })
        );
        assert!(canvas.editor.undo());
        assert!(canvas.image_point(screen(9.0, 10.0), false).is_none());
    }
}
