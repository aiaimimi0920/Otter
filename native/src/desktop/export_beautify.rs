//! 合成完成后的美化包装；只在后台处理，不改变内部标注/裁剪坐标。
use super::*;
#[path = "export_beautify_style.rs"]
mod style;
pub(in crate::desktop) use style::Settings;

fn check(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Acquire) {
        Err(Error::new(
            windows::Win32::Foundation::E_ABORT,
            "Beautify cancelled",
        ))
    } else {
        Ok(())
    }
}
fn allocate(size: (u32, u32)) -> Result<(Vec<u8>, crate::byte_budget::Lease)> {
    let count = checked_image_bytes(size.0, size.1)
        .ok_or_else(|| Error::new(E_INVALIDARG, "Beautify output exceeds pixel limits"))?;
    let mut lease = crate::pixels::reserve(count).ok_or_else(|| {
        Error::new(
            windows::Win32::Foundation::E_OUTOFMEMORY,
            "Beautify pixel budget exhausted",
        )
    })?;
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(count).map_err(|_| {
        Error::new(
            windows::Win32::Foundation::E_OUTOFMEMORY,
            "Cannot allocate beautify pixels",
        )
    })?;
    if !lease.grow_to(bytes.capacity()) {
        return Err(Error::new(
            windows::Win32::Foundation::E_OUTOFMEMORY,
            "Beautify capacity exceeds budget",
        ));
    }
    bytes.resize(count, 0);
    Ok((bytes, lease))
}

pub(super) fn apply(
    bitmap: &IWICBitmap,
    settings: &Settings,
    cancel: &AtomicBool,
) -> Result<Image> {
    check(cancel)?;
    let (mut width, mut height) = (0, 0);
    unsafe {
        bitmap.GetSize(&mut width, &mut height)?;
    }
    let (mut bytes, lease) = allocate((width, height))?;
    unsafe {
        bitmap.CopyPixels(std::ptr::null(), width * 4, &mut bytes)?;
    }
    let inner = Image {
        width,
        height,
        pixels: crate::pixels::Pixels::from_reserved(bytes, lease),
    };
    compose(&inner, settings, cancel)
}

// 4×4 子像素覆盖；与原生内容擦除采用相同采样精度，不承诺浏览器边缘逐字节一致。
fn coverage(x: u32, y: u32, width: u32, height: u32, radius: f64) -> f64 {
    let r = radius.min(width.min(height) as f64 / 2.0);
    if r <= 0.0 {
        return 1.0;
    }
    let mut hits = 0;
    for sy in 0..4 {
        for sx in 0..4 {
            let px = x as f64 + (sx as f64 + 0.5) / 4.0;
            let py = y as f64 + (sy as f64 + 0.5) / 4.0;
            let cx = px.clamp(r, width as f64 - r);
            let cy = py.clamp(r, height as f64 - r);
            if (px - cx).powi(2) + (py - cy).powi(2) <= r * r {
                hits += 1;
            }
        }
    }
    hits as f64 / 16.0
}

