//! 图像空间旋转；绘制、命中和窗口区域共享同一中心与角度方向。
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

pub fn rotate(point: Point, center: Point, degrees: f32) -> Point {
    let (sin, cos) = (degrees % 360.0).to_radians().sin_cos();
    let (x, y) = (point.x - center.x, point.y - center.y);
    Point {
        x: center.x + x * cos - y * sin,
        y: center.y + x * sin + y * cos,
    }
}

/// 编辑点集后补偿旋转中心变化，维持未编辑点的可见位置；调用者校验结果范围。
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
