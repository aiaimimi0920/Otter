//! 未裁剪会话的栅格与矢量使用同一逻辑坐标框；不合成标注，避免失去独立擦除能力。
use super::*;
use crate::{crop_geometry::Size, erase_layers::Layers};
impl Images {
    /// 保留完整底图；主干保存的栅格标注属于可见框，需单独放回完整源框中的裁剪位置。
    pub fn prepare_cropped_editor_layers(
        &self,
        restored: crate::session_crop::Restored,
        cancel: &AtomicBool,
    ) -> Result<Option<Layers>> {
        let crop = restored
            .editor_crop()
            .map_err(|_| Error::new(E_INVALIDARG, "Invalid restored editor crop"))?;
        let Some(mut layers) = self.prepare_editor_layers(crop.source, cancel)? else {
            return Ok(None);
        };
        if let Some(annotation) = &self.rasterized_annotations {
            let projected = crate::raster_projection::project(
                &annotation.pixels,
                (annotation.width, annotation.height),
                layers.size,
                crop.rect,
                || cancel.load(Ordering::Acquire),
            )
            .map_err(|error| {
                Error::new(
                    match error {
                        crate::blur::BlurError::Cancelled => E_ABORT,
                        crate::blur::BlurError::BudgetExceeded => E_OUTOFMEMORY,
                        _ => E_INVALIDARG,
                    },
                    "Cannot prepare restored crop annotation",
                )
            })?;
            layers.annotations = Some(projected);
            layers.annotation_size = Some(layers.size);
        }
        check(cancel)?;
        Ok(Some(layers))
    }
    pub fn prepare_editor_layers(
        &self,
        frame: Size,
        cancel: &AtomicBool,
    ) -> Result<Option<Layers>> {
        check(cancel)?;
        if !frame.width.is_finite()
            || !frame.height.is_finite()
            || frame.width <= 0.0
            || frame.height <= 0.0
            || frame.width > 8192.0
            || frame.height > 8192.0
            || frame.width.ceil() * frame.height.ceil() > 16_777_216.0
        {
            return Err(Error::new(E_INVALIDARG, "Invalid restored editor frame"));
        }
        let Some(source) = self.source.as_ref() else {
            if self.rasterized_annotations.is_some() {
                return Err(Error::new(
                    E_INVALIDARG,
                    "Restored annotation layer has no base frame",
                ));
            }
            return Ok(None);
        };
        let width = source.width as f64;
        let height = source.height as f64;
        let scale = (frame.width / width).min(frame.height / height);
        let placement = crate::crop_geometry::Rect {
            x: (frame.width - width * scale) / 2.0,
            y: (frame.height - height * scale) / 2.0,
            width: width * scale,
            height: height * scale,
        };
        let size = (frame.width.ceil() as u32, frame.height.ceil() as u32);
        let base = crate::raster_projection::project(
            &source.pixels,
            (source.width, source.height),
            size,
            placement,
            || cancel.load(Ordering::Acquire),
        )
        .map_err(|error| {
            Error::new(
                match error {
                    crate::blur::BlurError::Cancelled => E_ABORT,
                    crate::blur::BlurError::BudgetExceeded => E_OUTOFMEMORY,
                    _ => E_INVALIDARG,
                },
                "Cannot prepare restored editor base",
            )
        })?;
        let layers = Layers {
            size,
            base,
            annotation_size: self
                .rasterized_annotations
                .as_ref()
                .map(|image| (image.width, image.height)),
            annotations: self
                .rasterized_annotations
                .as_ref()
                .map(|image| image.pixels.clone()),
        };
        if !layers.valid() {
            return Err(Error::new(
                E_INVALIDARG,
                "Invalid restored annotation layer",
            ));
        }
        check(cancel)?;
        Ok(Some(layers))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crop_geometry::{Crop, Rect};

    #[test]
    fn crop_keeps_full_base_and_places_annotation_only_in_visible_rectangle() {
        let images = Images {
            sticker: Default::default(),
            accept_upstream: true,
            capture_original: None,
            unit: UnitId::parse("u").unwrap(),
            source: Some(Image {
                width: 8,
                height: 4,
                pixels: vec![255; 128].into(),
            }),
            preview: Some(Image {
                width: 2,
                height: 2,
                pixels: vec![0; 16].into(),
            }),
            rasterized_annotations: Some(Image {
                width: 2,
                height: 2,
                pixels: vec![128; 16].into(),
            }),
            restored_preview_locked: false,
        };
        let restored = crate::session_crop::Restored {
            frame: Rect {
                x: -200.0,
                y: 300.0,
                width: 2.0,
                height: 2.0,
            },
            crop: Crop {
                rect: Rect {
                    x: 3.0,
                    y: 1.0,
                    width: 2.0,
                    height: 2.0,
                },
                source: Size {
                    width: 8.0,
                    height: 4.0,
                },
            },
            flipped_x: false,
            flipped_y: false,
        };
        let cancel = AtomicBool::new(false);
        let layers = images
            .prepare_cropped_editor_layers(restored, &cancel)
            .unwrap()
            .unwrap();
        assert_eq!(layers.size, (8, 4));
        assert!(layers
            .base
            .same_storage(&images.source.as_ref().unwrap().pixels));
        let annotations = layers.annotations.as_ref().unwrap();
        for y in 0..4 {
            for x in 0..8 {
                let value = if (3..5).contains(&x) && (1..3).contains(&y) {
                    128
                } else {
                    0
                };
                assert_eq!(
                    &annotations[(y * 8 + x) * 4..(y * 8 + x + 1) * 4],
                    &[value; 4]
                );
            }
        }
        assert!(layers.valid());
        assert_eq!(
            images.rasterized_annotations.as_ref().unwrap().pixels.len(),
            16
        );
        cancel.store(true, Ordering::Release);
        assert_eq!(
            images
                .prepare_cropped_editor_layers(restored, &cancel)
                .err()
                .unwrap()
                .code(),
            E_ABORT
        );
    }
}
