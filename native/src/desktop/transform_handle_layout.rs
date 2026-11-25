//! Generic transform-handle layout uses client DIPs shared by drawing/hit testing; document transform centers remain unchanged.
use super::canvas::Canvas;
use crate::{editor::Tool, viewport::Point};
/// Same source as canvas previews; materialized group/path/effect previews do not add single-selection translation again.
pub(super) fn preview_bounds(
    canvas: &super::canvas::Canvas,
    index: usize,
) -> Option<crate::group_bounds::Bounds> {
    let preview = canvas
        .group_preview
        .binary_search_by_key(&index, |(i, _)| *i)
        .ok()
        .map(|position| &canvas.group_preview[position].1)
        .or_else(|| {
            canvas
                .path_drag
                .as_ref()
                .filter(|drag| drag.index == index)
                .map(|drag| &drag.preview)
        })
        .or_else(|| {
            canvas
                .selection_effect_preview
                .as_ref()
                .filter(|_| canvas.selected_index() == Some(index))
        });
    let mut bounds =
        crate::group_bounds::bounds(preview.unwrap_or(&canvas.editor.elements()[index]))?;
    if preview.is_none() && canvas.selected_index() == Some(index) {
        if let Some((_, offset)) = canvas.selection_drag {
            bounds.low.x += offset.x;
            bounds.low.y += offset.y;
            bounds.high.x += offset.x;
            bounds.high.y += offset.y;
        }
    }
    Some(bounds)
}
/// Drawing and hit testing share one traversal of the selected preview geometry.
pub(super) fn positions(canvas: &Canvas) -> (Option<Point>, Option<Point>) {
    if canvas.tool != Tool::Select {
        return (None, None);
    }
    let mut selection = canvas.selected_indices_ref().iter().copied();
    let Some(mut bounds) = selection
        .next()
        .and_then(|index| preview_bounds(canvas, index))
    else {
        return (None, None);
    };
    for index in selection {
        let Some(next) = preview_bounds(canvas, index) else {
            return (None, None);
        };
        bounds.low.x = bounds.low.x.min(next.low.x);
        bounds.low.y = bounds.low.y.min(next.low.y);
        bounds.high.x = bounds.high.x.max(next.high.x);
        bounds.high.y = bounds.high.y.max(next.high.y);
    }
    let viewport = canvas.viewport;
    let size = canvas.client_size;
    let rotation = fit(
        Point {
            x: viewport.origin.x + (bounds.low.x + bounds.high.x) * 0.5 * viewport.scale,
            y: viewport.origin.y + bounds.low.y * viewport.scale - 24.0,
        },
        size,
    );
    let scale = fit(
        Point {
            x: viewport.origin.x + bounds.high.x * viewport.scale + 18.0,
            y: viewport.origin.y + bounds.high.y * viewport.scale + 18.0,
        },
        size,
    )
    .and_then(|point| separate(point, rotation, size));
    (rotation, scale)
}

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
    // A minimum 24 DIP preserves hit radius/edge gaps; hide in smaller windows rather than clamp into inverted ranges.
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