fn shadow(
    inner: &Image,
    settings: &Settings,
    size: (u32, u32),
    cancel: &AtomicBool,
) -> Result<Option<crate::pixels::Pixels>> {
    if !settings.shadow {
        return Ok(None);
    }
    let offset = settings.shadow_offset;
    // 底部阴影源可能超出输出框，先保留完整偏移卡片，再卷积/裁剪，避免边界漏算。
    let source_size = (
        size.0,
        size.1
            .checked_add(offset)
            .ok_or_else(|| Error::new(E_INVALIDARG, "Beautify shadow dimensions overflow"))?,
    );
    let (mut bytes, lease) = allocate(source_size)?;
    for y in 0..inner.height {
        check(cancel)?;
        for x in 0..inner.width {
            let at = (((y + settings.padding + offset) * size.0 + x + settings.padding) * 4 + 3)
                as usize;
            bytes[at] =
                (255.0 * coverage(x, y, inner.width, inner.height, settings.radius)).round() as u8;
        }
    }
    let source = crate::pixels::Pixels::from_reserved(bytes, lease);
    // WHATWG Canvas shadows：标准差为 shadowBlur/2，阴影颜色 alpha 在卷积后应用。
    let sigma = settings.shadow_sigma;
    crate::blur::region_scaled(
        &source,
        source_size,
        crate::blur::Region {
            x: 0,
            y: 0,
            width: size.0,
            height: size.1,
        },
        1.0,
        (sigma, sigma),
        || cancel.load(Ordering::Acquire),
    )
    .map(Some)
    .map_err(|error| {
        Error::new(
            match error {
                crate::blur::BlurError::Cancelled => windows::Win32::Foundation::E_ABORT,
                crate::blur::BlurError::BudgetExceeded => windows::Win32::Foundation::E_OUTOFMEMORY,
                _ => E_INVALIDARG,
            },
            "Cannot prepare beautify shadow",
        )
    })
}

fn compose(inner: &Image, settings: &Settings, cancel: &AtomicBool) -> Result<Image> {
    check(cancel)?;
    if checked_image_bytes(inner.width, inner.height) != Some(inner.pixels.len()) {
        return Err(Error::new(E_INVALIDARG, "Invalid beautify source"));
    }
    let twice = settings
        .padding
        .checked_mul(2)
        .ok_or_else(|| Error::new(E_INVALIDARG, "Beautify padding overflow"))?;
    let size = (
        inner.width.checked_add(twice),
        inner.height.checked_add(twice),
    );
    let (Some(width), Some(height)) = size else {
        return Err(Error::new(E_INVALIDARG, "Beautify dimensions overflow"));
    };
    let (mut bytes, lease) = allocate((width, height))?;
    let shadow = shadow(inner, settings, (width, height), cancel)?;
    let denominator = (width as f64).powi(2) + (height as f64).powi(2);
    for y in 0..height {
        check(cancel)?;
        for x in 0..width {
            let at = ((y * width + x) * 4) as usize;
            let t = (((x as f64 + 0.5) * width as f64 + (y as f64 + 0.5) * height as f64)
                / denominator)
                .clamp(0.0, 1.0);
            let shade = shadow
                .as_ref()
                .map_or(1.0, |pixels| 1.0 - pixels[at + 3] as f64 / 255.0 * 0.35);
            for channel in 0..3 {
                let index = 2 - channel; // CSS RGB -> 预乘 BGRA；背景始终不透明。
                bytes[at + channel] = ((settings.colors.0[index] as f64 * (1.0 - t)
                    + settings.colors.1[index] as f64 * t)
                    * shade)
                    .round() as u8;
            }
            bytes[at + 3] = 255;
            if x >= settings.padding
                && y >= settings.padding
                && x - settings.padding < inner.width
                && y - settings.padding < inner.height
            {
                let ix = x - settings.padding;
                let iy = y - settings.padding;
                let from = ((iy * inner.width + ix) * 4) as usize;
                let cover = coverage(ix, iy, inner.width, inner.height, settings.radius);
                for channel in 0..3 {
                    // 分别执行白卡片填充和圆角裁剪的图片绘制，保留两次覆盖的边缘语义。
                    let card = 255.0 * cover + bytes[at + channel] as f64 * (1.0 - cover);
                    let alpha = inner.pixels[from + 3] as f64 / 255.0 * cover;
                    bytes[at + channel] = (inner.pixels[from + channel] as f64 * cover
                        + card * (1.0 - alpha))
                        .round() as u8;
                }
            }
        }
    }
    check(cancel)?;
    Ok(Image {
        width,
        height,
        pixels: crate::pixels::Pixels::from_reserved(bytes, lease),
    })
}

#[cfg(test)]
#[path = "export_beautify_tests.rs"]
mod tests;
