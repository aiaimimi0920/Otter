//! 使用完整覆盖画布作独立采样基准，核对缩小、空洞、负坐标和早帧覆盖顺序。
use super::*;

fn frame(x: i64, y: i64, width: u32, height: u32, seed: u8) -> SpatialFrame {
    let mut bytes = Vec::new();
    for row in 0..height {
        for col in 0..width {
            bytes.extend_from_slice(&[col as u8, row as u8, seed, 0]);
        }
    }
    SpatialFrame {
        frame: Frame::new(width, height, bytes.into()).unwrap(),
        x,
        y,
    }
}

fn assert_preview(frames: &[SpatialFrame]) {
    let cancel = AtomicBool::new(false);
    let (left, top, width, height) = spatial::bounds(frames, &cancel).unwrap();
    let mut full = vec![0; (width * height * 4) as usize];
    // 后帧先铺，早帧后覆盖，独立于生产路径的覆盖标记和范围查找。
    for item in frames.iter().rev() {
        for y in 0..item.frame.height {
            for x in 0..item.frame.width {
                let source = ((y * item.frame.width + x) * 4) as usize;
                let target = (((item.y - top + i64::from(y)) as u32 * width
                    + (item.x - left + i64::from(x)) as u32)
                    * 4) as usize;
                full[target..target + 3].copy_from_slice(&item.frame.bytes()[source..source + 3]);
                full[target + 3] = 255;
            }
        }
    }
    let preview = render(frames, &cancel).unwrap();
    assert_eq!((preview.x, preview.y), (left, top));
    assert!(preview.width.max(preview.height) <= 384);
    for y in 0..preview.height {
        for x in 0..preview.width {
            let sx =
                ((u64::from(x) * 2 + 1) * u64::from(width) / (u64::from(preview.width) * 2)) as u32;
            let sy = ((u64::from(y) * 2 + 1) * u64::from(height) / (u64::from(preview.height) * 2))
                as u32;
            let source = ((sy * width + sx) * 4) as usize;
            let target = ((y * preview.width + x) * 4) as usize;
            assert_eq!(
                &preview.pixels[target..target + 4],
                &full[source..source + 4],
                "sample {x},{y}"
            );
        }
    }
}

#[test]
fn preview_matches_full_coverage_for_overlap_holes_and_thin_frames() {
    for frames in [
        vec![frame(-10, -5, 18, 12, 1), frame(-2, 1, 16, 9, 2)],
        vec![frame(-400, -20, 600, 160, 1), frame(100, 30, 480, 250, 2)],
        vec![frame(-10, -400, 160, 600, 1), frame(30, 100, 250, 480, 2)],
        vec![frame(-400, 0, 1, 8, 1), frame(400, 0, 1, 8, 2)],
        vec![frame(0, -400, 8, 1, 1), frame(0, 400, 8, 1, 2)],
        (0..128)
            .map(|i| frame(i * 5 - 320, i % 3, 8, 8, i as u8))
            .collect(),
        vec![frame(0, 0, 500, 8, 1), frame(0, 0, 500, 8, 2)],
    ] {
        assert_preview(&frames);
    }
}
