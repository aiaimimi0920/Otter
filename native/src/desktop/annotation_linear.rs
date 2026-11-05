//! line/arrow共用多点路径：平头零长路径为空，箭头前缀和头与命中/边界共用纯几何。
use crate::{
    editor::Stroke,
    viewport::{Point, Viewport},
};
use windows::{
    core::Result,
    Win32::Graphics::Direct2D::{Common::*, *},
};

pub(super) unsafe fn geometry(
    factory: &ID2D1Factory,
    points: impl Iterator<Item = Point>,
    closed: bool,
    viewport: Viewport,
) -> Result<ID2D1PathGeometry> {
    let convert = |p: Point| D2D_POINT_2F {
        x: viewport.origin.x + p.x * viewport.scale,
        y: viewport.origin.y + p.y * viewport.scale,
    };
    let path = factory.CreatePathGeometry()?;
    let sink = path.Open()?;
    let mut points = points;
    if let Some(first) = points.next() {
        sink.BeginFigure(
            convert(first),
            if closed {
                D2D1_FIGURE_BEGIN_FILLED
            } else {
                D2D1_FIGURE_BEGIN_HOLLOW
            },
        );
        for point in points {
            sink.AddLine(convert(point));
        }
        sink.EndFigure(if closed {
            D2D1_FIGURE_END_CLOSED
        } else {
            D2D1_FIGURE_END_OPEN
        });
    }
    sink.Close()?;
    Ok(path)
}

pub(super) unsafe fn draw(
    target: &ID2D1RenderTarget,
    brush: &ID2D1Brush,
    stroke: &Stroke,
    viewport: Viewport,
    style: Option<&ID2D1StrokeStyle>,
) -> Result<()> {
    if stroke.points.len() < 2 || stroke.points.iter().all(|p| *p == stroke.points[0]) {
        return Ok(());
    }
    let factory = target.GetFactory()?;
    let arrow = stroke
        .has_straight_arrow()
        .then(|| crate::arrow_geometry::build_path(&stroke.points, stroke.width))
        .flatten();
    let shaft = if let Some(arrow) = &arrow {
        geometry(&factory, arrow.shaft(), false, viewport)?
    } else {
        geometry(&factory, stroke.points.iter().copied(), false, viewport)?
    };
    target.DrawGeometry(&shaft, brush, stroke.width * viewport.scale, style);
    if let Some(arrow) = arrow {
        let head = geometry(&factory, arrow.arrow.head.into_iter(), true, viewport)?;
        target.FillGeometry(&head, brush, None);
    }
    Ok(())
}
