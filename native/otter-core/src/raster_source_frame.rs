//! Raster-annotation composition with independent visible boxes; sample directly into effect sources without additional full scaled images.
use crate::{
    blur::BlurError,
    crop_geometry::Rect,
    erase_layers::Layers,
    pixels::{self, Pixels},
    viewport::checked_image_bytes,
};

pub fn compose_in_frame(
    layers: &Layers,
    frame: Option<Rect>,
    cancelled: impl Fn() -> bool,
) -> Result<Pixels, BlurError> {
    let Some(frame) = frame else {
        return super::raster_source::compose(layers, cancelled);
    };
    let bytes = checked_image_bytes(layers.size.0, layers.size.1).ok_or(BlurError::InvalidInput)?;
    if !layers.valid()
        || !frame.x.is_finite()
        || !frame.y.is_finite()
        || frame.x.abs() > 1_000_000.0
        || frame.y.abs() > 1_000_000.0
        || !frame.width.is_finite()
        || !frame.height.is_finite()
        || frame.width <= 0.0
        || frame.height <= 0.0
        || frame.width > 1_000_000.0
        || frame.height > 1_000_000.0
    {
        return Err(BlurError::InvalidInput);
    }
    if cancelled() {
        return Err(BlurError::Cancelled);
    }
    let Some(overlay) = layers.annotations.as_ref() else {
        return Ok(layers.base.clone());
    };
    if layers.annotation_dimensions() == layers.size
        && frame
            == (Rect {
                x: 0.0,
                y: 0.0,
                width: layers.size.0 as f64,
                height: layers.size.1 as f64,
            })
    {
        return super::raster_source::compose(layers, cancelled);
    }
    let mut lease = pixels::reserve(bytes).ok_or(BlurError::BudgetExceeded)?;
    let mut output = Vec::new();
    output
        .try_reserve_exact(bytes)
        .map_err(|_| BlurError::BudgetExceeded)?;
    if !lease.grow_to(output.capacity()) {
        return Err(BlurError::BudgetExceeded);
    }
    output.extend_from_slice(&layers.base);
    let (width, height) = layers.size;
    let (aw, ah) = layers.annotation_dimensions();
    let right_edge = frame.x + frame.width;
    let bottom_edge = frame.y + frame.height;
    let cols = crate::image_projection::covered_pixels(frame.x, right_edge, width);
    let rows = if cols.is_empty() {
        0..0
    } else {
        crate::image_projection::covered_pixels(frame.y, bottom_edge, height)
    };
    // Performance audit optimization:
    // Precompute row-invariant column sample coordinates `(x0_b, x1_b, fx, right - left)` once under a temporary
    // pixel-budget lease (with inline fallback on budget pressure) instead of executing `/ frame.width` per pixel.
    let col_sample = |x: u32| -> Option<(usize, usize, f64, f64)> {
        let left = (x as f64).max(frame.x);
        let right = (x as f64 + 1.0).min(right_edge);
        if right <= left {
            return None;
        }
        let sx = (((left + right) * 0.5 - frame.x) * aw as f64 / frame.width - 0.5)
            .clamp(0.0, aw as f64 - 1.0);
        let x0 = sx.floor() as u32;
        let x1 = (x0 + 1).min(aw - 1);
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
    let aw_bytes = aw as usize * 4;
    let out_stride = width as usize * 4;
    for y in rows {
        if cancelled() {
            return Err(BlurError::Cancelled);
        }
        let top = (y as f64).max(frame.y);
        let bottom = (y as f64 + 1.0).min(bottom_edge);
        let height_cov = bottom - top;
        if height_cov <= 0.0 {
            continue;
        }
        let sy = (((top + bottom) * 0.5 - frame.y) * ah as f64 / frame.height - 0.5)
            .clamp(0.0, ah as f64 - 1.0);
        let y0 = sy.floor() as usize;
        let y1 = (y0 + 1).min(ah as usize - 1);
        let fy = sy - y0 as f64;
        let wy0 = 1.0 - fy;
        let row0 = &overlay[y0 * aw_bytes..(y0 + 1) * aw_bytes];
        let row1 = &overlay[y1 * aw_bytes..(y1 + 1) * aw_bytes];
        let base_row = &layers.base[y as usize * out_stride..(y as usize + 1) * out_stride];
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
            let mut sampled = [0u32; 4];
            for (channel, value) in sampled.iter_mut().enumerate() {
                let a = p00[channel] as f64 * wx0 + p10[channel] as f64 * fx;
                let b = p01[channel] as f64 * wx0 + p11[channel] as f64 * fx;
                *value = ((a * wy0 + b * fy) * coverage).round().clamp(0.0, 255.0) as u32;
            }
            if sampled == [0; 4] {
                continue;
            }
            let px_off = x as usize * 4;
            let keep = 255 - sampled[3];
            for channel in 0..4 {
                out_row[px_off + channel] = (sampled[channel]
                    + (base_row[px_off + channel] as u32 * keep + 127) / 255)
                    .min(255) as u8;
            }
        }
    }
    if cancelled() {
        return Err(BlurError::Cancelled);
    }
    Ok(Pixels::from_reserved(output, lease))
}

