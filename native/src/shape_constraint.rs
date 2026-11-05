//! 形状创建端点约束；按方向限制可用尺寸，锁定比例时两轴一起收缩。
use crate::viewport::Point;

pub fn endpoint(start: Point, current: Point, size: Point, square: bool, step: bool) -> Point {
    configured_endpoint(
        start,
        current,
        size,
        square,
        if step { 10.0 } else { 0.0 },
        1.0,
    )
}

/// 步进先量化尺寸，再限制可用范围；比例锁定时两轴不得分别截断。
pub fn configured_endpoint(
    start: Point,
    current: Point,
    size: Point,
    square: bool,
    step: f32,
    ratio: f32,
) -> Point {
    let ratio = if ratio.is_finite() && ratio > 0.0 {
        ratio
    } else {
        1.0
    };
    let start = clamp(start, size);
    let current = if square {
        current
    } else {
        clamp(current, size)
    };
    let dx = current.x - start.x;
    let dy = current.y - start.y;
    let sx = if dx >= 0.0 { 1.0 } else { -1.0 };
    let sy = if dy >= 0.0 { 1.0 } else { -1.0 };
    let available_x = if sx > 0.0 { size.x - start.x } else { start.x };
    let available_y = if sy > 0.0 { size.y - start.y } else { start.y };
    let quantize = |value: f32, maximum: f32| {
        let value = if step.is_finite() && step > 0.0 {
            (value / step + 0.5).floor() * step
        } else {
            value
        };
        value.min(maximum)
    };
    let (width, height) = if square {
        let height = quantize(
            (dx.abs() / ratio).max(dy.abs()),
            (available_x / ratio).min(available_y),
        );
        (height * ratio, height)
    } else {
        (
            quantize(dx.abs(), available_x),
            quantize(dy.abs(), available_y),
        )
    };
    Point {
        x: start.x + sx * width,
        y: start.y + sy * height,
    }
}

fn clamp(point: Point, size: Point) -> Point {
    Point {
        x: point.x.clamp(0.0, size.x),
        y: point.y.clamp(0.0, size.y),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn p(x: f32, y: f32) -> Point {
        Point { x, y }
    }
    #[test]
    fn square_uses_larger_axis_and_shrinks_both_at_boundary() {
        let size = p(100.0, 80.0);
        assert_eq!(
            endpoint(p(20.0, 20.0), p(50.0, 30.0), size, true, false),
            p(50.0, 50.0)
        );
        assert_eq!(
            endpoint(p(80.0, 40.0), p(130.0, 70.0), size, true, false),
            p(100.0, 60.0)
        );
        assert_eq!(
            endpoint(p(20.0, 10.0), p(-50.0, -30.0), size, true, false),
            p(10.0, 0.0)
        );
    }
    #[test]
    fn step_quantizes_magnitudes_then_clamps_without_breaking_square() {
        let size = p(100.0, 80.0);
        assert_eq!(
            endpoint(p(40.0, 40.0), p(25.0, 55.0), size, false, true),
            p(20.0, 60.0)
        );
        assert_eq!(
            endpoint(p(83.0, 40.0), p(99.0, 45.0), size, true, true),
            p(100.0, 57.0)
        );
        assert_eq!(
            endpoint(p(20.0, 20.0), p(23.0, 24.0), size, true, true),
            p(20.0, 20.0)
        );
    }
}
