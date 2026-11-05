//! 内容橡皮擦的预乘 BGRA 内核；圆帽圆连接，原图只读，取消不返回半成品。
use crate::{
    pixels::{self, Pixels},
    viewport::{checked_image_bytes, Point},
};
#[path = "erase_frame.rs"]
mod frame;
#[path = "erase_span.rs"]
mod span;
pub use frame::{apply_history_in_frame, apply_in_frame, apply_in_projection};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EraseError {
    InvalidInput,
    BudgetExceeded,
    Cancelled,
}

fn allocate<T: Default + Clone>(
    count: usize,
) -> Result<(Vec<T>, crate::byte_budget::Lease), EraseError> {
    let bytes = count
        .checked_mul(std::mem::size_of::<T>())
        .ok_or(EraseError::BudgetExceeded)?;
    let mut lease = pixels::reserve(bytes).ok_or(EraseError::BudgetExceeded)?;
    let mut data = Vec::new();
    data.try_reserve_exact(count)
        .map_err(|_| EraseError::BudgetExceeded)?;
    if !lease.grow_to(data.capacity() * std::mem::size_of::<T>()) {
        return Err(EraseError::BudgetExceeded);
    }
    data.resize(count, T::default());
    Ok((data, lease))
}

fn valid(point: Point) -> bool {
    point.x.is_finite()
        && point.y.is_finite()
        && point.x.abs() <= 1_000_000.0
        && point.y.abs() <= 1_000_000.0
}

/// 4×4 子像素覆盖取并集；重复点、自交和重复段不会叠加擦除软边。
/// UI 尺寸范围与主干一致（4–96）；调用者负责把逻辑坐标换算到像素坐标。
pub fn apply(
    source: &Pixels,
    size: (u32, u32),
    points: &[Point],
    width: f32,
    cancelled: impl FnMut() -> bool,
) -> Result<Pixels, EraseError> {
    if !width.is_finite() || !(4.0..=96.0).contains(&width) {
        return Err(EraseError::InvalidInput);
    }
    apply_scaled(source, size, points, width, (1.0, 1.0), cancelled)
}

fn apply_scaled(
    source: &Pixels,
    size: (u32, u32),
    points: &[Point],
    width: f32,
    scale: (f64, f64),
    mut cancelled: impl FnMut() -> bool,
) -> Result<Pixels, EraseError> {
    if checked_image_bytes(size.0, size.1) != Some(source.len())
        || points.is_empty()
        || points.len() > 65_536
        || !points.iter().copied().all(valid)
        || !width.is_finite()
        || width <= 0.0
        || width > 1_000_000.0
    {
        return Err(EraseError::InvalidInput);
    }
    if cancelled() {
        return Err(EraseError::Cancelled);
    }
    let logical_radius = width as f64 / 2.0;
    // 椭圆胶囊的圆形外包只用于保守扫描；实际覆盖在逻辑坐标中计算。
    let radius = logical_radius * scale.0.max(scale.1);
    let region = span::region(points, radius, size);
    if cancelled() {
        return Err(EraseError::Cancelled);
    }
    let Some([region_left, region_top, region_right, region_bottom]) = region else {
        return Ok(source.clone());
    };
    // 整个受影响范围均透明时，不分配掩码或候选整图；逐行仍可取消。
    let mut has_content = false;
    for y in region_top..region_bottom {
        if cancelled() {
            return Err(EraseError::Cancelled);
        }
        let start = (y as usize * size.0 as usize + region_left as usize) * 4;
        let end = (y as usize * size.0 as usize + region_right as usize) * 4;
        if source[start..end].iter().any(|byte| *byte != 0) {
            has_content = true;
            break;
        }
    }
    if !has_content {
        return Ok(source.clone());
    }
    let mask_width = (region_right - region_left) as usize;
    let mask_height = (region_bottom - region_top) as usize;
    let (mut coverage, _mask_lease) = allocate::<u16>(mask_width * mask_height)?;
    // 直到首个像素实际改变才复制底图；包围框有内容但笔迹未命中仍返回原共享快照。
    let mut output: Option<(Vec<u8>, crate::byte_budget::Lease)> = None;
    for i in 0..points.len() {
        if cancelled() {
            return Err(EraseError::Cancelled);
        }
        let a = points[i.saturating_sub(1)];
        let b = points[i];
        let top = ((a.y.min(b.y) as f64 - radius).floor().max(0.0) as u32).min(size.1);
        let bottom = ((a.y.max(b.y) as f64 + radius).ceil().max(0.0) as u32).min(size.1);
        let dx = (b.x as f64 - a.x as f64) / scale.0;
        let dy = (b.y as f64 - a.y as f64) / scale.1;
        let length = dx * dx + dy * dy;
        for y in top..bottom {
            if cancelled() {
                return Err(EraseError::Cancelled);
            }
            let Some((left, right)) = span::row(a, b, radius, y, size.0) else {
                continue;
            };
            for x in left.max(region_left)..right.min(region_right) {
                let index = y as usize * size.0 as usize + x as usize;
                let mask_index =
                    (y - region_top) as usize * mask_width + (x - region_left) as usize;
                if coverage[mask_index] == u16::MAX {
                    continue;
                }
                let mut mask = coverage[mask_index];
                for sy in 0..4 {
                    for sx in 0..4 {
                        let px = (x as f64 + (sx as f64 + 0.5) / 4.0 - a.x as f64) / scale.0;
                        let py = (y as f64 + (sy as f64 + 0.5) / 4.0 - a.y as f64) / scale.1;
                        let t = if length == 0.0 {
                            0.0
                        } else {
                            ((px * dx + py * dy) / length).clamp(0.0, 1.0)
                        };
                        if (px - t * dx).powi(2) + (py - t * dy).powi(2)
                            <= logical_radius * logical_radius
                        {
                            mask |= 1 << (sy * 4 + sx);
                        }
                    }
                }
                if mask == coverage[mask_index] {
                    continue;
                }
                coverage[mask_index] = mask;
                let keep = 16 - mask.count_ones();
                let start = index * 4;
                let mut pixel = [0u8; 4];
                for channel in 0..4 {
                    pixel[channel] = ((source[start + channel] as u32 * keep + 8) / 16) as u8;
                }
                if pixel == source[start..start + 4] {
                    continue;
                }
                if output.is_none() {
                    if cancelled() {
                        return Err(EraseError::Cancelled);
                    }
                    let (mut bytes, lease) = allocate::<u8>(source.len())?;
                    bytes.copy_from_slice(source);
                    output = Some((bytes, lease));
                }
                output.as_mut().unwrap().0[start..start + 4].copy_from_slice(&pixel);
            }
        }
    }
    if cancelled() {
        return Err(EraseError::Cancelled);
    }
    Ok(match output {
        Some((bytes, lease)) => Pixels::from_reserved(bytes, lease),
        None => source.clone(),
    })
}

#[cfg(test)]
#[path = "erase_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "erase_reference_tests.rs"]
mod reference_tests;
