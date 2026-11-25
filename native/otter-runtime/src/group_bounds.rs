//! 组变换的图像空间视觉边界；曲线极值无需采样数组，箭头头部复用有界几何实现。
use crate::{
    editor::{Stroke, Tool},
    rotation,
    viewport::Point,
};

#[derive(Clone, Copy)]
pub struct Bounds {
    pub low: Point,
    pub high: Point,
}
impl Bounds {
    fn include(&mut self, p: Point) {
        self.low.x = self.low.x.min(p.x);
        self.low.y = self.low.y.min(p.y);
        self.high.x = self.high.x.max(p.x);
        self.high.y = self.high.y.max(p.y);
    }
    pub fn center(self) -> Point {
        Point {
            x: (self.low.x + self.high.x) / 2.0,
            y: (self.low.y + self.high.y) / 2.0,
        }
    }
}

pub fn center<'a>(strokes: impl Iterator<Item = &'a Stroke>) -> Option<Point> {
    let mut result: Option<Bounds> = None;
    for stroke in strokes {
        let bounds = bounds(stroke)?;
        if let Some(total) = &mut result {
            total.include(bounds.low);
            total.include(bounds.high);
        } else {
            result = Some(bounds);
        }
    }
    result.map(Bounds::center)
}

pub fn bounds(stroke: &Stroke) -> Option<Bounds> {
    let first = *stroke.points.first()?;
    let pivot = rotation::center(&stroke.points);
    let rotated = |p| rotation::rotate(p, pivot, stroke.rotation);
    let first = rotated(first);
    let mut out = Bounds {
        low: first,
        high: first,
    };
    let rect = matches!(
        stroke.tool,
        Tool::Rectangle
            | Tool::RoundedRectangle
            | Tool::Ellipse
            | Tool::Triangle
            | Tool::Polygon
            | Tool::Text
            | Tool::Serial
    );
    if rect {
        let a = stroke.points[0];
        let b = *stroke.points.last()?;
        for p in [a, b, Point { x: a.x, y: b.y }, Point { x: b.x, y: a.y }] {
            out.include(rotated(p));
        }
    } else if stroke.tool == Tool::Bezier && stroke.points.len() >= 4 {
        for segment in stroke.points.windows(4).step_by(3) {
            cubic_bounds(
                [segment[0], segment[1], segment[2], segment[3]].map(rotated),
                &mut out,
            );
        }
        if stroke.arrow {
            if let Some(arrow) = crate::bezier_arrow::build(&stroke.points, stroke.width) {
                for p in arrow.head {
                    out.include(rotated(p));
                }
            }
        }
    } else if stroke.has_straight_arrow() {
        if let Some(arrow) = crate::arrow_geometry::build_path(&stroke.points, stroke.width) {
            for point in arrow.shaft().chain(arrow.arrow.head) {
                out.include(rotated(point));
            }
        }
    } else {
        for p in &stroke.points {
            out.include(rotated(*p));
        }
    }
    let padding = if rect {
        0.0
    } else {
        stroke.width.max(1.0) / 2.0
            + if stroke.tool == Tool::Bezier {
                0.25
            } else {
                0.0
            }
    };
    out.low.x -= padding;
    out.low.y -= padding;
    out.high.x += padding;
    out.high.y += padding;
    Some(out)
}

fn cubic_bounds(p: [Point; 4], out: &mut Bounds) {
    out.include(p[0]);
    out.include(p[3]);
    for axis in [|p: Point| p.x as f64, |p: Point| p.y as f64] {
        let [v0, v1, v2, v3] = p.map(axis);
        let a = -v0 + 3.0 * v1 - 3.0 * v2 + v3;
        let b = 2.0 * (v0 - 2.0 * v1 + v2);
        let c = v1 - v0;
        let roots = if a.abs() < 1e-12 {
            [if b.abs() > 1e-12 { -c / b } else { -1.0 }, -1.0]
        } else {
            let d = b * b - 4.0 * a * c;
            if d < 0.0 {
                [-1.0; 2]
            } else {
                [(-b + d.sqrt()) / (2.0 * a), (-b - d.sqrt()) / (2.0 * a)]
            }
        };
        for t in roots.into_iter().filter(|t| *t > 0.0 && *t < 1.0) {
            let u = 1.0 - t;
            let at = |axis: fn(Point) -> f64| {
                (u * u * u * axis(p[0])
                    + 3.0 * u * u * t * axis(p[1])
                    + 3.0 * u * t * t * axis(p[2])
                    + t * t * t * axis(p[3])) as f32
            };
            out.include(Point {
                x: at(|p| p.x as f64),
                y: at(|p| p.y as f64),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::Editor;
    fn stroke(tool: Tool, a: Point, b: Point) -> Stroke {
        let mut editor = Editor::default();
        editor.begin(tool, a).unwrap();
        editor.extend(b).unwrap();
        assert!(editor.commit());
        editor.elements()[0].clone()
    }
    #[test]
    fn rotated_shape_and_asymmetric_group_use_union_not_average() {
        let mut shape = stroke(
            Tool::Rectangle,
            Point::default(),
            Point { x: 20.0, y: 10.0 },
        );
        shape.rotation = 90.0;
        let result = bounds(&shape).unwrap();
        assert!((result.low.x - 5.0).abs() < 0.001);
        assert!((result.high.y - 15.0).abs() < 0.001);
        let other = stroke(
            Tool::Rectangle,
            Point { x: 30.0, y: 0.0 },
            Point { x: 100.0, y: 10.0 },
        );
        let pivot = center([&shape, &other].into_iter()).unwrap();
        assert!((pivot.x - 52.5).abs() < 0.001);
        assert!((pivot.y - 5.0).abs() < 0.001);
        assert!(center(std::iter::empty()).is_none());
    }
    #[test]
    fn cubic_extremum_excludes_control_polygon_and_includes_padding() {
        let mut curve = stroke(Tool::Bezier, Point::default(), Point { x: 100.0, y: 0.0 });
        curve.points = vec![
            Point::default(),
            Point { x: 0.0, y: 100.0 },
            Point { x: 100.0, y: 100.0 },
            Point { x: 100.0, y: 0.0 },
        ];
        curve.width = 4.0;
        let result = bounds(&curve).unwrap();
        assert_eq!(result.low.y, -2.25);
        assert_eq!(result.high.y, 77.25);
        assert_eq!(result.center(), Point { x: 50.0, y: 37.5 });
    }
    #[test]
    fn arrow_head_and_path_width_expand_visual_bounds() {
        let mut arrow = stroke(Tool::Arrow, Point::default(), Point { x: 100.0, y: 0.0 });
        arrow.width = 4.0;
        let result = bounds(&arrow).unwrap();
        assert_eq!(result.low.y, -12.0);
        assert_eq!(result.high.y, 12.0);
        assert_eq!(result.low.x, -2.0);
        assert_eq!(result.high.x, 102.0);
    }
}
