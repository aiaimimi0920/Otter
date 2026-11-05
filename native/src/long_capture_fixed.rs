//! 固定边缘快速路径：仅识别轴向两端逐像素不变的连续区域，不推断浮动或半透明工具栏。
use super::{approximate, ApproximateMatch, Axis, Error, Frame, MatchOptions};
use super::{Composite, Positioned, MAX_FRAMES};
use std::sync::atomic::{AtomicBool, Ordering};

/// origin 指内容区起点；输出 origin 向前移动 leading，首尾固定栏各保留一次。
/// 调用方提供已确认边界。此入口重新验证所有帧固定栏，变化时拒绝而非静默裁掉。
pub fn stitch_fixed_edges(
    axis: Axis,
    frames: &[Positioned],
    leading: u32,
    trailing: u32,
    cancel: &AtomicBool,
) -> Result<Composite, Error> {
    check(cancel)?;
    let first = frames.first().ok_or(Error::Empty)?;
    if frames.len() > MAX_FRAMES {
        return Err(Error::FrameLimit);
    }
    let length = match axis {
        Axis::Vertical => first.frame.height,
        Axis::Horizontal => first.frame.width,
    };
    if leading
        .checked_add(trailing)
        .is_none_or(|sum| sum >= length)
    {
        return Err(Error::Dimensions);
    }
    // 先检查逻辑输入总量，再扫描固定栏；共享大帧的重复输入也不能放大验证工作量。
    let mut total = 0u64;
    for input in frames {
        check(cancel)?;
        total += u64::from(input.frame.width) * u64::from(input.frame.height);
        if total > super::MAX_INPUT_PIXELS {
            return Err(Error::InputLimit);
        }
    }
    for input in frames.iter().skip(1) {
        check(cancel)?;
        if first.frame.dimensions() != input.frame.dimensions() {
            return Err(Error::Dimensions);
        }
        for line in (0..leading).chain(length - trailing..length) {
            check(cancel)?;
            if !equal_line(&first.frame, &input.frame, axis, line) {
                return Err(Error::ChangedEdges);
            }
        }
    }
    super::compose::stitch_edges(
        axis,
        frames,
        cancel,
        crate::pixels::reserve,
        (leading, trailing),
    )
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FixedMatch {
    Unchanged,
    /// 未建立非零内容运动证据，不授权删除任何边缘。
    Unresolved(ApproximateMatch),
    Motion {
        leading: u32,
        trailing: u32,
        delta: i64,
        mean_error: f64,
    },
}
pub(super) fn equal_line(a: &Frame, b: &Frame, axis: Axis, line: u32) -> bool {
    match axis {
        Axis::Vertical => {
            let stride = a.width as usize * 4;
            let start = line as usize * stride;
            a.pixels[start..start + stride] == b.pixels[start..start + stride]
        }
        Axis::Horizontal => (0..a.height as usize).all(|row| {
            let start = (row * a.width as usize + line as usize) * 4;
            a.pixels[start..start + 4] == b.pixels[start..start + 4]
        }),
    }
}
fn check(cancel: &AtomicBool) -> Result<(), Error> {
    if cancel.load(Ordering::Acquire) {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}
/// 精确静止的首尾边缘排除后，借用内部区域执行近似位移分析。
/// 固定区域只是两帧证据；调用方仍需跨帧确认，且最终合成须保留一次首尾边缘。
pub fn match_fixed_edges(
    previous: &Frame,
    current: &Frame,
    axis: Axis,
    options: MatchOptions,
    cancel: &AtomicBool,
) -> Result<FixedMatch, Error> {
    check(cancel)?;
    let length = match axis {
        Axis::Vertical => previous.height,
        Axis::Horizontal => previous.width,
    };
    if previous.dimensions() != current.dimensions()
        || options.minimum_overlap == 0
        || options.minimum_overlap > length
        || !options.maximum_mean_error.is_finite()
        || !(0.0..=12.0).contains(&options.maximum_mean_error)
    {
        return Err(Error::Dimensions);
    }
    let mut leading = 0;
    while leading < length {
        check(cancel)?;
        if !equal_line(previous, current, axis, leading) {
            break;
        }
        leading += 1;
    }
    if leading == length {
        return Ok(FixedMatch::Unchanged);
    }
    let mut trailing = 0;
    while trailing < length - leading {
        check(cancel)?;
        if !equal_line(previous, current, axis, length - trailing - 1) {
            break;
        }
        trailing += 1;
    }
    let end = length - trailing;
    if end - leading < options.minimum_overlap {
        return Ok(FixedMatch::Unresolved(ApproximateMatch::NoOverlap));
    }
    let result =
        approximate::match_region(previous, current, axis, options, cancel, (leading, end))?;
    match result {
        ApproximateMatch::Offset { delta, mean_error } if delta != 0 => Ok(FixedMatch::Motion {
            leading,
            trailing,
            delta,
            mean_error,
        }),
        other => Ok(FixedMatch::Unresolved(other)),
    }
}

#[cfg(test)]
#[path = "long_capture_fixed_tests.rs"]
mod tests;
