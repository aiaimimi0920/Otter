//! 通用变换柄布局使用客户区 DIP，与绘制/命中共用；不改变文档的变换中心。
use crate::viewport::Point;
pub(super) fn logical_size(width: u32, height: u32, dpi: u32) -> Point {
    let scale = 96.0 / dpi.max(1) as f32;
    Point {
        x: width as f32 * scale,
        y: height as f32 * scale,
    }
}
pub(super) fn fit(point: Point, size: Option<Point>) -> Option<Point> {
    if !point.x.is_finite() || !point.y.is_finite() {
        return None;
    }
    let Some(size) = size else { return Some(point) };
    // 最小 24 DIP 保留命中半径与边缘间距；过小窗口隐藏，而非 clamp 产生倒置区间。
    if !size.x.is_finite() || !size.y.is_finite() || size.x < 24.0 || size.y < 24.0 {
        return None;
    }
    Some(Point {
        x: point.x.clamp(8.0, size.x - 8.0),
        y: point.y.clamp(8.0, size.y - 8.0),
    })
}
pub(super) fn separate(point: Point, other: Option<Point>, size: Option<Point>) -> Option<Point> {
    let Some(other) = other else {
        return Some(point);
    };
    let clear = |p: Point| (p.x - other.x).hypot(p.y - other.y) >= 16.0;
    if clear(point) {
        return Some(point);
    }
    [
        Point {
            x: point.x - 18.0,
            y: point.y,
        },
        Point {
            x: point.x,
            y: point.y - 18.0,
        },
        Point {
            x: point.x + 18.0,
            y: point.y,
        },
        Point {
            x: point.x,
            y: point.y + 18.0,
        },
    ]
    .into_iter()
    .filter_map(|p| fit(p, size))
    .find(|p| clear(*p))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn physical_sizes_convert_to_same_dip_layout_at_different_dpi() {
        for (width, height, dpi) in [(200, 100, 96), (300, 150, 144), (400, 200, 192)] {
            let size = logical_size(width, height, dpi);
            assert_eq!(size, Point { x: 200.0, y: 100.0 });
            assert_eq!(
                fit(Point { x: -20.0, y: 120.0 }, Some(size)),
                Some(Point { x: 8.0, y: 92.0 })
            );
            assert_eq!(
                fit(Point { x: 250.0, y: -30.0 }, Some(size)),
                Some(Point { x: 192.0, y: 8.0 })
            );
        }
    }
    #[test]
    fn tiny_or_invalid_client_never_exposes_unhittable_handles() {
        assert!(fit(Point::default(), Some(Point { x: 23.0, y: 100.0 })).is_none());
        assert!(fit(Point::default(), Some(Point { x: 100.0, y: 0.0 })).is_none());
        assert!(fit(
            Point {
                x: f32::NAN,
                y: 0.0
            },
            None
        )
        .is_none());
        assert_eq!(
            fit(Point { x: 4.0, y: 20.0 }, None),
            Some(Point { x: 4.0, y: 20.0 })
        );
    }
    #[test]
    fn offscreen_selection_handles_stay_distinct_and_hit_the_displayed_positions() {
        use crate::editor::Tool;
        let mut canvas = super::super::canvas::Canvas::new(Some(super::super::image::Image {
            width: 100,
            height: 100,
            pixels: vec![255; 100 * 100 * 4].into(),
        }))
        .unwrap();
        canvas.client_size = Some(Point { x: 100.0, y: 100.0 });
        canvas.viewport = crate::viewport::Viewport {
            origin: Point::default(),
            scale: 1.0,
        };
        canvas
            .editor
            .begin(Tool::Rectangle, Point { x: 200.0, y: 200.0 })
            .unwrap();
        canvas.editor.extend(Point { x: 220.0, y: 220.0 }).unwrap();
        canvas.editor.commit();
        canvas.select_tool(Tool::Select);
        canvas.selected = Some((0, canvas.editor.revision()));
        let rotation = super::super::rotation_handle::position(&canvas).unwrap();
        let scale = super::super::scale_handle::position(&canvas).unwrap();
        assert!((rotation.x - scale.x).hypot(rotation.y - scale.y) >= 16.0);
        for point in [rotation, scale] {
            assert!((8.0..=92.0).contains(&point.x));
            assert!((8.0..=92.0).contains(&point.y));
        }
        assert!(canvas.pointer_down(rotation));
        assert!(canvas.rotation_drag.is_some());
        canvas.cancel_gesture();
        assert!(canvas.pointer_down(scale));
        assert!(canvas.scale_drag.is_some());
        canvas.cancel_gesture();
        assert_eq!(
            canvas.editor.elements()[0].points[0],
            Point { x: 200.0, y: 200.0 }
        );
    }
}
