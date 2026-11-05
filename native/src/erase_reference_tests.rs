//! 独立全图采样参考，不使用行裁剪或候选区间，防止优化漏擦圆帽和斜线边缘。
use super::*;

fn reference(source: &Pixels, size: (u32, u32), points: &[Point], width: f32) -> Vec<u8> {
    let mut output = source.to_vec();
    let r2 = (width as f64 / 2.0).powi(2);
    for y in 0..size.1 {
        for x in 0..size.0 {
            let mut covered = 0;
            for sy in 0..4 {
                for sx in 0..4 {
                    let px = x as f64 + (sx as f64 + 0.5) / 4.0;
                    let py = y as f64 + (sy as f64 + 0.5) / 4.0;
                    let hit = (0..points.len()).any(|i| {
                        let a = points[i.saturating_sub(1)];
                        let b = points[i];
                        let (ax, ay, bx, by) = (a.x as f64, a.y as f64, b.x as f64, b.y as f64);
                        let length = (bx - ax).powi(2) + (by - ay).powi(2);
                        let t = if length == 0.0 {
                            0.0
                        } else {
                            ((px - ax) * (bx - ax) + (py - ay) * (by - ay)) / length
                        }
                        .clamp(0.0, 1.0);
                        let cx = ax + t * (bx - ax);
                        let cy = ay + t * (by - ay);
                        (px - cx).powi(2) + (py - cy).powi(2) <= r2
                    });
                    covered += u32::from(hit);
                }
            }
            for c in 0..4 {
                let i = ((y * size.0 + x) * 4 + c) as usize;
                output[i] = ((source[i] as u32 * (16 - covered) + 8) / 16) as u8;
            }
        }
    }
    output
}

#[test]
fn clipped_scan_matches_full_reference_for_fractional_reversed_and_extreme_segments() {
    let source: Pixels = [40, 80, 100, 128].repeat(24 * 24).into();
    let p = |x, y| Point { x, y };
    let mut paths = vec![
        vec![p(12.125, 12.375)],
        vec![p(-1_000_000.0, 12.5), p(1_000_000.0, 12.5)],
        vec![p(12.5, -1_000_000.0), p(12.5, 1_000_000.0)],
        vec![p(-999_999.75, -999_999.5), p(999_999.75, 999_999.5)],
        vec![p(30.0, -4.0), p(-4.0, 30.0), p(30.0, 30.0)],
        vec![p(-100.0, -100.0), p(-99.0, -99.0)],
    ];
    let mut state = 0x1234_5678u32;
    for _ in 0..64 {
        let mut coordinate = || {
            state = state.wrapping_mul(1664525).wrapping_add(1013904223);
            (state % 192) as f32 / 4.0 - 12.0
        };
        paths.push((0..3).map(|_| p(coordinate(), coordinate())).collect());
    }
    for (i, path) in paths.iter().enumerate() {
        let width = [4.0, 7.25, 20.0, 96.0][i % 4];
        let expected = reference(&source, (24, 24), path, width);
        let result = apply(&source, (24, 24), path, width, || false).unwrap();
        assert_eq!(&*result, expected, "path {i}: {path:?}");
        let mut reversed = path.clone();
        reversed.reverse();
        assert_eq!(
            apply(&source, (24, 24), &reversed, width, || false).unwrap(),
            result,
            "reversed {i}"
        );
    }
}
