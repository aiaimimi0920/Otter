//! 每个 D2D 绘制目标独占效果缓存；跨目标/设备不可复用，生命周期随 Target。
use crate::blur_image::BlurImage;
use std::collections::VecDeque;
use windows::{
    core::{Error, Result},
    Win32::Foundation::E_INVALIDARG,
    Win32::Graphics::{
        Direct2D::{Common::*, *},
        Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM,
    },
};

const BLUR_CACHE_BYTES: usize = 32 * 1024 * 1024;
const BLUR_CACHE_ENTRIES: usize = 32;
#[derive(Default)]
pub(super) struct Cache {
    pub(super) mosaic: super::mosaic_cache::Cache,
    blur: VecDeque<(BlurImage, ID2D1Bitmap, [u32; 4])>,
    blur_bytes: usize,
    #[cfg(test)]
    pub(super) blur_uploads: usize,
}
fn same_image(a: &BlurImage, b: &BlurImage) -> bool {
    a.width() == b.width() && a.height() == b.height() && a.pixels().same_storage(b.pixels())
}
impl Cache {
    pub(super) fn retain_tools(&mut self, blur: bool, mosaic: bool) {
        if !blur {
            self.blur.clear();
            self.blur_bytes = 0;
        }
        if !mosaic {
            self.mosaic = Default::default();
        }
    }

