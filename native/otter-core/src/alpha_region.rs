//! 透明图像的有界行区间。扩展一像素以保留线性采样边缘，不按 alpha 阈值削弱可见内容。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[derive(Debug)]
pub struct AlphaRegion {
    pub rectangles: Vec<Rect>,
    pub conservative: bool,
}

const MAX_RUNS: usize = 65_536;

pub fn build(width: u32, height: u32, pixels: &[u8]) -> Option<AlphaRegion> {
    build_layers(width, height, pixels, None)
}

/// source-over 合成的非透明区域为两层 alpha 的并集，不需要生成完整合成像素副本。
pub fn build_layers(
    width: u32,
    height: u32,
    pixels: &[u8],
    annotations: Option<&[u8]>,
) -> Option<AlphaRegion> {
    build_layers_cancellable(width, height, pixels, annotations, || false)
}

/// 后台恢复逐行检查取消；取消结果不进入碎片化降级扫描。
pub fn build_layers_cancellable(
    width: u32,
    height: u32,
    pixels: &[u8],
    annotations: Option<&[u8]>,
    cancelled: impl Fn() -> bool,
) -> Option<AlphaRegion> {
    if crate::viewport::checked_image_bytes(width, height) != Some(pixels.len())
        || annotations.is_some_and(|layer| layer.len() != pixels.len())
    {
        return None;
    }
    let exact = scan(width, height, pixels, annotations, false, &cancelled);
    if cancelled() {
        return None;
    }
    if let Some(rectangles) = exact {
        Some(AlphaRegion {
            rectangles,
            conservative: false,
        })
    } else {
        // 极端碎片化 alpha 用每行包围段保守降级；仍保持空行穿透，不创建无界 GDI 区域。
        Some(AlphaRegion {
            rectangles: scan(width, height, pixels, annotations, true, &cancelled)?,
            conservative: true,
        })
    }
}

fn scan(
    width: u32,
    height: u32,
    pixels: &[u8],
    annotations: Option<&[u8]>,
    coarse: bool,
    cancelled: &impl Fn() -> bool,
) -> Option<Vec<Rect>> {
    let mut rectangles: Vec<Rect> = Vec::new();
    for y in 0..height {
        if cancelled() {
            return None;
        }
        let mut start = None;
        let mut first = None;
        let mut last = 0;
        for x in 0..width {
            let filled = occupied(width, height, pixels, annotations, x, y);
            if filled {
                start.get_or_insert(x);
                first.get_or_insert(x);
                last = x + 1;
            }
            if !coarse && (!filled || x + 1 == width) {
                if let Some(left) = start.take() {
                    push_run(&mut rectangles, left, y, if filled { x + 1 } else { x });
                    if rectangles.len() > MAX_RUNS {
                        return None;
                    }
                }
            }
        }
        if coarse {
            if let Some(left) = first {
                push_run(&mut rectangles, left, y, last);
            }
        }
    }
    if cancelled() {
        None
    } else {
        Some(rectangles)
    }
}

fn push_run(rectangles: &mut Vec<Rect>, left: u32, y: u32, right: u32) {
    if let Some(last) = rectangles.last_mut() {
        if last.left == left as i32 && last.right == right as i32 && last.bottom == y as i32 {
            last.bottom += 1;
            return;
        }
    }
    rectangles.push(Rect {
        left: left as i32,
        top: y as i32,
        right: right as i32,
        bottom: y as i32 + 1,
    });
}

fn occupied(
    width: u32,
    height: u32,
    pixels: &[u8],
    annotations: Option<&[u8]>,
    x: u32,
    y: u32,
) -> bool {
    for row in y.saturating_sub(1)..=(y + 1).min(height - 1) {
        for column in x.saturating_sub(1)..=(x + 1).min(width - 1) {
            let alpha = ((row * width + column) * 4 + 3) as usize;
            if pixels[alpha] != 0 || annotations.is_some_and(|layer| layer[alpha] != 0) {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    #[test]
    fn cancellable_scan_matches_existing_mask_and_stops_between_rows() {
        let pixels = vec![255; 32 * 32 * 4];
        let expected = super::build(32, 32, &pixels).unwrap();
        let actual = super::build_layers_cancellable(32, 32, &pixels, None, || false).unwrap();
        assert_eq!(actual.rectangles, expected.rectangles);
        assert_eq!(actual.conservative, expected.conservative);
        let checks = std::cell::Cell::new(0);
        assert!(super::build_layers_cancellable(32, 32, &pixels, None, || {
            checks.set(checks.get() + 1);
            checks.get() >= 3
        })
        .is_none());
        assert!(
            checks.get() <= 4,
            "cancel must not trigger coarse fallback scan"
        );
        assert!(super::build_layers_cancellable(32, 32, &pixels, None, || true).is_none());
    }
    use super::*;
    #[test]
    fn raster_layer_alpha_survives_base_holes_without_composite_allocation() {
        let mut base = vec![0; 16 * 16 * 4];
        let mut annotations = base.clone();
        base[(3 * 16 + 3) * 4 + 3] = 128;
        annotations[(12 * 16 + 12) * 4 + 3] = 1;
        let union = build_layers(16, 16, &base, Some(&annotations)).unwrap();
        assert_eq!(
            union.rectangles,
            vec![
                Rect {
                    left: 2,
                    top: 2,
                    right: 5,
                    bottom: 5
                },
                Rect {
                    left: 11,
                    top: 11,
                    right: 14,
                    bottom: 14
                },
            ]
        );
        base.fill(0);
        assert_eq!(
            build_layers(16, 16, &base, Some(&annotations))
                .unwrap()
                .rectangles,
            build(16, 16, &annotations).unwrap().rectangles
        );
        annotations.fill(0);
        assert!(build_layers(16, 16, &base, Some(&annotations))
            .unwrap()
            .rectangles
            .is_empty());
        assert!(build_layers(16, 16, &base, Some(&[0; 4])).is_none());
    }
    #[test]
    fn solid_image_is_one_region_and_empty_image_is_empty() {
        assert_eq!(
            build(8, 8, &[255; 256]).unwrap().rectangles,
            vec![Rect {
                left: 0,
                top: 0,
                right: 8,
                bottom: 8
            }]
        );
        assert!(build(8, 8, &[0; 256]).unwrap().rectangles.is_empty());
        assert!(build(8, 8, &[0; 4]).is_none());
    }
    #[test]
    fn faint_pixel_retains_sampling_border_but_not_distant_transparency() {
        let mut pixels = vec![0; 12 * 12 * 4];
        pixels[(6 * 12 + 6) * 4 + 3] = 1;
        let region = build(12, 12, &pixels).unwrap();
        assert!(!region.conservative);
        assert_eq!(
            region.rectangles,
            vec![Rect {
                left: 5,
                top: 5,
                right: 8,
                bottom: 8
            }]
        );
    }

    #[test]
    fn fragmented_alpha_has_a_bounded_conservative_fallback() {
        let mut pixels = vec![0; 1024 * 1024 * 4];
        for y in (2..1024).step_by(4) {
            for x in (2..1024).step_by(4) {
                pixels[(y * 1024 + x) * 4 + 3] = 255;
            }
        }
        let region = build(1024, 1024, &pixels).unwrap();
        assert!(region.conservative);
        assert!(region.rectangles.len() <= 1024);
        assert!(!region
            .rectangles
            .iter()
            .any(|rect| rect.top <= 0 && rect.bottom > 0));
    }
}
