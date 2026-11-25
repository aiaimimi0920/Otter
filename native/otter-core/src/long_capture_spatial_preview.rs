//! 直接从共享原帧采样二维预览；不分配完整覆盖画布，较早帧拥有重叠像素。
use super::*;
pub(super) fn render(
    frames: &[SpatialFrame],
    cancel: &AtomicBool,
) -> Result<SpatialComposite, Error> {
    let (left, top, full_width, full_height) = spatial::bounds(frames, cancel)?;
    let edge = full_width.max(full_height);
    let width =
        (u64::from(full_width) * 384 / u64::from(edge)).clamp(1, u64::from(full_width)) as u32;
    let height =
        (u64::from(full_height) * 384 / u64::from(edge)).clamp(1, u64::from(full_height)) as u32;
    let length = width as usize * height as usize * 4;
    let mut lease = crate::pixels::reserve(length).ok_or(Error::Budget)?;
    let mut pixels = Vec::new();
    pixels
        .try_reserve_exact(length)
        .map_err(|_| Error::Budget)?;
    if !lease.grow_to(pixels.capacity()) {
        return Err(Error::Budget);
    }
    pixels.resize(length, 0);
    // 每个采样坐标只计算一次。按行跳过不相交帧，按帧的水平范围访问像素，
    // 避免每个预览像素从头搜索全部截图；alpha 仍只表示早帧已覆盖。
    let mut samples = [0i64; 384];
    for x in 0..width {
        samples[x as usize] =
            left + ((u64::from(x) * 2 + 1) * u64::from(full_width) / (u64::from(width) * 2)) as i64;
    }
    let samples = &samples[..width as usize];
    let spans: Vec<_> = frames
        .iter()
        .map(|item| {
            samples.partition_point(|x| *x < item.x)
                ..samples.partition_point(|x| *x < item.x + i64::from(item.frame.width))
        })
        .collect();
    for y in 0..height {
        check(cancel)?;
        let sy = top
            + ((u64::from(y) * 2 + 1) * u64::from(full_height) / (u64::from(height) * 2)) as i64;
        let row =
            &mut pixels[y as usize * width as usize * 4..(y + 1) as usize * width as usize * 4];
        let mut remaining = width;
        for (item, span) in frames.iter().zip(&spans) {
            let fy = sy - item.y;
            if fy < 0 || fy >= i64::from(item.frame.height) {
                continue;
            }
            for x in span.clone() {
                let target = x * 4;
                if row[target + 3] != 0 {
                    continue;
                }
                let fx = samples[x] - item.x;
                let source = (fy as usize * item.frame.width as usize + fx as usize) * 4;
                row[target..target + 3].copy_from_slice(&item.frame.bytes()[source..source + 3]);
                row[target + 3] = 255;
                remaining -= 1;
            }
            if remaining == 0 {
                break;
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
#[path = "long_capture_spatial_preview_tests.rs"]
mod tests;
