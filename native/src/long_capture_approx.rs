//! 有界近似平移分析；稀疏搜索后密集复核，不能代替固定栏/动态区域分析。
use super::view::View;
use super::{Axis, Error, Frame};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Copy, Debug)]
pub struct MatchOptions {
    pub minimum_overlap: u32,
    pub maximum_shift: u32,
    /// BGRA 各通道平均绝对误差上限，限定 0..=12，避免任意放宽导致误拼。
    pub maximum_mean_error: f64,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ApproximateMatch {
    Offset { delta: i64, mean_error: f64 },
    Ambiguous,
    LowTexture,
    NoOverlap,
}
#[derive(Clone, Copy)]
struct Score {
    error: f64,
    texture: f64,
}
fn check(cancel: &AtomicBool) -> Result<(), Error> {
    if cancel.load(Ordering::Acquire) {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}
fn dimensions(frame: &Frame, axis: Axis) -> (u32, u32) {
    match axis {
        Axis::Vertical => (frame.height, frame.width),
        Axis::Horizontal => (frame.width, frame.height),
    }
}
fn position(index: u32, count: u32, length: u32) -> u32 {
    if count <= 1 {
        0
    } else {
        index * (length - 1) / (count - 1)
    }
}
fn score(
    previous: View<'_>,
    current: View<'_>,
    axis: Axis,
    delta: i64,
    samples: (u32, u32),
    cancel: &AtomicBool,
) -> Result<Score, Error> {
    let (length, cross) = previous.dimensions(axis);
    let shift = delta.unsigned_abs() as u32;
    let overlap = length - shift;
    let rows = overlap.min(samples.0);
    let columns = cross.min(samples.1);
    let start_a = if delta > 0 { shift } else { 0 };
    let start_b = if delta < 0 { shift } else { 0 };
    let mut error = 0u64;
    let mut texture = 0u64;
    for row in 0..rows {
        check(cancel)?;
        let along = position(row, rows, overlap);
        for column in 0..columns {
            let x = position(column, columns, cross);
            let a = previous.pixel(axis, start_a + along, x);
            let b = current.pixel(axis, start_b + along, x);
            for channel in 0..4 {
                error += a[channel].abs_diff(b[channel]) as u64;
            }
            // 两帧都必须有局部纹理；不把两张不同纯色图解释成移动。
            if overlap > 1 {
                let next = if along + 1 < overlap {
                    along + 1
                } else {
                    along - 1
                };
                let an = previous.pixel(axis, start_a + next, x);
                let bn = current.pixel(axis, start_b + next, x);
                let ta: u64 = (0..3).map(|c| a[c].abs_diff(an[c]) as u64).sum();
                let tb: u64 = (0..3).map(|c| b[c].abs_diff(bn[c]) as u64).sum();
                texture += ta.min(tb);
            }
        }
    }
    let count = u64::from(rows) * u64::from(columns);
    Ok(Score {
        error: error as f64 / (count * 4) as f64,
        texture: texture as f64 / (count * 3) as f64,
    })
}

/// 搜索调用方限定的双向位移（包括零位移）；时间上界为 65535 个候选，各采样至多 256 点。
/// 无图像副本或候选数组；成功结果仍是采样推断，调用方不得当作逐像素精确匹配证明。
pub fn match_approximate(
    previous: &Frame,
    current: &Frame,
    axis: Axis,
    options: MatchOptions,
    cancel: &AtomicBool,
) -> Result<ApproximateMatch, Error> {
    match_region(
        previous,
        current,
        axis,
        options,
        cancel,
        (0, dimensions(previous, axis).0),
    )
}

// 借用原始帧的轴向区域，不产生裁剪像素副本；固定栏分析与全帧分析共用评分规则。
pub(super) fn match_region(
    previous: &Frame,
    current: &Frame,
    axis: Axis,
    options: MatchOptions,
    cancel: &AtomicBool,
    region: (u32, u32),
) -> Result<ApproximateMatch, Error> {
    check(cancel)?;
    let (frame_length, _) = dimensions(previous, axis);
    if region.0 >= region.1 || region.1 > frame_length {
        return Err(Error::Dimensions);
    }
    let (x, y, width, height) = match axis {
        Axis::Vertical => (0, region.0, previous.width, region.1 - region.0),
        Axis::Horizontal => (region.0, 0, region.1 - region.0, previous.height),
    };
    if previous.dimensions() != current.dimensions() {
        return Err(Error::Dimensions);
    }
    match_views(
        View::new(previous, x, y, width, height)?,
        View::new(current, x, y, width, height)?,
        axis,
        options,
        cancel,
    )
}
pub(super) fn match_views(
    previous: View<'_>,
    current: View<'_>,
    axis: Axis,
    options: MatchOptions,
    cancel: &AtomicBool,
) -> Result<ApproximateMatch, Error> {
    check(cancel)?;
    let (length, _) = previous.dimensions(axis);
    if previous.dimensions(axis) != current.dimensions(axis)
        || options.minimum_overlap == 0
        || options.minimum_overlap > length
        || !options.maximum_mean_error.is_finite()
        || !(0.0..=12.0).contains(&options.maximum_mean_error)
    {
        return Err(Error::Dimensions);
    }
    let limit = options.maximum_shift.min(length - options.minimum_overlap) as i64;
    let mut best: Option<(i64, Score)> = None;
    let mut second = f64::INFINITY;
    for delta in -limit..=limit {
        let candidate = score(previous, current, axis, delta, (16, 16), cancel)?;
        if candidate.texture < 4.0 {
            continue;
        }
        if best.is_none_or(|(_, old)| candidate.error < old.error) {
            if let Some((_, old)) = best {
                second = second.min(old.error);
            }
            best = Some((delta, candidate));
        } else {
            second = second.min(candidate.error);
        }
    }
    let Some((delta, candidate)) = best else {
        return Ok(ApproximateMatch::LowTexture);
    };
    if candidate.error > options.maximum_mean_error {
        return Ok(ApproximateMatch::NoOverlap);
    }
    if second <= candidate.error + 2.0 {
        return Ok(ApproximateMatch::Ambiguous);
    }
    let verified = score(previous, current, axis, delta, (128, 192), cancel)?;
    if verified.texture < 4.0 {
        return Ok(ApproximateMatch::LowTexture);
    }
    if verified.error > options.maximum_mean_error {
        return Ok(ApproximateMatch::NoOverlap);
    }
    Ok(ApproximateMatch::Offset {
        delta,
        mean_error: verified.error,
    })
}

#[cfg(test)]
#[path = "long_capture_approx_tests.rs"]
mod tests;
