//! Luminance analysis/SDR fallback on borrowed FP16 scRGB capture buffers; no capture-session creation/GPU-mapping ownership.
use crate::pixels::Pixels;
use std::sync::atomic::{AtomicBool, Ordering};
#[path = "capture_color_hdr.rs"]
pub mod hdr;
pub use hdr::HdrFrame;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Layout,
    Budget,
    Cancelled,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Analysis {
    pub maximum_nits: f32,
    pub average_nits: f32,
}
pub struct SdrFrame {
    pub width: u32,
    pub height: u32,
    pub pixels: Pixels,
}
pub struct ScRgb<'a> {
    data: &'a [u8],
    width: u32,
    height: u32,
    stride: usize,
    output_bytes: usize,
}
impl<'a> ScRgb<'a> {
    /// Little-endian R16G16B16A16_FLOAT rows allow padding; borrows cannot outlive mapped resources.
    pub fn new(data: &'a [u8], width: u32, height: u32, stride: usize) -> Result<Self, Error> {
        let output_bytes =
            crate::viewport::checked_image_bytes(width, height).ok_or(Error::Layout)?;
        let row_bytes = (width as usize).checked_mul(8).ok_or(Error::Layout)?;
        let required = stride.checked_mul(height as usize).ok_or(Error::Layout)?;
        if stride < row_bytes || required > data.len() {
            return Err(Error::Layout);
        }
        Ok(Self {
            data,
            width,
            height,
            stride,
            output_bytes,
        })
    }
    fn rows(&self) -> impl Iterator<Item = &[u8]> {
        self.data
            .chunks_exact(self.stride)
            .take(self.height as usize)
            .map(|row| &row[..self.width as usize * 8])
    }
    /// Auto only needs a threshold decision; full-frame metadata belongs to HDR encoding.
    pub fn exceeds_sdr_white(
        &self,
        white_nits: f32,
        gain: f32,
        cancel: &AtomicBool,
    ) -> Result<bool, Error> {
        check(cancel)?;
        let threshold = white_nits.max(1.0) + 5.0;
        if !white_nits.is_finite() || threshold >= 10_000.0 {
            return Ok(false);
        }
        let gain = normalized_gain(gain);
        for row in self.rows() {
            check(cancel)?;
            if row
                .chunks_exact(8)
                .any(|pixel| luminance(rgb(pixel, gain)) * 80.0 > threshold)
            {
                check(cancel)?;
                return Ok(true);
            }
        }
        check(cancel)?;
        Ok(false)
    }
    /// Match mainline SDR fallback: white-level normalization, clamping, sRGB OETF; not tone compression preserving HDR highlights.
    pub fn to_sdr(
        &self,
        white_nits: f32,
        gain: f32,
        cancel: &AtomicBool,
    ) -> Result<SdrFrame, Error> {
        self.convert(white_nits, gain, cancel, crate::pixels::reserve)
    }
    fn convert(
        &self,
        white_nits: f32,
        gain: f32,
        cancel: &AtomicBool,
        reserve: impl FnOnce(usize) -> Option<crate::byte_budget::Lease>,
    ) -> Result<SdrFrame, Error> {
        check(cancel)?;
        let white = if white_nits.is_finite() {
            white_nits.max(80.0)
        } else {
            203.0
        };
        let gain = normalized_gain(gain);
        let scale = 80.0 / white;
        let mut lease = reserve(self.output_bytes).ok_or(Error::Budget)?;
        check(cancel)?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(self.output_bytes)
            .map_err(|_| Error::Budget)?;
        if !lease.grow_to(bytes.capacity()) {
            return Err(Error::Budget);
        }
        let lut = sdr_lut(self.output_bytes / 4, gain, scale, cancel)?;
        let lookup = |lo: u8, hi: u8| {
            let bits = u16::from_le_bytes([lo, hi]);
            match &lut {
                Some(lut) if bits & 0x8000 == 0 => lut[bits as usize],
                Some(_) => 0,
                None => srgb((nonnegative_half(bits) * gain) * scale),
            }
        };
        let row_out = self.width as usize * 4;
        for row in self.rows() {
            check(cancel)?;
            let row_start = bytes.len();
            bytes.resize(row_start + row_out, 0);
            let out_row = &mut bytes[row_start..];
            for (pixel, dst) in row.chunks_exact(8).zip(out_row.chunks_exact_mut(4)) {
                dst.copy_from_slice(&[
                    lookup(pixel[4], pixel[5]),
                    lookup(pixel[2], pixel[3]),
                    lookup(pixel[0], pixel[1]),
                    255,
                ]);
            }
        }
        check(cancel)?;
        Ok(SdrFrame {
            width: self.width,
            height: self.height,
            pixels: Pixels::from_reserved(bytes, lease),
        })
    }
}
const SDR_LUT_MIN_PIXELS: usize = 32_768;
fn sdr_lut(
    pixels: usize,
    gain: f32,
    scale: f32,
    cancel: &AtomicBool,
) -> Result<Option<[u8; 32768]>, Error> {
    // Compute small images directly instead of fixed 32768 conversions for few pixels; large images retain half lookup optimization.
    if pixels < SDR_LUT_MIN_PIXELS {
        return Ok(None);
    }
    let mut lut = [0u8; 32768];
    for (bits, slot) in lut.iter_mut().enumerate() {
        if bits.is_multiple_of(256) {
            check(cancel)?;
        }
        *slot = srgb((nonnegative_half(bits as u16) * gain) * scale);
    }
    Ok(Some(lut))
}
fn luminance([r, g, b]: [f32; 3]) -> f32 {
    0.2126 * r + 0.7152 * g + 0.0722 * b
}
fn check(cancel: &AtomicBool) -> Result<(), Error> {
    if cancel.load(Ordering::Acquire) {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}
fn normalized_gain(gain: f32) -> f32 {
    if gain.is_finite() {
        gain.clamp(1.0, 8.0)
    } else {
        1.0
    }
}
fn rgb(pixel: &[u8], gain: f32) -> [f32; 3] {
    [0, 2, 4].map(|offset| {
        nonnegative_half(u16::from_le_bytes([pixel[offset], pixel[offset + 1]])) * gain
    })
}
// Match mainline: negatives/NaN/Inf become zero; no new half dependency or integer-truncation interpretation of float bytes.
fn nonnegative_half(bits: u16) -> f32 {
    let exponent = u32::from((bits >> 10) & 31);
    let fraction = u32::from(bits & 1023);
    if bits & 0x8000 != 0 || exponent == 31 {
        0.0
    } else if exponent == 0 {
        fraction as f32 / 16_777_216.0
    } else {
        f32::from_bits(((exponent + 112) << 23) | (fraction << 13))
    }
}
fn srgb(linear: f32) -> u8 {
    let linear = linear.clamp(0.0, 1.0);
    let encoded = if linear <= 0.003_130_8 {
        12.92 * linear
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    };
    (encoded * 255.0).round() as u8
}
#[cfg(test)]
#[path = "capture_color_tests.rs"]
mod tests;
