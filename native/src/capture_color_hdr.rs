//! HDR 输出专用 RGB16 大端数据；不能作为 BGRA 预览使用。色度与 PQ 规则对齐主干。
use super::{check, normalized_gain, rgb, Analysis, Error, ScRgb};
use crate::{byte_budget::Lease, pixels::Pixels};
use std::sync::atomic::AtomicBool;

#[derive(Debug)]
pub struct HdrFrame {
    pub width: u32,
    pub height: u32,
    pub rgb16_be: Pixels,
    pub analysis: Analysis,
    pub mastering_minimum_nits: f32,
    pub mastering_maximum_nits: f32,
}
impl ScRgb<'_> {
    /// 直接转换借用的浮点行；输出保留高于 SDR 白电平的亮度，不生成中间 SDR 图像。
    pub fn to_hdr(
        &self,
        minimum_nits: f32,
        maximum_nits: f32,
        gain: f32,
        cancel: &AtomicBool,
    ) -> Result<HdrFrame, Error> {
        self.convert_hdr(
            minimum_nits,
            maximum_nits,
            gain,
            cancel,
            crate::pixels::reserve,
        )
    }
    fn convert_hdr(
        &self,
        minimum_nits: f32,
        maximum_nits: f32,
        gain: f32,
        cancel: &AtomicBool,
        reserve: impl FnOnce(usize) -> Option<Lease>,
    ) -> Result<HdrFrame, Error> {
        check(cancel)?;
        let length = (self.width as usize)
            .checked_mul(self.height as usize)
            .and_then(|pixels| pixels.checked_mul(6))
            .ok_or(Error::Layout)?;
        let mut lease = reserve(length).ok_or(Error::Budget)?;
        check(cancel)?;
        let analysis = self.analyze(gain, cancel)?;
        let gain = normalized_gain(gain);
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(length).map_err(|_| Error::Budget)?;
        if !lease.grow_to(bytes.capacity()) {
            return Err(Error::Budget);
        }
        for row in self.rows() {
            check(cancel)?;
            for pixel in row.chunks_exact(8) {
                for channel in bt2020(rgb(pixel, gain)) {
                    bytes.extend_from_slice(&pq(channel * 80.0).to_be_bytes());
                }
            }
        }
        check(cancel)?;
        let maximum = if maximum_nits.is_finite() {
            maximum_nits
        } else {
            1000.0
        };
        let maximum = maximum.max(analysis.maximum_nits).clamp(1.0, 10_000.0);
        let minimum = if minimum_nits.is_finite() {
            minimum_nits.clamp(0.0, maximum)
        } else {
            0.0
        };
        Ok(HdrFrame {
            width: self.width,
            height: self.height,
            rgb16_be: Pixels::from_reserved(bytes, lease),
            analysis,
            mastering_minimum_nits: minimum,
            mastering_maximum_nits: maximum,
        })
    }
}
fn bt2020([r, g, b]: [f32; 3]) -> [f32; 3] {
    [
        0.627_404 * r + 0.329_282 * g + 0.043_313_6 * b,
        0.069_097 * r + 0.919_54 * g + 0.011_361_2 * b,
        0.016_391_6 * r + 0.088_013_2 * g + 0.895_595 * b,
    ]
}
fn pq(nits: f32) -> u16 {
    let powered = (nits / 10_000.0).clamp(0.0, 1.0).powf(2610.0 / 16_384.0);
    let value = ((3424.0 / 4096.0 + (2413.0 / 128.0) * powered)
        / (1.0 + (2392.0 / 128.0) * powered))
        .powf(2523.0 / 32.0);
    (value * u16::MAX as f32).round() as u16
}
#[cfg(test)]
#[path = "capture_color_hdr_tests.rs"]
mod tests;
