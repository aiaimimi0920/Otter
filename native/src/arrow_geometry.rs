//! 与旧标注的直箭头尺寸合同一致；几何在图像空间计算，供绘制和命中共享。
use crate::viewport::Point;
#[path = "arrow_path_geometry.rs"]
mod path;
pub use path::{build_path, PathArrow};

pub struct Arrow {
    pub base: Point,
    pub head: [Point; 3],
}

pub fn straight(start: Point, end: Point, width: f32) -> Option<Arrow> {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let distance = dx.hypot(dy);
    if !distance.is_finite() || distance <= 0.0 || !width.is_finite() || width <= 0.0 {
        return None;
    }
    let length = (width * 6.0).max(24.0).min(distance);
    let half = (width * 5.0).max(16.0) / 2.0;
    let (ux, uy) = (dx / distance, dy / distance);
    let base = Point {
        x: end.x - ux * length,
        y: end.y - uy * length,
    };
    Some(Arrow {
        base,
        head: [
            end,
            Point {
                x: base.x - uy * half,
                y: base.y + ux * half,
            },
            Point {
                x: base.x + uy * half,
                y: base.y - ux * half,
            },
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn baseline_dimensions_and_short_shaft_are_preserved() {
        let arrow = straight(Point::default(), Point { x: 100.0, y: 0.0 }, 4.0).unwrap();
        assert_eq!(arrow.base, Point { x: 76.0, y: 0.0 });
        assert_eq!(arrow.head[1], Point { x: 76.0, y: 10.0 });
        let short = straight(Point::default(), Point { x: 2.0, y: 0.0 }, 4.0).unwrap();
        assert_eq!(short.base, Point::default());
        assert!(straight(Point::default(), Point::default(), 4.0).is_none());
        assert!(straight(
            Point::default(),
            Point {
                x: f32::NAN,
                y: 0.0
            },
            4.0
        )
        .is_none());
    }
}
