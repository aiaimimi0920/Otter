//! Effect sources composite base images/transparent raster annotations with source-over; exclude vector effects to prevent recursive sampling.
pub use crate::raster_source_frame::compose_in_frame;
pub use crate::raster_source_frame::compose_in_frames;
use crate::{
    blur::BlurError,
    erase_layers::Layers,
    pixels::{self, Pixels},
};
pub fn compose(layers: &Layers, cancelled: impl Fn() -> bool) -> Result<Pixels, BlurError> {
    if !layers.valid() {
        return Err(BlurError::InvalidInput);
    }
    if layers.annotation_dimensions() != layers.size {
        return compose_in_frame(
            layers,
            Some(crate::crop_geometry::Rect {
                x: 0.0,
                y: 0.0,
                width: layers.size.0 as f64,
                height: layers.size.1 as f64,
            }),
            cancelled,
        );
    }
    if cancelled() {
        return Err(BlurError::Cancelled);
    }
    let Some(overlay) = layers.annotations.as_ref() else {
        return Ok(layers.base.clone());
    };
    // valid() already binds base-image/same-sized-annotation lengths to pixel dimensions.
    let bytes = layers.base.len();
    let mut lease = pixels::reserve(bytes).ok_or(BlurError::BudgetExceeded)?;
    let mut output = Vec::new();
    output
        .try_reserve_exact(bytes)
        .map_err(|_| BlurError::BudgetExceeded)?;
    if !lease.grow_to(output.capacity()) {
        return Err(BlurError::BudgetExceeded);
    }
    // Performance audit optimization:
    // Copy `layers.base` directly instead of zero-filling 33 MB (`output.resize(bytes, 0)`) and then blending
    // across >95% transparent `[0, 0, 0, 0]` annotation pixels. Checking all 4 bytes (`ov == &[0; 4]`) preserves
    // exact bit-identity (`(base * 255 + 127) / 255 == base` for every `base in 0..=255`), and `ov[3] == 255`
    // copies opaque strokes without division.
    output.extend_from_slice(&layers.base);
    let stride = layers.size.0 as usize * 4;
    for (target_row, overlay_row) in output
        .chunks_exact_mut(stride)
        .zip(overlay.chunks_exact(stride))
    {
        if cancelled() {
            return Err(BlurError::Cancelled);
        }
        if overlay_row.iter().all(|&b| b == 0) {
            continue;
        }
        for (pixel, ov) in target_row
            .chunks_exact_mut(4)
            .zip(overlay_row.chunks_exact(4))
        {
            if ov == [0, 0, 0, 0] {
                continue;
            }
            if ov[3] == 255 {
                pixel.copy_from_slice(ov);
                continue;
            }
            let keep = 255 - u32::from(ov[3]);
            for channel in 0..4 {
                pixel[channel] = (u32::from(ov[channel])
                    + (u32::from(pixel[channel]) * keep + 127) / 255)
                    .min(255) as u8;
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
    fn premultiplied_overlay_composes_all_channels_and_plain_source_stays_shared() {
        let mut layers = Layers {
            annotation_size: None,
            size: (1, 1),
            base: vec![100, 50, 0, 128].into(),
            annotations: None,
        };
        assert!(compose(&layers, || false)
            .unwrap()
            .same_storage(&layers.base));
        layers.annotations = Some(vec![0, 20, 60, 128].into());
        assert_eq!(&compose(&layers, || false).unwrap()[..], &[50, 45, 60, 192]);
        assert_eq!(&layers.base[..], &[100, 50, 0, 128]);
        assert!(matches!(
            compose(&layers, || true),
            Err(BlurError::Cancelled)
        ));
        layers.annotations = Some(vec![0; 8].into());
        assert!(matches!(
            compose(&layers, || false),
            Err(BlurError::InvalidInput)
        ));
    }
}
