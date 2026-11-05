//! 已知取景框位移先验证共同区域，再在同一借用区域分析页面的剩余轴向滚动。
use super::{view::View, *};
use std::sync::atomic::Ordering;
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SpatialMatch {
    Offset { x: i64, y: i64, mean_error: f64 },
    Rejected(ApproximateMatch),
}
/// movement 是物理取景框位移，不是未经验证的图像坐标。成功仍不改变任何会话状态。
/// 已知位移的共同区域逐像素复核；剩余滚动复用有界采样分析，不宣称逐像素精确证明。
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
    // 切换后普通滚动仍应保留一维精确匹配能力；不能仅因渐变纹理低就拒绝唯一精确位移。
    if movement == (0, 0) {
        if let Match::Offset(delta) =
            match_exact(previous, current, axis, options.minimum_overlap, cancel)?
        {
            if delta.unsigned_abs() <= u64::from(options.maximum_shift) {
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
    let error = overlap_error(a, b, cancel)?;
    // 已知移动不要求纹理；同色内容仍能验证，不能据此推断一个未知残差位移。
    if error <= options.maximum_mean_error.min(3.0) {
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
fn overlap_error(a: View<'_>, b: View<'_>, cancel: &AtomicBool) -> Result<f64, Error> {
    let mut total = 0u64;
    for y in 0..a.height {
        check(cancel)?;
        for x in 0..a.width {
            let pa = a.pixel(Axis::Vertical, y, x);
            let pb = b.pixel(Axis::Vertical, y, x);
            // 与主干 RGB 截图一致，不把 alpha 差异当作页面运动。
            total += (0..3)
                .map(|c| u64::from(pa[c].abs_diff(pb[c])))
                .sum::<u64>();
        }
    }
    check(cancel)?;
    Ok(total as f64 / (u64::from(a.width) * u64::from(a.height) * 3) as f64)
}
#[cfg(test)]
#[path = "long_capture_spatial_match_tests.rs"]
mod tests;
