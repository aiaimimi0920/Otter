//! 预乘 BGRA 到逻辑画布的有界投影；透明留白、双线性采样和边界覆盖，不修改源图。
use crate::{
    blur::BlurError,
    crop_geometry::Rect,
    pixels::{self, Pixels},
    viewport::checked_image_bytes,
};

pub fn project(
    source: &Pixels,
    source_size: (u32, u32),
    target_size: (u32, u32),
    placement: Rect,
    cancelled: impl Fn() -> bool,
) -> Result<Pixels, BlurError> {
    project_region(
        source,
        source_size,
        target_size,
        crate::image_projection::Projection {
            source: Rect {
                x: 0.0,
                y: 0.0,
                width: source_size.0 as f64,
                height: source_size.1 as f64,
            },
            target: placement,
        },
        cancelled,
    )
}

/// 独立有界源区到目标区；保留同一像素预算/取消/全图双线性邻域，不构造巨大placement。
pub fn project_region(
    source: &Pixels,
    source_size: (u32, u32),
    target_size: (u32, u32),
    mapping: crate::image_projection::Projection,
    cancelled: impl Fn() -> bool,
) -> Result<Pixels, BlurError> {
    use crate::image_projection::{Projection, Sampling};
    let bytes = checked_image_bytes(target_size.0, target_size.1).ok_or(BlurError::InvalidInput)?;
    if checked_image_bytes(source_size.0, source_size.1) != Some(source.len())
        || [mapping.source, mapping.target].iter().any(|r| {
            ![r.x, r.y, r.width, r.height, r.x + r.width, r.y + r.height]
                .iter()
                .all(|v| v.is_finite() && v.abs() <= 1_000_000.0)
                || r.width <= 0.0
                || r.height <= 0.0
        })
    {
        return Err(BlurError::InvalidInput);
    }
    if cancelled() {
        return Err(BlurError::Cancelled);
    }
    if source_size == target_size
        && mapping.source
            == (Rect {
                x: 0.0,
                y: 0.0,
                width: source_size.0 as f64,
                height: source_size.1 as f64,
            })
        && mapping.target
            == (Rect {
                x: 0.0,
                y: 0.0,
                width: source_size.0 as f64,
                height: source_size.1 as f64,
            })
    {
        return Ok(source.clone());
    }
    let mut lease = pixels::reserve(bytes).ok_or(BlurError::BudgetExceeded)?;
    let mut output = Vec::new();
    output
        .try_reserve_exact(bytes)
        .map_err(|_| BlurError::BudgetExceeded)?;
    if !lease.grow_to(output.capacity()) {
        return Err(BlurError::BudgetExceeded);
    }
    output.resize(bytes, 0);
    if cancelled() {
        return Err(BlurError::Cancelled);
    }
    let projection = Projection::clipped(mapping.source, mapping.target, source_size)
        .ok_or(BlurError::InvalidInput)?;
    let Sampling::Region(projection) = projection else {
        return Ok(Pixels::from_reserved(output, lease));
    };
    let placement = projection.target;
    let region = projection.source;
    let (width, height) = target_size;
    let (sw, sh) = source_size;
    for y in 0..height {
        if cancelled() {
            return Err(BlurError::Cancelled);
        }
        let top = (y as f64).max(placement.y);
        let bottom = (y as f64 + 1.0).min(placement.y + placement.height);
        if bottom <= top {
            continue;
        }
        for x in 0..width {
            let left = (x as f64).max(placement.x);
            let right = (x as f64 + 1.0).min(placement.x + placement.width);
            if right <= left {
                continue;
            }
            let coverage = (right - left) * (bottom - top);
            let sx = (region.x
                + ((left + right) * 0.5 - placement.x) * region.width / placement.width
                - 0.5)
                .clamp(0.0, sw as f64 - 1.0);
            let sy = (region.y
                + ((top + bottom) * 0.5 - placement.y) * region.height / placement.height
                - 0.5)
                .clamp(0.0, sh as f64 - 1.0);
            let (x0, y0) = (sx.floor() as u32, sy.floor() as u32);
            let (x1, y1) = ((x0 + 1).min(sw - 1), (y0 + 1).min(sh - 1));
            let (fx, fy) = (sx - x0 as f64, sy - y0 as f64);
            let offset = ((y * width + x) * 4) as usize;
            for channel in 0..4 {
                let at = |x: u32, y: u32| source[((y * sw + x) * 4) as usize + channel] as f64;
                let a = at(x0, y0) * (1.0 - fx) + at(x1, y0) * fx;
                let b = at(x0, y1) * (1.0 - fx) + at(x1, y1) * fx;
                output[offset + channel] = ((a * (1.0 - fy) + b * fy) * coverage)
                    .round()
                    .clamp(0.0, 255.0) as u8;
            }
        }
    }
    if cancelled() {
        return Err(BlurError::Cancelled);
    }
    Ok(Pixels::from_reserved(output, lease))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identity_shares_storage_and_letterbox_remains_transparent() {
        let source: Pixels = [20, 40, 60, 128].repeat(4).into();
        let full = Rect {
            x: 0.0,
            y: 0.0,
            width: 2.0,
            height: 2.0,
        };
        assert!(project(&source, (2, 2), (2, 2), full, || false)
            .unwrap()
            .same_storage(&source));
        let output = project(&source, (2, 2), (4, 2), Rect { x: 1.0, ..full }, || false).unwrap();
        for row in output.chunks_exact(16) {
            assert_eq!(&row[..4], &[0; 4]);
            assert_eq!(&row[4..12], &[20, 40, 60, 128, 20, 40, 60, 128]);
            assert_eq!(&row[12..], &[0; 4]);
        }
    }
    #[test]
    fn crop_and_fractional_edge_coverage_sample_premultiplied_channels() {
        let source: Pixels = vec![10, 20, 30, 255, 40, 50, 60, 255].into();
        let cropped = project(
            &source,
            (2, 1),
            (1, 1),
            Rect {
                x: -1.0,
                y: 0.0,
                width: 2.0,
                height: 1.0,
            },
            || false,
        )
        .unwrap();
        assert_eq!(&cropped[..], &[40, 50, 60, 255]);
        let half: Pixels = vec![20, 40, 60, 128].into();
        let output = project(
            &half,
            (1, 1),
            (2, 1),
            Rect {
                x: 0.5,
                y: 0.0,
                width: 1.0,
                height: 1.0,
            },
            || false,
        )
        .unwrap();
        assert_eq!(&output[..], &[10, 20, 30, 64, 10, 20, 30, 64]);
    }
    #[test]
    fn cancellation_and_invalid_geometry_preserve_source() {
        let source: Pixels = vec![255; 16].into();
        let rect = Rect {
            x: 1.0,
            y: 0.0,
            width: 2.0,
            height: 2.0,
        };
        let calls = std::cell::Cell::new(0);
        assert!(matches!(
            project(&source, (2, 2), (4, 4), rect, || {
                calls.set(calls.get() + 1);
                calls.get() > 2
            }),
            Err(BlurError::Cancelled)
        ));
        assert!(matches!(
            project(
                &source,
                (2, 2),
                (4, 4),
                Rect {
                    width: f64::NAN,
                    ..rect
                },
                || false
            ),
            Err(BlurError::InvalidInput)
        ));
        assert!(source.iter().all(|value| *value == 255));
    }
}
