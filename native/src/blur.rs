//! 图像空间高斯模糊；只计算目标区域及核所需邻域，不读取窗口或修改原始像素。
//! 输入/输出为预乘 BGRA，图像外为透明；工作线程可在每行边界取消。
use crate::{
    pixels::{self, Pixels},
    viewport::checked_image_bytes,
};
#[path = "blur_scaled.rs"]
mod scaled;
pub use scaled::region_scaled;

#[derive(Clone, Copy, Debug)]
pub struct Region {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlurError {
    InvalidInput,
    BudgetExceeded,
    Cancelled,
}

fn allocate<T: Default + Clone>(
    count: usize,
) -> Result<(Vec<T>, crate::byte_budget::Lease), BlurError> {
    let bytes = count
        .checked_mul(std::mem::size_of::<T>())
        .ok_or(BlurError::BudgetExceeded)?;
    let mut lease = pixels::reserve(bytes).ok_or(BlurError::BudgetExceeded)?;
    let mut data = Vec::new();
    data.try_reserve_exact(count)
        .map_err(|_| BlurError::BudgetExceeded)?;
    let capacity = data
        .capacity()
        .checked_mul(std::mem::size_of::<T>())
        .ok_or(BlurError::BudgetExceeded)?;
    if !lease.grow_to(capacity) {
        return Err(BlurError::BudgetExceeded);
    }
    data.resize(count, T::default());
    Ok((data, lease))
}

/// sigma 是图像像素标准差；整数兼容入口，核截断到 ±3 sigma。
/// 两次一维卷积保留浮点中间值，避免分块计算引入额外量化接缝。
pub fn region(
    source: &Pixels,
    size: (u32, u32),
    area: Region,
    sigma: u8,
    cancelled: impl Fn() -> bool,
) -> Result<Pixels, BlurError> {
    region_precise(source, size, area, sigma as f32, cancelled)
}

/// 保留缩放产生的小数标准差；整数入口复用同一算法，核半径向上取整。
pub fn region_precise(
    source: &Pixels,
    size: (u32, u32),
    area: Region,
    sigma: f32,
    cancelled: impl Fn() -> bool,
) -> Result<Pixels, BlurError> {
    if !sigma.is_finite() || !(1.0..=64.0).contains(&sigma) {
        return Err(BlurError::InvalidInput);
    }
    region_axes(source, size, area, (sigma, sigma), cancelled)
}

fn region_axes(
    source: &Pixels,
    size: (u32, u32),
    area: Region,
    sigma: (f32, f32),
    cancelled: impl Fn() -> bool,
) -> Result<Pixels, BlurError> {
    let (width, height) = size;
    let count = checked_image_bytes(area.width, area.height).ok_or(BlurError::InvalidInput)?;
    if checked_image_bytes(width, height) != Some(source.len())
        || area.x.checked_add(area.width).is_none_or(|end| end > width)
        || area
            .y
            .checked_add(area.height)
            .is_none_or(|end| end > height)
    {
        return Err(BlurError::InvalidInput);
    }
    if cancelled() {
        return Err(BlurError::Cancelled);
    }
    let horizontal_kernel = scaled::Kernel::new(sigma.0)?;
    let separate_vertical = (sigma.0 != sigma.1)
        .then(|| scaled::Kernel::new(sigma.1))
        .transpose()?;
    let vertical_kernel = separate_vertical.as_ref().unwrap_or(&horizontal_kernel);
    let top = (area.y as i32 - vertical_kernel.radius).max(0);
    let bottom = (area.y as i32 + area.height as i32 + vertical_kernel.radius).min(height as i32);
    let row_size = area.width as usize * 4;
    let rows = ((bottom - top) as usize).min(vertical_kernel.weights.len());
    let (mut horizontal, _temporary_lease) = allocate::<f32>(row_size * rows)?;
    let mut next_row = top;
    let (mut output, lease) = allocate::<u8>(count)?;
    for y in 0..area.height as usize {
        if cancelled() {
            return Err(BlurError::Cancelled);
        }
        // 每个源行只计算一次；覆盖的槽位一定早于本输出行的核起点。
        let required_end = (area.y as i32 + y as i32 + vertical_kernel.radius + 1).min(bottom);
        while next_row < required_end {
            if cancelled() {
                return Err(BlurError::Cancelled);
            }
            let slot = next_row as usize % rows;
            let row = &mut horizontal[slot * row_size..(slot + 1) * row_size];
            row.fill(0.0);
            for x in 0..area.width as usize {
                for (k, &weight) in horizontal_kernel.weights.iter().enumerate() {
                    let source_x = area.x as i32 + x as i32 + k as i32 - horizontal_kernel.radius;
                    if source_x < 0 || source_x >= width as i32 {
                        continue;
                    }
                    let input = (next_row as usize * width as usize + source_x as usize) * 4;
                    for channel in 0..4 {
                        row[x * 4 + channel] += source[input + channel] as f32 * weight;
                    }
                }
            }
            next_row += 1;
        }
        for x in 0..area.width as usize {
            let mut sum = [0.0f32; 4];
            for (k, &weight) in vertical_kernel.weights.iter().enumerate() {
                let source_y = area.y as i32 + y as i32 + k as i32 - vertical_kernel.radius;
                if source_y < top || source_y >= bottom {
                    continue;
                }
                let input = (source_y as usize % rows) * row_size + x * 4;
                for channel in 0..4 {
                    sum[channel] += horizontal[input + channel] * weight;
                }
            }
            let destination = (y * area.width as usize + x) * 4;
            for channel in 0..4 {
                output[destination + channel] = sum[channel].round().clamp(0.0, 255.0) as u8;
            }
        }
    }
    if cancelled() {
        return Err(BlurError::Cancelled);
    }
    Ok(Pixels::from_reserved(output, lease))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fractional_kernel_matches_independent_impulse_reference() {
        let mut bytes = vec![0; 17 * 17 * 4];
        bytes[(8 * 17 + 8) * 4..(8 * 17 + 8) * 4 + 4].fill(255);
        let source: Pixels = bytes.into();
        let area = Region {
            x: 0,
            y: 0,
            width: 17,
            height: 17,
        };
        let sigma = 1.4f64;
        let radius = (3.0 * sigma).ceil() as i32;
        let weight = |x: i32| (-(x as f64).powi(2) / (2.0 * sigma * sigma)).exp();
        let total: f64 = (-radius..=radius).map(weight).sum();
        let result = region_precise(&source, (17, 17), area, sigma as f32, || false).unwrap();
        for y in -radius..=radius {
            for x in -radius..=radius {
                let expected = (255.0 * weight(x) * weight(y) / (total * total)).round() as u8;
                let index = (((8 + y) * 17 + 8 + x) * 4) as usize;
                assert_eq!(result[index], expected);
            }
        }
        let rounded = region(&source, (17, 17), area, 1, || false).unwrap();
        assert_ne!(&result[..], &rounded[..]);
        for sigma in [f32::NAN, f32::INFINITY, 0.5, 65.0] {
            assert!(matches!(
                region_precise(&source, (17, 17), area, sigma, || false),
                Err(BlurError::InvalidInput)
            ));
        }
    }
    #[test]
    fn rolling_rows_match_independent_single_row_evaluation_after_wraparound() {
        let source: Pixels = (0..17 * 80)
            .flat_map(|i| {
                let alpha = (i % 256) as u8;
                [alpha / 3, alpha / 2, alpha, alpha]
            })
            .collect::<Vec<_>>()
            .into();
        let area = Region {
            x: 3,
            y: 7,
            width: 9,
            height: 60,
        };
        for sigma in [1, 3, 16, 64] {
            let full = region(&source, (17, 80), area, sigma, || false).unwrap();
            for row in [0, 1, 7, 13, 31, 59] {
                let independent = region(
                    &source,
                    (17, 80),
                    Region {
                        y: area.y + row,
                        height: 1,
                        ..area
                    },
                    sigma,
                    || false,
                )
                .unwrap();
                let start = row as usize * 9 * 4;
                assert_eq!(
                    &full[start..start + 36],
                    &independent[..],
                    "sigma={sigma} row={row}"
                );
            }
        }
    }
    #[test]
    fn impulse_spreads_symmetrically_and_preserves_premultiplied_channels() {
        let mut bytes = vec![0; 15 * 15 * 4];
        bytes[(7 * 15 + 7) * 4..(7 * 15 + 7) * 4 + 4].copy_from_slice(&[32, 64, 128, 128]);
        let source: Pixels = bytes.into();
        let result = region(
            &source,
            (15, 15),
            Region {
                x: 0,
                y: 0,
                width: 15,
                height: 15,
            },
            1,
            || false,
        )
        .unwrap();
        let alpha = |x: usize, y: usize| result[(y * 15 + x) * 4 + 3];
        assert!(alpha(7, 7) > alpha(6, 7) && alpha(6, 7) > 0);
        assert_eq!(alpha(6, 7), alpha(8, 7));
        assert_eq!(alpha(7, 6), alpha(7, 8));
        assert!(result
            .chunks_exact(4)
            .all(|p| p[0] <= p[3] && p[1] <= p[3] && p[2] <= p[3]));
        assert_eq!(source[(7 * 15 + 7) * 4 + 3], 128);
    }
    #[test]
    fn region_uses_source_halo_and_matches_full_frame_crop() {
        let source: Pixels = (0..32 * 20)
            .flat_map(|i| [(i % 200) as u8, 40, 80, 255])
            .collect::<Vec<_>>()
            .into();
        let full = region(
            &source,
            (32, 20),
            Region {
                x: 0,
                y: 0,
                width: 32,
                height: 20,
            },
            3,
            || false,
        )
        .unwrap();
        let part = region(
            &source,
            (32, 20),
            Region {
                x: 7,
                y: 5,
                width: 13,
                height: 9,
            },
            3,
            || false,
        )
        .unwrap();
        for y in 0..9 {
            assert_eq!(
                &part[y * 13 * 4..(y + 1) * 13 * 4],
                &full[((y + 5) * 32 + 7) * 4..((y + 5) * 32 + 20) * 4]
            );
        }
        assert!(full[3] < 255); // 图像外透明，不把边缘像素无限复制。
    }
    #[test]
    fn cancellation_and_invalid_regions_do_not_modify_source() {
        let source: Pixels = vec![255; 16 * 16 * 4].into();
        let area = Region {
            x: 0,
            y: 0,
            width: 16,
            height: 16,
        };
        assert_eq!(
            region(&source, (16, 16), area, 0, || false),
            Err(BlurError::InvalidInput)
        );
        assert_eq!(
            region(
                &source,
                (16, 16),
                Region {
                    x: u32::MAX,
                    ..area
                },
                1,
                || false
            ),
            Err(BlurError::InvalidInput)
        );
        let calls = std::cell::Cell::new(0);
        assert_eq!(
            region(&source, (16, 16), area, 2, || {
                calls.set(calls.get() + 1);
                calls.get() > 4
            }),
            Err(BlurError::Cancelled)
        );
        assert!(source.iter().all(|b| *b == 255));
    }
}
