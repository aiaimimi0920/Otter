//! 对照主干 transform controller 的连续变换参数；无文档写入、无分配、无窗口依赖。
use crate::viewport::Point;

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub enum Axis {
    #[default]
    Free,
    X,
    Y,
}
impl Axis {
    pub fn constrain(self, offset: Point) -> Point {
        Point {
            x: if self == Self::Y { 0.0 } else { offset.x },
            y: if self == Self::X { 0.0 } else { offset.y },
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Gesture {
    start: Point,
    pivot: Point,
    axis: Axis,
}
impl Gesture {
    pub fn new(start: Point, pivot: Point, axis: Axis) -> Option<Self> {
        (valid(start) && valid(pivot)).then_some(Self { start, pivot, axis })
    }
    pub fn rotation(self, current: Point) -> Option<f32> {
        if !valid(current) {
            return None;
        }
        let angle = match self.axis {
            Axis::X => (current.x as f64 - self.start.x as f64) * 0.5,
            Axis::Y => (current.y as f64 - self.start.y as f64) * 0.5,
            Axis::Free => {
                let start = (self.start.y as f64 - self.pivot.y as f64)
                    .atan2(self.start.x as f64 - self.pivot.x as f64);
                let end = (current.y as f64 - self.pivot.y as f64)
                    .atan2(current.x as f64 - self.pivot.x as f64);
                (end - start).to_degrees()
            }
        };
        Some(angle as f32)
    }
    pub fn scale(self, current: Point, uniform: bool) -> Option<Point> {
        if !valid(current) {
            return None;
        }
        let start = (
            self.start.x as f64 - self.pivot.x as f64,
            self.start.y as f64 - self.pivot.y as f64,
        );
        let end = (
            current.x as f64 - self.pivot.x as f64,
            current.y as f64 - self.pivot.y as f64,
        );
        let ratio = |value: f64, base: f64| {
            if base.abs() < 0.0001 {
                1.0
            } else {
                (value / base).clamp(0.1, 8.0) as f32
            }
        };
        if uniform {
            let factor = ratio(end.0.hypot(end.1), start.0.hypot(start.1));
            Some(Point {
                x: factor,
                y: factor,
            })
        } else {
            Some(Point {
                x: if self.axis == Axis::Y {
                    1.0
                } else {
                    ratio(end.0, start.0)
                },
                y: if self.axis == Axis::X {
                    1.0
                } else {
                    ratio(end.1, start.1)
                },
            })
        }
    }
}
fn valid(point: Point) -> bool {
    point.x.is_finite()
        && point.y.is_finite()
        && point.x.abs() <= 1_000_000.0
        && point.y.abs() <= 1_000_000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn radial_and_axis_rotation_match_baseline_and_do_not_accumulate() {
        let start = Point { x: 20.0, y: 10.0 };
        let pivot = Point { x: 10.0, y: 10.0 };
        let free = Gesture::new(start, pivot, Axis::Free).unwrap();
        assert_eq!(free.rotation(Point { x: 10.0, y: 20.0 }), Some(90.0));
        assert_eq!(free.rotation(Point { x: 10.0, y: 0.0 }), Some(-90.0));
        assert_eq!(free.rotation(start), Some(0.0));
        assert_eq!(
            Gesture::new(start, pivot, Axis::X)
                .unwrap()
                .rotation(Point { x: 40.0, y: 100.0 }),
            Some(10.0)
        );
        assert_eq!(
            Gesture::new(start, pivot, Axis::Y)
                .unwrap()
                .rotation(Point { x: 40.0, y: 100.0 }),
            Some(45.0)
        );
    }
    #[test]
    fn axis_scaling_uniform_override_clamps_and_near_zero_match_baseline() {
        let start = Point { x: 10.0, y: 20.0 };
        let end = Point { x: 20.0, y: 60.0 };
        for (axis, expected) in [
            (Axis::Free, Point { x: 2.0, y: 3.0 }),
            (Axis::X, Point { x: 2.0, y: 1.0 }),
            (Axis::Y, Point { x: 1.0, y: 3.0 }),
        ] {
            let gesture = Gesture::new(start, Point::default(), axis).unwrap();
            assert_eq!(gesture.scale(end, false), Some(expected));
            let uniform = gesture.scale(end, true).unwrap();
            assert!((uniform.x - 8.0f32.sqrt()).abs() < 0.00001);
            assert_eq!(uniform.x, uniform.y);
        }
        let gesture = Gesture::new(start, Point::default(), Axis::Free).unwrap();
        assert_eq!(
            gesture.scale(
                Point {
                    x: -10.0,
                    y: 1_000_000.0
                },
                false
            ),
            Some(Point { x: 0.1, y: 8.0 })
        );
        let zero =
            Gesture::new(Point { x: 0.00001, y: 0.0 }, Point::default(), Axis::Free).unwrap();
        assert_eq!(zero.scale(end, false), Some(Point { x: 1.0, y: 1.0 }));
        assert_eq!(zero.scale(end, true), Some(Point { x: 1.0, y: 1.0 }));
    }
    #[test]
    fn invalid_pointer_samples_never_produce_transform_parameters() {
        let gesture = Gesture::new(Point::default(), Point::default(), Axis::Free).unwrap();
        for x in [f32::NAN, f32::INFINITY, 1_000_001.0] {
            let point = Point { x, y: 0.0 };
            assert!(Gesture::new(point, Point::default(), Axis::Free).is_none());
            assert!(gesture.rotation(point).is_none());
            assert!(gesture.scale(point, false).is_none());
        }
    }
}
