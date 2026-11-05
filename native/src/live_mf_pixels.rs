//! NV12 surface 直接转换到预算内的 PBGRA；避免额外的整帧可见 NV12 副本。
use super::{
    samples::{Locked, MAX_BUFFER},
    Layout,
};
use crate::pixels;
use windows::{
    core::{Error, Interface, Result},
    Win32::{
        Foundation::{E_INVALIDARG, E_OUTOFMEMORY},
        Media::MediaFoundation::*,
    },
};
pub(super) fn decode(
    sample: &IMFSample,
    media: &IMFMediaType,
    layout: Layout,
) -> Result<pixels::Pixels> {
    unsafe {
        if sample.GetTotalLength()? > MAX_BUFFER {
            return Err(invalid("Decoded sample budget"));
        }
        let buffer = sample.ConvertToContiguousBuffer()?;
        if let Ok(surface) = buffer.cast::<IMF2DBuffer>() {
            let size = layout.width as usize * layout.height as usize * 3 / 2;
            if surface.GetContiguousLength()? as usize != size {
                return Err(invalid("Decoded surface size mismatch"));
            }
            let (mut storage, _lease) = allocate(size)?;
            surface.ContiguousCopyTo(&mut storage)?;
            convert(&storage, layout.width as usize, layout)
        } else {
            let stride = match media.GetUINT32(&MF_MT_DEFAULT_STRIDE) {
                Ok(v) => v as i32,
                Err(e) if e.code() == MF_E_ATTRIBUTENOTFOUND => layout.width as i32,
                Err(e) => return Err(e),
            };
            if stride < layout.width as i32 || stride > 8192 || stride % 2 != 0 {
                return Err(invalid("Decoder stride bounds"));
            }
            let (mut ptr, mut capacity, mut length) = (std::ptr::null_mut(), 0, 0);
            buffer.Lock(&mut ptr, Some(&mut capacity), Some(&mut length))?;
            let _lock = Locked(&buffer);
            if ptr.is_null() || length > capacity || length > MAX_BUFFER {
                return Err(invalid("Decoder buffer bounds"));
            }
            convert(
                std::slice::from_raw_parts(ptr, length as usize),
                stride as usize,
                layout,
            )
        }
    }
}
fn allocate(size: usize) -> Result<(Vec<u8>, crate::byte_budget::Lease)> {
    let fail = || Error::new(E_OUTOFMEMORY, "Decoder pixel budget exhausted");
    let mut lease = pixels::reserve(size).ok_or_else(fail)?;
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(size).map_err(|_| fail())?;
    if !lease.grow_to(bytes.capacity()) {
        return Err(fail());
    }
    bytes.resize(size, 0);
    Ok((bytes, lease))
}
fn invalid(message: &str) -> Error {
    Error::new(E_INVALIDARG, message)
}
fn convert(source: &[u8], stride: usize, layout: Layout) -> Result<pixels::Pixels> {
    let (w, h, sw, sh) = (
        layout.visible_width as usize,
        layout.visible_height as usize,
        layout.width as usize,
        layout.height as usize,
    );
    if w < 2
        || h < 2
        || w > sw
        || h > sh
        || sw > 4096
        || sh > 4096
        || w % 2 != 0
        || h % 2 != 0
        || sh % 2 != 0
        || stride < sw
        || stride > 8192
        || !stride.is_multiple_of(2)
        || sw * sh > 8_294_400
        || source.len() < stride * sh * 3 / 2
    {
        return Err(invalid("Decoder NV12 bounds"));
    }
    let (mut out, lease) = allocate(w * h * 4)?;
    for y in 0..h {
        for x in 0..w {
            let luma = i32::from(source[y * stride + x]) - 16;
            // UV 从存储高度后开始，不可用可见高度，否则 padding 图像会读错色度。
            let uv = stride * sh + (y / 2) * stride + (x & !1);
            let u = i32::from(source[uv]) - 128;
            let v = i32::from(source[uv + 1]) - 128;
            let channel = |n: i32| ((n + 128) >> 8).clamp(0, 255) as u8;
            out[(y * w + x) * 4..(y * w + x + 1) * 4].copy_from_slice(&[
                channel(298 * luma + 541 * u),
                channel(298 * luma - 55 * u - 136 * v),
                channel(298 * luma + 459 * v),
                255,
            ]);
        }
    }
    Ok(pixels::Pixels::from_reserved(out, lease))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn padded_surface_uses_storage_chroma_and_preserves_bgra() {
        let layout = Layout {
            width: 4,
            height: 4,
            visible_width: 2,
            visible_height: 2,
        };
        let mut source = [99; 24];
        source[..2].fill(16);
        source[4..6].fill(235);
        source[16..18].fill(128);
        let image = convert(&source, 4, layout).unwrap();
        assert_eq!(
            &*image,
            &[0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255]
        );
        assert!(convert(&source[..17], 4, layout).is_err());
        assert!(convert(&source, 3, layout).is_err());
        source[..2].fill(63);
        source[4..6].fill(63);
        source[16..18].copy_from_slice(&[102, 240]);
        for p in convert(&source, 4, layout).unwrap().chunks_exact(4) {
            assert!(p[0] <= 2 && p[1] <= 2 && p[2] >= 253 && p[3] == 255);
        }
    }
}
