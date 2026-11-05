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
    for y in 0..height {
        check(cancel)?;
        let sy = top
            + ((u64::from(y) * 2 + 1) * u64::from(full_height) / (u64::from(height) * 2)) as i64;
        for x in 0..width {
            let sx = left
                + ((u64::from(x) * 2 + 1) * u64::from(full_width) / (u64::from(width) * 2)) as i64;
            for item in frames {
                let fx = sx - item.x;
                let fy = sy - item.y;
                if fx < 0
                    || fy < 0
                    || fx >= i64::from(item.frame.width)
                    || fy >= i64::from(item.frame.height)
                {
                    continue;
                }
                let source = (fy as usize * item.frame.width as usize + fx as usize) * 4;
                let target = (y as usize * width as usize + x as usize) * 4;
                pixels[target..target + 3].copy_from_slice(&item.frame.bytes()[source..source + 3]);
                pixels[target + 3] = 255;
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
