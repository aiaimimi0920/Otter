//! Image-space rotation; drawing, hit testing, and window regions share centers/angular direction.
use crate::viewport::Point;

pub fn center(points: &[Point]) -> Point {
    let Some(&first) = points.first() else {
        return Point::default();
    };
    let (mut low, mut high) = (first, first);
    for p in points {
        low.x = low.x.min(p.x);
        low.y = low.y.min(p.y);
        high.x = high.x.max(p.x);
        high.y = high.y.max(p.y);
    }
    Point {
        x: (low.x + high.x) / 2.0,
        y: (low.y + high.y) / 2.0,
    }
}

/// Fixed-center/angle point transformations reuse one rotation-coefficient pair across point sets.
#[derive(Clone, Copy)]
pub struct Rotation {
    center: Point,
    sin: f32,
    cos: f32,
}

impl Rotation {
    pub fn new(center: Point, degrees: f32) -> Self {
        let (sin, cos) = (degrees % 360.0).to_radians().sin_cos();
        Self { center, sin, cos }
    }

    #[inline]
    pub fn apply(self, point: Point) -> Point {
        let (x, y) = (point.x - self.center.x, point.y - self.center.y);
        Point {
            x: self.center.x + x * self.cos - y * self.sin,
            y: self.center.y + x * self.sin + y * self.cos,
        }
    }
}

pub fn rotate(point: Point, center: Point, degrees: f32) -> Point {
    Rotation::new(center, degrees).apply(point)
}

/// Compensate rotation-center changes after point editing to preserve untouched points' visible positions; callers validate output ranges.
pub fn preserve_center(before: &[Point], after: &mut [Point], degrees: f32) {
    let old = center(before);
    let new = center(after);
    let delta = Point {
        x: old.x - new.x,
        y: old.y - new.y,
    };
    let rotated = rotate(delta, Point::default(), degrees);
    for point in after {
        point.x += delta.x - rotated.x;
        point.y += delta.y - rotated.y;
    }
}

pub fn bounds(bounds: (f32, f32, f32, f32), center: Point, degrees: f32) -> (f32, f32, f32, f32) {
    let (left, top, right, bottom) = bounds;
    let mut out = (
        f32::INFINITY,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
    );
    for point in [
        Point { x: left, y: top },
        Point { x: right, y: top },
        Point {
            x: right,
            y: bottom,
        },
        Point { x: left, y: bottom },
    ] {
        let p = rotate(point, center, degrees);
        out.0 = out.0.min(p.x);
        out.1 = out.1.min(p.y);
        out.2 = out.2.max(p.x);
        out.3 = out.3.max(p.y);
    }
    out
}
