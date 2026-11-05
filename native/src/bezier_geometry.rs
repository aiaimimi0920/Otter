//! 基线单段三次贝塞尔的创建控制点；渲染与历史保留控制点，不保存采样折线。
use crate::viewport::Point;

/// 基线延伸柄位于端点外侧 28 图像像素；达到 64 段时不再提供入口。
pub fn extension_handles(points: &[Point]) -> Option<[Point; 2]> {
    if points.len() < 4 || points.len() >= 193 || !(points.len() - 1).is_multiple_of(3) {
        return None;
    }
    Some(std::array::from_fn(|side| {
        let (anchor, tangent) = if side == 0 {
            (points[0], points[1])
        } else {
            (points[points.len() - 1], points[points.len() - 2])
        };
        let length = (anchor.x - tangent.x).hypot(anchor.y - tangent.y);
        let direction = if length > 0.0 {
            Point {
                x: (anchor.x - tangent.x) / length,
                y: (anchor.y - tangent.y) / length,
            }
        } else {
            Point {
                x: if side == 0 { -1.0 } else { 1.0 },
                y: 0.0,
            }
        };
        Point {
            x: anchor.x + direction.x * 28.0,
            y: anchor.y + direction.y * 28.0,
        }
    }))
}

pub fn create(start: Point, end: Point) -> [Point; 4] {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    [
        start,
        Point {
            x: start.x + dx / 3.0 - dy / 4.0,
            y: start.y + dy / 3.0 + dx / 4.0,
        },
        Point {
            x: start.x + dx * 2.0 / 3.0 - dy / 4.0,
            y: start.y + dy * 2.0 / 3.0 + dx / 4.0,
        },
        end,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn horizontal_curve_matches_baseline_control_offsets() {
        assert_eq!(
            create(Point::default(), Point { x: 60.0, y: 0.0 }),
            [
                Point::default(),
                Point { x: 20.0, y: 15.0 },
                Point { x: 40.0, y: 15.0 },
                Point { x: 60.0, y: 0.0 }
            ]
        );
    }
}