/// Two independent texture boxes share existing bounded projection/composition, without baking into active base images.
pub fn compose_in_frames(
    layers: &Layers,
    base_projection: Option<crate::image_projection::Projection>,
    annotation_frame: Option<Rect>,
    cancelled: impl Fn() -> bool,
) -> Result<Pixels, BlurError> {
    let Some(frame) = base_projection else {
        return compose_in_frame(layers, annotation_frame, cancelled);
    };
    if let Some((x, y)) = frame.density_for(layers.size) {
        let annotation_frame = annotation_frame.map(|r| Rect {
            x: r.x * x,
            y: r.y * y,
            width: r.width * x,
            height: r.height * y,
        });
        return compose_in_frame(layers, annotation_frame, cancelled);
    }
    if !layers.valid() {
        return Err(BlurError::InvalidInput);
    }
    let base = crate::raster_projection::project_region(
        &layers.base,
        layers.size,
        layers.size,
        frame,
        &cancelled,
    )?;
    compose_in_frame(
        &Layers {
            base,
            ..layers.clone()
        },
        annotation_frame,
        cancelled,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn offset_frame_fits_entire_overlay_and_keeps_outside_base() {
        let layers = Layers {
            annotation_size: None,
            size: (8, 4),
            base: [100, 0, 0, 255].repeat(32).into(),
            annotations: Some(
                (0..32)
                    .flat_map(|i| {
                        if i % 8 < 4 {
                            [0, 0, 128, 128]
                        } else {
                            [0, 128, 0, 128]
                        }
                    })
                    .collect::<Vec<_>>()
                    .into(),
            ),
        };
        let result = compose_in_frame(
            &layers,
            Some(Rect {
                x: 4.0,
                y: 1.0,
                width: 4.0,
                height: 2.0,
            }),
            || false,
        )
        .unwrap();
        assert_eq!(&result[0..4], &[100, 0, 0, 255]);
        assert_eq!(&result[(8 + 4) * 4..(8 + 5) * 4], &[50, 0, 128, 255]);
        assert_eq!(&result[(8 + 7) * 4..(8 + 8) * 4], &[50, 128, 0, 255]);
    }
    #[test]
    fn fractional_frame_clipped_at_canvas_edges_preserves_outside_base() {
        let layers = Layers {
            annotation_size: Some((1, 1)),
            size: (3, 4),
            base: [80, 40, 0, 255].repeat(12).into(),
            annotations: Some(vec![0, 0, 200, 200].into()),
        };
        let frame = Rect {
            x: -0.5,
            y: 2.5,
            width: 1.0,
            height: 2.0,
        };
        let output = compose_in_frame(&layers, Some(frame), || false).unwrap();
        for (index, pixel) in output.chunks_exact(4).enumerate() {
            assert_eq!(
                pixel,
                match index {
                    6 => &[64, 32, 50, 255],
                    9 => &[49, 24, 100, 255],
                    _ => &[80, 40, 0, 255],
                }
            );
        }
        let outside = Rect { y: -2.0, ..frame };
        let output = compose_in_frame(&layers, Some(outside), || false).unwrap();
        assert_eq!(&output[..], &layers.base[..]);
    }

    #[test]
    fn fractional_edges_use_coverage_and_cancel_is_atomic() {
        let layers = Layers {
            annotation_size: None,
            size: (2, 2),
            base: vec![0; 16].into(),
            annotations: Some(vec![255; 16].into()),
        };
        let frame = Some(Rect {
            x: 0.5,
            y: 0.5,
            width: 1.0,
            height: 1.0,
        });
        assert_eq!(
            &compose_in_frame(&layers, frame, || false).unwrap()[..],
            &[64; 16]
        );
        assert!(matches!(
            compose_in_frame(&layers, frame, || true),
            Err(BlurError::Cancelled)
        ));
        assert_eq!(&layers.base[..], &[0; 16]);
    }
}
