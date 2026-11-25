//! 基线的有界曲线采样与弧长裁短；控制点仍是编辑源数据。
use crate::viewport::Point;

pub struct Geometry {
    pub shaft: Vec<Point>,
    pub head: [Point; 3],
}

pub fn build(points: &[Point], width: f32) -> Option<Geometry> {
    if points.len() < 4
        || points.len() > 193
        || !(points.len() - 1).is_multiple_of(3)
        || !width.is_finite()
        || width <= 0.0
        || points.iter().any(|p| !p.x.is_finite() || !p.y.is_finite())
    {
        return None;
    }
    let segments = (points.len() - 1) / 3;
    let depth = (4096usize / segments).ilog2().min(9);
    let mut samples = Vec::with_capacity(segments * (1 << depth) + 1);
    samples.push(points[0]);
    for segment in points.windows(4).step_by(3) {
        subdivide(
            [segment[0], segment[1], segment[2], segment[3]],
            depth,
            &mut samples,
        );
    }
    let end = *points.last()?;
    let tangent = points[..points.len() - 1]
        .iter()
        .rev()
        .find(|p| distance(**p, end) > 0.001)?;
    let requested = (width * 6.0).max(24.0);
    let total: f32 = samples.windows(2).map(|p| distance(p[0], p[1])).sum();
    let length = requested.min(total / 2.0);
    if !length.is_finite() || length <= 0.0 {
        return None;
    }
    let tangent_length = distance(*tangent, end);
    let ux = (end.x - tangent.x) / tangent_length;
    let uy = (end.y - tangent.y) / tangent_length;
    let base = Point {
        x: end.x - ux * length,
        y: end.y - uy * length,
    };
    let half = (width * 5.0).max(16.0) * length / requested / 2.0;
    let head = [
        end,
        Point {
            x: base.x - uy * half,
            y: base.y + ux * half,
        },
        Point {
            x: base.x + uy * half,
            y: base.y - ux * half,
        },
    ];
    let mut trim = 0.0;
    let mut index = samples.len() - 1;
    while index > 0 && trim < length {
        trim += distance(samples[index], samples[index - 1]);
        index -= 1;
    }
    samples.truncate(index + 1);
    samples.push(base);
    Some(Geometry {
        shaft: samples,
        head,
    })
}

fn distance(a: Point, b: Point) -> f32 {
    (a.x - b.x).hypot(a.y - b.y)
}
fn midpoint(a: Point, b: Point) -> Point {
    Point {
        x: (a.x + b.x) / 2.0,
        y: (a.y + b.y) / 2.0,
    }
}
fn subdivide(p: [Point; 4], depth: u32, output: &mut Vec<Point>) {
    let curvature =
        |a: Point, b: Point, c: Point| (a.x - 2.0 * b.x + c.x).hypot(a.y - 2.0 * b.y + c.y);
    if depth == 0 || curvature(p[0], p[1], p[2]).max(curvature(p[1], p[2], p[3])) <= 0.25 {
        output.push(p[3]);
        return;
    }
    let a = midpoint(p[0], p[1]);
    let b = midpoint(p[1], p[2]);
    let c = midpoint(p[2], p[3]);
    let d = midpoint(a, b);
    let e = midpoint(b, c);
    let f = midpoint(d, e);
    subdivide([p[0], a, d, f], depth - 1, output);
    subdivide([f, e, c, p[3]], depth - 1, output);
}

#[cfg(test)]
mod tests {
    use super::*;
    fn p(x: f32, y: f32) -> Point {
        Point { x, y }
    }
    #[test]
    fn straight_cubic_uses_half_arc_limit_and_scaled_head_width() {
        let arrow = build(&[p(0.0, 0.0), p(4.0, 0.0), p(8.0, 0.0), p(12.0, 0.0)], 4.0).unwrap();
        assert_eq!(arrow.shaft.last(), Some(&p(6.0, 0.0)));
        assert_eq!(arrow.head, [p(12.0, 0.0), p(6.0, 2.5), p(6.0, -2.5)]);
    }
    #[test]
    fn cusps_are_not_collapsed_to_chord_and_sampling_is_bounded() {
        let points = [p(0.0, 0.0), p(1000.0, 0.0), p(-1000.0, 0.0), p(10.0, 0.0)];
        let arrow = build(&points, 4.0).unwrap();
        assert!(arrow.shaft.len() > 4);
        assert!(arrow.shaft.len() <= 514);
        assert!(build(&[p(1.0, 1.0); 4], 4.0).is_none());
        assert!(build(&points, f32::NAN).is_none());
    }
}
