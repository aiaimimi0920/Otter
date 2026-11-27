//! Build positively wound, rotation-preserving holes, then exclude their union
//! from the visible target once. Overlapping lights never darken each other.
use super::*;
use windows::core::{Interface, Result};

pub(in crate::desktop) unsafe fn hole(
    f: &ID2D1Factory,
    tool: Tool,
    r: D2D_RECT_F,
    radius: f32,
) -> Result<ID2D1Geometry> {
    match tool {
        Tool::SpotRect => f.CreateRectangleGeometry(&r)?.cast(),
        Tool::SpotRounded => {
            let radius = radius
                .min((r.right - r.left) * 0.5)
                .min((r.bottom - r.top) * 0.5);
            f.CreateRoundedRectangleGeometry(&D2D1_ROUNDED_RECT {
                rect: r,
                radiusX: radius,
                radiusY: radius,
            })?
            .cast()
        }
        Tool::SpotEllipse => f
            .CreateEllipseGeometry(&D2D1_ELLIPSE {
                point: D2D_POINT_2F {
                    x: (r.left + r.right) * 0.5,
                    y: (r.top + r.bottom) * 0.5,
                },
                radiusX: (r.right - r.left) * 0.5,
                radiusY: (r.bottom - r.top) * 0.5,
            })?
            .cast(),
        _ => unreachable!("not a spotlight"),
    }
}
pub(super) unsafe fn draw<'a>(
    target: &ID2D1RenderTarget,
    brush: &ID2D1SolidColorBrush,
    percent: u8,
    strokes: impl Iterator<Item = (&'a Stroke, Viewport)>,
) -> Result<()> {
    let f = target.GetFactory()?;
    let mut holes = Vec::new();
    for (s, v) in strokes {
        let [a, b] = s.points.as_slice() else {
            continue;
        };
        let (a, b) = (transform(*a, v), transform(*b, v));
        if a.x == b.x || a.y == b.y {
            continue;
        }
        let r = D2D_RECT_F {
            left: a.x.min(b.x),
            right: a.x.max(b.x),
            top: a.y.min(b.y),
            bottom: a.y.max(b.y),
        };
        let geometry = hole(&f, s.tool, r, s.corner_radius * v.scale)?;
        let geometry = if s.rotation != 0. {
            let (sin, cos) = s.rotation.to_radians().sin_cos();
            let (x, y) = ((a.x + b.x) * 0.5, (a.y + b.y) * 0.5);
            let mut m = D2D1_LAYER_PARAMETERS::default().maskTransform;
            m.M11 = cos;
            m.M12 = sin;
            m.M21 = -sin;
            m.M22 = cos;
            m.M31 = x - x * cos + y * sin;
            m.M32 = y - x * sin - y * cos;
            f.CreateTransformedGeometry(&geometry, &m)?.cast()?
        } else {
            geometry
        };
        holes.push(Some(geometry));
    }
    if holes.is_empty() {
        return Ok(());
    }
    let group = f.CreateGeometryGroup(D2D1_FILL_MODE_WINDING, &holes)?;
    let (left, top, right, bottom) = super::super::annotation_rotation::visible_bounds(target, 0.)?;
    let outer = f.CreateRectangleGeometry(&D2D_RECT_F {
        left,
        top,
        right,
        bottom,
    })?;
    let mask = f.CreatePathGeometry()?;
    let sink = mask.Open()?;
    outer.CombineWithGeometry(&group, D2D1_COMBINE_MODE_EXCLUDE, None, 0.25, &sink)?;
    sink.Close()?;
    brush.SetColor(&D2D1_COLOR_F {
        a: f32::from(percent) / 100.,
        ..Default::default()
    });
    target.FillGeometry(&mask, brush, None);
    Ok(())
}