    /// 仅保留当前合成文档（含草稿/替换预览）引用的快照；历史不能撑住 GPU 缓存。
    pub(super) fn retain_blurs<'a>(&mut self, images: impl Iterator<Item = &'a BlurImage> + Clone) {
        self.blur
            .retain(|(stored, _, _)| images.clone().any(|image| same_image(stored, image)));
        self.blur_bytes = self
            .blur
            .iter()
            .map(|(_, _, rect)| rect[2] as usize * rect[3] as usize * 4)
            .sum();
    }

    pub(super) unsafe fn blur(
        &mut self,
        target: &ID2D1RenderTarget,
        image: &BlurImage,
    ) -> Result<ID2D1Bitmap> {
        self.blur_region(target, image, [0, 0, image.width(), image.height()])
    }

    /// 以源图行跨度直接上传子区，不复制 CPU 像素；调用期间 image 保持存活。
    pub(super) unsafe fn blur_region(
        &mut self,
        target: &ID2D1RenderTarget,
        image: &BlurImage,
        rect: [u32; 4],
    ) -> Result<ID2D1Bitmap> {
        let [x, y, width, height] = rect;
        if width == 0
            || height == 0
            || x.checked_add(width).is_none_or(|end| end > image.width())
            || y.checked_add(height).is_none_or(|end| end > image.height())
        {
            return Err(Error::new(E_INVALIDARG, "Invalid blur cache region"));
        }
        if let Some(index) = self
            .blur
            .iter()
            .position(|(stored, _, region)| *region == rect && same_image(stored, image))
        {
            let entry = self.blur.remove(index).unwrap();
            let bitmap = entry.1.clone();
            self.blur.push_back(entry);
            return Ok(bitmap);
        }
        let bytes = width as usize * height as usize * 4;
        // 先淘汰再上传；大于预算的单图临时绘制，不挤掉其他可复用的小图。
        if bytes <= BLUR_CACHE_BYTES {
            while self.blur_bytes + bytes > BLUR_CACHE_BYTES
                || self.blur.len() >= BLUR_CACHE_ENTRIES
            {
                if let Some((_, _, removed)) = self.blur.pop_front() {
                    self.blur_bytes -= removed[2] as usize * removed[3] as usize * 4;
                }
            }
        }
        let bitmap = target.CreateBitmap(
            D2D_SIZE_U { width, height },
            Some(
                image
                    .pixels()
                    .as_ptr()
                    .add((y as usize * image.width() as usize + x as usize) * 4)
                    .cast(),
            ),
            image.width() * 4,
            &D2D1_BITMAP_PROPERTIES {
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                dpiX: 96.0,
                dpiY: 96.0,
            },
        )?;
        if bytes <= BLUR_CACHE_BYTES {
            self.blur.push_back((image.clone(), bitmap.clone(), rect));
            self.blur_bytes += bytes;
        }
        #[cfg(test)]
        {
            self.blur_uploads += 1;
        }
        Ok(bitmap)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::{
        core::Interface,
        Win32::{Graphics::Imaging::*, System::Com::*},
    };
    unsafe fn target() -> ID2D1RenderTarget {
        let wic: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap();
        let backing = wic
            .CreateBitmap(32, 32, &GUID_WICPixelFormat32bppPBGRA, WICBitmapCacheOnLoad)
            .unwrap();
        let factory: ID2D1Factory =
            D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).unwrap();
        factory
            .CreateWicBitmapRenderTarget(&backing, &D2D1_RENDER_TARGET_PROPERTIES::default())
            .unwrap()
    }

    #[test]
    fn repeated_composition_reuses_multiple_snapshots_and_removal_prunes_one() {
        let _apartment = super::super::Apartment::new().unwrap();
        unsafe {
            let target = target();
            let brush = target
                .CreateSolidColorBrush(&D2D1_COLOR_F::default(), None)
                .unwrap();
            let mut editor = crate::editor::Editor::default();
            for value in [64, 192] {
                let image = BlurImage::new(32, 32, vec![value; 32 * 32 * 4].into(), 8).unwrap();
                editor
                    .begin_blur(crate::viewport::Point { x: 16.0, y: 16.0 }, image)
                    .unwrap();
                assert!(editor.commit());
            }
            let mut cache = Cache::default();
            for _ in 0..3 {
                target.BeginDraw();
                super::super::annotations::draw_committed(
                    &target,
                    &brush,
                    &editor,
                    crate::viewport::Viewport {
                        origin: crate::viewport::Point::default(),
                        scale: 1.0,
                    },
                    &mut cache,
                )
                .unwrap();
                target.EndDraw(None, None).unwrap();
            }
            assert_eq!(cache.blur_uploads, 2);
            assert_eq!(cache.blur.len(), 2);
            assert!(editor.remove(0));
            target.BeginDraw();
            super::super::annotations::draw_committed(
                &target,
                &brush,
                &editor,
                crate::viewport::Viewport {
                    origin: crate::viewport::Point::default(),
                    scale: 1.0,
                },
                &mut cache,
            )
            .unwrap();
            target.EndDraw(None, None).unwrap();
            assert_eq!(cache.blur_uploads, 2);
            assert_eq!(cache.blur.len(), 1);
            assert_eq!(cache.blur_bytes, 32 * 32 * 4);
        }
    }

    #[test]
    fn blur_lru_enforces_total_bytes_and_touches_recent_entries() {
        let _apartment = super::super::Apartment::new().unwrap();
        unsafe {
            let target = target();
            let mut cache = Cache::default();
            let images: Vec<_> = (0..5)
                .map(|_| BlurImage::new(2048, 1024, vec![0; 2048 * 1024 * 4].into(), 8).unwrap())
                .collect();
            for image in &images[..4] {
                cache.blur(&target, image).unwrap();
            }
            assert_eq!(cache.blur_bytes, BLUR_CACHE_BYTES);
            cache.blur(&target, &images[0]).unwrap();
            cache.blur(&target, &images[4]).unwrap();
            assert_eq!(cache.blur.len(), 4);
            assert_eq!(cache.blur_bytes, BLUR_CACHE_BYTES);
            assert!(cache
                .blur
                .iter()
                .all(|(stored, _, _)| !same_image(stored, &images[1])));
            assert!(cache
                .blur
                .iter()
                .any(|(stored, _, _)| same_image(stored, &images[0])));
            assert_eq!(cache.blur_uploads, 5);
            cache.retain_blurs(std::iter::empty());
            assert_eq!(cache.blur_bytes, 0);
            assert!(cache.blur.is_empty());
        }
    }

    #[test]
    fn tiny_blur_images_are_also_bounded_by_entry_count() {
        let _apartment = super::super::Apartment::new().unwrap();
        unsafe {
            let target = target();
            let mut cache = Cache::default();
            let first = BlurImage::new(1, 1, vec![0; 4].into(), 8).unwrap();
            let weak = first.pixels().downgrade();
            cache.blur(&target, &first).unwrap();
            drop(first);
            for _ in 0..BLUR_CACHE_ENTRIES {
                let image = BlurImage::new(1, 1, vec![0; 4].into(), 8).unwrap();
                cache.blur(&target, &image).unwrap();
            }
            assert!(weak.upgrade().is_none());
            assert_eq!(cache.blur.len(), BLUR_CACHE_ENTRIES);
            assert_eq!(cache.blur_bytes, BLUR_CACHE_ENTRIES * 4);
            cache.retain_tools(false, true);
            assert_eq!(cache.blur_bytes, 0);
        }
    }
    #[test]
    fn blur_bitmap_reuses_shared_snapshot_but_not_equal_bytes_from_another_image() {
        let _apartment = super::super::Apartment::new().unwrap();
        unsafe {
            let wic: IWICImagingFactory =
                CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap();
            let backing = wic
                .CreateBitmap(16, 16, &GUID_WICPixelFormat32bppPBGRA, WICBitmapCacheOnLoad)
                .unwrap();
            let factory: ID2D1Factory =
                D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).unwrap();
            let target = factory
                .CreateWicBitmapRenderTarget(&backing, &D2D1_RENDER_TARGET_PROPERTIES::default())
                .unwrap();
            let image = BlurImage::new(16, 16, vec![0; 16 * 16 * 4].into(), 8).unwrap();
            let weak = image.pixels().downgrade();
            let mut cache = Cache::default();
            let first = cache.blur(&target, &image).unwrap();
            assert_eq!(
                cache.blur(&target, &image.clone()).unwrap().as_raw(),
                first.as_raw()
            );
            drop(image);
            assert!(weak.upgrade().is_some());
            let replacement = BlurImage::new(16, 16, vec![0; 16 * 16 * 4].into(), 8).unwrap();
            assert_ne!(
                cache.blur(&target, &replacement).unwrap().as_raw(),
                first.as_raw()
            );
            assert!(weak.upgrade().is_some());
            cache.retain_blurs(std::iter::once(&replacement));
            assert!(weak.upgrade().is_none());
            let replacement_weak = replacement.pixels().downgrade();
            drop(replacement);
            cache.retain_tools(false, false);
            assert!(replacement_weak.upgrade().is_none());
            drop(cache);
            assert!(replacement_weak.upgrade().is_none());
        }
    }
}
