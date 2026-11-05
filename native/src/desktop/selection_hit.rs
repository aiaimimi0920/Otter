//! 单击时按绘制逆序查询 D2D 几何，不逐帧命中；容差按视口缩放转换到图像坐标。
use crate::{
    editor::{Editor, Stroke, Tool},
    viewport::{Point, Viewport},
};
use windows::{
    core::{Interface, Result},
    Win32::Graphics::Direct2D::{Common::*, *},
};

pub(super) unsafe fn pick(
    factory: &ID2D1Factory,
    editor: &Editor,
    point: Point,
    tolerance: f32,
) -> Result<Option<usize>> {
    for rank in (0..=3).rev() {
        for (index, stroke) in editor.elements().iter().enumerate().rev() {
            if stroke.tool.render_rank() != rank {
                continue;
            }
            if stroke.visible_paint() && contains(factory, stroke, point, tolerance)? {
                return Ok(Some(index));
            }
        }
    }
    Ok(None)
}

unsafe fn path(factory: &ID2D1Factory, points: &[Point], closed: bool) -> Result<ID2D1Geometry> {
    let geometry = factory.CreatePathGeometry()?;
    let sink = geometry.Open()?;
    sink.BeginFigure(
        convert(points[0]),
        if closed {
            D2D1_FIGURE_BEGIN_FILLED
        } else {
            D2D1_FIGURE_BEGIN_HOLLOW
        },
    );
    for point in &points[1..] {
        sink.AddLine(convert(*point));
    }
    sink.EndFigure(if closed {
        D2D1_FIGURE_END_CLOSED
    } else {
        D2D1_FIGURE_END_OPEN
    });
    sink.Close()?;
    geometry.cast()
}

