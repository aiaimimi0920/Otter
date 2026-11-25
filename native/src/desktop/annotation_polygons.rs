//! Native closed paths share the vertex contract; fill, dashed borders, and outer opacity reuse the shape pipeline.
use crate::{
    editor::{Stroke, Tool},
    polygon_geometry,
    viewport::{Point, Viewport},
};
use windows::{
    core::Result,
    Win32::Graphics::Direct2D::{Common::*, *},
};

pub(super) unsafe fn draw_cached(
    target: &ID2D1RenderTarget,
    brush: &ID2D1SolidColorBrush,
    stroke: &Stroke,
    viewport: Viewport,
    dash: Option<&ID2D1StrokeStyle>,
    cacheable: bool,
    cache: &mut super::effect_cache::annotation_cache::Cache,
) -> Result<()> {
    let factory = target.GetFactory()?;
    let geometry = cache.single_path(cacheable, stroke, viewport.scale, |local| {
        geometry(&factory, stroke, local)
    })?;
    super::effect_cache::annotation_cache::with_origin(target, viewport.origin, |target| {
        if let Some(fill) = stroke.fill {
            super::annotation_shapes::set_color(brush, fill, stroke.paint_alpha.fill());
            target.FillGeometry(&geometry, brush, None);
        }
        super::annotation_shapes::set_color(brush, stroke.color, stroke.paint_alpha.stroke());
        if stroke.visible_outline() {
            target.DrawGeometry(&geometry, brush, stroke.width * viewport.scale, dash);
        }
    });
    Ok(())
}

pub(super) unsafe fn geometry(
    factory: &ID2D1Factory,
    stroke: &Stroke,
    viewport: Viewport,
) -> Result<ID2D1PathGeometry> {
    let vertices = polygon_geometry::vertices(
        stroke.points[0],
        *stroke.points.last().unwrap(),
        stroke.tool == Tool::Triangle,
        stroke.sides,
    );
    let corners = polygon_geometry::corners(&vertices, stroke.corner_radius);
    let transform = |p: Point| D2D_POINT_2F {
        x: viewport.origin.x + p.x * viewport.scale,
        y: viewport.origin.y + p.y * viewport.scale,
    };
    let geometry = factory.CreatePathGeometry()?;
    let sink = geometry.Open()?;
    sink.BeginFigure(transform(corners[0].exit), D2D1_FIGURE_BEGIN_FILLED);
    for corner in corners.iter().skip(1).chain(corners.iter().take(1)) {
        sink.AddLine(transform(corner.entry));
        if stroke.corner_radius > 0.0 {
            sink.AddQuadraticBezier(&D2D1_QUADRATIC_BEZIER_SEGMENT {
                point1: transform(corner.point),
                point2: transform(corner.exit),
            });
        } else {
            sink.AddLine(transform(corner.exit));
        }
    }
    sink.EndFigure(D2D1_FIGURE_END_CLOSED);
    sink.Close()?;
    Ok(geometry)
}
