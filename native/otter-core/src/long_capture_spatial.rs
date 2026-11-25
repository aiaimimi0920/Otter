//! 二维已定位截图合成；帧的几何范围决定覆盖，输出 alpha 标记该像素已写入。
use super::*;
use std::sync::atomic::Ordering;
#[path = "long_capture_coverage.rs"]
pub mod coverage;

pub struct SpatialFrame {
    pub frame: Frame,
    pub x: i64,
    pub y: i64,
}
pub struct SpatialComposite {
    pub width: u32,
    pub height: u32,
    pub x: i64,
    pub y: i64,
    pub pixels: Pixels,
}
fn check(cancel: &AtomicBool) -> Result<(), Error> {
    if cancel.load(Ordering::Acquire) {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}
pub(super) fn bounds(
    frames: &[SpatialFrame],
    cancel: &AtomicBool,
) -> Result<(i64, i64, u32, u32), Error> {
    check(cancel)?;
    let first = frames.first().ok_or(Error::Empty)?;
    if frames.len() > MAX_FRAMES {
        return Err(Error::FrameLimit);
    }
    let (mut left, mut top, mut right, mut bottom) = (first.x, first.y, first.x, first.y);
    let mut total = 0u64;
    for item in frames {
        check(cancel)?;
        total += u64::from(item.frame.width) * u64::from(item.frame.height);
        if total > MAX_INPUT_PIXELS {
            return Err(Error::InputLimit);
        }
        left = left.min(item.x);
        top = top.min(item.y);
        right = right.max(
            item.x
                .checked_add(i64::from(item.frame.width))
                .ok_or(Error::OutputLimit)?,
        );
        bottom = bottom.max(
            item.y
                .checked_add(i64::from(item.frame.height))
                .ok_or(Error::OutputLimit)?,
        );
    }
    let width = right
        .checked_sub(left)
        .and_then(|n| u32::try_from(n).ok())
        .ok_or(Error::OutputLimit)?;
    let height = bottom
        .checked_sub(top)
        .and_then(|n| u32::try_from(n).ok())
        .ok_or(Error::OutputLimit)?;
    output_bytes(width, height)?;
    // 与主干二维画布一致：覆盖画布上限独立于一维长图上限。
    if u64::from(width) * u64::from(height) > 64 * 1024 * 1024 {
        return Err(Error::OutputLimit);
    }
    Ok((left, top, width, height))
}
/// 位置须由上层完成重叠验证；此函数不把任意已定位帧当作匹配成功的证据。
/// 先裁出最大完整矩形，只为实际交付的范围分配像素。
pub fn stitch_spatial(
    frames: &[SpatialFrame],
    cancel: &AtomicBool,
) -> Result<SpatialComposite, Error> {
    compose(frames, cancel, crate::pixels::reserve)
}
pub(super) fn check_output(
    target: OutputTarget,
    frames: &[SpatialFrame],
    bounds: (i64, i64, u32, u32),
    cancel: &AtomicBool,
) -> Result<(), Error> {
    let (left, top, width, height) = bounds;
    if target.check(width, height).is_err() {
        // An oversized envelope may still crop to a valid document (e.g. a diagonal sweep).
        let (_, _, width, height) = coverage::largest(frames, left, top, cancel)?;
        target.check(width, height)?;
    }
    Ok(())
}
fn compose(
    frames: &[SpatialFrame],
    cancel: &AtomicBool,
    reserve: impl FnOnce(usize) -> Option<crate::byte_budget::Lease>,
) -> Result<SpatialComposite, Error> {
    let (left, top, _, _) = bounds(frames, cancel)?;
    let (x, y, width, height) = coverage::largest(frames, left, top, cancel)?;
    let (left, top) = (left + i64::from(x), top + i64::from(y));
    let (right, bottom) = (left + i64::from(width), top + i64::from(height));
    let length = output_bytes(width, height)?;
    let mut lease = reserve(length).ok_or(Error::Budget)?;
    check(cancel)?;
    let mut pixels = Vec::new();
    pixels
        .try_reserve_exact(length)
        .map_err(|_| Error::Budget)?;
    if !lease.grow_to(pixels.capacity()) {
        return Err(Error::Budget);
    }
    pixels.resize(length, 0);
    let stride = width as usize * 4;
    for item in frames {
        check(cancel)?;
        let start_x = item.x.max(left);
        let end_x = (item.x + i64::from(item.frame.width)).min(right);
        let start_y = item.y.max(top);
        let end_y = (item.y + i64::from(item.frame.height)).min(bottom);
        if start_x >= end_x || start_y >= end_y {
            continue;
        }
        let source_stride = item.frame.width as usize * 4;
        let row_bytes = (end_x - start_x) as usize * 4;
        for row in start_y..end_y {
            check(cancel)?;
            let dest = (row - top) as usize * stride + (start_x - left) as usize * 4;
            let source = (row - item.y) as usize * source_stride + (start_x - item.x) as usize * 4;
            for (target, sample) in pixels[dest..dest + row_bytes]
                .chunks_exact_mut(4)
                .zip(item.frame.pixels[source..source + row_bytes].chunks_exact(4))
            {
                if target[3] == 0 {
                    target[..3].copy_from_slice(&sample[..3]);
                    target[3] = 255;
                }
            }
        }
    }
    check(cancel)?;
    Ok(SpatialComposite {
        width,
        height,
        x: left,
        y: top,
        pixels: Pixels::from_reserved(pixels, lease),
    })
}

#[cfg(test)]
#[path = "long_capture_spatial_tests.rs"]
mod tests;
