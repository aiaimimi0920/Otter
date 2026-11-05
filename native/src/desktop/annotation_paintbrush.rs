//! 笔刷每层将圆盘与连接带合并为一个 winding 路径，自交只合成一次透明度。
use crate::{
    editor::Stroke,
    paintbrush_samples::{bands, Disk, Samples},
    viewport::{Point, Viewport},
};
use windows::{
    core::Result,
    Win32::Graphics::Direct2D::{Common::*, *},
};

pub(super) unsafe fn geometry(
    factory: &ID2D1Factory,
    stroke: &Stroke,
    viewport: Viewport,
    scale: f32,
) -> Result<ID2D1PathGeometry> {
    let path = factory.CreatePathGeometry()?;
    let sink = path.Open()?;
    sink.SetFillMode(D2D1_FILL_MODE_WINDING);
    let convert = |p: Point| D2D_POINT_2F {
        x: viewport.origin.x + p.x * viewport.scale,
        y: viewport.origin.y + p.y * viewport.scale,
    };
    let mut previous: Option<Disk> = None;
    if let Some(samples) = Samples::new(&stroke.points, stroke.width, Some(&stroke.brush_weights)) {
        for mut disk in crate::brush_reduction::Reduced::new(samples) {
            disk.radius *= scale;
            let radius = disk.radius * viewport.scale;
            let center = convert(disk.point);
            sink.BeginFigure(
                D2D_POINT_2F {
                    x: center.x + radius,
                    y: center.y,
                },
                D2D1_FIGURE_BEGIN_FILLED,
            );
            for x in [center.x - radius, center.x + radius] {
                sink.AddArc(&D2D1_ARC_SEGMENT {
                    point: D2D_POINT_2F { x, y: center.y },
                    size: D2D_SIZE_F {
                        width: radius,
                        height: radius,
                    },
                    rotationAngle: 0.0,
                    sweepDirection: D2D1_SWEEP_DIRECTION_CLOCKWISE,
                    arcSize: D2D1_ARC_SIZE_SMALL,
                });
            }
            sink.EndFigure(D2D1_FIGURE_END_CLOSED);
            if let Some(a) = previous {
                let length = (disk.point.x - a.point.x)
                    .hypot(disk.point.y - a.point.y)
                    .max(f32::MIN_POSITIVE);
                let nx = -(disk.point.y - a.point.y) / length;
                let ny = (disk.point.x - a.point.x) / length;
                let corners = [
                    Point {
                        x: a.point.x - nx * a.radius,
                        y: a.point.y - ny * a.radius,
                    },
                    Point {
                        x: disk.point.x - nx * disk.radius,
                        y: disk.point.y - ny * disk.radius,
                    },
                    Point {
                        x: disk.point.x + nx * disk.radius,
                        y: disk.point.y + ny * disk.radius,
                    },
                    Point {
                        x: a.point.x + nx * a.radius,
                        y: a.point.y + ny * a.radius,
                    },
                ];
                sink.BeginFigure(convert(corners[0]), D2D1_FIGURE_BEGIN_FILLED);
                for &point in &corners[1..] {
                    sink.AddLine(convert(point));
                }
                sink.EndFigure(D2D1_FIGURE_END_CLOSED);
            }
            previous = Some(disk);
        }
    }
    sink.Close()?;
    Ok(path)
}

pub(super) unsafe fn draw(
    target: &ID2D1RenderTarget,
    brush: &ID2D1SolidColorBrush,
    stroke: &Stroke,
    viewport: Viewport,
) -> Result<()> {
    let crate::stroke_style::Color(r, g, b) = stroke.color;
    for (scale, alpha) in bands(stroke.opacity * stroke.paint_alpha.stroke()) {
        if alpha <= 0.0 {
            continue;
        }
        let path = geometry(&target.GetFactory()?, stroke, viewport, scale)?;
        brush.SetColor(&D2D1_COLOR_F {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: alpha,
        });
        target.FillGeometry(&path, brush, None);
    }
    Ok(())
}
