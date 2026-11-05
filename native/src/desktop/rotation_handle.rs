//! 旋转柄手势拥有固定起点与版本；几何预览不写历史，松开整组提交，取消释放缓冲。
use super::canvas::Canvas;
use crate::{editor::Tool, transform_gesture::Gesture, viewport::Point};
use windows::Win32::Graphics::Direct2D::{Common::*, *};

pub(super) struct Drag {
    revision: u64,
    pivot: Point,
    own: bool,
    gesture: Gesture,
    angle: f32,
}

pub(super) fn position(canvas: &Canvas) -> Option<Point> {
    if canvas.tool != Tool::Select {
        return None;
    }
    let indices = canvas.selected_indices();
    let mut low = Point {
        x: f32::INFINITY,
        y: f32::INFINITY,
    };
    let mut high = Point {
        x: f32::NEG_INFINITY,
        y: f32::NEG_INFINITY,
    };
    for index in indices {
        let stroke = canvas
            .group_preview
            .iter()
            .find(|(target, _)| *target == index)
            .map(|(_, s)| s)
            .unwrap_or(&canvas.editor.elements()[index]);
        let bounds = crate::group_bounds::bounds(stroke)?;
        low.x = low.x.min(bounds.low.x);
        low.y = low.y.min(bounds.low.y);
        high.x = high.x.max(bounds.high.x);
        high.y = high.y.max(bounds.high.y);
    }
    super::transform_handle_layout::fit(
        Point {
            x: canvas.viewport.origin.x + (low.x + high.x) * 0.5 * canvas.viewport.scale,
            y: canvas.viewport.origin.y + low.y * canvas.viewport.scale - 24.0,
        },
        canvas.client_size,
    )
}

impl Canvas {
    pub(super) fn begin_rotation_handle(&mut self, screen: Point) -> bool {
        let Some(handle) = position(self) else {
            return false;
        };
        if (handle.x - screen.x).hypot(handle.y - screen.y) > 7.0 {
            return false;
        }
        let indices = self.selected_indices();
        let Some(pivot) = crate::group_bounds::center(
            indices.iter().map(|index| &self.editor.elements()[*index]),
        ) else {
            return false;
        };
        let start = self.viewport.image_point(screen);
        let Some(gesture) = Gesture::new(start, pivot, self.rotation_axis) else {
            return false;
        };
        self.group_preview = indices
            .iter()
            .map(|index| (*index, self.editor.elements()[*index].clone()))
            .collect();
        self.rotation_drag = Some(Drag {
            revision: self.editor.revision(),
            pivot,
            own: self.transform_own,
            gesture,
            angle: 0.0,
        });
        // 共用活动变换标记，保持打开图片/异步结果/滚轮的既有准入边界。
        self.selection_drag = Some((start, Point::default()));
        true
    }
    pub(super) fn move_rotation_handle(&mut self, screen: Point) -> bool {
        let Some(drag) = self.rotation_drag.as_ref() else {
            return false;
        };
        if drag.revision != self.editor.revision() {
            self.cancel_gesture();
            return true;
        }
        let Some(angle) = drag.gesture.rotation(self.viewport.image_point(screen)) else {
            return false;
        };
        if angle == drag.angle {
            return false;
        }
        let old_angle = drag.angle;
        // 如后续对象越界，恢复已更新对象到上一有效角度，避免部分组预览。
        for position in 0..self.group_preview.len() {
            let (index, preview) = &mut self.group_preview[position];
            let original = &self.editor.elements()[*index];
            let pivot = if drag.own {
                crate::group_bounds::bounds(original).unwrap().center()
            } else {
                drag.pivot
            };
            if preview
                .update_rotation_preview(original, pivot, angle)
                .is_err()
            {
                for (index, preview) in &mut self.group_preview[..position] {
                    let original = &self.editor.elements()[*index];
                    let pivot = if drag.own {
                        crate::group_bounds::bounds(original).unwrap().center()
                    } else {
                        drag.pivot
                    };
                    preview
                        .update_rotation_preview(original, pivot, old_angle)
                        .expect("previous valid rotation");
                }
                return false;
            }
        }
        self.rotation_drag.as_mut().unwrap().angle = angle;
        true
    }
    pub(super) fn commit_rotation_handle(&mut self) {
        let Some(drag) = self.rotation_drag.take() else {
            return;
        };
        self.selection_drag = None;
        self.group_preview = Vec::new();
        if drag.revision != self.editor.revision() {
            return;
        }
        let indices = self.selected_indices();
        match self
            .editor
            .rotate_many_mode(&indices, drag.pivot, drag.angle, drag.own)
        {
            Ok(true) => {
                let revision = self.editor.revision();
                self.selected = indices.first().map(|index| (*index, revision));
                self.multi_selection = (indices.len() > 1).then_some((indices, revision));
            }
            Ok(false) => {}
            Err(error) => eprintln!("native-rotation-handle: {error:?}; preview discarded"),
        }
    }
}

