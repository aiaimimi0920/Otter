//! 有界 DIB/DIBV5 解码；不访问外部 ICC、不信任 sizeImage、不按输入尺寸直接分配。
use super::image::Image;
#[path = "clipboard_dib_rle.rs"]
mod rle;
use windows::{
    core::{Error, Result},
    Win32::Foundation::*,
};

fn invalid() -> Error {
    Error::new(E_INVALIDARG, "Invalid or unsupported clipboard DIB")
}
fn word(bytes: &[u8], offset: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(
        bytes
            .get(offset..offset + 2)
            .ok_or_else(invalid)?
            .try_into()
            .unwrap(),
    ))
}
fn dword(bytes: &[u8], offset: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        bytes
            .get(offset..offset + 4)
            .ok_or_else(invalid)?
            .try_into()
            .unwrap(),
    ))
}
struct Layout {
    width: u32,
    height: u32,
    bottom_up: bool,
    stride: usize,
    offset: usize,
    pixel_bytes: usize,
    bits: u16,
    palette: std::ops::Range<usize>,
    compressed: bool,
    masks: [u32; 4],
    count: usize,
}
fn masks_valid(masks: [u32; 4], bits: u16) -> bool {
    let allowed = if bits == 32 {
        u32::MAX
    } else {
        (1u32 << bits) - 1
    };
    let mut occupied = 0;
    for (index, mask) in masks.into_iter().enumerate() {
        if mask == 0 {
            if index != 3 {
                return false;
            } else {
                continue;
            }
        }
        let shifted = mask >> mask.trailing_zeros();
        if mask & !allowed != 0 || mask & occupied != 0 || shifted & shifted.wrapping_add(1) != 0 {
            return false;
        }
        occupied |= mask;
    }
    true
}
fn layout(bytes: &[u8]) -> Result<Layout> {
    let header = dword(bytes, 0)? as usize;
    if !matches!(header, 40 | 52 | 56 | 108 | 124) || bytes.len() < header {
        return Err(invalid());
    }
    let width = dword(bytes, 4)? as i32;
    let signed_height = dword(bytes, 8)? as i32;
    let height = signed_height.unsigned_abs();
    if width <= 0 || word(bytes, 12)? != 1 {
        return Err(invalid());
    }
    let count = crate::viewport::checked_image_bytes(width as u32, height).ok_or_else(invalid)?;
    let bits = word(bytes, 14)?;
    let compression = dword(bytes, 16)?;
    let compressed = matches!((bits, compression), (8, 1) | (4, 2));
    if !matches!(bits, 1 | 4 | 8 | 16 | 24 | 32)
        || (!compressed
            && (!matches!(compression, 0 | 3 | 6)
                || (matches!(bits, 1 | 4 | 8 | 24) && compression != 0)))
        || (compressed && signed_height <= 0)
    {
        return Err(invalid());
    }
    if header >= 108 && !matches!(dword(bytes, 56)?, 0 | 0x73524742 | 0x57696e20) {
        return Err(invalid());
    }
    if header >= 108 && dword(bytes, 56)? == 0 && bytes[60..108].iter().any(|byte| *byte != 0) {
        return Err(invalid()); // 尚不支持校准色彩变换，不能静默误解颜色。
    }
    // 不跟随 PROFILE_LINKED，也不在不支持 ICC 的路径悄悄忽略嵌入配置。
    if header == 124 && (dword(bytes, 112)? != 0 || dword(bytes, 116)? != 0) {
        return Err(invalid());
    }
    let mut offset = header;
    let mut masks = if bits == 16 {
        [0x7c00, 0x03e0, 0x001f, 0]
    } else {
        [0xff0000, 0x00ff00, 0x0000ff, 0]
    };
    if matches!(compression, 3 | 6) {
        if header == 40 {
            offset += if compression == 6 { 16 } else { 12 };
        }
        if compression == 6 && header != 40 && header < 56 {
            return Err(invalid());
        }
        masks = [
            dword(bytes, 40)?,
            dword(bytes, 44)?,
            dword(bytes, 48)?,
            if header >= 56 || compression == 6 {
                dword(bytes, 52)?
            } else {
                0
            },
        ];
    } else if bits == 32 && header >= 56 {
        // 常见 V5 生产者在 BI_RGB 下仍填写 alpha mask，保留其透明通道。
        masks[3] = dword(bytes, 52)?;
    }
    if bits >= 16 && !masks_valid(masks, bits) {
        return Err(invalid());
    }
    if compression == 6 && masks[3] == 0 {
        return Err(invalid());
    }
    let declared_colors = dword(bytes, 32)? as usize;
    let colors = if bits <= 8 && declared_colors == 0 {
        1usize << bits
    } else {
        declared_colors
    };
    if colors > 256 || (bits <= 8 && colors > 1usize << bits) {
        return Err(invalid());
    }
    let palette_start = offset;
    offset = offset.checked_add(colors * 4).ok_or_else(invalid)?;
    let stride = (width as usize * bits as usize).div_ceil(32) * 4;
    let end = stride
        .checked_mul(height as usize)
        .and_then(|length| offset.checked_add(length))
        .ok_or_else(invalid)?;
    if offset > bytes.len() || (!compressed && end > bytes.len()) {
        return Err(invalid());
    }
    Ok(Layout {
        width: width as u32,
        height,
        bottom_up: signed_height > 0,
        stride,
        offset,
        pixel_bytes: bits as usize / 8,
        bits,
        palette: palette_start..offset,
        compressed,
        masks,
        count,
    })
}
fn channel(value: u32, mask: u32, fallback: u8) -> u8 {
    if mask == 0 {
        return fallback;
    }
    let shift = mask.trailing_zeros();
    let maximum = (mask >> shift) as u64;
    ((((value & mask) >> shift) as u64 * 255 + maximum / 2) / maximum) as u8
}
#[cfg(test)]
fn decode(bytes: &[u8]) -> Result<Image> {
    decode_with_cancel(bytes, || false)
}

