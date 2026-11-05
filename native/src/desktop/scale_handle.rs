//! 缩放柄仅拥有预览；松开生成候选，由宿主统一交付同步/模糊后台提交。
use super::canvas::Canvas;
use crate::{
    editor::{EditError, Stroke, Tool},
    transform_gesture::Gesture,
    viewport::Point,
};
use windows::Win32::Graphics::Direct2D::{Common::*, *};

pub(super) struct Drag {
    revision: u64,
    pivot: Point,
    own: bool,
    gesture: Gesture,
    factors: Point,
}
pub(super) fn position(canvas: &Canvas) -> Option<Point> {
    if canvas.tool != Tool::Select {
        return None;
    }
    let mut right = f32::NEG_INFINITY;
    let mut bottom = f32::NEG_INFINITY;
    for index in canvas.selected_indices() {
        let stroke = canvas
            .group_preview
            .iter()
            .find(|(target, _)| *target == index)
            .map(|(_, s)| s)
            .unwrap_or(&canvas.editor.elements()[index]);
        let bounds = crate::group_bounds::bounds(stroke)?;
        right = right.max(bounds.high.x);
        bottom = bottom.max(bounds.high.y);
    }
    let point = super::transform_handle_layout::fit(
        Point {
            x: canvas.viewport.origin.x + right * canvas.viewport.scale + 18.0,
            y: canvas.viewport.origin.y + bottom * canvas.viewport.scale + 18.0,
        },
        canvas.client_size,
    )?;
    super::transform_handle_layout::separate(
        point,
        super::rotation_handle::position(canvas),
        canvas.client_size,
    )
}
fn update(
    preview: &mut Stroke,
    original: &Stroke,
    factors: Point,
    pivot: Point,
    own: bool,
) -> Result<(), EditError> {
    let measured = if original.tool == Tool::Text {
        let text = original
            .text
            .as_ref()
            .and_then(|text| text.with_font_size(text.font_size() * (factors.x + factors.y) / 2.0))
            .ok_or(EditError::BudgetExceeded)?;
        Some(
            unsafe { super::annotation_text::measure(&text) }
                .map_err(|_| EditError::InvalidPoint)?,
        )
    } else {
        None
    };
    let pivot = if own {
        crate::group_bounds::bounds(original)
            .ok_or(EditError::InvalidPoint)?
            .center()
    } else {
        pivot
    };
    preview.update_resize_preview(original, factors, measured, Some(pivot))
}
impl Canvas {
    pub(super) fn clear_scale_display(&mut self) -> bool {
        if self.scale_display_revision.take().is_some() {
            self.group_preview = Vec::new();
            true
        } else {
            false
        }
    }
    pub(super) fn validate_scale_display(&mut self) {
        if let Some(revision) = self.scale_display_revision {
            if revision != self.editor.revision()
                || !self
                    .selected_indices()
                    .iter()
                    .copied()
                    .eq(self.group_preview.iter().map(|(index, _)| *index))
            {
                self.clear_scale_display();
            }
        }
    }
    pub(super) fn begin_scale_handle(&mut self, screen: Point) -> bool {
        let Some(handle) = position(self) else {
            return false;
        };
        if (handle.x - screen.x).abs() > 7.0 || (handle.y - screen.y).abs() > 7.0 {
            return false;
        }
        let indices = self.selected_indices();
        let Some(pivot) = crate::group_bounds::center(
            indices.iter().map(|index| &self.editor.elements()[*index]),
        ) else {
            return false;
        };
        let start = self.viewport.image_point(screen);
        let Some(gesture) = Gesture::new(start, pivot, self.scale_axis) else {
            return false;
        };
        self.group_preview = indices
            .iter()
            .map(|index| (*index, self.editor.elements()[*index].clone()))
            .collect();
        self.scale_drag = Some(Drag {
            revision: self.editor.revision(),
            pivot,
            own: self.transform_own,
            gesture,
            factors: Point { x: 1.0, y: 1.0 },
        });
        self.selection_drag = Some((start, Point::default()));
        true
    }
    pub(super) fn move_scale_handle(&mut self, screen: Point, uniform: bool) -> bool {
        let Some(drag) = self.scale_drag.as_ref() else {
            return false;
        };
        if drag.revision != self.editor.revision() {
            self.cancel_gesture();
            return true;
        }
        let Some(factors) = drag
            .gesture
            .scale(self.viewport.image_point(screen), uniform)
        else {
            return false;
        };
        if factors == drag.factors {
            return false;
        }
        for position in 0..self.group_preview.len() {
            let (index, preview) = &mut self.group_preview[position];
            if update(
                preview,
                &self.editor.elements()[*index],
                factors,
                drag.pivot,
                drag.own,
            )
            .is_err()
            {
                // 回滚可能需要再次排版；若排版资源此时也失败，取消整次预览而非保留半组。
                let restored = self.group_preview[..position]
                    .iter_mut()
                    .all(|(index, preview)| {
                        update(
                            preview,
                            &self.editor.elements()[*index],
                            drag.factors,
                            drag.pivot,
                            drag.own,
                        )
                        .is_ok()
                    });
                if !restored {
                    self.cancel_gesture();
                }
                return !restored;
            }
        }
        self.scale_drag.as_mut().unwrap().factors = factors;
        true
    }
    pub(super) fn finish_scale_handle(&mut self) {
        let Some(drag) = self.scale_drag.take() else {
            return;
        };
        self.selection_drag = None;
        if drag.revision != self.editor.revision() {
            self.group_preview = Vec::new();
            return;
        }
        let indices = self.selected_indices();
        match self.editor.prepare_resize_many_mode(
            &indices,
            drag.pivot,
            drag.factors,
            drag.own,
            |text| {
                unsafe { super::annotation_text::measure(text) }
                    .map_err(|_| EditError::InvalidPoint)
            },
        ) {
            Ok(plan) => self.scale_pending = plan,
            Err(error) => eprintln!("native-scale-handle: {error:?}; preview discarded"),
        }
        if self
            .scale_pending
            .as_ref()
            .is_some_and(|plan| plan.strokes().any(|(_, stroke)| stroke.tool == Tool::Blur))
        {
            self.scale_display_revision = Some(drag.revision);
        } else {
            self.group_preview = Vec::new();
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
    target.DrawRectangle(
        &D2D_RECT_F {
            left: point.x - 4.0,
            top: point.y - 4.0,
            right: point.x + 4.0,
            bottom: point.y + 4.0,
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
    fn scale_axis_limits_geometry_and_shift_overrides_axis() {
        use crate::transform_gesture::Axis;
        for axis in [Axis::X, Axis::Y] {
            for uniform in [false, true] {
                let mut canvas = Canvas::new(Some(super::super::image::Image {
                    width: 100,
                    height: 100,
                    pixels: vec![255; 100 * 100 * 4].into(),
                }))
                .unwrap();
                canvas.viewport = crate::viewport::Viewport {
                    origin: Point { x: 5.0, y: 7.0 },
                    scale: 2.0,
                };
                canvas
                    .editor
                    .begin(Tool::Rectangle, Point { x: 20.0, y: 20.0 })
                    .unwrap();
                canvas.editor.extend(Point { x: 40.0, y: 40.0 }).unwrap();
                canvas.editor.commit();
                canvas.select_tool(Tool::Select);
                canvas.selected = Some((0, canvas.editor.revision()));
                canvas.scale_axis = axis;
                let handle = position(&canvas).unwrap();
                let pivot = Point { x: 65.0, y: 67.0 };
                let end = Point {
                    x: pivot.x + 2.0 * (handle.x - pivot.x),
                    y: pivot.y + 2.0 * (handle.y - pivot.y),
                };
                assert!(canvas.pointer_down(handle));
                assert!(canvas.pointer_move_modified(end, uniform, false));
                let points = &canvas.group_preview[0].1.points;
                let width = if uniform || axis == Axis::X {
                    40.0
                } else {
                    20.0
                };
                let height = if uniform || axis == Axis::Y {
                    40.0
                } else {
                    20.0
                };
                assert!((points[1].x - points[0].x - width).abs() < 0.001);
                assert!((points[1].y - points[0].y - height).abs() < 0.001);
                assert!(canvas.pointer_up_modified(end, uniform, false));
                assert!(canvas
                    .editor
                    .commit_resize(canvas.scale_pending.take().unwrap())
                    .unwrap());
                let points = &canvas.editor.elements()[0].points;
                assert!((points[1].x - points[0].x - width).abs() < 0.001);
                assert!((points[1].y - points[0].y - height).abs() < 0.001);
            }
        }
    }
    #[test]
    fn group_scale_handle_prepares_one_transaction_and_cancel_releases_preview() {
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
            canvas.editor.commit();
        }
        canvas.select_tool(Tool::Select);
        let revision = canvas.editor.revision();
        canvas.selected = Some((0, revision));
        canvas.multi_selection = Some((vec![0, 1], revision));
        let handle = position(&canvas).unwrap();
        let pivot = Point { x: 45.0, y: 45.0 };
        let end = Point {
            x: pivot.x + 2.0 * (handle.x - pivot.x),
            y: pivot.y + 1.5 * (handle.y - pivot.y),
        };
        assert!(canvas.pointer_down(handle));
        assert!(canvas.pointer_move(end));
        let preview: Vec<_> = canvas
            .group_preview
            .iter()
            .map(|(_, s)| s.points.clone())
            .collect();
        assert_eq!(canvas.editor.revision(), revision);
        assert!(canvas.pointer_up(end));
        assert!(canvas
            .editor
            .commit_resize(canvas.scale_pending.take().unwrap())
            .unwrap());
        assert_eq!(canvas.editor.revision(), revision + 1);
        assert_eq!(
            canvas
                .editor
                .elements()
                .iter()
                .map(|s| s.points.clone())
                .collect::<Vec<_>>(),
            preview
        );
        assert!(canvas.editor.undo());
        let revision = canvas.editor.revision();
        canvas.selected = Some((0, revision));
        canvas.multi_selection = Some((vec![0, 1], revision));
        assert!(canvas.pointer_down(handle));
        assert!(canvas.pointer_move(end));
        assert!(canvas.cancel_gesture());
        assert!(canvas.scale_pending.is_none());
        assert!(canvas.scale_drag.is_none());
        assert!(canvas.group_preview.is_empty());
        assert_eq!(canvas.editor.revision(), revision);
    }
}
