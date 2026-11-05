//! 屏幕、导出与命中共用 D2D 三次曲线路径，不将控制折线当作可见笔迹。
use crate::{
    editor::Stroke,
    viewport::{Point, Viewport},
};
use windows::{
    core::Result,
    Win32::Graphics::Direct2D::{Common::*, *},
};

pub(super) unsafe fn arrow_paths(
    factory: &ID2D1Factory,
    stroke: &Stroke,
    viewport: Viewport,
) -> Result<Option<(ID2D1PathGeometry, ID2D1PathGeometry)>> {
    let Some(arrow) = crate::bezier_arrow::build(&stroke.points, stroke.width) else {
        return Ok(None);
    };
    let path = |points: &[Point], closed: bool| -> Result<ID2D1PathGeometry> {
        let convert = |p: Point| D2D_POINT_2F {
            x: viewport.origin.x + p.x * viewport.scale,
            y: viewport.origin.y + p.y * viewport.scale,
        };
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
        for &point in &points[1..] {
            sink.AddLine(convert(point));
        }
        sink.EndFigure(if closed {
            D2D1_FIGURE_END_CLOSED
        } else {
            D2D1_FIGURE_END_OPEN
        });
        sink.Close()?;
        Ok(geometry)
    };
    Ok(Some((path(&arrow.shaft, false)?, path(&arrow.head, true)?)))
}

pub(super) unsafe fn geometry(
    factory: &ID2D1Factory,
    stroke: &Stroke,
    viewport: Viewport,
) -> Result<ID2D1PathGeometry> {
    let geometry = factory.CreatePathGeometry()?;
    let sink = geometry.Open()?;
    if stroke.points.len() >= 4 && (stroke.points.len() - 1).is_multiple_of(3) {
        let convert = |point: Point| D2D_POINT_2F {
            x: viewport.origin.x + point.x * viewport.scale,
            y: viewport.origin.y + point.y * viewport.scale,
        };
        sink.BeginFigure(convert(stroke.points[0]), D2D1_FIGURE_BEGIN_HOLLOW);
        for segment in stroke.points[1..].chunks_exact(3) {
            sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                point1: convert(segment[0]),
                point2: convert(segment[1]),
                point3: convert(segment[2]),
            });
        }
        sink.EndFigure(D2D1_FIGURE_END_OPEN);
    }
    sink.Close()?;
    Ok(geometry)
}
