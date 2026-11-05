//! 独立可见框的栅格标注合成；直接采样到效果源图，不分配额外缩放整图。
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
    for y in 0..height {
        if cancelled() {
            return Err(BlurError::Cancelled);
        }
        let top = (y as f64).max(frame.y);
        let bottom = (y as f64 + 1.0).min(frame.y + frame.height);
        if bottom <= top {
            continue;
        }
        for x in 0..width {
            let left = (x as f64).max(frame.x);
            let right = (x as f64 + 1.0).min(frame.x + frame.width);
            if right <= left {
                continue;
            }
            let coverage = (right - left) * (bottom - top);
            let sx = (((left + right) * 0.5 - frame.x) * aw as f64 / frame.width - 0.5)
                .clamp(0.0, aw as f64 - 1.0);
            let sy = (((top + bottom) * 0.5 - frame.y) * ah as f64 / frame.height - 0.5)
                .clamp(0.0, ah as f64 - 1.0);
            let x0 = sx.floor() as u32;
            let y0 = sy.floor() as u32;
            let fx = sx - x0 as f64;
            let fy = sy - y0 as f64;
            let mut sampled = [0u32; 4];
            for (channel, value) in sampled.iter_mut().enumerate() {
                let at = |x: u32, y: u32| overlay[((y * aw + x) * 4) as usize + channel] as f64;
                let x1 = (x0 + 1).min(aw - 1);
                let y1 = (y0 + 1).min(ah - 1);
                let a = at(x0, y0) * (1.0 - fx) + at(x1, y0) * fx;
                let b = at(x0, y1) * (1.0 - fx) + at(x1, y1) * fx;
                *value = ((a * (1.0 - fy) + b * fy) * coverage)
                    .round()
                    .clamp(0.0, 255.0) as u32;
            }
            let offset = ((y * width + x) * 4) as usize;
            for channel in 0..4 {
                output[offset + channel] = (sampled[channel]
                    + (layers.base[offset + channel] as u32 * (255 - sampled[3]) + 127) / 255)
                    .min(255) as u8;
            }
        }
    }
    if cancelled() {
        return Err(BlurError::Cancelled);
    }
    Ok(Pixels::from_reserved(output, lease))
}

/// 两个独立纹理框共享既有有界投影与合成；不烘焙回活动底图。
pub fn compose_in_frames(
    layers: &Layers,
    base_projection: Option<crate::image_projection::Projection>,
    annotation_frame: Option<Rect>,
    cancelled: impl Fn() -> bool,
) -> Result<Pixels, BlurError> {
    let Some(frame) = base_projection else {
        return compose_in_frame(layers, annotation_frame, cancelled);
    };
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
