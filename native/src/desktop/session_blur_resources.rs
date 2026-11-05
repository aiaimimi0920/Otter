//! 单个恢复窗口的惰性模糊资源缓存；源图只投影一次，精确强度共享，不跨窗口复用。
use super::{image::Image, session_prepare::Images};
use crate::{
    blur::{self, BlurError, Region},
    blur_image::BlurImage,
    crop_geometry::{Crop, Size},
    session_vectors::Error,
};
use std::{
    collections::HashMap,
    sync::atomic::{AtomicBool, Ordering},
};
pub(super) struct Resources<'a> {
    images: Option<&'a Images>,
    frame: Size,
    crop: Option<Crop>,
    editor_crop: Option<crate::session_crop::Restored>,
    cancel: &'a AtomicBool,
    source: Option<Image>,
    cache: HashMap<u32, BlurImage>,
}
impl<'a> Resources<'a> {
    pub(super) fn new(
        images: Option<&'a Images>,
        frame: Size,
        crop: Option<Crop>,
        cancel: &'a AtomicBool,
    ) -> Self {
        Self {
            images,
            frame,
            crop,
            editor_crop: None,
            cancel,
            source: None,
            cache: HashMap::new(),
        }
    }
    /// 完整源坐标的编辑器恢复入口；保留惰性分配，窗口中没有模糊标注则不生成效果图。
    pub(super) fn for_cropped_editor(
        images: Option<&'a Images>,
        restored: crate::session_crop::Restored,
        cancel: &'a AtomicBool,
    ) -> Self {
        let mut resources = Self::new(images, restored.crop.source, None, cancel);
        resources.editor_crop = Some(restored);
        resources
    }
    pub(super) fn get(&mut self, strength: f32) -> Result<BlurImage, Error> {
        if self.cancel.load(Ordering::Acquire) {
            return Err(Error::Cancelled);
        }
        if !strength.is_finite() || !(1.0..=64.0).contains(&strength) {
            return Err(Error::Style);
        }
        let key = strength.to_bits();
        if let Some(image) = self.cache.get(&key) {
            return Ok(image.clone());
        }
        if self.source.is_none() {
            self.source = Some(
                {
                    let images = self.images.ok_or(Error::Geometry)?;
                    if let Some(restored) = self.editor_crop {
                        images.prepare_cropped_effect_source(restored, self.cancel)
                    } else {
                        images.prepare_effect_source(self.frame, self.crop, self.cancel)
                    }
                }
                .map_err(|error| {
                    use windows::Win32::Foundation::{E_ABORT, E_OUTOFMEMORY};
                    if error.code() == E_ABORT {
                        Error::Cancelled
                    } else if error.code() == E_OUTOFMEMORY {
                        Error::Budget
                    } else {
                        Error::Geometry
                    }
                })?,
            );
        }
        let source = self.source.as_ref().expect("prepared source");
        let pixels = blur::region_precise(
            &source.pixels,
            (source.width, source.height),
            Region {
                x: 0,
                y: 0,
                width: source.width,
                height: source.height,
            },
            strength,
            || self.cancel.load(Ordering::Acquire),
        )
        .map_err(|error| match error {
            BlurError::Cancelled => Error::Cancelled,
            BlurError::BudgetExceeded => Error::Budget,
            _ => Error::Geometry,
        })?;
        let image = BlurImage::with_strength(source.width, source.height, pixels, strength)
            .ok_or(Error::Geometry)?;
        if self.cancel.load(Ordering::Acquire) {
            return Err(Error::Cancelled);
        }
        self.cache.try_reserve(1).map_err(|_| Error::Budget)?;
        self.cache.insert(key, image.clone());
        Ok(image)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cropped_editor_effect_retains_outside_source_and_offsets_overlay_once() {
        use crate::crop_geometry::Rect;
        let images = Images {
            sticker: Default::default(),
            accept_upstream: true,
            capture_original: None,
            unit: crate::workspace_id::UnitId::parse("crop").unwrap(),
            source: Some(Image {
                width: 8,
                height: 4,
                pixels: [40, 0, 0, 255].repeat(32).into(),
            }),
            preview: Some(Image {
                width: 2,
                height: 2,
                pixels: vec![0; 16].into(),
            }),
            rasterized_annotations: Some(Image {
                width: 2,
                height: 2,
                pixels: [0, 30, 0, 128].repeat(4).into(),
            }),
            restored_preview_locked: false,
        };
        let restored = crate::session_crop::Restored {
            frame: Rect {
                x: 900.0,
                y: -100.0,
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
        let mut resources = Resources::for_cropped_editor(Some(&images), restored, &cancel);
        assert!(resources.source.is_none());
        let first = resources.get(2.5).unwrap();
        assert_eq!((first.width(), first.height()), (8, 4));
        let source = resources.source.as_ref().unwrap();
        for y in 0..4 {
            for x in 0..8 {
                let expected = if (3..5).contains(&x) && (1..3).contains(&y) {
                    [20, 30, 0, 255]
                } else {
                    [40, 0, 0, 255]
                };
                assert_eq!(
                    &source.pixels[(y * 8 + x) * 4..(y * 8 + x + 1) * 4],
                    &expected
                );
            }
        }
        let expected = blur::region_precise(
            &source.pixels,
            (8, 4),
            Region {
                x: 0,
                y: 0,
                width: 8,
                height: 4,
            },
            2.5,
            || false,
        )
        .unwrap();
        assert_eq!(&first.pixels()[..], &expected[..]);
        assert!(first
            .pixels()
            .same_storage(resources.get(2.5).unwrap().pixels()));
        assert_eq!(
            &images.source.as_ref().unwrap().pixels[..4],
            &[40, 0, 0, 255]
        );
        cancel.store(true, Ordering::Release);
        assert!(matches!(resources.get(2.5), Err(Error::Cancelled)));
    }
    #[test]
    fn cropped_editor_effect_does_not_treat_preview_as_full_source() {
        let images = Images {
            sticker: Default::default(),
            accept_upstream: true,
            capture_original: None,
            unit: crate::workspace_id::UnitId::parse("crop").unwrap(),
            source: None,
            preview: Some(Image {
                width: 2,
                height: 2,
                pixels: vec![255; 16].into(),
            }),
            rasterized_annotations: None,
            restored_preview_locked: false,
        };
        let text = r#"{"id":"crop","x":0,"y":0,"w":2,"h":2,"imageEditState":{"cropRect":{"x":1,"y":1,"w":2,"h":2},"sourceSize":{"w":4,"h":4}}}"#;
        let window = serde_json::from_str(text).unwrap();
        let restored = crate::session_crop::restore(&window).unwrap().unwrap();
        let cancel = AtomicBool::new(false);
        let mut resources = Resources::for_cropped_editor(Some(&images), restored, &cancel);
        assert!(matches!(resources.get(2.0), Err(Error::Geometry)));
        assert!(resources.source.is_none());
        assert!(resources.cache.is_empty());
    }
    #[test]
    fn cache_is_lazy_exact_strength_and_cancellation_aware() {
        let images = Images {
            sticker: Default::default(),
            accept_upstream: true,
            capture_original: None,
            unit: crate::workspace_id::UnitId::parse("u").unwrap(),
            source: Some(Image {
                width: 2,
                height: 2,
                pixels: vec![255; 16].into(),
            }),
            preview: None,
            rasterized_annotations: None,
            restored_preview_locked: false,
        };
        let cancel = AtomicBool::new(false);
        let mut resources = Resources::new(
            Some(&images),
            Size {
                width: 4.0,
                height: 2.0,
            },
            None,
            &cancel,
        );
        assert!(resources.source.is_none());
        let first = resources.get(4.4).unwrap();
        let again = resources.get(4.4).unwrap();
        assert!(first.pixels().same_storage(again.pixels()));
        let changed = resources.get(4.5).unwrap();
        assert!(!first.pixels().same_storage(changed.pixels()));
        assert_eq!((first.width(), first.height()), (4, 2));
        assert_eq!(resources.cache.len(), 2);
        cancel.store(true, Ordering::Release);
        assert!(matches!(resources.get(4.4), Err(Error::Cancelled)));
    }
}
