//! 直线端点约束，与基线的先角度、后坐标步进顺序保持一致。
use crate::viewport::Point;

pub fn endpoint(start: Point, current: Point, shift: bool, control: bool) -> Point {
    configured_endpoint(start, current, shift, control, false)
}

pub fn configured_endpoint(
    start: Point,
    current: Point,
    shift: bool,
    control: bool,
    angle_snap: bool,
) -> Point {
    let mut dx = current.x - start.x;
    let mut dy = current.y - start.y;
    if shift || angle_snap {
        let step = if shift {
            std::f32::consts::FRAC_PI_4
        } else {
            5_f32.to_radians()
        };
        let angle = js_round(dy.atan2(dx) / step) * step;
        let length = dx.hypot(dy);
        dx = angle.cos() * length;
        dy = angle.sin() * length;
    }
    if control {
        dx = js_round(dx / 10.0) * 10.0;
        dy = js_round(dy / 10.0) * 10.0;
    }
    Point {
        x: start.x + dx,
        y: start.y + dy,
    }
}

// JS Math.round 的负半整数向正无穷舍入，不使用 Rust 的远离零舍入。
fn js_round(value: f32) -> f32 {
    (value + 0.5).floor()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn configured_five_degree_snap_yields_to_shift_and_preserves_length() {
        let start = Point::default();
        let angle = 17_f32.to_radians();
        let current = Point {
            x: angle.cos() * 50.0,
            y: angle.sin() * 50.0,
        };
        let snapped = configured_endpoint(start, current, false, false, true);
        assert!((snapped.y.atan2(snapped.x).to_degrees() - 15.0).abs() < 0.001);
        assert!((snapped.x.hypot(snapped.y) - 50.0).abs() < 0.001);
        let shifted = configured_endpoint(start, current, true, false, true);
        assert!(shifted.y.abs() < 0.001);
        assert!((shifted.x - 50.0).abs() < 0.001);
    }
    #[test]
    fn angle_preserves_length_and_step_follows_angle() {
        let start = Point { x: 10.0, y: 20.0 };
        let current = Point { x: 40.0, y: 40.0 };
        assert_eq!(endpoint(start, current, false, false), current);
        let locked = endpoint(start, current, true, false);
        assert!(((locked.x - start.x) - (locked.y - start.y)).abs() < 0.001);
        assert!(((locked.x - start.x).hypot(locked.y - start.y) - 1300_f32.sqrt()).abs() < 0.001);
        assert_eq!(
            endpoint(start, current, true, true),
            Point { x: 40.0, y: 50.0 }
        );
    }
    #[test]
    fn negative_half_step_matches_javascript_and_zero_length_is_safe() {
        let start = Point::default();
        assert_eq!(
            endpoint(start, Point { x: -15.0, y: 15.0 }, false, true),
            Point { x: -10.0, y: 20.0 }
        );
        assert_eq!(endpoint(start, start, true, true), start);
    }
}
