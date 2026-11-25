//! 恢复会话的底图与标注使用同一逻辑坐标系；保持图层独立，避免失去独立擦除能力。
use super::*;
use crate::{crop_geometry::Size, erase_layers::Layers};
impl Images {
    pub fn prepare_editor_layers(
        &self,
        frame: Size,
        cancel: &AtomicBool,
    ) -> Result<Option<Layers>> {
        self.prepare_layers(frame, true, false, cancel)
    }

    /// 完整裁剪源框不是 HWND；沿用文档的单图限制，投影仍受全局像素租约约束。
    pub fn prepare_full_crop_layers(
        &self,
        crop: crate::crop_geometry::Crop,
        rasterized_source: bool,
        cancel: &AtomicBool,
    ) -> Result<Option<Layers>> {
        self.prepare_layers(crop.source, false, rasterized_source, cancel)
    }

    pub fn prepare_rasterized_full_layers(
        &self,
        frame: Size,
        cancel: &AtomicBool,
    ) -> Result<Option<Layers>> {
        check(cancel)?;
        // The full window keeps the normal restore limits. Only pixel density
        // may differ, and the stored PNG is already admitted on its own grid.
        let size = (frame.width.ceil() as u32, frame.height.ceil() as u32);
        if !frame.width.is_finite()
            || !frame.height.is_finite()
            || frame.width <= 0.
            || frame.height <= 0.
            || size.0 > 8192
            || size.1 > 8192
            || u64::from(size.0) * u64::from(size.1) > 16_777_216
            || self.source.is_none()
        {
            return Err(Error::new(
                E_INVALIDARG,
                "Invalid restored full working grid",
            ));
        }
        self.raster_layers()
    }

    fn prepare_layers(
        &self,
        frame: Size,
        window_aligned: bool,
        rasterized_source: bool,
        cancel: &AtomicBool,
    ) -> Result<Option<Layers>> {
        check(cancel)?;
        let size = if window_aligned {
            (frame.width.ceil() as u32, frame.height.ceil() as u32)
        } else {
            frame
                .crop_raster_size()
                .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid restored editor frame"))?
        };
        if !frame.width.is_finite()
            || !frame.height.is_finite()
            || frame.width <= 0.0
            || frame.height <= 0.0
            || (window_aligned
                && (size.0 > 8192
                    || size.1 > 8192
                    || u64::from(size.0) * u64::from(size.1) > 16_777_216))
            || crate::viewport::checked_image_bytes(size.0, size.1).is_none()
        {
            return Err(Error::new(E_INVALIDARG, "Invalid restored editor frame"));
        }
        let Some(source) = self.source.as_ref() else {
            if self.rasterized_annotations.is_some() || rasterized_source {
                return Err(Error::new(
                    E_INVALIDARG,
                    "Restored annotation layer has no base frame",
                ));
            }
            return Ok(None);
        };
        let width = source.width as f64;
        let height = source.height as f64;
        let (grid_w, grid_h) = (f64::from(size.0), f64::from(size.1));
        let placement = if rasterized_source {
            // The previously saved grid already includes contain padding. Resample
            // the entire frame, not a new contain fit that narrows it on every reopen.
            crate::crop_geometry::Rect {
                x: 0.,
                y: 0.,
                width: grid_w,
                height: grid_h,
            }
        } else {
            let scale = (frame.width / width).min(frame.height / height);
            let (mut x, mut y) = (
                (frame.width - width * scale) / 2.,
                (frame.height - height * scale) / 2.,
            );
            let (mut w, mut h) = (width * scale, height * scale);
            // Contain in logical space first; rounding the raster dimensions
            // must not change the letterbox or the image aspect on screen.
            if !window_aligned && (frame.width.ceil(), frame.height.ceil()) != (grid_w, grid_h) {
                x *= grid_w / frame.width;
                w *= grid_w / frame.width;
                y *= grid_h / frame.height;
                h *= grid_h / frame.height;
            }
            crate::crop_geometry::Rect {
                x,
                y,
                width: w,
                height: h,
            }
        };
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
    fn crop_keeps_full_base_and_independent_visible_overlay() {
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
        let crop = Crop {
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
        };
        let cancel = AtomicBool::new(false);
        let layers = images
            .prepare_full_crop_layers(crop, false, &cancel)
            .unwrap()
            .unwrap();
        assert_eq!(layers.size, (8, 4));
        assert!(layers
            .base
            .same_storage(&images.source.as_ref().unwrap().pixels));
        let annotations = layers.annotations.as_ref().unwrap();
        assert!(annotations.same_storage(&images.rasterized_annotations.as_ref().unwrap().pixels));
        assert_eq!(layers.annotation_dimensions(), (2, 2));
        assert!(layers.valid());
        cancel.store(true, Ordering::Release);
        assert_eq!(
            images
                .prepare_full_crop_layers(crop, false, &cancel)
                .err()
                .unwrap()
                .code(),
            E_ABORT
        );
    }
}
