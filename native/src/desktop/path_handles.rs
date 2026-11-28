//! Path-handle gestures; line endpoints and Bezier control points share preview, cancellation, and history-commit lifetimes.
use super::canvas::Canvas;
use crate::{
    editor::{Stroke, Tool},
    rotation,
    viewport::{Point, Viewport},
};
use windows::Win32::Graphics::Direct2D::{Common::*, *};

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
#[path = "bezier_handle_tests.rs"]
mod tests;

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
#[path = "line_handle_tests.rs"]
mod line_tests;
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
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
    Lens(usize, Point),
    Serial(usize),
    Text(usize),
    Point(usize),
    Extend(bool),
    Shape(usize),
}
fn measure(
    text: &crate::text_annotation::TextAnnotation,
) -> Result<Point, crate::editor::EditError> {
    unsafe { super::annotation_text::measure(text) }
        .map_err(|_| crate::editor::EditError::InvalidPoint)
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
            && source.serial_handles().is_none()
            && source.text_handles().is_none()
            && source.lens_handles().is_none()
        {
            return false;
        }
        let point = self.viewport.image_point(screen);
        let center = rotation::center(&source.points);
        let local = rotation::rotate(point, center, -source.rotation);
        let shape_handles = source.shape_handles();
        let handle = if let Some(lens_pts) = source.lens_edit_handles() {
            let corner_hit = lens_pts[1..]
                .iter()
                .enumerate()
                .map(|(i, p)| (i, (p.x - local.x).hypot(p.y - local.y)))
                .filter(|(_, d)| *d <= 6.5 / self.viewport.scale)
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(i, _)| Handle::Lens(i + 2, Point::default()));
            if corner_hit.is_some() {
                corner_hit
            } else {
                let pad = 4.0 / self.viewport.scale;
                let center_hit = (lens_pts[0].x - local.x).hypot(lens_pts[0].y - local.y)
                    <= 8.0 / self.viewport.scale;
                let box_hit = source.lens_source_box().is_some_and(|(s0, s1)| {
                    local.x >= s0.x - pad
                        && local.x <= s1.x + pad
                        && local.y >= s0.y - pad
                        && local.y <= s1.y + pad
                });
                (center_hit || box_hit).then_some(Handle::Lens(
                    0,
                    Point {
                        x: lens_pts[0].x - local.x,
                        y: lens_pts[0].y - local.y,
                    },
                ))
            }
        } else if let Some(p) = source.serial_handles().or_else(|| source.text_handles()) {
            p.iter()
                .enumerate()
                .map(|(i, p)| (i, (p.x - local.x).hypot(p.y - local.y)))
                .filter(|(_, d)| *d <= 6.0 / self.viewport.scale)
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(i, _)| {
                    if source.tool == Tool::Serial {
                        Handle::Serial(i)
                    } else {
                        Handle::Text(i)
                    }
                })
        } else {
            shape_handles
                .and_then(|points| {
                    points.iter().position(|p| {
                        (p.x - local.x).hypot(p.y - local.y) <= 6.0 / self.viewport.scale
                    })
                })
                .map(Handle::Shape)
                .or_else(|| {
                    source
                        .points
                        .iter()
                        .position(|p| {
                            (p.x - local.x).hypot(p.y - local.y) <= 6.0 / self.viewport.scale
                        })
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
                })
        };
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
            Handle::Lens(i, off) => source.preview_lens_handle(
                i,
                Point {
                    x: point.x + off.x,
                    y: point.y + off.y,
                },
            ),
            Handle::Serial(i) => source.preview_serial_handle(i, point),
            Handle::Text(i) => source.preview_text_handle(i, point.x, measure),
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
            Handle::Lens(i, off) => self.editor.move_lens_handle(
                drag.index,
                i,
                Point {
                    x: drag.point.x + off.x,
                    y: drag.point.y + off.y,
                },
            ),
            Handle::Serial(i) => self.editor.move_serial_handle(drag.index, i, drag.point),
            Handle::Text(i) => self
                .editor
                .move_text_handle(drag.index, i, drag.point.x, measure),
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
        && stroke.serial_handles().is_none()
        && stroke.text_handles().is_none()
        && stroke.lens_handles().is_none()
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
    if let Some(lens_pts) = stroke.lens_edit_handles() {
        if let Some((s0, s1)) = stroke.lens_source_box() {
            let p0 = convert(s0);
            let p1 = convert(Point { x: s1.x, y: s0.y });
            let p2 = convert(s1);
            let p3 = convert(Point { x: s0.x, y: s1.y });
            brush.SetColor(&D2D1_COLOR_F {
                r: 0.85,
                g: 1.0,
                b: 0.22,
                a: 0.92,
            });
            for (a, b) in [(p0, p1), (p1, p2), (p2, p3), (p3, p0)] {
                target.DrawLine(a, b, brush, 1.2, None);
            }
        }
        let src = convert(lens_pts[0]);
        brush.SetColor(&D2D1_COLOR_F {
            r: 0.06,
            g: 0.07,
            b: 0.09,
            a: 0.85,
        });
        target.FillEllipse(
            &D2D1_ELLIPSE {
                point: src,
                radiusX: 6.5,
                radiusY: 6.5,
            },
            brush,
        );
        brush.SetColor(&D2D1_COLOR_F {
            r: 0.85,
            g: 1.0,
            b: 0.22,
            a: 1.0,
        });
        target.DrawEllipse(
            &D2D1_ELLIPSE {
                point: src,
                radiusX: 6.5,
                radiusY: 6.5,
            },
            brush,
            1.5,
            None,
        );
        target.DrawLine(
            D2D_POINT_2F {
                x: src.x - 4.0,
                y: src.y,
            },
            D2D_POINT_2F {
                x: src.x + 4.0,
                y: src.y,
            },
            brush,
            1.4,
            None,
        );
        target.DrawLine(
            D2D_POINT_2F {
                x: src.x,
                y: src.y - 4.0,
            },
            D2D_POINT_2F {
                x: src.x,
                y: src.y + 4.0,
            },
            brush,
            1.4,
            None,
        );
        brush.SetColor(&D2D1_COLOR_F {
            r: 0.1,
            g: 0.8,
            b: 1.0,
            a: 1.0,
        });
        for &corner in &lens_pts[1..] {
            let pt = convert(corner);
            target.FillRectangle(
                &D2D_RECT_F {
                    left: pt.x - 3.5,
                    top: pt.y - 3.5,
                    right: pt.x + 3.5,
                    bottom: pt.y + 3.5,
                },
                brush,
            );
        }
        return;
    }
    let shape_handles = stroke.shape_handles();
    let label_handles = stroke.serial_handles().or_else(|| stroke.text_handles());
    let handles = label_handles
        .as_ref()
        .map(|p| p.as_slice())
        .or_else(|| shape_handles.as_ref().map(|points| points.as_slice()))
        .unwrap_or(stroke.points.as_slice());
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
