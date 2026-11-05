//! 胶囊段与像素行的保守候选区间，避免细长斜线扫描整个二维包围框。
use crate::viewport::Point;

/// 整次手势的图像内覆盖范围；不按未裁剪的外部坐标跨度分配。
pub(super) fn region(points: &[Point], radius: f64, size: (u32, u32)) -> Option<[u32; 4]> {
    let first = *points.first()?;
    let (mut left, mut top, mut right, mut bottom) = (first.x, first.y, first.x, first.y);
    for point in points {
        left = left.min(point.x);
        top = top.min(point.y);
        right = right.max(point.x);
        bottom = bottom.max(point.y);
    }
    let left = (left as f64 - radius).floor().max(0.0).min(size.0 as f64) as u32;
    let top = (top as f64 - radius).floor().max(0.0).min(size.1 as f64) as u32;
    let right = (right as f64 + radius).ceil().max(0.0).min(size.0 as f64) as u32;
    let bottom = (bottom as f64 + radius).ceil().max(0.0).min(size.1 as f64) as u32;
    (left < right && top < bottom).then_some([left, top, right, bottom])
}

/// 调用者已经校验有限坐标、正半径及图像尺寸；输出为右开区间。
pub(super) fn row(a: Point, b: Point, radius: f64, y: u32, width: u32) -> Option<(u32, u32)> {
    let (ax, ay, bx, by) = (a.x as f64, a.y as f64, b.x as f64, b.y as f64);
    let low = y as f64 - radius;
    let high = y as f64 + 1.0 + radius;
    let (start, end) = if ay == by {
        if ay < low || ay > high {
            return None;
        }
        (0.0, 1.0)
    } else {
        let t0 = (low - ay) / (by - ay);
        let t1 = (high - ay) / (by - ay);
        let start = t0.min(t1).max(0.0);
        let end = t0.max(t1).min(1.0);
        if start > end {
            return None;
        }
        (start, end)
    };
    let x0 = ax + (bx - ax) * start;
    let x1 = ax + (bx - ax) * end;
    // 额外一个像素抵御超大坐标相减/除法舍入；最终覆盖仍由精确子像素测试决定。
    let left = ((x0.min(x1) - radius).floor() - 1.0)
        .max(0.0)
        .min(width as f64) as u32;
    let right = ((x0.max(x1) + radius).ceil() + 1.0)
        .max(0.0)
        .min(width as f64) as u32;
    (left < right).then_some((left, right))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn small_gesture_bounds_do_not_scale_with_image_size() {
        assert_eq!(
            region(&[Point { x: 100.0, y: 200.0 }], 10.0, (8192, 8192)),
            Some([90, 190, 110, 210])
        );
        assert_eq!(
            region(
                &[Point {
                    x: -100.0,
                    y: -100.0
                }],
                10.0,
                (8192, 8192)
            ),
            None
        );
        assert_eq!(
            region(&[Point { x: 0.0, y: 0.0 }], 10.0, (8192, 8192)),
            Some([0, 0, 10, 10])
        );
    }
    #[test]
    fn diagonal_candidates_scale_with_stroke_width_not_bounding_box_area() {
        let a = Point { x: 0.0, y: 0.0 };
        let b = Point {
            x: 4096.0,
            y: 4096.0,
        };
        let candidates: u64 = (0..4096)
            .filter_map(|y| row(a, b, 2.0, y, 4096))
            .map(|(left, right)| u64::from(right - left))
            .sum();
        assert!(candidates < 50_000, "candidate pixels: {candidates}");
        assert!(candidates * 300 < 4096 * 4096);
        for y in 0..4096 {
            let (left, right) = row(a, b, 2.0, y, 4096).unwrap();
            assert!(left <= y && right > y);
            assert_eq!(row(b, a, 2.0, y, 4096), Some((left, right)));
        }
    }
}