#[cfg(test)]
mod movement_tests {
    use super::*;
    use crate::{
        desktop::{canvas::Canvas, rotation_handle, scale_handle},
        editor::Tool,
        viewport::Viewport,
    };
    fn p(x: f32, y: f32) -> Point {
        Point { x, y }
    }
    fn canvas(tool: Tool, group: bool, scale: f32) -> Canvas {
        let mut c = Canvas::new(Some(crate::desktop::image::Image {
            width: 200,
            height: 200,
            pixels: vec![255; 200 * 200 * 4].into(),
        }))
        .unwrap();
        c.pinned = true;
        c.viewport = Viewport {
            origin: p(40., 50.),
            scale,
        };
        c.editor.set_style(
            c.editor
                .style()
                .with_fill(Some(crate::stroke_style::Color(0, 0, 0))),
        );
        for x in if group { vec![50., 130.] } else { vec![50.] } {
            if tool == Tool::Blur {
                let image = crate::blur_image::BlurImage::new(
                    200,
                    200,
                    c.image.as_ref().unwrap().pixels.clone(),
                    4,
                )
                .unwrap();
                assert!(c.editor.begin_blur(p(x, 80.), image).unwrap());
            } else {
                assert!(c.editor.begin(tool, p(x, 80.)).unwrap());
            }
            c.editor.extend(p(x + 40., 120.)).unwrap();
            assert!(c.editor.commit());
        }
        if tool == Tool::Rectangle {
            assert!(c.editor.rotate(0, 37.).unwrap());
        }
        c.select_tool(Tool::Select);
        let rev = c.editor.revision();
        c.selected = Some((0, rev));
        if group {
            c.multi_selection = Some((vec![0, 1], rev));
        }
        c
    }
    fn handles(c: &Canvas) -> [Point; 2] {
        [
            rotation_handle::position(c).unwrap(),
            scale_handle::position(c).unwrap(),
        ]
    }
    fn close(actual: Point, expected: Point) {
        assert!(
            (actual.x - expected.x).hypot(actual.y - expected.y) < 0.002,
            "{actual:?} != {expected:?}"
        );
    }
    #[test]
    fn both_handles_follow_each_translation_preview_once_and_remain_at_the_committed_position() {
        for tool in [Tool::Rectangle, Tool::Mosaic, Tool::Blur] {
            for group in [false, true] {
                for scale in [0.5, 1., 2.] {
                    let mut c = canvas(tool, group, scale);
                    let before = handles(&c);
                    let start = p(40. + 70. * scale, 50. + 100. * scale);
                    let revision = c.editor.revision();
                    assert!(c.pointer_down(start), "{tool:?} {group}");
                    assert!(c.selection_drag.is_some());
                    assert!(
                        c.rotation_drag.is_none()
                            && c.scale_drag.is_none()
                            && c.path_drag.is_none()
                    );
                    let mut last = start;
                    for delta in [p(12., 0.), p(-8., 6.), p(4., -7.)] {
                        last = p(start.x + delta.x * scale, start.y + delta.y * scale);
                        assert!(c.pointer_move(last));
                        for (now, original) in handles(&c).into_iter().zip(before) {
                            close(
                                now,
                                p(original.x + delta.x * scale, original.y + delta.y * scale),
                            );
                        }
                        assert_eq!(c.editor.revision(), revision);
                    }
                    let preview = handles(&c);
                    assert!(c.pointer_up(last));
                    assert_eq!(c.editor.revision(), revision + 1);
                    for (actual, expected) in handles(&c).into_iter().zip(preview) {
                        close(actual, expected);
                    }
                    // The second drag starts at the translated center, not stale coordinates for small-annotation handles.
                    assert!(c.pointer_down(last));
                    assert!(
                        c.selection_drag.is_some()
                            && c.rotation_drag.is_none()
                            && c.scale_drag.is_none()
                            && c.path_drag.is_none(),
                        "second drag: {tool:?}, group={group}, scale={scale}, at={last:?}"
                    );
                    assert!(c.pointer_move(p(last.x + 3. * scale, last.y + 4. * scale)));
                    c.cancel_gesture();
                    for (actual, expected) in handles(&c).into_iter().zip(preview) {
                        close(actual, expected);
                    }
                    assert!(c.editor.undo());
                    assert!(rotation_handle::position(&c).is_none());
                    assert!(scale_handle::position(&c).is_none());
                }
            }
        }
    }
    #[test]
    fn path_handle_preview_updates_common_handles_without_waiting_for_commit() {
        let mut c = canvas(Tool::Rectangle, false, 1.);
        assert!(c.editor.rotate(0, -37.).unwrap());
        c.selected = Some((0, c.editor.revision()));
        let before = handles(&c);
        assert!(c.pointer_down(p(90., 130.)));
        assert!(c.path_drag.is_some());
        assert!(c.pointer_move(p(80., 120.)));
        let expected = crate::group_bounds::bounds(&c.path_drag.as_ref().unwrap().preview).unwrap();
        let actual = preview_bounds(&c, 0).unwrap();
        close(actual.low, expected.low);
        close(actual.high, expected.high);
        assert_ne!(handles(&c), before);
        c.cancel_gesture();
        assert_eq!(handles(&c), before);
    }
}
