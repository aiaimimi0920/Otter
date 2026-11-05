//! 绘制目标私有的有界 LRU；不能跨 D2D 目标复用，目标销毁时一起释放。
use crate::viewport::Point;
use std::collections::VecDeque;
use windows::{
    core::{Error, Result},
    Win32::{
        Foundation::E_INVALIDARG,
        Graphics::{
            Direct2D::{Common::*, *},
            Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM,
        },
    },
};

const CAPACITY: usize = 8;
#[derive(Clone, Copy, PartialEq, Eq)]
struct Key {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    cell: u32,
}
#[derive(Default)]
pub(super) struct Cache {
    entries: VecDeque<(Key, ID2D1Bitmap)>,
    #[cfg(test)]
    pub(super) uploads: usize,
}
impl Cache {
    pub(super) unsafe fn bitmap(
        &mut self,
        target: &ID2D1RenderTarget,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        cell: f32,
    ) -> Result<ID2D1Bitmap> {
        if width == 0
            || height == 0
            || width > 256
            || height > 256
            || !cell.is_finite()
            || cell <= 0.0
            || cell > 1_000_000.0
        {
            return Err(Error::new(E_INVALIDARG, "Invalid mosaic cache tile"));
        }
        // 网格生成最终使用 round/max(2)，等价尺寸共享位图但模型仍保留小数。
        let cell = cell.round().max(2.0) as u32;
        let key = Key {
            x,
            y,
            width,
            height,
            cell,
        };
        if let Some(index) = self.entries.iter().position(|(stored, _)| *stored == key) {
            let entry = self.entries.remove(index).unwrap();
            let bitmap = entry.1.clone();
            self.entries.push_back(entry);
            return Ok(bitmap);
        }
        // 先释放最旧应用引用，再分配新块；失败不缓存半成品。
        if self.entries.len() == CAPACITY {
            self.entries.pop_front();
        }
        let pixels = crate::mosaic::generate(
            width,
            height,
            cell as f32,
            Point {
                x: x as f32,
                y: y as f32,
            },
        )
        .map_err(|e| Error::new(E_INVALIDARG, format!("Mosaic texture rejected: {e:?}")))?;
        let bitmap = target.CreateBitmap(
            D2D_SIZE_U { width, height },
            Some(pixels.as_ptr().cast()),
            width * 4,
            &D2D1_BITMAP_PROPERTIES {
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                dpiX: 96.0,
                dpiY: 96.0,
            },
        )?;
        self.entries.push_back((key, bitmap.clone()));
        #[cfg(test)]
        {
            self.uploads += 1;
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
    #[test]
    fn repeated_tile_reuses_bitmap_and_lru_is_bounded() {
        let _apartment = super::super::Apartment::new().unwrap();
        unsafe {
            let wic: IWICImagingFactory =
                CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap();
            let backing = wic
                .CreateBitmap(32, 32, &GUID_WICPixelFormat32bppPBGRA, WICBitmapCacheOnLoad)
                .unwrap();
            let factory: ID2D1Factory =
                D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).unwrap();
            let target = factory
                .CreateWicBitmapRenderTarget(&backing, &D2D1_RENDER_TARGET_PROPERTIES::default())
                .unwrap();
            let mut cache = Cache::default();
            let first = cache.bitmap(&target, 0, 0, 32, 32, 12.0).unwrap();
            assert_eq!(
                cache.bitmap(&target, 0, 0, 32, 32, 12.0).unwrap().as_raw(),
                first.as_raw()
            );
            for x in 1..8 {
                cache.bitmap(&target, x * 32, 0, 32, 32, 12.0).unwrap();
            }
            cache.bitmap(&target, 0, 0, 32, 32, 12.0).unwrap();
            cache.bitmap(&target, 256, 0, 32, 32, 12.0).unwrap();
            assert_eq!(cache.entries.len(), 8);
            assert!(cache.entries.iter().all(|(key, _)| key.x != 32));
            assert_eq!(
                cache.bitmap(&target, 0, 0, 32, 32, 12.0).unwrap().as_raw(),
                first.as_raw()
            );
            assert!(cache.bitmap(&target, 0, 0, 257, 32, 12.0).is_err());
            assert_eq!(cache.entries.len(), 8);
            let other_cell = cache.bitmap(&target, 0, 0, 32, 32, 24.0).unwrap();
            assert_ne!(other_cell.as_raw(), first.as_raw());
            assert_eq!(
                cache.bitmap(&target, 0, 0, 32, 32, 24.0).unwrap().as_raw(),
                other_cell.as_raw()
            );
            assert_eq!(
                cache.bitmap(&target, 0, 0, 32, 32, 23.75).unwrap().as_raw(),
                other_cell.as_raw()
            );
            assert!(cache.bitmap(&target, 0, 0, 32, 32, f32::NAN).is_err());
            // 不同尺寸不能误用旧纹理；失败调用也不污染合法键。
            assert_ne!(
                cache.bitmap(&target, 0, 0, 16, 16, 12.0).unwrap().as_raw(),
                first.as_raw()
            );
        }
    }
}
