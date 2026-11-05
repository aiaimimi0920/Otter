//! 借用 FP16 scRGB 捕获缓冲的亮度分析与 SDR 回退；不创建截图会话、不拥有 GPU 映射。
use crate::pixels::Pixels;
use std::sync::atomic::{AtomicBool, Ordering};
#[path = "capture_color_hdr.rs"]
mod hdr;
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
    /// R16G16B16A16_FLOAT 小端行缓冲；允许行尾填充，借用不得超出映射资源生命周期。
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
    pub fn analyze(&self, gain: f32, cancel: &AtomicBool) -> Result<Analysis, Error> {
        check(cancel)?;
        let gain = normalized_gain(gain);
        let mut maximum = 0f32;
        let mut sum = 0f64;
        for row in self.rows() {
            check(cancel)?;
            for pixel in row.chunks_exact(8) {
                let [r, g, b] = rgb(pixel, gain);
                let luminance = 0.2126 * r + 0.7152 * g + 0.0722 * b;
                maximum = maximum.max(luminance);
                sum += f64::from(luminance);
            }
        }
        check(cancel)?;
        Ok(Analysis {
            maximum_nits: (maximum * 80.0).clamp(0.0, 10_000.0),
            average_nits: ((sum / (u64::from(self.width) * u64::from(self.height)) as f64) as f32
                * 80.0)
                .clamp(0.0, 10_000.0),
        })
    }
    /// 对齐主干 SDR 回退：按白电平归一化、限幅、sRGB OETF；不是保留 HDR 高光的压缩算子。
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
        for row in self.rows() {
            check(cancel)?;
            for pixel in row.chunks_exact(8) {
                let [r, g, b] = rgb(pixel, gain);
                bytes.extend_from_slice(&[srgb(b * scale), srgb(g * scale), srgb(r * scale), 255]);
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
pub fn exceeds_sdr_white(analysis: Analysis, white_nits: f32) -> bool {
    analysis.maximum_nits.is_finite()
        && white_nits.is_finite()
        && analysis.maximum_nits > white_nits.max(1.0) + 5.0
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
// 对齐主干：负值、NaN、Inf 置零；不增加 half 依赖，也不通过整数截断解释浮点字节。
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