unsafe fn contains(
    factory: &ID2D1Factory,
    stroke: &Stroke,
    point: Point,
    tolerance: f32,
) -> Result<bool> {
    let Some(&a) = stroke.points.first() else {
        return Ok(false);
    };
    if matches!(stroke.tool, Tool::Line | Tool::Arrow) && stroke.points.iter().all(|p| *p == a) {
        return Ok(false);
    }
    let b = *stroke.points.last().unwrap();
    let rect = D2D_RECT_F {
        left: a.x.min(b.x),
        top: a.y.min(b.y),
        right: a.x.max(b.x),
        bottom: a.y.max(b.y),
    };
    let flatten = (tolerance / 8.0).clamp(0.0001, 0.25);
    let point = convert(crate::rotation::rotate(
        point,
        crate::rotation::center(&stroke.points),
        -stroke.rotation,
    ));
    let mut filled = stroke.visible_fill();
    if stroke.tool == Tool::Paintbrush {
        let path = super::annotation_paintbrush::geometry(
            factory,
            stroke,
            Viewport {
                origin: Point::default(),
                scale: 1.0,
            },
            1.0,
        )?;
        return Ok(path.FillContainsPoint(point, None, flatten)?.as_bool()
            || path
                .StrokeContainsPoint(point, tolerance * 2.0, None, None, flatten)?
                .as_bool());
    }
    let geometry: ID2D1Geometry = match stroke.tool {
        Tool::Text => {
            filled = true;
            factory.CreateRectangleGeometry(&rect)?.cast()?
        }
        Tool::Rectangle => factory.CreateRectangleGeometry(&rect)?.cast()?,
        Tool::RoundedRectangle => factory
            .CreateRoundedRectangleGeometry(&D2D1_ROUNDED_RECT {
                rect,
                radiusX: stroke.corner_radius.min((rect.right - rect.left) / 2.0),
                radiusY: stroke.corner_radius.min((rect.bottom - rect.top) / 2.0),
            })?
            .cast()?,
        Tool::Serial => {
            filled = true;
            factory
                .CreateEllipseGeometry(&D2D1_ELLIPSE {
                    point: D2D_POINT_2F {
                        x: (a.x + b.x) / 2.0,
                        y: (a.y + b.y) / 2.0,
                    },
                    radiusX: (a.x - b.x).abs() / 2.0,
                    radiusY: (a.y - b.y).abs() / 2.0,
                })?
                .cast()?
        }
        Tool::Ellipse => factory
            .CreateEllipseGeometry(&D2D1_ELLIPSE {
                point: D2D_POINT_2F {
                    x: (a.x + b.x) / 2.0,
                    y: (a.y + b.y) / 2.0,
                },
                radiusX: (a.x - b.x).abs() / 2.0,
                radiusY: (a.y - b.y).abs() / 2.0,
            })?
            .cast()?,
        Tool::Triangle | Tool::Polygon => super::annotation_polygons::geometry(
            factory,
            stroke,
            Viewport {
                origin: Point::default(),
                scale: 1.0,
            },
        )?
        .cast()?,
        _ if stroke.has_straight_arrow() => {
            filled = false;
            let Some(arrow) = crate::arrow_geometry::build_path(&stroke.points, stroke.width)
            else {
                return Ok(false);
            };
            let viewport = Viewport {
                origin: Point::default(),
                scale: 1.0,
            };
            let head = super::annotation_linear::geometry(
                factory,
                arrow.arrow.head.into_iter(),
                true,
                viewport,
            )?;
            if head.FillContainsPoint(point, None, flatten)?.as_bool()
                || head
                    .StrokeContainsPoint(point, tolerance * 2.0, None, None, flatten)?
                    .as_bool()
            {
                return Ok(true);
            }
            super::annotation_linear::geometry(factory, arrow.shaft(), false, viewport)?.cast()?
        }
        Tool::Bezier => {
            filled = false;
            if stroke.arrow {
                if let Some((shaft, head)) = super::annotation_bezier::arrow_paths(
                    factory,
                    stroke,
                    Viewport {
                        origin: Point::default(),
                        scale: 1.0,
                    },
                )? {
                    if head.FillContainsPoint(point, None, flatten)?.as_bool()
                        || head
                            .StrokeContainsPoint(point, tolerance * 2.0, None, None, flatten)?
                            .as_bool()
                    {
                        return Ok(true);
                    }
                    return Ok(shaft
                        .StrokeContainsPoint(
                            point,
                            stroke.width + tolerance * 2.0,
                            None,
                            None,
                            flatten,
                        )?
                        .as_bool());
                }
            }
            super::annotation_bezier::geometry(
                factory,
                stroke,
                Viewport {
                    origin: Point::default(),
                    scale: 1.0,
                },
            )?
            .cast()?
        }
        Tool::Brush
        | Tool::Highlighter
        | Tool::Line
        | Tool::Polyline
        | Tool::Mosaic
        | Tool::Blur => {
            filled = false;
            if stroke.points.len() == 1 {
                if stroke.tool == Tool::Line {
                    return Ok(false);
                }
                return Ok((point.x - a.x).hypot(point.y - a.y) <= stroke.width / 2.0 + tolerance);
            }
            path(factory, &stroke.points, false)?
        }
        Tool::Pan | Tool::Select | Tool::Arrow | Tool::Paintbrush | Tool::Eraser => {
            return Ok(false)
        }
    };
    if filled && geometry.FillContainsPoint(point, None, flatten)?.as_bool() {
        return Ok(true);
    }
    if !stroke.visible_outline() && !matches!(stroke.tool, Tool::Mosaic | Tool::Blur) {
        return Ok(false);
    }
    // 选择容差允许点中虚线间隙，避免细短虚线难以操作；不改变实际绘制。
    let cap = if matches!(
        stroke.tool,
        Tool::Brush | Tool::Highlighter | Tool::Polyline | Tool::Mosaic | Tool::Blur
    ) {
        D2D1_CAP_STYLE_ROUND
    } else {
        D2D1_CAP_STYLE_FLAT
    };
    let style = factory.CreateStrokeStyle(
        &D2D1_STROKE_STYLE_PROPERTIES {
            startCap: cap,
            endCap: cap,
            lineJoin: D2D1_LINE_JOIN_ROUND,
            miterLimit: 10.0,
            ..Default::default()
        },
        None,
    )?;
    Ok(geometry
        .StrokeContainsPoint(point, stroke.width + tolerance * 2.0, &style, None, flatten)?
        .as_bool())
}

fn convert(point: Point) -> D2D_POINT_2F {
    D2D_POINT_2F {
        x: point.x,
        y: point.y,
    }
}

#[cfg(test)]
#[path = "selection_tests.rs"]
mod tests;
