//! 从恢复图片准备逻辑可见框的效果源图；不混入矢量效果，避免递归模糊。
use super::*;
use crate::crop_geometry::{self, Crop, Rect, Size};
impl Images {
    pub fn prepare_full_working_effect_source(
        &self,
        frame: Size,
        cancel: &AtomicBool,
    ) -> Result<Image> {
        let layers = self
            .prepare_rasterized_full_layers(frame, cancel)?
            .ok_or_else(|| Error::new(E_INVALIDARG, "Missing full working effect source"))?;
        let pixels = crate::raster_source::compose_in_frames(
            &layers,
            frame.raster_density(layers.size),
            Some(Rect {
                x: 0.,
                y: 0.,
                width: frame.width,
                height: frame.height,
            }),
            || cancel.load(Ordering::Acquire),
        )
        .map_err(failure)?;
        check(cancel)?;
        Ok(Image {
            width: layers.size.0,
            height: layers.size.1,
            pixels,
        })
    }

    /// 裁剪文档的编辑效果使用完整源框，而不是仅可见区域；重置裁剪时框外纹理仍然存在。
    pub fn prepare_cropped_effect_source(
        &self,
        restored: crate::session_crop::Restored,
        cancel: &AtomicBool,
    ) -> Result<Image> {
        let crop = restored
            .editor_crop()
            .map_err(|_| Error::new(E_INVALIDARG, "Invalid restored editor crop"))?;
        let layers = self
            .prepare_full_crop_layers(crop, restored.rasterized_source, cancel)?
            .ok_or_else(|| Error::new(E_INVALIDARG, "Restored crop requires full image source"))?;
        // 直接按可见框采样独立标注，避免额外分配并遍历一张完整源图尺寸的透明工作纹理。
        let pixels = crate::raster_source::compose_in_frames(
            &layers,
            crop.source.raster_density(layers.size),
            Some(crop.rect),
            || cancel.load(Ordering::Acquire),
        )
        .map_err(failure)?;
        check(cancel)?;
        Ok(Image {
            width: layers.size.0,
            height: layers.size.1,
            pixels,
        })
    }
    /// frame 只取正常展开后的逻辑尺寸，屏幕位置不得进入像素采样。
    /// 底图已应用持久翻转；标注纹理已在保存时翻转，不能再次镜像。
    pub fn prepare_effect_source(
        &self,
        frame: Size,
        crop: Option<Crop>,
        cancel: &AtomicBool,
    ) -> Result<Image> {
        check(cancel)?;
        if !frame.width.is_finite()
            || !frame.height.is_finite()
            || frame.width <= 0.0
            || frame.height <= 0.0
            || frame.width > 8192.0
            || frame.height > 8192.0
            || frame.width.ceil() * frame.height.ceil() > 16_777_216.0
        {
            return Err(Error::new(E_INVALIDARG, "Invalid restored effect frame"));
        }
        let source = self
            .source
            .as_ref()
            .or(self.preview.as_ref())
            .ok_or_else(|| Error::new(E_INVALIDARG, "Restored effect requires image source"))?;
        let pixels = Size {
            width: source.width as f64,
            height: source.height as f64,
        };
        let local = Rect {
            x: 0.0,
            y: 0.0,
            width: frame.width,
            height: frame.height,
        };
        let placement = if let Some(crop) = crop {
            crop_geometry::image_placement(local, crop, pixels)
                .map_err(|_| Error::new(E_INVALIDARG, "Invalid restored effect crop"))?
        } else {
            let scale = (frame.width / pixels.width).min(frame.height / pixels.height);
            Rect {
                x: (frame.width - pixels.width * scale) / 2.0,
                y: (frame.height - pixels.height * scale) / 2.0,
                width: pixels.width * scale,
                height: pixels.height * scale,
            }
        };
        let size = (frame.width.ceil() as u32, frame.height.ceil() as u32);
        let base = crate::raster_projection::project(
            &source.pixels,
            (source.width, source.height),
            size,
            placement,
            || cancel.load(Ordering::Acquire),
        )
        .map_err(failure)?;
        let layers = crate::erase_layers::Layers {
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
        let pixels = crate::raster_source::compose_in_frame(&layers, Some(local), || {
            cancel.load(Ordering::Acquire)
        })
        .map_err(failure)?;
        check(cancel)?;
        Ok(Image {
            width: size.0,
            height: size.1,
            pixels,
        })
    }
}
fn failure(error: crate::blur::BlurError) -> Error {
    let code = match error {
        crate::blur::BlurError::Cancelled => E_ABORT,
        crate::blur::BlurError::BudgetExceeded => E_OUTOFMEMORY,
        _ => E_INVALIDARG,
    };
    Error::new(code, "Cannot project restored effect source")
}
#[cfg(test)]
mod tests {
    use super::*;

