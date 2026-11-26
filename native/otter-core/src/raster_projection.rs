//! Bounded premultiplied-BGRA projection to logical canvases; transparent padding, bilinear sampling, edge coverage, and immutable sources.
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

/// Independent bounded source-to-target regions retain pixel budgets/cancellation/full-image bilinear neighborhoods, without huge placement construction.
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
    let right_edge = placement.x + placement.width;
    let bottom_edge = placement.y + placement.height;
    let cols = crate::image_projection::covered_pixels(placement.x, right_edge, width);
    let rows = if cols.is_empty() {
        0..0
    } else {
        crate::image_projection::covered_pixels(placement.y, bottom_edge, height)
    };
    // Performance audit optimization:
    // Column sample coordinates `(x0, x1, fx)` and horizontal edge coverage `(right - left)` are invariant across
    // all rows `y`, but IEEE-754 floating-point rules prevented LLVM from hoisting `/ placement.width` (~8.29M `divsd`
    // instructions per 4K frame). Precompute a budgeted column table like `raster_flip_resize`, falling back to inline
    // evaluation if the pixel budget is tight so saves/projections never fail prematurely.
    let col_sample = |x: u32| -> Option<(usize, usize, f64, f64)> {
        let left = (x as f64).max(placement.x);
        let right = (x as f64 + 1.0).min(right_edge);
        if right <= left {
            return None;
        }
        let sx = (region.x + ((left + right) * 0.5 - placement.x) * region.width / placement.width
            - 0.5)
            .clamp(0.0, sw as f64 - 1.0);
        let x0 = sx.floor() as u32;
        let x1 = (x0 + 1).min(sw - 1);
        Some((
            x0 as usize * 4,
            x1 as usize * 4,
            sx - x0 as f64,
            right - left,
        ))
    };
    let table_bytes = cols.len() * std::mem::size_of::<Option<(usize, usize, f64, f64)>>();
    let x_table = (!rows.is_empty() && table_bytes > 0)
        .then(|| {
            let mut table_lease = pixels::reserve(table_bytes)?;
            let mut vec = Vec::new();
            vec.try_reserve_exact(cols.len()).ok()?;
            if !table_lease
                .grow_to(vec.capacity() * std::mem::size_of::<Option<(usize, usize, f64, f64)>>())
            {
                return None;
            }
            for x in cols.clone() {
                vec.push(col_sample(x));
            }
            Some((vec, table_lease))
        })
        .flatten();
    let sw_bytes = sw as usize * 4;
    let out_stride = width as usize * 4;
    for y in rows {
        if cancelled() {
            return Err(BlurError::Cancelled);
        }
        let top = (y as f64).max(placement.y);
        let bottom = (y as f64 + 1.0).min(bottom_edge);
        let height_cov = bottom - top;
        if height_cov <= 0.0 {
            continue;
        }
        let sy = (region.y
            + ((top + bottom) * 0.5 - placement.y) * region.height / placement.height
            - 0.5)
            .clamp(0.0, sh as f64 - 1.0);
        let y0 = sy.floor() as usize;
        let y1 = (y0 + 1).min(sh as usize - 1);
        let fy = sy - y0 as f64;
        let wy0 = 1.0 - fy;
        let row0 = &source[y0 * sw_bytes..(y0 + 1) * sw_bytes];
        let row1 = &source[y1 * sw_bytes..(y1 + 1) * sw_bytes];
        let out_row = &mut output[y as usize * out_stride..(y as usize + 1) * out_stride];
        for (idx, x) in cols.clone().enumerate() {
            let sample = match &x_table {
                Some((vec, _)) => vec[idx],
                None => col_sample(x),
            };
            let Some((x0_b, x1_b, fx, width_cov)) = sample else {
                continue;
            };
            let coverage = width_cov * height_cov;
            let wx0 = 1.0 - fx;
            let p00 = &row0[x0_b..x0_b + 4];
            let p10 = &row0[x1_b..x1_b + 4];
            let p01 = &row1[x0_b..x0_b + 4];
            let p11 = &row1[x1_b..x1_b + 4];
            let dst = &mut out_row[x as usize * 4..x as usize * 4 + 4];
            for channel in 0..4 {
                let a = p00[channel] as f64 * wx0 + p10[channel] as f64 * fx;
                let b = p01[channel] as f64 * wx0 + p11[channel] as f64 * fx;
                dst[channel] = ((a * wy0 + b * fy) * coverage).round().clamp(0.0, 255.0) as u8;
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
    fn clipped_fractional_placement_leaves_uncovered_rows_and_columns_clear() {
        let source: Pixels = vec![80, 120, 160, 200].into();
        let bounds = Rect {
            x: -0.5,
            y: 2.5,
            width: 1.0,
            height: 2.0,
        };
        let output = project(&source, (1, 1), (3, 4), bounds, || false).unwrap();
        for (index, pixel) in output.chunks_exact(4).enumerate() {
            assert_eq!(
                pixel,
                match index {
                    6 => &[20, 30, 40, 50],
                    9 => &[40, 60, 80, 100],
                    _ => &[0; 4],
                }
            );
        }
        let outside = Rect { x: 3.0, ..bounds };
        let output = project(&source, (1, 1), (3, 4), outside, || false).unwrap();
        assert!(output.iter().all(|value| *value == 0));
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