pub(super) unsafe fn draw(
    target: &ID2D1RenderTarget,
    brush: &ID2D1SolidColorBrush,
    point: Option<Point>,
) {
    let Some(point) = point else { return };
    brush.SetColor(&D2D1_COLOR_F {
        r: 0.1,
        g: 0.8,
        b: 1.0,
        a: 1.0,
    });
    target.DrawEllipse(
        &D2D1_ELLIPSE {
            point: D2D_POINT_2F {
                x: point.x,
                y: point.y,
            },
            radiusX: 5.0,
            radiusY: 5.0,
        },
        brush,
        1.5,
        None,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn axis_handle_uses_image_displacement_and_ignores_other_axis() {
        use crate::transform_gesture::Axis;
        for (axis, delta) in [
            (Axis::X, Point { x: 40.0, y: 200.0 }),
            (Axis::Y, Point { x: 200.0, y: 40.0 }),
        ] {
            let mut canvas = Canvas::new(Some(super::super::image::Image {
                width: 100,
                height: 100,
                pixels: vec![255; 100 * 100 * 4].into(),
            }))
            .unwrap();
            canvas.viewport = crate::viewport::Viewport {
                origin: Point { x: 10.0, y: 20.0 },
                scale: 2.0,
            };
            canvas
                .editor
                .begin(Tool::Rectangle, Point { x: 20.0, y: 40.0 })
                .unwrap();
            canvas.editor.extend(Point { x: 40.0, y: 60.0 }).unwrap();
            assert!(canvas.editor.commit());
            canvas.select_tool(Tool::Select);
            canvas.rotation_axis = axis;
            // 移动约束不参与旋转角度计算。
            canvas.move_axis = if axis == Axis::X { Axis::Y } else { Axis::X };
            let revision = canvas.editor.revision();
            canvas.selected = Some((0, revision));
            let handle = position(&canvas).unwrap();
            let end = Point {
                x: handle.x + delta.x,
                y: handle.y + delta.y,
            };
            assert!(canvas.pointer_down(handle));
            assert!(canvas.pointer_move(end));
            assert!((canvas.group_preview[0].1.rotation - 10.0).abs() < 0.001);
            assert_eq!(canvas.editor.revision(), revision);
            assert!(canvas.pointer_up(end));
            assert!((canvas.editor.elements()[0].rotation - 10.0).abs() < 0.001);
            assert!(canvas.editor.undo());
            assert_eq!(canvas.editor.elements()[0].rotation, 0.0);
        }
    }
    #[test]
    fn group_handle_commits_once_and_stale_drag_is_discarded() {
        let mut canvas = Canvas::new(Some(super::super::image::Image {
            width: 100,
            height: 100,
            pixels: vec![255; 100 * 100 * 4].into(),
        }))
        .unwrap();
        canvas.viewport = crate::viewport::Viewport {
            origin: Point::default(),
            scale: 1.0,
        };
        for x in [20.0, 60.0] {
            canvas
                .editor
                .begin(Tool::Rectangle, Point { x, y: 40.0 })
                .unwrap();
            canvas
                .editor
                .extend(Point {
                    x: x + 10.0,
                    y: 50.0,
                })
                .unwrap();
            assert!(canvas.editor.commit());
        }
        canvas.select_tool(Tool::Select);
        let revision = canvas.editor.revision();
        canvas.selected = Some((0, revision));
        canvas.multi_selection = Some((vec![0, 1], revision));
        let handle = position(&canvas).unwrap();
        assert!(canvas.pointer_down(handle));
        assert!(canvas.pointer_up(Point { x: 75.0, y: 45.0 }));
        assert_eq!(canvas.editor.revision(), revision + 1);
        assert_eq!(canvas.selected_indices(), vec![0, 1]);
        assert!((canvas.editor.elements()[0].rotation - 90.0).abs() < 0.001);
        assert!(canvas.editor.undo());
        assert_eq!(
            canvas.editor.elements()[0].points[0],
            Point { x: 20.0, y: 40.0 }
        );
        let revision = canvas.editor.revision();
        canvas.selected = Some((0, revision));
        canvas.multi_selection = Some((vec![0, 1], revision));
        assert!(canvas.pointer_down(handle));
        assert!(canvas.editor.undo());
        assert!(canvas.pointer_move(Point { x: 75.0, y: 45.0 }));
        assert!(canvas.rotation_drag.is_none());
        assert!(canvas.group_preview.is_empty());
    }
}
