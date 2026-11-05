//! Canvas 当前路径转换为 D2D 几何；不把闭合、圆弧或多个子路径压平成股票专用图形。
use super::commands::Segment;
use windows::{
    core::Result,
    Win32::Graphics::Direct2D::{Common::*, *},
};
fn point(p: [f32; 2]) -> D2D_POINT_2F {
    D2D_POINT_2F { x: p[0], y: p[1] }
}
pub(super) fn geometry(
    factory: &ID2D1Factory,
    path: &[Segment],
    evenodd: bool,
    fill: bool,
) -> Result<ID2D1PathGeometry> {
    unsafe {
        let geometry = factory.CreatePathGeometry()?;
        let sink = geometry.Open()?;
        sink.SetFillMode(if evenodd {
            D2D1_FILL_MODE_ALTERNATE
        } else {
            D2D1_FILL_MODE_WINDING
        });
        let mut current = None;
        let mut start = [0.; 2];
        for segment in path {
            let begin = match segment {
                Segment::Move { point } => Some(*point),
                Segment::Line { point } if current.is_none() => Some(*point),
                Segment::Curve { points } if current.is_none() => Some(points[0]),
                Segment::Arc {
                    center,
                    radius,
                    start,
                    ..
                } if current.is_none() => Some([
                    center[0] + radius * start.cos(),
                    center[1] + radius * start.sin(),
                ]),
                _ => None,
            };
            if let Some(p) = begin {
                if current.is_some() {
                    sink.EndFigure(if fill {
                        D2D1_FIGURE_END_CLOSED
                    } else {
                        D2D1_FIGURE_END_OPEN
                    });
                }
                sink.BeginFigure(point(p), D2D1_FIGURE_BEGIN_FILLED);
                start = p;
                current = Some(p);
            }
            match segment {
                Segment::Move { .. } => {}
                Segment::Line { point: p } => {
                    sink.AddLine(point(*p));
                    current = Some(*p);
                }
                Segment::Curve { points } => {
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: point(points[0]),
                        point2: point(points[1]),
                        point3: point(points[2]),
                    });
                    current = Some(points[2]);
                }
                Segment::Arc {
                    center,
                    radius,
                    start: angle,
                    end,
                    ccw,
                } => {
                    let first = [
                        center[0] + radius * angle.cos(),
                        center[1] + radius * angle.sin(),
                    ];
                    sink.AddLine(point(first));
                    current = Some(first);
                    if *radius == 0. {
                        continue;
                    }
                    let tau = std::f32::consts::TAU;
                    let raw = end - angle;
                    let sweep = if *ccw {
                        if raw <= -tau {
                            -tau
                        } else {
                            -(-raw).rem_euclid(tau)
                        }
                    } else if raw >= tau {
                        tau
                    } else {
                        raw.rem_euclid(tau)
                    };
                    let count = (sweep.abs() / std::f32::consts::PI).ceil() as u32;
                    for i in 1..=count {
                        let a = angle + sweep * i as f32 / count as f32;
                        let p = [center[0] + radius * a.cos(), center[1] + radius * a.sin()];
                        sink.AddArc(&D2D1_ARC_SEGMENT {
                            point: point(p),
                            size: D2D_SIZE_F {
                                width: *radius,
                                height: *radius,
                            },
                            rotationAngle: 0.,
                            sweepDirection: if *ccw {
                                D2D1_SWEEP_DIRECTION_COUNTER_CLOCKWISE
                            } else {
                                D2D1_SWEEP_DIRECTION_CLOCKWISE
                            },
                            arcSize: D2D1_ARC_SIZE_SMALL,
                        });
                        current = Some(p);
                    }
                }
                Segment::Close if current.is_some() => {
                    sink.EndFigure(D2D1_FIGURE_END_CLOSED);
                    // closePath 后当前点回到起点，后续 lineTo 可以继续同一起点。
                    sink.BeginFigure(point(start), D2D1_FIGURE_BEGIN_FILLED);
                    current = Some(start);
                }
                Segment::Close => {}
            }
        }
        if current.is_some() {
            sink.EndFigure(if fill {
                D2D1_FIGURE_END_CLOSED
            } else {
                D2D1_FIGURE_END_OPEN
            });
        }
        sink.Close()?;
        Ok(geometry)
    }
}
