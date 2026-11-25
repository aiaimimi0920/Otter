//! 连续原生路径保持整条笔迹的虚线相位；一次 DrawGeometry，不逐点重复绘制圆帽。
use crate::{
    editor::Stroke,
    viewport::{Point, Viewport},
};
use windows::{
    core::Result,
    Win32::Graphics::Direct2D::{Common::*, *},
};

pub(super) unsafe fn draw_cached(
    target: &ID2D1RenderTarget,
    brush: &ID2D1Brush,
    stroke: &Stroke,
    viewport: Viewport,
    style: Option<&ID2D1StrokeStyle>,
    cacheable: bool,
    cache: &mut super::effect_cache::annotation_cache::Cache,
) -> Result<()> {
    let Some(&first) = stroke.points.first() else {
        return Ok(());
    };
    let width = stroke.width * viewport.scale;
    if stroke.points.len() == 1 {
        target.FillEllipse(
            &D2D1_ELLIPSE {
                point: D2D_POINT_2F {
                    x: viewport.origin.x + first.x * viewport.scale,
                    y: viewport.origin.y + first.y * viewport.scale,
                },
                radiusX: width / 2.0,
                radiusY: width / 2.0,
            },
            brush,
        );
        return Ok(());
    }
    let factory = target.GetFactory()?;
    let geometry = cache.single_path(cacheable, stroke, viewport.scale, |local| {
        build_geometry(&factory, stroke, local)
    })?;
    super::effect_cache::annotation_cache::with_origin(target, viewport.origin, |target| {
        target.DrawGeometry(&geometry, brush, width, style);
    });
    Ok(())
}

unsafe fn build_geometry(
    factory: &ID2D1Factory,
    stroke: &Stroke,
    viewport: Viewport,
) -> Result<ID2D1PathGeometry> {
    let convert = |p: Point| D2D_POINT_2F {
        x: viewport.origin.x + p.x * viewport.scale,
        y: viewport.origin.y + p.y * viewport.scale,
    };
    let geometry = factory.CreatePathGeometry()?;
    let sink = geometry.Open()?;
    sink.BeginFigure(convert(stroke.points[0]), D2D1_FIGURE_BEGIN_HOLLOW);
    for &point in &stroke.points[1..] {
        sink.AddLine(convert(point));
    }
    sink.EndFigure(D2D1_FIGURE_END_OPEN);
    sink.Close()?;
    Ok(geometry)
}
