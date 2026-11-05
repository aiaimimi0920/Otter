//! 笔刷采样时序只属于草稿；保存无量纲权重，变换后不重新解释绘制速度。
use crate::viewport::Point;

#[derive(Clone, Copy, Debug)]
pub struct Motion {
    pub point: Point,
    pub time: f64,
    pub weight: f32,
}

pub fn weight(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.4, 1.0)
    } else {
        0.85
    }
}

pub fn sample(point: Point, time: f64, previous: Option<Motion>) -> Motion {
    let time = if time.is_finite() {
        time
    } else {
        previous.map_or(0.0, |p| p.time)
    };
    let Some(previous) = previous else {
        return Motion {
            point,
            time,
            weight: 0.85,
        };
    };
    let distance = (point.x - previous.point.x).hypot(point.y - previous.point.y);
    let elapsed = time - previous.time;
    let prior = weight(previous.weight);
    let target = if elapsed > 0.0 {
        let speed = distance / elapsed.clamp(4.0, 80.0) as f32;
        0.42 + 0.58 / (1.0 + speed * 1.5)
    } else {
        prior
    };
    let blend = (1.0 - (-distance / 18.0).exp()).min(0.32);
    Motion {
        point,
        time,
        weight: prior + (target - prior) * blend,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn speed_changes_weight_but_invalid_time_preserves_it() {
        let first = sample(Point::default(), 100.0, None);
        let end = Point { x: 30.0, y: 0.0 };
        let fast = sample(end, 104.0, Some(first));
        let slow = sample(end, 180.0, Some(first));
        assert!(fast.weight < slow.weight);
        assert_eq!(sample(end, 99.0, Some(first)).weight, first.weight);
        assert_eq!(sample(end, f64::NAN, Some(first)).weight, first.weight);
        assert_eq!(
            sample(Point::default(), 180.0, Some(first)).weight,
            first.weight
        );
        assert_eq!(weight(f32::NAN), 0.85);
    }
}
