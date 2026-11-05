//! 路径控制柄手势；直线端点与贝塞尔控制点共用预览、取消和历史提交生命周期。
use super::canvas::Canvas;
use crate::{
    editor::{Stroke, Tool},
    rotation,
    viewport::{Point, Viewport},
};
use windows::Win32::Graphics::Direct2D::{Common::*, *};

#[cfg(test)]
#[path = "bezier_handle_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "line_handle_tests.rs"]
mod line_tests;
#[cfg(test)]
#[path = "shape_handle_tests.rs"]
mod shape_tests;

pub(super) struct Drag {
    pub index: usize,
    pub revision: u64,
    handle: Handle,
    changed: bool,
    point: Point,
    pub preview: Stroke,
}

enum Handle {
    Point(usize),
    Extend(bool),
    Shape(usize),
}

impl Canvas {
    pub(super) fn begin_path_handle(&mut self, screen: Point) -> bool {
        let Some(index) = self.selected_index() else {
            return false;
        };
        let source = &self.editor.elements()[index];
        if !matches!(
            source.tool,
            Tool::Bezier | Tool::Line | Tool::Arrow | Tool::Polyline
        ) && source.shape_handles().is_none()
        {
            return false;
        }
        let point = self.viewport.image_point(screen);
        let center = rotation::center(&source.points);
        let local = rotation::rotate(point, center, -source.rotation);
        let shape_handles = source.shape_handles();
        let handle = shape_handles
            .and_then(|points| {
                points
                    .iter()
                    .position(|p| (p.x - local.x).hypot(p.y - local.y) <= 6.0 / self.viewport.scale)
            })
            .map(Handle::Shape)
            .or_else(|| {
                source
                    .points
                    .iter()
                    .position(|p| (p.x - local.x).hypot(p.y - local.y) <= 6.0 / self.viewport.scale)
                    .map(Handle::Point)
                    .or_else(|| {
                        if source.tool != Tool::Bezier {
                            return None;
                        }
                        crate::bezier_geometry::extension_handles(&source.points).and_then(
                            |points| {
                                points
                                    .iter()
                                    .position(|p| {
                                        (p.x - local.x).hypot(p.y - local.y)
                                            <= 6.0 / self.viewport.scale
                                    })
                                    .map(|side| Handle::Extend(side == 0))
                            },
                        )
                    })
            });
        let Some(handle) = handle else {
            return false;
        };
        let point = local;
        self.path_drag = Some(Drag {
            index,
            revision: self.editor.revision(),
            handle,
            changed: false,
            point,
            preview: source.clone(),
        });
        true
    }

    pub(super) fn move_path_handle_preview(&mut self, screen: Point) -> bool {
        let Some(drag) = &mut self.path_drag else {
            return false;
        };
        if drag.revision != self.editor.revision() {
            self.path_drag = None;
            return true;
        }
        let source = &self.editor.elements()[drag.index];
        let point = rotation::rotate(
            self.viewport.image_point(screen),
            rotation::center(&source.points),
            -source.rotation,
        );
        if point == drag.point {
            return false;
        }
        let preview = match drag.handle {
            Handle::Shape(handle) => source.preview_shape_handle(handle, point),
            Handle::Point(handle) if source.tool != Tool::Bezier => {
                source.preview_line_endpoint(handle, point)
            }
            Handle::Point(handle) => source.preview_bezier_handle(handle, point),
            Handle::Extend(from_start) => source.preview_extend_bezier(point, from_start),
        };
        match preview {
            Ok(Some(preview)) => {
                drag.point = point;
                drag.preview = preview;
                drag.changed = true;
                true
            }
            _ => {
                self.path_drag = None;
                true
            }
        }
    }

    pub(super) fn commit_path_handle(&mut self) -> bool {
        let Some(drag) = self.path_drag.take() else {
            return false;
        };
        if drag.revision != self.editor.revision() || !drag.changed {
            return true;
        }
        let result = match drag.handle {
            Handle::Shape(handle) => self
                .editor
                .move_shape_handle(drag.index, handle, drag.point),
            Handle::Point(handle) if self.editor.elements()[drag.index].tool != Tool::Bezier => {
                self.editor
                    .move_line_endpoint(drag.index, handle, drag.point)
            }
            Handle::Point(handle) => self
                .editor
                .move_bezier_handle(drag.index, handle, drag.point),
            Handle::Extend(from_start) => self
                .editor
                .extend_bezier(drag.index, drag.point, from_start),
        };
        match result {
            Ok(true) => self.selected = Some((drag.index, self.editor.revision())),
            Ok(false) => {}
            Err(error) => eprintln!("native-path-handle: {error:?}; preview discarded"),
        }
        true
    }
}

pub(super) unsafe fn draw(
    target: &ID2D1RenderTarget,
    brush: &ID2D1SolidColorBrush,
    stroke: &Stroke,
    viewport: Viewport,
) {
    if !matches!(
        stroke.tool,
        Tool::Bezier | Tool::Line | Tool::Arrow | Tool::Polyline
    ) && stroke.shape_handles().is_none()
    {
        return;
    }
    let center = rotation::center(&stroke.points);
    let convert = |p: Point| {
        let p = rotation::rotate(p, center, stroke.rotation);
        D2D_POINT_2F {
            x: viewport.origin.x + p.x * viewport.scale,
            y: viewport.origin.y + p.y * viewport.scale,
        }
    };
    brush.SetColor(&D2D1_COLOR_F {
        r: 0.1,
        g: 0.8,
        b: 1.0,
        a: 1.0,
    });
    for segment in stroke.points.windows(4).step_by(3) {
        if stroke.tool != Tool::Bezier {
            break;
        }
        target.DrawLine(convert(segment[0]), convert(segment[1]), brush, 1.0, None);
        target.DrawLine(convert(segment[2]), convert(segment[3]), brush, 1.0, None);
    }
    let shape_handles = stroke.shape_handles();
    let handles = shape_handles
        .as_ref()
        .map_or(stroke.points.as_slice(), |points| points.as_slice());
    for (index, &point) in handles.iter().enumerate() {
        let point = convert(point);
        if stroke.tool != Tool::Bezier || index.is_multiple_of(3) {
            target.FillRectangle(
                &D2D_RECT_F {
                    left: point.x - 3.0,
                    top: point.y - 3.0,
                    right: point.x + 3.0,
                    bottom: point.y + 3.0,
                },
                brush,
            );
        } else {
            target.DrawEllipse(
                &D2D1_ELLIPSE {
                    point,
                    radiusX: 4.0,
                    radiusY: 4.0,
                },
                brush,
                1.0,
                None,
            );
        }
    }
    if stroke.tool != Tool::Bezier {
        return;
    }
    if let Some(handles) = crate::bezier_geometry::extension_handles(&stroke.points) {
        for handle in handles {
            let point = convert(handle);
            target.DrawEllipse(
                &D2D1_ELLIPSE {
                    point,
                    radiusX: 5.0,
                    radiusY: 5.0,
                },
                brush,
                1.0,
                None,
            );
            target.DrawLine(
                D2D_POINT_2F {
                    x: point.x - 3.0,
                    y: point.y,
                },
                D2D_POINT_2F {
                    x: point.x + 3.0,
                    y: point.y,
                },
                brush,
                1.0,
                None,
            );
            target.DrawLine(
                D2D_POINT_2F {
                    x: point.x,
                    y: point.y - 3.0,
                },
                D2D_POINT_2F {
                    x: point.x,
                    y: point.y + 3.0,
                },
                brush,
                1.0,
                None,
            );
        }
    }
}
