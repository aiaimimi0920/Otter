//! 二维已定位截图合成；透明 alpha 只作内部覆盖标记，输入截图颜色与 alpha 不决定覆盖。
use super::*;
use std::sync::atomic::Ordering;
#[path = "long_capture_coverage.rs"]
mod coverage;

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
/// 先建立透明覆盖画布，再原地压紧最大完整矩形，不同时分配第二张输出图。
pub fn stitch_spatial(
    frames: &[SpatialFrame],
    cancel: &AtomicBool,
) -> Result<SpatialComposite, Error> {
    compose(frames, cancel, crate::pixels::reserve)
}
fn compose(
    frames: &[SpatialFrame],
    cancel: &AtomicBool,
    reserve: impl FnOnce(usize) -> Option<crate::byte_budget::Lease>,
) -> Result<SpatialComposite, Error> {
    let (left, top, width, height) = bounds(frames, cancel)?;
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
        let x = (item.x - left) as usize;
        let y = (item.y - top) as usize;
        let source_stride = item.frame.width as usize * 4;
        for row in 0..item.frame.height as usize {
            check(cancel)?;
            let start = (y + row) * stride + x * 4;
            let source = &item.frame.pixels[row * source_stride..(row + 1) * source_stride];
            for (target, sample) in pixels[start..start + source_stride]
                .chunks_exact_mut(4)
                .zip(source.chunks_exact(4))
            {
                if target[3] == 0 {
                    target[..3].copy_from_slice(&sample[..3]);
                    target[3] = 255;
                }
            }
        }
    }
    let (x, y, cropped_width, cropped_height) = coverage::largest(&pixels, width, height, cancel)?;
    let cropped_stride = cropped_width as usize * 4;
    for row in 0..cropped_height as usize {
        check(cancel)?;
        let source = (y as usize + row) * stride + x as usize * 4;
        pixels.copy_within(source..source + cropped_stride, row * cropped_stride);
    }
    pixels.truncate(cropped_stride * cropped_height as usize);
    check(cancel)?;
    // truncate 不释放 capacity：租约继续按完整分配计费，不能因裁剪少计实际驻留内存。
    Ok(SpatialComposite {
        width: cropped_width,
        height: cropped_height,
        x: left + i64::from(x),
        y: top + i64::from(y),
        pixels: Pixels::from_reserved(pixels, lease),
    })
}

#[cfg(test)]
#[path = "long_capture_spatial_tests.rs"]
mod tests;
