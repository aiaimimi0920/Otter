//! Verify shared regions for known viewport displacement, then analyze residual axial page scrolling in the same borrowed region.
use super::{view::View, *};
use std::sync::atomic::Ordering;
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SpatialMatch {
    Offset { x: i64, y: i64, mean_error: f64 },
    Rejected(ApproximateMatch),
}
/// movement is physical viewport displacement, not unverified image coordinates. Success still changes no session state.
/// Recheck known-displacement shared regions pixelwise; residual scrolling reuses bounded sampling without claiming exact per-pixel proof.
pub fn match_spatial(
    previous: &Frame,
    current: &Frame,
    movement: (i64, i64),
    axis: Axis,
    options: MatchOptions,
    cancel: &AtomicBool,
) -> Result<SpatialMatch, Error> {
    check(cancel)?;
    if previous.dimensions() != current.dimensions()
        || options.minimum_overlap == 0
        || !options.maximum_mean_error.is_finite()
        || !(0.0..=12.0).contains(&options.maximum_mean_error)
    {
        return Err(Error::Dimensions);
    }
    let (dx, dy) = movement;
    // Ordinary scrolling after switching retains 1D exact matching; low gradient texture alone cannot reject uniquely exact displacement.
    if movement == (0, 0) {
        match match_exact(previous, current, axis, options.minimum_overlap, cancel)? {
            Match::Duplicate => {
                return Ok(SpatialMatch::Offset {
                    x: 0,
                    y: 0,
                    mean_error: 0.0,
                });
            }
            Match::Offset(delta) if delta.unsigned_abs() <= u64::from(options.maximum_shift) => {
                let (x, y) = match axis {
                    Axis::Vertical => (0, delta),
                    Axis::Horizontal => (delta, 0),
                };
                return Ok(SpatialMatch::Offset {
                    x,
                    y,
                    mean_error: 0.0,
                });
            }
            _ => {}
        }
    }
    let width = u64::from(previous.width)
        .checked_sub(dx.unsigned_abs())
        .filter(|n| *n > 0);
    let height = u64::from(previous.height)
        .checked_sub(dy.unsigned_abs())
        .filter(|n| *n > 0);
    let (Some(width), Some(height)) = (width, height) else {
        return Ok(SpatialMatch::Rejected(ApproximateMatch::NoOverlap));
    };
    let a = View::new(
        previous,
        dx.max(0) as u32,
        dy.max(0) as u32,
        width as u32,
        height as u32,
    )?;
    let b = View::new(
        current,
        (-dx).max(0) as u32,
        (-dy).max(0) as u32,
        width as u32,
        height as u32,
    )?;
    if options.minimum_overlap > a.dimensions(axis).0 {
        return Ok(SpatialMatch::Rejected(ApproximateMatch::NoOverlap));
    }
    let ceiling = options.maximum_mean_error.min(3.0);
    let error = overlap_error(a, b, ceiling, cancel)?;
    // Known movement requires no texture; same-color content can validate but cannot imply unknown residual displacement.
    if error <= ceiling {
        return Ok(SpatialMatch::Offset {
            x: dx,
            y: dy,
            mean_error: error,
        });
    }
    match approximate::match_views(a, b, axis, options, cancel)? {
        ApproximateMatch::Offset { delta, mean_error } => {
            let (x, y) = match axis {
                Axis::Vertical => (dx, dy.checked_add(delta).ok_or(Error::OutputLimit)?),
                Axis::Horizontal => (dx.checked_add(delta).ok_or(Error::OutputLimit)?, dy),
            };
            Ok(SpatialMatch::Offset { x, y, mean_error })
        }
        rejected => Ok(SpatialMatch::Rejected(rejected)),
    }
}
fn check(cancel: &AtomicBool) -> Result<(), Error> {
    if cancel.load(Ordering::Acquire) {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}
fn overlap_error(
    a: View<'_>,
    b: View<'_>,
    ceiling: f64,
    cancel: &AtomicBool,
) -> Result<f64, Error> {
    let count = (u64::from(a.width) * u64::from(a.height) * 3) as f64;
    let mut total = 0u64;
    for y in 0..a.height {
        check(cancel)?;
        for x in 0..a.width {
            let pa = a.pixel(Axis::Vertical, y, x);
            let pb = b.pixel(Axis::Vertical, y, x);
            // Match mainline RGB captures; alpha differences are not page motion.
            total += (0..3)
                .map(|c| u64::from(pa[c].abs_diff(pb[c])))
                .sum::<u64>();
        }
        // Full-region denominators bound minimum error; exceeding thresholds directly enters residual-scrolling analysis.
        // Only failed candidates terminate early; accepted mean_error still includes all RGB pixels.
        if total as f64 / count > ceiling {
            check(cancel)?;
            return Ok(f64::INFINITY);
        }
    }
    check(cancel)?;
    Ok(total as f64 / count)
}
#[cfg(test)]
#[path = "long_capture_spatial_match_tests.rs"]
mod tests;
