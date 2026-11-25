//! Fixed-edge fast path recognizes only pixel-identical contiguous regions at axial ends, not floating/translucent toolbars.
use super::{approximate, ApproximateMatch, Axis, Error, Frame, MatchOptions};
use super::{Composite, Positioned, MAX_FRAMES};
use std::sync::atomic::{AtomicBool, Ordering};

/// origin marks content start; output origin moves backward by leading, retaining each fixed end bar once.
/// Callers supply confirmed bounds. Revalidate fixed bars in all frames and reject changes rather than silently crop.
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
    // Bound total logical input before fixed-bar scanning; repeated references to large shared frames cannot amplify validation work.
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
    /// Without evidence of nonzero content movement, no edge removal is authorized.
    Unresolved(ApproximateMatch),
    Motion {
        leading: u32,
        trailing: u32,
        delta: i64,
        mean_error: f64,
    },
}
pub(super) fn equal_line(a: &Frame, b: &Frame, axis: Axis, line: u32) -> bool {
    equal_lines(a, b, axis, line, line)
}
pub(super) fn equal_lines(a: &Frame, b: &Frame, axis: Axis, line_a: u32, line_b: u32) -> bool {
    match axis {
        Axis::Vertical => {
            let stride = a.width as usize * 4;
            let start_a = line_a as usize * stride;
            let start_b = line_b as usize * stride;
            a.pixels[start_a..start_a + stride] == b.pixels[start_b..start_b + stride]
        }
        Axis::Horizontal => (0..a.height as usize).all(|row| {
            let start_a = (row * a.width as usize + line_a as usize) * 4;
            let start_b = (row * b.width as usize + line_b as usize) * 4;
            a.pixels[start_a..start_a + 4] == b.pixels[start_b..start_b + 4]
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
/// Exclude exactly stationary leading/trailing edges, then borrow interior regions for approximate displacement analysis.
/// Fixed regions are only two-frame evidence; callers still confirm across frames and final composition retains both edges once.
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
