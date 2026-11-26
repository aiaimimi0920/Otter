//! Convert bounded NV12 buffers to budgeted PBGRA; MF first copies 2D surfaces into a known range.
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
/// Reusable budgeted scratch buffer for `IMF2DBuffer::ContiguousCopyTo` on hardware H.264 decode paths.
/// Eliminates ~3.1-12.4 MiB per-frame `VirtualAlloc`/`VirtualFree` + zero-page fault churn at 30-60 fps
/// while releasing its pixel-budget lease immediately when `VideoDecoder` closes or resets on error.
#[derive(Default)]
pub(super) struct Nv12Scratch {
    bytes: Vec<u8>,
    _lease: Option<crate::byte_budget::Lease>,
}

impl Nv12Scratch {
    pub(super) fn clear(&mut self) {
        self.bytes = Vec::new();
        self._lease = None;
    }

    fn slice_mut(&mut self, size: usize) -> Result<&mut [u8]> {
        if self.bytes.len() != size {
            self.clear();
            let (bytes, lease) = allocate_zeroed(size)?;
            self.bytes = bytes;
            self._lease = Some(lease);
        }
        Ok(&mut self.bytes)
    }
}

pub(super) fn decode(
    sample: &IMFSample,
    media: &IMFMediaType,
    layout: Layout,
    scratch: &mut Nv12Scratch,
) -> Result<pixels::Pixels> {
    unsafe {
        if sample.GetTotalLength()? > MAX_BUFFER {
            return Err(invalid("Decoded sample budget"));
        }
        let buffer = sample.ConvertToContiguousBuffer()?;
        if let Ok(surface) = buffer.cast::<IMF2DBuffer>() {
            validate_layout(layout.width as usize, layout)?;
            let size = layout.width as usize * layout.height as usize * 3 / 2;
            if surface.GetContiguousLength()? as usize != size {
                return Err(invalid("Decoded surface size mismatch"));
            }
            // Lock2D scanline/pitch do not establish the full NV12 backing range and cannot justify constructing a raw UV slice.
            let storage = scratch.slice_mut(size)?;
            surface.ContiguousCopyTo(storage)?;
            convert(storage, layout.width as usize, layout)
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
fn reserve_vec(size: usize) -> Result<(Vec<u8>, crate::byte_budget::Lease)> {
    let fail = || Error::new(E_OUTOFMEMORY, "Decoder pixel budget exhausted");
    let mut lease = pixels::reserve(size).ok_or_else(fail)?;
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(size).map_err(|_| fail())?;
    if !lease.grow_to(bytes.capacity()) {
        return Err(fail());
    }
    Ok((bytes, lease))
}
fn allocate_zeroed(size: usize) -> Result<(Vec<u8>, crate::byte_budget::Lease)> {
    let (mut bytes, lease) = reserve_vec(size)?;
    bytes.resize(size, 0);
    Ok((bytes, lease))
}
fn invalid(message: &str) -> Error {
    Error::new(E_INVALIDARG, message)
}
fn validate_layout(stride: usize, layout: Layout) -> Result<(usize, usize, usize, usize)> {
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
        || !w.is_multiple_of(2)
        || !h.is_multiple_of(2)
        || !sh.is_multiple_of(2)
        || stride < sw
        || stride > 8192
        || !stride.is_multiple_of(2)
        || sw * sh > 8_294_400
    {
        return Err(invalid("Decoder NV12 bounds"));
    }
    Ok((w, h, sw, sh))
}
fn convert(source: &[u8], stride: usize, layout: Layout) -> Result<pixels::Pixels> {
    let (w, h, _, sh) = validate_layout(stride, layout)?;
    if source.len() < stride * sh * 3 / 2 {
        return Err(invalid("Decoder NV12 bounds"));
    }
    let (y_plane, uv_plane) = source.split_at(stride * sh);
    let total = w * h * 4;
    let (mut out, lease) = reserve_vec(total)?;
    let channel = |n: i32| ((n + 128) >> 8).clamp(0, 255) as u8;
    // Reuse each UV chroma pair for a 2x2 pixel block and fill the reserved output directly, avoiding full-frame zeroing and 3/4 of the chroma multiplications.
    for (y, rows) in (0..h)
        .step_by(2)
        .zip(out.spare_capacity_mut()[..total].chunks_exact_mut(w * 8))
    {
        let y0 = &y_plane[y * stride..y * stride + w];
        let y1 = &y_plane[(y + 1) * stride..(y + 1) * stride + w];
        let uv_row = &uv_plane[y / 2 * stride..y / 2 * stride + w];
        let (out0, out1) = rows.split_at_mut(w * 4);
        for x in (0..w).step_by(2) {
            let u = i32::from(uv_row[x]) - 128;
            let v = i32::from(uv_row[x + 1]) - 128;
            let bu = 541 * u;
            let guv = -55 * u - 136 * v;
            let rv = 459 * v;
            let write_pair = |row_y: &[u8], row_out: &mut [std::mem::MaybeUninit<u8>]| {
                let l0 = 298 * (i32::from(row_y[x]) - 16);
                let l1 = 298 * (i32::from(row_y[x + 1]) - 16);
                let encoded = [
                    channel(l0 + bu),
                    channel(l0 + guv),
                    channel(l0 + rv),
                    255,
                    channel(l1 + bu),
                    channel(l1 + guv),
                    channel(l1 + rv),
                    255,
                ];
                for (slot, value) in row_out[x * 4..x * 4 + 8].iter_mut().zip(encoded) {
                    slot.write(value);
                }
            };
            write_pair(y0, out0);
            write_pair(y1, out1);
        }
    }
    // w/h are positive and even; the two-row, two-pixel traversal above initializes exactly total bytes without raw source-pointer reads.
    unsafe { out.set_len(total) };
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
    #[test]
    fn paired_nv12_rows_match_scalar_reference_for_padded_color_planes() {
        let layout = Layout {
            width: 8,
            height: 6,
            visible_width: 6,
            visible_height: 4,
        };
        let stride = 12;
        let source: Vec<u8> = (0..stride * 6 * 3 / 2)
            .map(|n| (n * 73 + 19) as u8)
            .collect();
        let output = convert(&source, stride, layout).unwrap();
        for (index, actual) in output.chunks_exact(4).enumerate() {
            let (x, y) = (index % 6, index / 6);
            let luma = i32::from(source[y * stride + x]) - 16;
            let uv = stride * 6 + y / 2 * stride + (x & !1);
            let u = i32::from(source[uv]) - 128;
            let v = i32::from(source[uv + 1]) - 128;
            let channel = |n: i32| ((n + 128) >> 8).clamp(0, 255) as u8;
            assert_eq!(
                actual,
                [
                    channel(298 * luma + 541 * u),
                    channel(298 * luma - 55 * u - 136 * v),
                    channel(298 * luma + 459 * v),
                    255,
                ]
            );
        }
        assert!(convert(&source[..source.len() - 1], stride, layout).is_err());
    }
}