    fn fractional_crop() -> (Images, crate::session_crop::Restored) {
        let images = Images {
            sticker: Default::default(),
            accept_upstream: true,
            capture_original: None,
            unit: UnitId::parse("crop").unwrap(),
            source: Some(Image {
                width: 8,
                height: 4,
                pixels: [40, 60, 80, 255].repeat(32).into(),
            }),
            preview: None,
            rasterized_annotations: Some(Image {
                width: 2,
                height: 2,
                pixels: vec![30, 0, 0, 128, 0, 60, 0, 128, 0, 0, 80, 128, 0, 0, 0, 0].into(),
            }),
            restored_preview_locked: false,
        };
        let rect = Rect {
            x: 2.25,
            y: 0.5,
            width: 3.5,
            height: 2.75,
        };
        let restored = crate::session_crop::Restored {
            frame: Rect {
                x: -200.0,
                y: 300.0,
                ..rect
            },
            crop: Crop {
                rect,
                source: Size {
                    width: 8.0,
                    height: 4.0,
                },
            },
            flipped_x: false,
            flipped_y: false,
            rasterized_source: false,
        };
        (images, restored)
    }

    #[test]
    fn cropped_effect_matches_two_pass_pixels_at_fractional_edges() {
        let (images, restored) = fractional_crop();
        let cancel = AtomicBool::new(false);
        let geometry = restored.editor_crop().unwrap();
        let mut layers = images
            .prepare_full_crop_layers(geometry, false, &cancel)
            .unwrap()
            .unwrap();
        let pixels = crate::raster_projection::project(
            layers.annotations.as_ref().unwrap(),
            layers.annotation_dimensions(),
            layers.size,
            geometry.rect,
            || false,
        )
        .unwrap();
        layers.annotations = Some(pixels);
        layers.annotation_size = Some(layers.size);
        let two_pass = crate::raster_source::compose(&layers, || false).unwrap();
        let output = images
            .prepare_cropped_effect_source(restored, &cancel)
            .unwrap();
        assert_eq!(output.pixels, two_pass);
        assert_eq!(&output.pixels[..4], &[40, 60, 80, 255]);
        assert_eq!(
            images
                .prepare_cropped_effect_source(restored, &AtomicBool::new(true))
                .err()
                .unwrap()
                .code(),
            E_ABORT
        );
    }

    #[test]
    #[ignore = "must run alone because it reserves the process-wide pixel budget"]
    fn cropped_effect_needs_only_one_full_size_output_lease() {
        use crate::pixels::{reserve, DECODED_PIXEL_LIMIT};
        let (images, restored) = fractional_crop();
        let base = images.source.as_ref().unwrap().pixels.len();
        let overlay = images.rasterized_annotations.as_ref().unwrap().pixels.len();
        let pressure = reserve(DECODED_PIXEL_LIMIT - base - overlay - base).unwrap();
        let cancel = AtomicBool::new(false);
        let output = images
            .prepare_cropped_effect_source(restored, &cancel)
            .unwrap();
        assert_eq!(output.pixels.len(), base);
        assert!(reserve(1).is_none());
        drop(output);
        assert!(reserve(base).is_some());
        drop(images);
        drop(pressure);
        assert!(reserve(DECODED_PIXEL_LIMIT).is_some());
    }

    #[test]
    fn restored_effect_source_combines_contain_margins_and_independent_overlay() {
        let images = Images {
            sticker: Default::default(),
            accept_upstream: true,
            capture_original: None,
            unit: UnitId::parse("u").unwrap(),
            source: Some(Image {
                width: 1,
                height: 1,
                pixels: vec![100, 0, 0, 255].into(),
            }),
            preview: None,
            rasterized_annotations: Some(Image {
                width: 1,
                height: 1,
                pixels: vec![0, 20, 60, 128].into(),
            }),
            restored_preview_locked: false,
        };
        let cancel = AtomicBool::new(false);
        let output = images
            .prepare_effect_source(
                Size {
                    width: 3.0,
                    height: 1.0,
                },
                None,
                &cancel,
            )
            .unwrap();
        assert_eq!(
            &output.pixels[..],
            &[0, 20, 60, 128, 50, 20, 60, 255, 0, 20, 60, 128]
        );
        assert_eq!(
            &images.source.as_ref().unwrap().pixels[..],
            &[100, 0, 0, 255]
        );
        assert!(images
            .prepare_effect_source(
                Size {
                    width: 3.0,
                    height: 1.0
                },
                None,
                &AtomicBool::new(true)
            )
            .is_err());
    }
}
