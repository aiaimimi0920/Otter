//! 效果源图为底图与透明栅格标注的 source-over 合成；不包含矢量效果，避免递归采样。
pub use crate::raster_source_frame::compose_in_frame;
pub use crate::raster_source_frame::compose_in_frames;
use crate::{
    blur::BlurError,
    erase_layers::Layers,
    pixels::{self, Pixels},
    viewport::checked_image_bytes,
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
    let bytes = checked_image_bytes(layers.size.0, layers.size.1).ok_or(BlurError::InvalidInput)?;
    if layers.base.len() != bytes
        || layers
            .annotations
            .as_ref()
            .is_some_and(|p| p.len() != bytes)
    {
        return Err(BlurError::InvalidInput);
    }
    if cancelled() {
        return Err(BlurError::Cancelled);
    }
    let Some(overlay) = layers.annotations.as_ref() else {
        return Ok(layers.base.clone());
    };
    let mut lease = pixels::reserve(bytes).ok_or(BlurError::BudgetExceeded)?;
    let mut output = Vec::new();
    output
        .try_reserve_exact(bytes)
        .map_err(|_| BlurError::BudgetExceeded)?;
    if !lease.grow_to(output.capacity()) {
        return Err(BlurError::BudgetExceeded);
    }
    output.resize(bytes, 0);
    let stride = layers.size.0 as usize * 4;
    for (row, target) in output.chunks_exact_mut(stride).enumerate() {
        if cancelled() {
            return Err(BlurError::Cancelled);
        }
        let start = row * stride;
        for (column, pixel) in target.chunks_exact_mut(4).enumerate() {
            let index = start + column * 4;
            let keep = 255 - overlay[index + 3] as u32;
            for channel in 0..4 {
                pixel[channel] = (overlay[index + channel] as u32
                    + (layers.base[index + channel] as u32 * keep + 127) / 255)
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
