//! 与旧形状合同一致的顶点和圆角；边数固定上限，几何不依赖 Win32。
use crate::viewport::Point;

pub fn vertices(a: Point, b: Point, triangle: bool, sides: u8) -> Vec<Point> {
    let (left, top) = (a.x.min(b.x), a.y.min(b.y));
    let (width, height) = ((a.x - b.x).abs(), (a.y - b.y).abs());
    if triangle {
        return vec![
            Point {
                x: left + width / 2.0,
                y: top,
            },
            Point {
                x: left + width,
                y: top + height,
            },
            Point {
                x: left,
                y: top + height,
            },
        ];
    }
    let count = sides.clamp(3, 64);
    (0..count)
        .map(|i| {
            let angle =
                std::f32::consts::TAU * i as f32 / count as f32 - std::f32::consts::FRAC_PI_2;
            Point {
                x: left + width / 2.0 + width / 2.0 * angle.cos(),
                y: top + height / 2.0 + height / 2.0 * angle.sin(),
            }
        })
        .collect()
}

pub struct Corner {
    pub point: Point,
    pub entry: Point,
    pub exit: Point,
}
pub fn corners(points: &[Point], radius: f32) -> Vec<Corner> {
    if points.len() < 3 || points.len() > 64 {
        return Vec::new();
    }
    points
        .iter()
        .enumerate()
        .map(|(index, &point)| {
            let previous = points[(index + points.len() - 1) % points.len()];
            let next = points[(index + 1) % points.len()];
            let direction = |to: Point| {
                let (x, y) = (to.x - point.x, to.y - point.y);
                let length = x.hypot(y);
                if length < 0.0001 {
                    (0.0, 0.0, 0.0)
                } else {
                    (x / length, y / length, length)
                }
            };
            let (px, py, pl) = direction(previous);
            let (nx, ny, nl) = direction(next);
            let angle = (px * nx + py * ny).clamp(-1.0, 1.0).acos();
            let tangent = (angle / 2.0).tan();
            let offset = if angle > 0.0001 && tangent > 0.0001 {
                radius / tangent
            } else {
                radius
            };
            let offset = offset.min(radius).min(pl.min(nl) / 2.0).max(0.0);
            Corner {
                point,
                entry: Point {
                    x: point.x + px * offset,
                    y: point.y + py * offset,
                },
                exit: Point {
                    x: point.x + nx * offset,
                    y: point.y + ny * offset,
                },
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn triangle_is_not_an_inscribed_three_sided_polygon() {
        let a = Point::default();
        let b = Point { x: 100.0, y: 80.0 };
        let triangle = vertices(a, b, true, 3);
        assert_eq!(triangle[1], b);
        assert_eq!(triangle, vertices(b, a, true, 3));
        assert_ne!(triangle, vertices(a, b, false, 3));
        assert_eq!(vertices(a, b, false, 255).len(), 64);
    }
    #[test]
    fn rounded_corners_are_bounded_by_half_adjacent_edges() {
        let points = vertices(Point::default(), Point { x: 100.0, y: 80.0 }, true, 3);
        for corner in corners(&points, 10000.0) {
            assert!(corner.entry.x.is_finite() && corner.exit.y.is_finite());
            assert!(
                (corner.entry.x - corner.point.x).hypot(corner.entry.y - corner.point.y) <= 50.001
            );
        }
        for corner in corners(&points, 0.0) {
            assert_eq!(corner.entry, corner.point);
            assert_eq!(corner.exit, corner.point);
        }
    }
}