pub(super) fn decode_with_cancel(bytes: &[u8], cancelled: impl Fn() -> bool) -> Result<Image> {
    let check = || {
        if cancelled() {
            Err(Error::new(E_ABORT, "DIB decode cancelled"))
        } else {
            Ok(())
        }
    };
    check()?;
    let layout = layout(bytes)?;
    let mut lease = crate::pixels::reserve(layout.count)
        .ok_or_else(|| Error::new(E_OUTOFMEMORY, "DIB pixel budget exhausted"))?;
    let mut pixels = Vec::new();
    pixels
        .try_reserve_exact(layout.count)
        .map_err(|_| Error::new(E_OUTOFMEMORY, "Cannot allocate DIB pixels"))?;
    if !lease.grow_to(pixels.capacity()) {
        return Err(Error::new(E_OUTOFMEMORY, "DIB capacity exceeds budget"));
    }
    pixels.resize(layout.count, 0);
    if layout.compressed {
        rle::decode(bytes, &layout, &mut pixels, &check)?;
        check()?;
        return Ok(Image {
            width: layout.width,
            height: layout.height,
            pixels: crate::pixels::Pixels::from_reserved(pixels, lease),
        });
    }
    for (y, row) in pixels
        .chunks_exact_mut(layout.width as usize * 4)
        .enumerate()
    {
        check()?;
        let source_y = if layout.bottom_up {
            layout.height as usize - 1 - y
        } else {
            y
        };
        let source = &bytes[layout.offset + source_y * layout.stride..];
        for (x, pixel) in row.chunks_exact_mut(4).enumerate() {
            if layout.bits <= 8 {
                // DIB 的子字节索引从高位开始；RGBQUAD 的保留字节不是 alpha。
                let bit = x * layout.bits as usize;
                let shift = 8 - layout.bits as usize - bit % 8;
                let index =
                    ((source[bit / 8] >> shift) & ((1u16 << layout.bits) - 1) as u8) as usize;
                let color = bytes[layout.palette.clone()]
                    .get(index * 4..index * 4 + 4)
                    .ok_or_else(invalid)?;
                pixel[..3].copy_from_slice(&color[..3]);
                pixel[3] = 255;
                continue;
            }
            let start = x * layout.pixel_bytes;
            let mut value = [0; 4];
            value[..layout.pixel_bytes].copy_from_slice(&source[start..start + layout.pixel_bytes]);
            let value = u32::from_le_bytes(value);
            let alpha = channel(value, layout.masks[3], 255);
            for (output, mask) in
                pixel[..3]
                    .iter_mut()
                    .zip([layout.masks[2], layout.masks[1], layout.masks[0]])
            {
                *output = ((channel(value, mask, 0) as u32 * alpha as u32 + 127) / 255) as u8;
            }
            pixel[3] = alpha;
        }
    }
    check()?;
    Ok(Image {
        width: layout.width,
        height: layout.height,
        pixels: crate::pixels::Pixels::from_reserved(pixels, lease),
    })
}

#[cfg(test)]
#[path = "clipboard_dib_tests.rs"]
mod tests;
